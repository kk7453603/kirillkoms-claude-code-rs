use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::agent::{AgentCommand, AgentConfig, AgentEvent, AgentId, AgentInstance, AgentState};
use crate::context::SharedContext;
use crate::errors::{MultiAgentError, Result};
use crate::router::MessageRouter;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of agents
    pub max_agents: usize,
    
    /// Health check interval
    pub health_check_interval: Duration,
    
    /// Resource limits
    pub total_memory_limit_mb: u64,
    pub total_cpu_limit_percent: f32,
    
    /// Default agent restart policy
    pub default_restart_policy: crate::agent::RestartPolicy,
    
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_agents: 10,
            health_check_interval: Duration::from_secs(30),
            total_memory_limit_mb: 8192, // 8GB
            total_cpu_limit_percent: 400.0, // 4 cores
            default_restart_policy: crate::agent::RestartPolicy::OnFailure { max_retries: 3 },
            enable_metrics: true,
        }
    }
}

pub struct AgentPool {
    config: PoolConfig,
    agents: Arc<DashMap<AgentId, Arc<AgentInstance>>>,
    context: SharedContext,
    router: Arc<RwLock<MessageRouter>>,
    shutdown_tx: mpsc::Sender<()>,
    metrics_tx: mpsc::Sender<PoolMetrics>,
}

#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub total_agents: usize,
    pub running_agents: usize,
    pub failed_agents: usize,
    pub total_memory_mb: u64,
    pub total_cpu_percent: f32,
    pub messages_routed: u64,
}

impl AgentPool {
    pub async fn new(config: PoolConfig) -> Result<Self> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        let (metrics_tx, mut metrics_rx) = mpsc::channel(100);
        
        let pool = Self {
            config,
            agents: Arc::new(DashMap::new()),
            context: SharedContext::new(1000),
            router: Arc::new(RwLock::new(MessageRouter::new())),
            shutdown_tx,
            metrics_tx,
        };
        
        // Start health check task
        let agents_clone = pool.agents.clone();
        let health_interval = pool.config.health_check_interval;
        
        tokio::spawn(async move {
            let mut ticker = interval(health_interval);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        Self::health_check(&agents_clone).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down health check task");
                        break;
                    }
                }
            }
        });
        
        Ok(pool)
    }
    
    /// Spawn a new agent
    pub async fn spawn(&self, config: AgentConfig) -> Result<AgentId> {
        // Check capacity
        if self.agents.len() >= self.config.max_agents {
            return Err(MultiAgentError::PoolCapacityExceeded {
                max: self.config.max_agents,
                current: self.agents.len(),
            });
        }
        
        // Check if agent already exists
        if self.agents.contains_key(&config.id) {
            return Err(MultiAgentError::AgentAlreadyExists { id: config.id });
        }
        
        // Create agent instance
        let agent = self.create_agent_instance(config).await?;
        let agent_id = agent.config.id;
        
        // Store in pool
        self.agents.insert(agent_id, Arc::new(agent));
        
        // Start the agent
        self.start_agent(agent_id).await?;
        
        info!("Spawned agent {}", agent_id);
        Ok(agent_id)
    }
    
    /// Get agent by ID
    pub async fn get(&self, agent_id: AgentId) -> Option<Arc<AgentInstance>> {
        self.agents.get(&agent_id).map(|entry| entry.clone())
    }
    
    /// List all agents
    pub async fn list(&self) -> Vec<(AgentId, AgentState)> {
        let mut agents = Vec::new();
        for entry in self.agents.iter() {
            let state = *entry.value().state.read().await;
            agents.push((*entry.key(), state));
        }
        agents
    }
    
    /// Start an agent
    pub async fn start_agent(&self, agent_id: AgentId) -> Result<()> {
        let agent = self.agents.get(&agent_id)
            .ok_or(MultiAgentError::AgentNotFound { id: agent_id })?;
        
        let mut state = agent.state.write().await;
        if !matches!(*state, AgentState::Idle | AgentState::Stopped) {
            return Ok(());
        }
        
        *state = AgentState::Starting;
        drop(state);
        
        // Launch agent process
        match self.launch_agent_process(agent.clone()).await {
            Ok(_) => {
                *agent.state.write().await = AgentState::Running;
                Ok(())
            }
            Err(e) => {
                *agent.state.write().await = AgentState::Failed(-1);
                Err(MultiAgentError::AgentStartFailed {
                    id: agent_id,
                    reason: e.to_string(),
                })
            }
        }
    }
    
    /// Stop an agent
    pub async fn stop_agent(&self, agent_id: AgentId, force: bool) -> Result<()> {
        let agent = self.agents.get(&agent_id)
            .ok_or(MultiAgentError::AgentNotFound { id: agent_id })?;
        
        let mut state = agent.state.write().await;
        if !matches!(*state, AgentState::Running | AgentState::Paused) {
            return Ok(());
        }
        
        *state = AgentState::Stopping;
        drop(state);
        
        // Send stop command
        let _ = agent.command_tx.send(AgentCommand::Stop { force }).await;
        
        // Kill process if exists
        if let Some(mut process) = agent.process.write().await.take() {
            if force {
                let _ = process.kill().await;
            } else {
                // Give process time to shutdown gracefully
                tokio::time::sleep(Duration::from_secs(5)).await;
                let _ = process.kill().await;
            }
        }
        
        *agent.state.write().await = AgentState::Stopped;
        Ok(())
    }
    
    /// Remove an agent from the pool
    pub async fn remove(&self, agent_id: AgentId) -> Result<()> {
        // Stop the agent first
        self.stop_agent(agent_id, true).await?;
        
        // Remove from pool
        self.agents.remove(&agent_id)
            .ok_or(MultiAgentError::AgentNotFound { id: agent_id })?;
        
        info!("Removed agent {}", agent_id);
        Ok(())
    }
    
    /// Send message to an agent
    pub async fn send_message(&self, agent_id: AgentId, message: String) -> Result<String> {
        let agent = self.agents.get(&agent_id)
            .ok_or(MultiAgentError::AgentNotFound { id: agent_id })?;
        
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        
        agent.command_tx
            .send(AgentCommand::SendMessage { content: message, reply_tx })
            .await
            .map_err(|_| MultiAgentError::ChannelSendError)?;
        
        reply_rx.await
            .map_err(|_| MultiAgentError::ChannelReceiveError)
    }
    
    /// Broadcast message to all agents
    pub async fn broadcast(&self, message: String) -> HashMap<AgentId, Result<String>> {
        let mut results = HashMap::new();
        
        for entry in self.agents.iter() {
            let agent_id = *entry.key();
            let result = self.send_message(agent_id, message.clone()).await;
            results.insert(agent_id, result);
        }
        
        results
    }
    
    /// Get pool metrics
    pub async fn metrics(&self) -> PoolMetrics {
        let mut metrics = PoolMetrics {
            total_agents: self.agents.len(),
            running_agents: 0,
            failed_agents: 0,
            total_memory_mb: 0,
            total_cpu_percent: 0.0,
            messages_routed: 0,
        };
        
        for entry in self.agents.iter() {
            let state = *entry.value().state.read().await;
            match state {
                AgentState::Running => metrics.running_agents += 1,
                AgentState::Failed(_) => metrics.failed_agents += 1,
                _ => {}
            }
            
            let agent_metrics = &entry.value().metrics;
            metrics.total_memory_mb += *agent_metrics.memory_usage_mb.read().await;
            metrics.total_cpu_percent += *agent_metrics.cpu_usage_percent.read().await;
        }
        
        metrics
    }
    
    /// Shutdown the pool
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down agent pool");
        
        // Stop all agents
        let agent_ids: Vec<_> = self.agents.iter()
            .map(|entry| *entry.key())
            .collect();
        
        for agent_id in agent_ids {
            let _ = self.stop_agent(agent_id, false).await;
        }
        
        // Signal shutdown to background tasks
        let _ = self.shutdown_tx.send(()).await;
        
        Ok(())
    }
    
    // Private helper methods
    
    async fn create_agent_instance(&self, config: AgentConfig) -> Result<AgentInstance> {
        let (command_tx, mut command_rx) = mpsc::channel(100);
        let (event_tx, mut event_rx) = mpsc::channel(100);
        
        let agent = AgentInstance {
            config,
            state: RwLock::new(AgentState::Idle),
            process: RwLock::new(None),
            started_at: RwLock::new(None),
            restart_count: RwLock::new(0),
            metrics: Default::default(),
            command_tx: command_tx.clone(),
            event_tx: event_tx.clone(),
        };
        
        // Spawn command handler
        let agent_id = agent.config.id;
        let agent_clone = Arc::new(agent.clone());
        
        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    AgentCommand::SendMessage { content, reply_tx } => {
                        // Placeholder for actual message handling
                        let _ = reply_tx.send(format!("Echo from {}: {}", agent_id, content));
                    }
                    _ => {
                        // Handle other commands
                    }
                }
            }
        });
        
        Ok(agent)
    }
    
    async fn launch_agent_process(&self, agent: Arc<AgentInstance>) -> Result<()> {
        use tokio::process::Command;
        
        let mut cmd = Command::new(&agent.config.command[0]);
        
        // Add arguments
        for arg in &agent.config.command[1..] {
            cmd.arg(arg);
        }
        
        // Set environment variables
        for (key, value) in &agent.config.env_vars {
            cmd.env(key, value);
        }
        
        // Set working directory
        if let Some(ref dir) = agent.config.working_dir {
            cmd.current_dir(dir);
        }
        
        // Spawn process
        let child = cmd.spawn()?;
        
        *agent.process.write().await = Some(child);
        *agent.started_at.write().await = Some(std::time::Instant::now());
        
        Ok(())
    }
    
    async fn health_check(agents: &DashMap<AgentId, Arc<AgentInstance>>) {
        for entry in agents.iter() {
            let agent = entry.value();
            let state = *agent.state.read().await;
            
            if let AgentState::Running = state {
                // Check if process is still alive
                if let Some(ref mut process) = &mut *agent.process.write().await {
                    match process.try_wait() {
                        Ok(Some(status)) => {
                            let exit_code = status.code().unwrap_or(-1);
                            *agent.state.write().await = AgentState::Failed(exit_code);
                            warn!("Agent {} crashed with exit code {}", agent.config.id, exit_code);
                            
                            // TODO: Handle restart policy
                        }
                        Ok(None) => {
                            // Process still running
                        }
                        Err(e) => {
                            error!("Failed to check agent {} status: {}", agent.config.id, e);
                        }
                    }
                }
            }
        }
    }
}