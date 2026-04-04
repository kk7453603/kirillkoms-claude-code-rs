use cc_multiagent::{
    AgentPool, PoolConfig, AgentConfig, MessageRouter, MessageTarget,
    SharedContext, AgentId, AgentState,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Multi-Agent Control Plane example");

    // Create agent pool with configuration
    let pool_config = PoolConfig {
        max_agents: 5,
        health_check_interval: Duration::from_secs(10),
        ..Default::default()
    };
    
    let pool = AgentPool::new(pool_config).await?;
    info!("Agent pool created");

    // Spawn multiple agents with different models
    let agent1 = pool.spawn(
        AgentConfig::new("researcher", "claude-3-opus")
            .with_command(vec!["claude-code".to_string(), "--model".to_string(), "opus".to_string()])
            .with_tag("research")
            .with_memory_limit(2048)
    ).await?;
    info!("Spawned agent 1 (researcher): {}", agent1);

    let agent2 = pool.spawn(
        AgentConfig::new("coder", "claude-3-sonnet")
            .with_command(vec!["claude-code".to_string(), "--model".to_string(), "sonnet".to_string()])
            .with_tag("coding")
            .with_memory_limit(4096)
    ).await?;
    info!("Spawned agent 2 (coder): {}", agent2);

    let agent3 = pool.spawn(
        AgentConfig::new("reviewer", "gpt-4")
            .with_command(vec!["claude-code".to_string(), "--model".to_string(), "gpt-4".to_string()])
            .with_tag("review")
            .with_memory_limit(2048)
    ).await?;
    info!("Spawned agent 3 (reviewer): {}", agent3);

    // Wait for agents to start
    sleep(Duration::from_secs(2)).await;

    // Check agent states
    let agents = pool.list().await;
    for (id, state) in &agents {
        info!("Agent {} state: {:?}", id, state);
    }

    // Example 1: Direct messaging
    info!("\n=== Example 1: Direct Messaging ===");
    match pool.send_message(agent1, "Research the latest developments in Rust async programming".to_string()).await {
        Ok(response) => info!("Response from researcher: {}", response),
        Err(e) => info!("Error: {}", e),
    }

    // Example 2: Broadcast to all agents
    info!("\n=== Example 2: Broadcast ===");
    let responses = pool.broadcast("What are you currently working on?".to_string()).await;
    for (agent_id, result) in responses {
        match result {
            Ok(response) => info!("Agent {} says: {}", agent_id, response),
            Err(e) => info!("Agent {} error: {}", agent_id, e),
        }
    }

    // Example 3: Task coordination workflow
    info!("\n=== Example 3: Task Coordination ===");
    
    // Step 1: Research phase
    let research_task = "Research best practices for error handling in Rust";
    info!("Sending research task to agent 1...");
    let research_result = pool.send_message(agent1, research_task.to_string()).await?;
    info!("Research complete: {}", research_result);

    // Step 2: Implementation phase
    let coding_task = format!("Based on this research: '{}', implement a comprehensive error handling module", research_result);
    info!("Sending coding task to agent 2...");
    let code_result = pool.send_message(agent2, coding_task).await?;
    info!("Code implementation complete: {}", code_result);

    // Step 3: Review phase
    let review_task = format!("Review this code implementation: '{}'", code_result);
    info!("Sending review task to agent 3...");
    let review_result = pool.send_message(agent3, review_task).await?;
    info!("Code review complete: {}", review_result);

    // Example 4: Pool metrics
    info!("\n=== Example 4: Pool Metrics ===");
    let metrics = pool.metrics().await;
    info!("Total agents: {}", metrics.total_agents);
    info!("Running agents: {}", metrics.running_agents);
    info!("Failed agents: {}", metrics.failed_agents);
    info!("Total CPU usage: {:.2}%", metrics.total_cpu_percent);
    info!("Total memory usage: {} MB", metrics.total_memory_mb);

    // Example 5: Graceful shutdown
    info!("\n=== Example 5: Graceful Shutdown ===");
    
    // Stop individual agent
    pool.stop_agent(agent1, false).await?;
    info!("Stopped agent 1");

    // Check updated state
    sleep(Duration::from_secs(1)).await;
    let agents = pool.list().await;
    for (id, state) in &agents {
        info!("Agent {} state after stop: {:?}", id, state);
    }

    // Shutdown entire pool
    info!("Shutting down agent pool...");
    pool.shutdown().await?;
    info!("Agent pool shutdown complete");

    Ok(())
}

// Example output:
// 
// [INFO] Starting Multi-Agent Control Plane example
// [INFO] Agent pool created
// [INFO] Spawned agent 1 (researcher): 550e8400-e29b-41d4-a716-446655440000
// [INFO] Spawned agent 2 (coder): 6ba7b810-9dad-11d1-80b4-00c04fd430c8
// [INFO] Spawned agent 3 (reviewer): 6ba7b811-9dad-11d1-80b4-00c04fd430c8
// [INFO] Agent 550e8400-e29b-41d4-a716-446655440000 state: Running
// [INFO] Agent 6ba7b810-9dad-11d1-80b4-00c04fd430c8 state: Running
// [INFO] Agent 6ba7b811-9dad-11d1-80b4-00c04fd430c8 state: Running
// 
// === Example 1: Direct Messaging ===
// [INFO] Response from researcher: Echo from 550e8400-e29b-41d4-a716-446655440000: Research the latest developments in Rust async programming
// 
// === Example 2: Broadcast ===
// [INFO] Agent 550e8400-e29b-41d4-a716-446655440000 says: Echo from 550e8400-e29b-41d4-a716-446655440000: What are you currently working on?
// [INFO] Agent 6ba7b810-9dad-11d1-80b4-00c04fd430c8 says: Echo from 6ba7b810-9dad-11d1-80b4-00c04fd430c8: What are you currently working on?
// [INFO] Agent 6ba7b811-9dad-11d1-80b4-00c04fd430c8 says: Echo from 6ba7b811-9dad-11d1-80b4-00c04fd430c8: What are you currently working on?