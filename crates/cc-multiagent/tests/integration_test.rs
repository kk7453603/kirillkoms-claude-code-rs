use cc_multiagent::prelude::*;
use cc_multiagent::router::{MessageBuilder, MessageRouter, MessageTarget};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_agent_lifecycle() {
    // Create pool
    let pool = AgentPool::new(PoolConfig::default()).await.unwrap();
    
    // Spawn agent
    let config = AgentConfig::new("test-agent", "test-model");
    let agent_id = pool.spawn(config).await.unwrap();
    
    // Verify agent exists
    let agent = pool.get(agent_id).await;
    assert!(agent.is_some());
    
    // List agents
    let agents = pool.list().await;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].0, agent_id);
    
    // Stop agent
    pool.stop_agent(agent_id, false).await.unwrap();
    
    // Remove agent
    pool.remove(agent_id).await.unwrap();
    
    // Verify agent removed
    let agents = pool.list().await;
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_message_passing() {
    let pool = AgentPool::new(PoolConfig::default()).await.unwrap();
    
    // Spawn two agents
    let agent1 = pool.spawn(AgentConfig::new("agent1", "model1")).await.unwrap();
    let agent2 = pool.spawn(AgentConfig::new("agent2", "model2")).await.unwrap();
    
    // Send message to specific agent
    let response = timeout(
        Duration::from_secs(5),
        pool.send_message(agent1, "Hello agent1".to_string())
    ).await;
    
    assert!(response.is_ok());
    let response = response.unwrap();
    assert!(response.is_ok());
    assert!(response.unwrap().contains("Echo from"));
    
    // Broadcast message
    let responses = pool.broadcast("Hello everyone".to_string()).await;
    assert_eq!(responses.len(), 2);
    
    pool.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_shared_context() {
    use serde_json::json;
    
    let context = SharedContext::new(100);
    let agent_id = uuid::Uuid::new_v4();
    
    // Set value
    context.set(agent_id, "key1".to_string(), json!("value1")).await.unwrap();
    
    // Get value
    let value = context.get("key1").await;
    assert!(value.is_some());
    assert_eq!(value.unwrap().value, json!("value1"));
    
    // Update value
    context.set(agent_id, "key1".to_string(), json!("value2")).await.unwrap();
    let value = context.get("key1").await;
    assert_eq!(value.unwrap().value, json!("value2"));
    
    // Delete value
    let deleted = context.delete(agent_id, "key1").await.unwrap();
    assert!(deleted.is_some());
    assert_eq!(deleted.unwrap().value, json!("value2"));
    
    // Verify deleted
    let value = context.get("key1").await;
    assert!(value.is_none());
    
    // Check version increments
    let version = context.version().await;
    assert_eq!(version, 3); // set, update, delete
}

#[tokio::test]
async fn test_agent_private_context() {
    use serde_json::json;
    
    let context = SharedContext::new(100);
    let agent1 = uuid::Uuid::new_v4();
    let agent2 = uuid::Uuid::new_v4();
    
    // Set private context for agent1
    context.set_agent_context(agent1, "private".to_string(), json!("agent1 data"))
        .await
        .unwrap();
    
    // Set private context for agent2
    context.set_agent_context(agent2, "private".to_string(), json!("agent2 data"))
        .await
        .unwrap();
    
    // Verify isolation
    let value1 = context.get_agent_context(agent1, "private").await;
    assert_eq!(value1.unwrap().value, json!("agent1 data"));
    
    let value2 = context.get_agent_context(agent2, "private").await;
    assert_eq!(value2.unwrap().value, json!("agent2 data"));
}

#[tokio::test]
async fn test_pool_capacity() {
    let config = PoolConfig {
        max_agents: 2,
        ..Default::default()
    };
    
    let pool = AgentPool::new(config).await.unwrap();
    
    // Spawn up to capacity
    let agent1 = pool.spawn(AgentConfig::new("agent1", "model")).await.unwrap();
    let agent2 = pool.spawn(AgentConfig::new("agent2", "model")).await.unwrap();
    
    // Try to exceed capacity
    let result = pool.spawn(AgentConfig::new("agent3", "model")).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        MultiAgentError::PoolCapacityExceeded { max, current } => {
            assert_eq!(max, 2);
            assert_eq!(current, 2);
        }
        _ => panic!("Expected PoolCapacityExceeded error"),
    }
    
    pool.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_message_routing() {
    use cc_multiagent::router::{MessageRouter, MessageTarget, MessageBuilder};
    use tokio::sync::mpsc;
    
    let router = MessageRouter::new();
    let agent1 = uuid::Uuid::new_v4();
    let agent2 = uuid::Uuid::new_v4();
    
    // Create channels
    let (tx1, mut rx1) = mpsc::channel(10);
    let (tx2, mut rx2) = mpsc::channel(10);
    
    // Register agents
    router.register_agent(agent1, tx1).await;
    router.register_agent(agent2, tx2).await;
    
    // Test direct routing
    let msg = MessageBuilder::new(agent1, MessageTarget::Agent(agent2))
        .content("Direct message")
        .build();
    
    let delivered = router.route(msg).await.unwrap();
    assert_eq!(delivered, vec![agent2]);
    
    let received = rx2.try_recv();
    assert!(received.is_ok());
    assert_eq!(received.unwrap().content, "Direct message");
    
    // Test broadcast routing
    let msg = MessageBuilder::new(agent1, MessageTarget::Broadcast)
        .content("Broadcast message")
        .build();
    
    let delivered = router.route(msg).await.unwrap();
    assert_eq!(delivered.len(), 2);
    assert!(delivered.contains(&agent1));
    assert!(delivered.contains(&agent2));
}

#[tokio::test]
async fn test_metrics_collection() {
    use cc_multiagent::metrics::{MetricsCollector, AgentMetrics};
    use chrono::Utc;
    
    let collector = MetricsCollector::new(24);
    let agent_id = uuid::Uuid::new_v4();
    
    // Record metrics
    for i in 0..5 {
        let metrics = AgentMetrics {
            agent_id,
            cpu_percent: 10.0 + i as f32,
            memory_mb: 100 + i * 10,
            messages_sent: i * 2,
            messages_received: i * 3,
            errors: 0,
            uptime_seconds: i * 60,
            last_updated: Utc::now(),
        };
        
        collector.record_agent(metrics).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Get metrics
    let history = collector.get_agent_metrics(
        agent_id,
        Utc::now() - chrono::Duration::minutes(1)
    ).await;
    
    assert_eq!(history.len(), 5);
    
    // Calculate stats
    let stats = collector.agent_stats(agent_id).await.unwrap();
    assert_eq!(stats.sample_count, 5);
    assert_eq!(stats.cpu_avg, 12.0); // (10+11+12+13+14)/5
    assert_eq!(stats.memory_avg, 120); // (100+110+120+130+140)/5
}