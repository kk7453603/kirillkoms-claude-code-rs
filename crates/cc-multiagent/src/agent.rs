use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

use crate::errors::Result;

pub type AgentId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is created but not started
    Idle,
    /// Agent is starting up
    Starting,
    /// Agent is running and healthy
    Running,
    /// Agent is paused
    Paused,
    /// Agent is stopping
    Stopping,
    /// Agent has stopped
    Stopped,
    /// Agent crashed or errored
    Failed(i32), // exit code
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent identifier
    pub id: AgentId,
    
    /// Human-readable name
    pub name: String,
    
    /// Agent type/model (e.g., "claude-3-opus", "gpt-4", "llama-3")
    pub model: String,
    
    /// Command to execute (e.g., ["claude-code", "--model", "opus"])
    pub command: Vec<String>,
    
    /// Working directory
    pub working_dir: Option<String>,
    
    /// Environment variables
    pub env_vars: Vec<(String, String)>,
    
    /// Max memory usage in MB (0 = unlimited)
    pub memory_limit_mb: u64,
    
    /// CPU affinity mask (empty = no affinity)
    pub cpu_affinity: Vec<usize>,
    
    /// Restart policy
    pub restart_policy: RestartPolicy,
    
    /// Tags for grouping/filtering
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RestartPolicy {
    Never,
    OnFailure { max_retries: u32 },
    Always { delay_ms: u64 },
}

#[derive(Debug)]
pub struct AgentInstance {
    pub config: AgentConfig,
    pub state: RwLock<AgentState>,
    pub process: RwLock<Option<Child>>,
    pub started_at: RwLock<Option<Instant>>,
    pub restart_count: RwLock<u32>,
    pub metrics: AgentMetrics,
    
    // Communication channels
    pub command_tx: mpsc::Sender<AgentCommand>,
    pub event_tx: mpsc::Sender<AgentEvent>,
}

#[derive(Debug, Default)]
pub struct AgentMetrics {
    pub total_messages_sent: Arc<RwLock<u64>>,
    pub total_messages_received: Arc<RwLock<u64>>,
    pub total_errors: Arc<RwLock<u64>>,
    pub cpu_usage_percent: Arc<RwLock<f32>>,
    pub memory_usage_mb: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
pub enum AgentCommand {
    Start,
    Stop { force: bool },
    Pause,
    Resume,
    SendMessage { content: String, reply_tx: oneshot::Sender<String> },
    UpdateContext { update: crate::context::ContextUpdate },
    GetState { reply_tx: oneshot::Sender<AgentState> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    StateChanged { from: AgentState, to: AgentState },
    MessageReceived { content: String },
    ErrorOccurred { error: String },
    MetricsUpdated { cpu_percent: f32, memory_mb: u64 },
}

#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent's unique identifier
    fn id(&self) -> AgentId;
    
    /// Get the agent's current state
    async fn state(&self) -> AgentState;
    
    /// Start the agent
    async fn start(&self) -> Result<()>;
    
    /// Stop the agent
    async fn stop(&self, force: bool) -> Result<()>;
    
    /// Send a message to the agent
    async fn send_message(&self, message: &str) -> Result<String>;
    
    /// Update shared context
    async fn update_context(&self, update: crate::context::ContextUpdate) -> Result<()>;
}

impl AgentConfig {
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            model: model.into(),
            command: vec!["claude-code".to_string()],
            working_dir: None,
            env_vars: vec![],
            memory_limit_mb: 0,
            cpu_affinity: vec![],
            restart_policy: RestartPolicy::OnFailure { max_retries: 3 },
            tags: vec![],
        }
    }
    
    pub fn with_command(mut self, command: Vec<String>) -> Self {
        self.command = command;
        self
    }
    
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }
    
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
    
    pub fn with_memory_limit(mut self, limit_mb: u64) -> Self {
        self.memory_limit_mb = limit_mb;
        self
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::new("default-agent", "claude-3-opus")
    }
}