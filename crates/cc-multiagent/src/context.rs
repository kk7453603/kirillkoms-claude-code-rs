use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::errors::Result;

/// Shared context between agents
#[derive(Debug, Clone)]
pub struct SharedContext {
    inner: Arc<RwLock<ContextInner>>,
}

#[derive(Debug)]
struct ContextInner {
    /// Global key-value store
    global_state: HashMap<String, ContextValue>,
    
    /// Per-agent private contexts
    agent_contexts: HashMap<Uuid, HashMap<String, ContextValue>>,
    
    /// Context version for optimistic locking
    version: u64,
    
    /// Last update timestamp
    last_updated: DateTime<Utc>,
    
    /// Update history
    history: Vec<ContextUpdate>,
    
    /// Maximum history size
    max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextValue {
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Uuid,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUpdate {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub operation: UpdateOperation,
    pub key: String,
    pub old_value: Option<ContextValue>,
    pub new_value: Option<ContextValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateOperation {
    Set,
    Delete,
    Merge,
    Append,
}

impl SharedContext {
    pub fn new(max_history: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ContextInner {
                global_state: HashMap::new(),
                agent_contexts: HashMap::new(),
                version: 0,
                last_updated: Utc::now(),
                history: Vec::new(),
                max_history,
            })),
        }
    }
    
    /// Get a value from global context
    pub async fn get(&self, key: &str) -> Option<ContextValue> {
        let inner = self.inner.read().await;
        inner.global_state.get(key).cloned()
    }
    
    /// Set a value in global context
    pub async fn set(&self, agent_id: Uuid, key: String, value: serde_json::Value) -> Result<()> {
        let mut inner = self.inner.write().await;
        
        let old_value = inner.global_state.get(&key).cloned();
        let new_value = ContextValue {
            value,
            updated_at: Utc::now(),
            updated_by: agent_id,
            tags: vec![],
        };
        
        // Record update
        let update = ContextUpdate {
            id: Uuid::new_v4(),
            agent_id,
            timestamp: Utc::now(),
            operation: UpdateOperation::Set,
            key: key.clone(),
            old_value,
            new_value: Some(new_value.clone()),
        };
        
        // Apply update
        inner.global_state.insert(key, new_value);
        inner.version += 1;
        inner.last_updated = Utc::now();
        
        // Add to history
        inner.history.push(update);
        if inner.history.len() > inner.max_history {
            inner.history.remove(0);
        }
        
        Ok(())
    }
    
    /// Delete a value from global context
    pub async fn delete(&self, agent_id: Uuid, key: &str) -> Result<Option<ContextValue>> {
        let mut inner = self.inner.write().await;
        
        let old_value = inner.global_state.remove(key);
        
        if let Some(ref old) = old_value {
            let update = ContextUpdate {
                id: Uuid::new_v4(),
                agent_id,
                timestamp: Utc::now(),
                operation: UpdateOperation::Delete,
                key: key.to_string(),
                old_value: Some(old.clone()),
                new_value: None,
            };
            
            inner.history.push(update);
            if inner.history.len() > inner.max_history {
                inner.history.remove(0);
            }
        }
        
        inner.version += 1;
        inner.last_updated = Utc::now();
        
        Ok(old_value)
    }
    
    /// Get agent-private context
    pub async fn get_agent_context(&self, agent_id: Uuid, key: &str) -> Option<ContextValue> {
        let inner = self.inner.read().await;
        inner.agent_contexts
            .get(&agent_id)
            .and_then(|ctx| ctx.get(key))
            .cloned()
    }
    
    /// Set agent-private context
    pub async fn set_agent_context(
        &self, 
        agent_id: Uuid, 
        key: String, 
        value: serde_json::Value
    ) -> Result<()> {
        let mut inner = self.inner.write().await;
        
        let agent_ctx = inner.agent_contexts.entry(agent_id).or_insert_with(HashMap::new);
        
        agent_ctx.insert(key, ContextValue {
            value,
            updated_at: Utc::now(),
            updated_by: agent_id,
            tags: vec![],
        });
        
        inner.version += 1;
        inner.last_updated = Utc::now();
        
        Ok(())
    }
    
    /// Get all global keys
    pub async fn keys(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.global_state.keys().cloned().collect()
    }
    
    /// Get context version
    pub async fn version(&self) -> u64 {
        self.inner.read().await.version
    }
    
    /// Get update history
    pub async fn history(&self) -> Vec<ContextUpdate> {
        self.inner.read().await.history.clone()
    }
    
    /// Subscribe to context updates
    pub async fn subscribe(&self) -> ContextSubscriber {
        // In a real implementation, this would return a channel receiver
        // For now, return a placeholder
        ContextSubscriber {
            context: self.clone(),
        }
    }
}

pub struct ContextSubscriber {
    context: SharedContext,
}

impl ContextSubscriber {
    pub async fn recv(&mut self) -> Option<ContextUpdate> {
        // Placeholder for subscription mechanism
        None
    }
}