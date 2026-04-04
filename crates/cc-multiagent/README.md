# cc-multiagent

Multi-Agent Control Plane for Claude Code RS - A powerful system for orchestrating multiple AI agents in parallel with shared context and inter-agent communication.

## Features

- **Agent Pool Management**: Spawn, monitor, and control multiple AI agents
- **Shared Context**: Global and agent-private key-value stores for collaboration
- **Message Routing**: Direct, broadcast, round-robin, and custom routing strategies
- **IPC Communication**: Unix socket-based inter-process communication
- **Health Monitoring**: Automatic health checks and restart policies
- **Resource Limits**: CPU and memory constraints per agent and pool-wide
- **Metrics Collection**: Comprehensive metrics with Prometheus export support
- **Type Safety**: Full Rust type safety with async/await support

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Control Plane                       │
├─────────────────┬────────────┬─────────────────────┤
│   Agent Pool    │  Message   │   Shared Context    │
│                 │  Router    │                     │
├─────────────────┴────────────┴─────────────────────┤
│                    IPC Layer                        │
├──────────┬──────────┬──────────┬──────────────────┤
│  Agent 1 │  Agent 2 │  Agent 3 │      ...         │
│  (Opus)  │ (Sonnet) │  (GPT-4) │                  │
└──────────┴──────────┴──────────┴──────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
cc-multiagent = { path = "../cc-multiagent" }
```

Basic usage:

```rust
use cc_multiagent::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create agent pool
    let pool = AgentPool::new(PoolConfig::default()).await?;
    
    // Spawn agents
    let researcher = pool.spawn(
        AgentConfig::new("researcher", "claude-3-opus")
            .with_tag("research")
    ).await?;
    
    let coder = pool.spawn(
        AgentConfig::new("coder", "claude-3-sonnet")
            .with_tag("coding")
    ).await?;
    
    // Send tasks
    let research = pool.send_message(
        researcher, 
        "Research Rust async patterns"
    ).await?;
    
    let code = pool.send_message(
        coder,
        format!("Implement this: {}", research)
    ).await?;
    
    println!("Implementation: {}", code);
    
    // Graceful shutdown
    pool.shutdown().await?;
    Ok(())
}
```

## Advanced Usage

### Shared Context

```rust
let context = SharedContext::new(1000);

// Set global state
context.set(
    agent_id,
    "project_status".to_string(),
    json!({"phase": "development", "progress": 0.75})
).await?;

// Agent-private state
context.set_agent_context(
    agent_id,
    "internal_state".to_string(),
    json!({"memory": "important data"})
).await?;
```

### Message Routing

```rust
use cc_multiagent::router::{MessageRouter, MessageTarget};

let router = MessageRouter::new();

// Direct message
let msg = MessageBuilder::new(from, MessageTarget::Agent(to))
    .content("Direct message")
    .build();

// Broadcast
let msg = MessageBuilder::new(from, MessageTarget::Broadcast)
    .content("Announcement")
    .build();

// Round-robin distribution
let msg = MessageBuilder::new(from, MessageTarget::RoundRobin)
    .content("Work item")
    .build();

router.route(msg).await?;
```

### Resource Limits

```rust
let config = PoolConfig {
    max_agents: 10,
    total_memory_limit_mb: 16384, // 16GB total
    total_cpu_limit_percent: 800.0, // 8 cores
    ..Default::default()
};

let agent_config = AgentConfig::new("worker", "model")
    .with_memory_limit(2048) // 2GB per agent
    .with_cpu_affinity(vec![0, 1]); // Pin to cores 0,1
```

### Metrics & Monitoring

```rust
use cc_multiagent::metrics::MetricsCollector;

let collector = MetricsCollector::new(24); // 24hr retention

// Collect metrics periodically
let metrics = pool.metrics().await;
collector.record_pool(metrics).await;

// Export to Prometheus
#[cfg(feature = "prometheus")]
{
    let exporter = PrometheusExporter::new();
    exporter.update_pool_metrics(&metrics);
    let prometheus_text = exporter.encode();
}
```

## Examples

See the `examples/` directory for complete examples:

- `basic_multi_agent.rs` - Simple multi-agent setup
- `collaborative_agents.rs` - Complex workflow with shared context
- More examples coming soon!

## Testing

Run tests with:

```bash
cargo test
```

Run examples:

```bash
cargo run --example basic_multi_agent
cargo run --example collaborative_agents
```

## Features

- `default` - Core functionality with gRPC and basic metrics
- `grpc` - gRPC support for distributed agents
- `metrics` - Metrics collection
- `prometheus` - Prometheus metrics export
- `opentelemetry` - OpenTelemetry support

## Integration with Claude Code RS

This crate is designed to integrate seamlessly with the main Claude Code RS project:

```rust
// In cc-cli or cc-engine
use cc_multiagent::prelude::*;

// Add multi-agent command
#[derive(Parser)]
enum Command {
    // ... existing commands ...
    
    /// Launch multi-agent control plane
    MultiAgent {
        #[clap(long, default_value = "5")]
        max_agents: usize,
        
        #[clap(long)]
        config: Option<PathBuf>,
    },
}
```

## Roadmap

- [ ] Distributed agent support (cross-machine)
- [ ] Web UI for monitoring
- [ ] Agent templates and presets
- [ ] Advanced scheduling policies
- [ ] State persistence and recovery
- [ ] WebRTC support for real-time communication
- [ ] Plugin system for custom agent types

## License

Licensed under the MIT License

## Contributing

Contributions are welcome! Please read the contributing guidelines in the main repository.

## Author

Created by Kirill (kk7453603) as part of the Claude Code RS project.