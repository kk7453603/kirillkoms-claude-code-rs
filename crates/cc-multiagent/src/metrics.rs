use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::AgentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: AgentId,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub errors: u64,
    pub uptime_seconds: u64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub total_agents: usize,
    pub running_agents: usize,
    pub failed_agents: usize,
    pub total_cpu_percent: f32,
    pub total_memory_mb: u64,
    pub total_messages: u64,
    pub messages_per_second: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub host_cpu_percent: f32,
    pub host_memory_mb: u64,
    pub host_memory_total_mb: u64,
    pub load_average: [f32; 3],
    pub timestamp: DateTime<Utc>,
}

pub struct MetricsCollector {
    agent_metrics: Arc<RwLock<HashMap<AgentId, Vec<AgentMetrics>>>>,
    pool_metrics: Arc<RwLock<Vec<PoolMetrics>>>,
    system_metrics: Arc<RwLock<Vec<SystemMetrics>>>,
    retention_duration: Duration,
}

impl MetricsCollector {
    pub fn new(retention_hours: i64) -> Self {
        Self {
            agent_metrics: Arc::new(RwLock::new(HashMap::new())),
            pool_metrics: Arc::new(RwLock::new(Vec::new())),
            system_metrics: Arc::new(RwLock::new(Vec::new())),
            retention_duration: Duration::hours(retention_hours),
        }
    }
    
    /// Record agent metrics
    pub async fn record_agent(&self, metrics: AgentMetrics) {
        let mut agent_metrics = self.agent_metrics.write().await;
        let history = agent_metrics.entry(metrics.agent_id).or_insert_with(Vec::new);
        history.push(metrics);
        
        // Clean old metrics
        let cutoff = Utc::now() - self.retention_duration;
        history.retain(|m| m.last_updated > cutoff);
    }
    
    /// Record pool metrics
    pub async fn record_pool(&self, metrics: PoolMetrics) {
        let mut pool_metrics = self.pool_metrics.write().await;
        pool_metrics.push(metrics);
        
        // Clean old metrics
        let cutoff = Utc::now() - self.retention_duration;
        pool_metrics.retain(|m| m.timestamp > cutoff);
    }
    
    /// Record system metrics
    pub async fn record_system(&self, metrics: SystemMetrics) {
        let mut system_metrics = self.system_metrics.write().await;
        system_metrics.push(metrics);
        
        // Clean old metrics
        let cutoff = Utc::now() - self.retention_duration;
        system_metrics.retain(|m| m.timestamp > cutoff);
    }
    
    /// Get agent metrics for a time range
    pub async fn get_agent_metrics(
        &self,
        agent_id: AgentId,
        since: DateTime<Utc>,
    ) -> Vec<AgentMetrics> {
        let agent_metrics = self.agent_metrics.read().await;
        
        agent_metrics.get(&agent_id)
            .map(|history| {
                history.iter()
                    .filter(|m| m.last_updated >= since)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get pool metrics for a time range
    pub async fn get_pool_metrics(&self, since: DateTime<Utc>) -> Vec<PoolMetrics> {
        let pool_metrics = self.pool_metrics.read().await;
        
        pool_metrics.iter()
            .filter(|m| m.timestamp >= since)
            .cloned()
            .collect()
    }
    
    /// Calculate agent statistics
    pub async fn agent_stats(&self, agent_id: AgentId) -> Option<AgentStats> {
        let metrics = self.get_agent_metrics(
            agent_id,
            Utc::now() - Duration::hours(1)
        ).await;
        
        if metrics.is_empty() {
            return None;
        }
        
        let cpu_avg = metrics.iter().map(|m| m.cpu_percent).sum::<f32>() / metrics.len() as f32;
        let cpu_max = metrics.iter().map(|m| m.cpu_percent).fold(0.0, f32::max);
        let memory_avg = metrics.iter().map(|m| m.memory_mb).sum::<u64>() / metrics.len() as u64;
        let memory_max = metrics.iter().map(|m| m.memory_mb).max().unwrap_or(0);
        let total_messages = metrics.last()?.messages_sent + metrics.last()?.messages_received;
        let total_errors = metrics.last()?.errors;
        let uptime = metrics.last()?.uptime_seconds;
        
        Some(AgentStats {
            agent_id,
            cpu_avg,
            cpu_max,
            memory_avg,
            memory_max,
            total_messages,
            total_errors,
            uptime_seconds: uptime,
            sample_count: metrics.len(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub agent_id: AgentId,
    pub cpu_avg: f32,
    pub cpu_max: f32,
    pub memory_avg: u64,
    pub memory_max: u64,
    pub total_messages: u64,
    pub total_errors: u64,
    pub uptime_seconds: u64,
    pub sample_count: usize,
}

/// Collect system metrics using sysinfo
pub async fn collect_system_metrics() -> SystemMetrics {
    use sysinfo::System;
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let load_avg = sys.load_average();
    
    SystemMetrics {
        host_cpu_percent: sys.global_cpu_usage(),
        host_memory_mb: (sys.used_memory() / 1024 / 1024) as u64,
        host_memory_total_mb: (sys.total_memory() / 1024 / 1024) as u64,
        load_average: [
            load_avg.one as f32,
            load_avg.five as f32,
            load_avg.fifteen as f32,
        ],
        timestamp: Utc::now(),
    }
}

#[cfg(feature = "prometheus")]
pub mod prometheus_exporter {
    use super::*;
    use prometheus_client::{
        encoding::EncodeLabelSet,
        metrics::{counter::Counter, gauge::Gauge, family::Family},
        registry::Registry,
    };
    
    #[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
    pub struct AgentLabels {
        pub agent_id: String,
        pub model: String,
    }
    
    pub struct PrometheusExporter {
        registry: Registry,
        agent_cpu: Family<AgentLabels, Gauge<f64>>,
        agent_memory: Family<AgentLabels, Gauge<f64>>,
        agent_messages: Family<AgentLabels, Counter>,
        pool_total_agents: Gauge<f64>,
        pool_running_agents: Gauge<f64>,
    }
    
    impl PrometheusExporter {
        pub fn new() -> Self {
            let mut registry = Registry::default();
            
            let agent_cpu = Family::default();
            registry.register(
                "multiagent_agent_cpu_percent",
                "CPU usage percentage per agent",
                agent_cpu.clone(),
            );
            
            let agent_memory = Family::default();
            registry.register(
                "multiagent_agent_memory_mb",
                "Memory usage in MB per agent",
                agent_memory.clone(),
            );
            
            let agent_messages = Family::default();
            registry.register(
                "multiagent_agent_messages_total",
                "Total messages processed per agent",
                agent_messages.clone(),
            );
            
            let pool_total_agents = Gauge::default();
            registry.register(
                "multiagent_pool_total_agents",
                "Total number of agents in pool",
                pool_total_agents.clone(),
            );
            
            let pool_running_agents = Gauge::default();
            registry.register(
                "multiagent_pool_running_agents",
                "Number of running agents",
                pool_running_agents.clone(),
            );
            
            Self {
                registry,
                agent_cpu,
                agent_memory,
                agent_messages,
                pool_total_agents,
                pool_running_agents,
            }
        }
        
        pub fn update_agent_metrics(&self, metrics: &AgentMetrics, model: &str) {
            let labels = AgentLabels {
                agent_id: metrics.agent_id.to_string(),
                model: model.to_string(),
            };
            
            self.agent_cpu.get_or_create(&labels).set(metrics.cpu_percent as f64);
            self.agent_memory.get_or_create(&labels).set(metrics.memory_mb as f64);
            self.agent_messages.get_or_create(&labels).inc_by(
                metrics.messages_sent + metrics.messages_received
            );
        }
        
        pub fn update_pool_metrics(&self, metrics: &PoolMetrics) {
            self.pool_total_agents.set(metrics.total_agents as f64);
            self.pool_running_agents.set(metrics.running_agents as f64);
        }
        
        pub fn encode(&self) -> String {
            use prometheus_client::encoding::text::encode;
            let mut buffer = String::new();
            encode(&mut buffer, &self.registry).unwrap();
            buffer
        }
    }
}