//! Multi-Agent Control Plane for Claude Code RS
//! 
//! This crate provides a control plane for managing multiple AI agents running in parallel,
//! with shared context, inter-agent communication, and lifecycle management.

pub mod agent;
pub mod pool;
pub mod context;
pub mod router;
pub mod ipc;
pub mod metrics;
pub mod errors;

pub use agent::{Agent, AgentConfig, AgentId, AgentState};
pub use pool::{AgentPool, PoolConfig};
pub use context::{SharedContext, ContextUpdate};
pub use router::{MessageRouter, RoutingStrategy};
pub use errors::{MultiAgentError, Result};

/// Re-export common types
pub mod prelude {
    pub use super::{
        Agent, AgentConfig, AgentId, AgentPool, 
        SharedContext, Result, MultiAgentError
    };
}