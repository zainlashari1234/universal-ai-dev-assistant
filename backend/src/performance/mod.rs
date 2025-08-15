// Sprint 2: Performance Optimization
pub mod cpu_optimizer;
pub mod memory_manager;
pub mod connection_pool;
pub mod async_executor;

pub use cpu_optimizer::*;
pub use memory_manager::*;
pub use connection_pool::*;
pub use async_executor::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: usize,
    pub request_latency_p95: Duration,
    pub throughput_rps: f64,
    pub error_rate: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_cpu_percent: f64,
    pub max_memory_mb: usize,
    pub max_connections: usize,
    pub target_latency_ms: u64,
    pub enable_auto_scaling: bool,
    pub enable_caching: bool,
}

/// Performance Monitor - tracks and optimizes system performance
pub struct PerformanceMonitor {
    config: PerformanceConfig,
    metrics: RwLock<PerformanceMetrics>,
    cpu_optimizer: CpuOptimizer,
    memory_manager: MemoryManager,
    connection_pool: ConnectionPoolManager,
}

impl PerformanceMonitor {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            cpu_optimizer: CpuOptimizer::new(config.max_cpu_percent),
            memory_manager: MemoryManager::new(config.max_memory_mb),
            connection_pool: ConnectionPoolManager::new(config.max_connections),
            metrics: RwLock::new(PerformanceMetrics::default()),
            config,
        }
    }
    
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting performance monitoring");
        
        // Start background monitoring tasks
        let monitor = self.clone();
        tokio::spawn(async move {
            monitor.monitoring_loop().await;
        });
        
        Ok(())
    }
    
    async fn monitoring_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.collect_metrics().await {
                warn!("Failed to collect metrics: {}", e);
            }
            
            if let Err(e) = self.optimize_performance().await {
                warn!("Failed to optimize performance: {}", e);
            }
        }
    }
    
    async fn collect_metrics(&self) -> Result<()> {
        let cpu_usage = self.cpu_optimizer.get_cpu_usage().await;
        let memory_usage = self.memory_manager.get_memory_usage().await;
        let active_connections = self.connection_pool.get_active_connections().await;
        
        let metrics = PerformanceMetrics {
            cpu_usage,
            memory_usage,
            active_connections,
            request_latency_p95: Duration::from_millis(50), // Placeholder
            throughput_rps: 100.0, // Placeholder
            error_rate: 0.01, // Placeholder
            timestamp: chrono::Utc::now(),
        };
        
        *self.metrics.write().await = metrics;
        Ok(())
    }
    
    async fn optimize_performance(&self) -> Result<()> {
        let metrics = self.metrics.read().await.clone();
        
        // CPU optimization
        if metrics.cpu_usage > self.config.max_cpu_percent {
            self.cpu_optimizer.reduce_load().await?;
        }
        
        // Memory optimization
        if metrics.memory_usage > self.config.max_memory_mb as f64 {
            self.memory_manager.free_memory().await?;
        }
        
        // Connection optimization
        if metrics.active_connections > self.config.max_connections {
            self.connection_pool.reduce_connections().await?;
        }
        
        Ok(())
    }
    
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }
}

impl Clone for PerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            metrics: RwLock::new(PerformanceMetrics::default()),
            cpu_optimizer: self.cpu_optimizer.clone(),
            memory_manager: self.memory_manager.clone(),
            connection_pool: self.connection_pool.clone(),
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_connections: 0,
            request_latency_p95: Duration::from_millis(0),
            throughput_rps: 0.0,
            error_rate: 0.0,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_cpu_percent: 80.0,
            max_memory_mb: 1024,
            max_connections: 100,
            target_latency_ms: 100,
            enable_auto_scaling: true,
            enable_caching: true,
        }
    }
}