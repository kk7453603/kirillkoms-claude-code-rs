use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::agent::AgentId;
use crate::errors::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub from: AgentId,
    pub to: MessageTarget,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageTarget {
    /// Send to specific agent
    Agent(AgentId),
    /// Broadcast to all agents
    Broadcast,
    /// Send to agents with specific tag
    Tagged(String),
    /// Round-robin to any available agent
    RoundRobin,
    /// Send to least loaded agent
    LeastLoaded,
}

#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Direct routing to specific agent
    Direct,
    /// Round-robin between agents
    RoundRobin { next_index: Arc<RwLock<usize>> },
    /// Route based on message content
    ContentBased { rules: Arc<RwLock<Vec<RoutingRule>>> },
    /// Route to least loaded agent
    LoadBalanced,
    /// Custom routing function
    Custom(Arc<dyn CustomRouter>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub pattern: String,
    pub target: MessageTarget,
    pub priority: u32,
}

#[async_trait]
pub trait CustomRouter: Send + Sync {
    async fn route(&self, message: &Message, agents: &[AgentId]) -> Option<AgentId>;
}

pub struct MessageRouter {
    /// Active routing strategies
    strategies: Vec<RoutingStrategy>,
    
    /// Message history
    history: Arc<RwLock<Vec<Message>>>,
    
    /// Routing statistics
    stats: Arc<RwLock<RoutingStats>>,
    
    /// Message channels for each agent
    agent_channels: Arc<RwLock<HashMap<AgentId, mpsc::Sender<Message>>>>,
}

#[derive(Debug, Default)]
pub struct RoutingStats {
    pub messages_routed: u64,
    pub messages_failed: u64,
    pub agent_message_count: HashMap<AgentId, u64>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            strategies: vec![RoutingStrategy::Direct],
            history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(RoutingStats::default())),
            agent_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a routing strategy
    pub async fn add_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategies.push(strategy);
    }
    
    /// Register an agent channel
    pub async fn register_agent(&self, agent_id: AgentId, channel: mpsc::Sender<Message>) {
        self.agent_channels.write().await.insert(agent_id, channel);
    }
    
    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: AgentId) {
        self.agent_channels.write().await.remove(&agent_id);
    }
    
    /// Route a message
    pub async fn route(&self, mut message: Message) -> Result<Vec<AgentId>> {
        message.timestamp = chrono::Utc::now();
        
        let channels = self.agent_channels.read().await;
        let agent_ids: Vec<AgentId> = channels.keys().cloned().collect();
        
        // Determine target agents based on message target
        let target_agents = match &message.to {
            MessageTarget::Agent(id) => vec![*id],
            MessageTarget::Broadcast => agent_ids.clone(),
            MessageTarget::Tagged(tag) => {
                // TODO: Implement tag-based filtering
                vec![]
            }
            MessageTarget::RoundRobin => {
                if let Some(agent_id) = self.select_round_robin(&agent_ids).await {
                    vec![agent_id]
                } else {
                    vec![]
                }
            }
            MessageTarget::LeastLoaded => {
                if let Some(agent_id) = self.select_least_loaded(&agent_ids).await {
                    vec![agent_id]
                } else {
                    vec![]
                }
            }
        };
        
        // Send to target agents
        let mut delivered = Vec::new();
        let mut stats = self.stats.write().await;
        
        for agent_id in target_agents {
            if let Some(channel) = channels.get(&agent_id) {
                match channel.send(message.clone()).await {
                    Ok(_) => {
                        delivered.push(agent_id);
                        stats.messages_routed += 1;
                        *stats.agent_message_count.entry(agent_id).or_insert(0) += 1;
                    }
                    Err(_) => {
                        stats.messages_failed += 1;
                    }
                }
            }
        }
        
        // Store in history
        let mut history = self.history.write().await;
        history.push(message);
        if history.len() > 1000 {
            history.remove(0);
        }
        
        Ok(delivered)
    }
    
    /// Create a message
    pub fn create_message(from: AgentId, to: MessageTarget, content: String) -> Message {
        Message {
            id: Uuid::new_v4(),
            from,
            to,
            content,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Get routing statistics
    pub async fn stats(&self) -> RoutingStats {
        self.stats.read().await.clone()
    }
    
    /// Get message history
    pub async fn history(&self, limit: usize) -> Vec<Message> {
        let history = self.history.read().await;
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }
    
    // Private helper methods
    
    async fn select_round_robin(&self, agents: &[AgentId]) -> Option<AgentId> {
        if agents.is_empty() {
            return None;
        }
        
        for strategy in &self.strategies {
            if let RoutingStrategy::RoundRobin { next_index } = strategy {
                let mut idx = next_index.write().await;
                let selected = agents[*idx % agents.len()];
                *idx = (*idx + 1) % agents.len();
                return Some(selected);
            }
        }
        
        // Fallback to first agent
        agents.first().cloned()
    }
    
    async fn select_least_loaded(&self, agents: &[AgentId]) -> Option<AgentId> {
        let stats = self.stats.read().await;
        
        agents.iter()
            .min_by_key(|&agent_id| {
                stats.agent_message_count.get(agent_id).unwrap_or(&0)
            })
            .cloned()
    }
}

/// Message builder for convenient message creation
pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    pub fn new(from: AgentId, to: MessageTarget) -> Self {
        Self {
            message: Message {
                id: Uuid::new_v4(),
                from,
                to,
                content: String::new(),
                metadata: HashMap::new(),
                timestamp: chrono::Utc::now(),
            },
        }
    }
    
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.message.content = content.into();
        self
    }
    
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.message.metadata.insert(key.into(), value);
        self
    }
    
    pub fn build(self) -> Message {
        self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_routing() {
        let router = MessageRouter::new();
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();
        
        // Create channels
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);
        
        // Register agents
        router.register_agent(agent1, tx1).await;
        router.register_agent(agent2, tx2).await;
        
        // Send broadcast message
        let msg = MessageBuilder::new(agent1, MessageTarget::Broadcast)
            .content("Hello everyone!")
            .build();
        
        let delivered = router.route(msg).await.unwrap();
        assert_eq!(delivered.len(), 2);
        
        // Verify both agents received the message
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }
}