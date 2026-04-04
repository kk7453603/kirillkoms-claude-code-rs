use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{AgentId, Result, MultiAgentError};

/// IPC protocol for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Agent registration
    Register {
        agent_id: AgentId,
        pid: u32,
        capabilities: Vec<String>,
    },
    /// Agent deregistration
    Unregister {
        agent_id: AgentId,
    },
    /// Send data to agent
    Data {
        from: AgentId,
        to: AgentId,
        payload: Vec<u8>,
    },
    /// Request/response pattern
    Request {
        id: Uuid,
        from: AgentId,
        to: AgentId,
        method: String,
        params: serde_json::Value,
    },
    Response {
        id: Uuid,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    /// Health check
    Ping {
        from: AgentId,
    },
    Pong {
        from: AgentId,
    },
    /// Shutdown signal
    Shutdown,
}

/// IPC server for the control plane
pub struct IpcServer {
    socket_path: PathBuf,
    connections: Arc<RwLock<HashMap<AgentId, IpcConnection>>>,
    message_handler: Arc<dyn IpcMessageHandler>,
}

/// IPC connection to an agent
pub struct IpcConnection {
    agent_id: AgentId,
    stream: UnixStream,
    tx: mpsc::Sender<IpcMessage>,
    rx: mpsc::Receiver<IpcMessage>,
}

/// Handler for IPC messages
#[async_trait::async_trait]
pub trait IpcMessageHandler: Send + Sync {
    async fn handle_message(&self, message: IpcMessage, from: AgentId) -> Option<IpcMessage>;
}

/// IPC client for agents
pub struct IpcClient {
    agent_id: AgentId,
    socket_path: PathBuf,
    stream: Option<UnixStream>,
    tx: mpsc::Sender<IpcMessage>,
    rx: mpsc::Receiver<IpcMessage>,
}

use std::collections::HashMap;

impl IpcServer {
    pub fn new(
        socket_path: PathBuf,
        message_handler: Arc<dyn IpcMessageHandler>,
    ) -> Self {
        Self {
            socket_path,
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handler,
        }
    }
    
    /// Start the IPC server
    pub async fn start(&self) -> Result<()> {
        // Remove existing socket file if it exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        
        let listener = UnixListener::bind(&self.socket_path)?;
        info!("IPC server listening on {:?}", self.socket_path);
        
        let connections = self.connections.clone();
        let handler = self.message_handler.clone();
        
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        Self::handle_connection(stream, connections.clone(), handler.clone()).await;
                    }
                    Err(e) => {
                        error!("Failed to accept IPC connection: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Send a message to a specific agent
    pub async fn send_to(&self, agent_id: AgentId, message: IpcMessage) -> Result<()> {
        let connections = self.connections.read().await;
        
        let connection = connections.get(&agent_id)
            .ok_or(MultiAgentError::AgentNotFound { id: agent_id })?;
        
        connection.tx.send(message).await
            .map_err(|_| MultiAgentError::ChannelSendError)?;
        
        Ok(())
    }
    
    /// Broadcast a message to all connected agents
    pub async fn broadcast(&self, message: IpcMessage) -> Result<()> {
        let connections = self.connections.read().await;
        
        for (_, connection) in connections.iter() {
            let _ = connection.tx.send(message.clone()).await;
        }
        
        Ok(())
    }
    
    async fn handle_connection(
        mut stream: UnixStream,
        connections: Arc<RwLock<HashMap<AgentId, IpcConnection>>>,
        handler: Arc<dyn IpcMessageHandler>,
    ) {
        let (tx, mut rx) = mpsc::channel(100);
        let mut agent_id: Option<AgentId> = None;
        
        // Read messages from stream
        let mut read_stream = stream.try_clone().unwrap();
        let connections_clone = connections.clone();
        let handler_clone = handler.clone();
        
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 4096];
            
            loop {
                match read_stream.read(&mut buffer).await {
                    Ok(0) => {
                        // Connection closed
                        if let Some(id) = agent_id {
                            connections_clone.write().await.remove(&id);
                            info!("Agent {} disconnected", id);
                        }
                        break;
                    }
                    Ok(n) => {
                        // Parse message
                        match bincode::deserialize::<IpcMessage>(&buffer[..n]) {
                            Ok(msg) => {
                                // Handle registration
                                if let IpcMessage::Register { agent_id: id, .. } = &msg {
                                    agent_id = Some(*id);
                                    
                                    let connection = IpcConnection {
                                        agent_id: *id,
                                        stream: read_stream.try_clone().unwrap(),
                                        tx: tx.clone(),
                                        rx: tokio::sync::mpsc::channel(100).1, // Placeholder
                                    };
                                    
                                    connections_clone.write().await.insert(*id, connection);
                                    info!("Agent {} registered", id);
                                }
                                
                                // Handle message
                                if let Some(id) = agent_id {
                                    if let Some(response) = handler_clone.handle_message(msg, id).await {
                                        let _ = tx.send(response).await;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to deserialize IPC message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from IPC stream: {}", e);
                        break;
                    }
                }
            }
        });
        
        // Write messages to stream
        while let Some(msg) = rx.recv().await {
            match bincode::serialize(&msg) {
                Ok(data) => {
                    if let Err(e) = stream.write_all(&data).await {
                        error!("Failed to write to IPC stream: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize IPC message: {}", e);
                }
            }
        }
    }
}

impl IpcClient {
    pub fn new(agent_id: AgentId, socket_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel(100);
        
        Self {
            agent_id,
            socket_path,
            stream: None,
            tx,
            rx,
        }
    }
    
    /// Connect to the IPC server
    pub async fn connect(&mut self) -> Result<()> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        self.stream = Some(stream);
        
        // Send registration message
        let register = IpcMessage::Register {
            agent_id: self.agent_id,
            pid: std::process::id(),
            capabilities: vec![],
        };
        
        self.send(register).await?;
        
        Ok(())
    }
    
    /// Send a message
    pub async fn send(&mut self, message: IpcMessage) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        let data = bincode::serialize(&message)?;
        stream.write_all(&data).await?;
        
        Ok(())
    }
    
    /// Receive a message
    pub async fn recv(&mut self) -> Result<IpcMessage> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        let mut buffer = vec![0u8; 4096];
        let n = stream.read(&mut buffer).await?;
        
        let message = bincode::deserialize(&buffer[..n])?;
        Ok(message)
    }
    
    /// Send a request and wait for response
    pub async fn request(
        &mut self,
        to: AgentId,
        method: String,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request_id = Uuid::new_v4();
        
        let request = IpcMessage::Request {
            id: request_id,
            from: self.agent_id,
            to,
            method,
            params,
        };
        
        self.send(request).await?;
        
        // Wait for response
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.wait_for_response(request_id)
        ).await;
        
        match timeout {
            Ok(result) => result,
            Err(_) => Err(MultiAgentError::Timeout("Request timeout".to_string()).into()),
        }
    }
    
    async fn wait_for_response(&mut self, request_id: Uuid) -> Result<serde_json::Value> {
        loop {
            let message = self.recv().await?;
            
            if let IpcMessage::Response { id, result, error } = message {
                if id == request_id {
                    if let Some(err) = error {
                        return Err(anyhow::anyhow!("Request failed: {}", err));
                    }
                    
                    return result.ok_or_else(|| anyhow::anyhow!("Empty response"));
                }
            }
        }
    }
}

use tracing::{info, error};