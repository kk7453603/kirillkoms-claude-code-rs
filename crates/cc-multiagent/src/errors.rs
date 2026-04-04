use thiserror::Error;
use std::sync::Arc;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, MultiAgentError>;

#[derive(Error, Debug)]
pub enum MultiAgentError {
    #[error("Agent {id} not found")]
    AgentNotFound { id: Uuid },
    
    #[error("Agent {id} already exists")]
    AgentAlreadyExists { id: Uuid },
    
    #[error("Agent {id} failed to start: {reason}")]
    AgentStartFailed { id: Uuid, reason: String },
    
    #[error("Agent {id} crashed: {reason}")]
    AgentCrashed { id: Uuid, reason: String },
    
    #[error("Pool capacity exceeded: max {max}, current {current}")]
    PoolCapacityExceeded { max: usize, current: usize },
    
    #[error("Context update failed: {reason}")]
    ContextUpdateFailed { reason: String },
    
    #[error("IPC error: {0}")]
    IpcError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Channel send error")]
    ChannelSendError,
    
    #[error("Channel receive error")]
    ChannelReceiveError,
    
    #[error("Process spawn error: {0}")]
    ProcessSpawnError(#[from] std::io::Error),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Timeout exceeded: {0}")]
    Timeout(String),
    
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}