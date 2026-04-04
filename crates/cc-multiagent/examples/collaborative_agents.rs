use cc_multiagent::prelude::*;
use serde_json::json;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Example: Multiple agents collaborating on a software project
/// - Architect: Designs the system
/// - Developer: Implements features
/// - Tester: Writes tests
/// - Documenter: Creates documentation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Collaborative Agents example");

    // Create shared context for project state
    let context = SharedContext::new(1000);
    
    // Initialize project in shared context
    context.set(
        uuid::Uuid::nil(), // System agent
        "project".to_string(),
        json!({
            "name": "cc-multiagent",
            "description": "Multi-Agent Control Plane for Claude Code RS",
            "modules": ["agent", "pool", "context", "router", "ipc"],
            "status": "in_progress"
        })
    ).await?;

    // Create agent pool
    let pool = AgentPool::new(PoolConfig::default()).await?;

    // Spawn specialized agents
    let architect = pool.spawn(
        AgentConfig::new("architect", "claude-3-opus")
            .with_tag("design")
            .with_env("ROLE", "system_architect")
    ).await?;
    info!("Spawned architect agent: {}", architect);

    let developer = pool.spawn(
        AgentConfig::new("developer", "claude-3-sonnet")
            .with_tag("implementation")
            .with_env("ROLE", "senior_developer")
    ).await?;
    info!("Spawned developer agent: {}", developer);

    let tester = pool.spawn(
        AgentConfig::new("tester", "claude-3-haiku")
            .with_tag("testing")
            .with_env("ROLE", "qa_engineer")
    ).await?;
    info!("Spawned tester agent: {}", tester);

    let documenter = pool.spawn(
        AgentConfig::new("documenter", "gpt-4")
            .with_tag("documentation")
            .with_env("ROLE", "technical_writer")
    ).await?;
    info!("Spawned documenter agent: {}", documenter);

    // Wait for agents to start
    sleep(Duration::from_secs(2)).await;

    // Collaborative workflow
    info!("\n=== Starting Collaborative Workflow ===");

    // Phase 1: Architecture Design
    info!("\n[Phase 1] Architecture Design");
    
    // Architect reads project context
    if let Some(project) = context.get("project").await {
        info!("Architect accessing project: {:?}", project.value);
    }
    
    // Architect designs a new feature
    let design_request = "Design a monitoring dashboard feature for the multi-agent system";
    let design = pool.send_message(architect, design_request.to_string()).await?;
    info!("Architecture design complete: {}", design);
    
    // Store design in shared context
    context.set(
        architect,
        "monitoring_design".to_string(),
        json!({
            "feature": "Monitoring Dashboard",
            "components": ["MetricsCollector", "WebUI", "AlertSystem"],
            "architect": architect.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    ).await?;

    // Phase 2: Implementation
    info!("\n[Phase 2] Implementation");
    
    // Developer reads the design from context
    let design_context = context.get("monitoring_design").await
        .map(|v| v.value)
        .unwrap_or(json!({}));
    
    let implementation_request = format!(
        "Implement the monitoring dashboard based on this design: {}",
        design_context
    );
    let implementation = pool.send_message(developer, implementation_request).await?;
    info!("Implementation complete: {}", implementation);
    
    // Update context with implementation status
    context.set(
        developer,
        "monitoring_implementation".to_string(),
        json!({
            "status": "implemented",
            "modules_created": ["metrics.rs", "dashboard.rs"],
            "developer": developer.to_string(),
            "lines_of_code": 500
        })
    ).await?;

    // Phase 3: Testing
    info!("\n[Phase 3] Testing");
    
    let test_request = format!(
        "Write comprehensive tests for the monitoring implementation: {}",
        implementation
    );
    let tests = pool.send_message(tester, test_request).await?;
    info!("Tests created: {}", tests);
    
    // Update test results in context
    context.set(
        tester,
        "test_results".to_string(),
        json!({
            "total_tests": 25,
            "passed": 23,
            "failed": 2,
            "coverage": "85%",
            "tester": tester.to_string()
        })
    ).await?;

    // Phase 4: Documentation
    info!("\n[Phase 4] Documentation");
    
    // Documenter reads all context
    let all_keys = context.keys().await;
    info!("Documenter accessing context keys: {:?}", all_keys);
    
    let doc_request = "Create comprehensive documentation for the monitoring dashboard feature";
    let documentation = pool.send_message(documenter, doc_request.to_string()).await?;
    info!("Documentation complete: {}", documentation);

    // Phase 5: Review Meeting (broadcast)
    info!("\n[Phase 5] Team Review Meeting");
    
    let meeting_message = "Team meeting: Please provide your status update on the monitoring dashboard feature";
    let updates = pool.broadcast(meeting_message.to_string()).await;
    
    info!("\nTeam Updates:");
    for (agent_id, result) in updates {
        if let Ok(update) = result {
            let role = match agent_id {
                id if id == architect => "Architect",
                id if id == developer => "Developer",
                id if id == tester => "Tester",
                id if id == documenter => "Documenter",
                _ => "Unknown",
            };
            info!("{}: {}", role, update);
        }
    }

    // Show final context state
    info!("\n=== Final Shared Context ===");
    for key in context.keys().await {
        if let Some(value) = context.get(&key).await {
            info!("Key: {} | Updated by: {} | Value: {}", 
                key, value.updated_by, value.value);
        }
    }

    // Show collaboration metrics
    info!("\n=== Collaboration Metrics ===");
    let history = context.history().await;
    info!("Total context updates: {}", history.len());
    
    let metrics = pool.metrics().await;
    info!("Active agents: {}/{}", metrics.running_agents, metrics.total_agents);
    info!("Messages exchanged: {}", metrics.messages_routed);

    // Simulate continuous monitoring
    info!("\n=== Starting Monitoring Loop ===");
    let mut monitor_interval = interval(Duration::from_secs(5));
    let mut ticks = 0;
    
    loop {
        monitor_interval.tick().await;
        ticks += 1;
        
        // Collect metrics from all agents
        let metrics = pool.metrics().await;
        info!("Monitor tick {}: CPU: {:.2}%, Memory: {}MB, Messages: {}", 
            ticks, metrics.total_cpu_percent, metrics.total_memory_mb, metrics.messages_routed);
        
        // Stop after 3 ticks
        if ticks >= 3 {
            break;
        }
        
        // Simulate some activity
        let _ = pool.send_message(
            developer, 
            "Check system health".to_string()
        ).await;
    }

    // Graceful shutdown
    info!("\n=== Shutting down collaborative agents ===");
    pool.shutdown().await?;
    info!("All agents stopped successfully");

    Ok(())
}