//! Multi-Agent Control Plane for Claude Code RS
//!
//! This crate provides a control plane for managing multiple AI agents running in parallel,
//! with shared context, inter-agent communication, and lifecycle management.

pub mod agent;
pub mod context;
pub mod errors;
pub mod ipc;
pub mod metrics;
pub mod pool;
pub mod router;

#[cfg(test)]
mod test_utils;

pub use agent::{Agent, AgentCommand, AgentConfig, AgentEvent, AgentId, AgentInstance, AgentState};
pub use context::{ContextUpdate, SharedContext};
pub use errors::{MultiAgentError, Result};
pub use pool::{AgentPool, PoolConfig, PoolMetrics};
pub use router::{MessageRouter, RoutingStrategy};

/// Re-export common types
pub mod prelude {
    pub use super::{
        Agent, AgentConfig, AgentId, AgentPool, MultiAgentError, PoolConfig, Result,
        SharedContext,
    };
}