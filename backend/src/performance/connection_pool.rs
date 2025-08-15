// Sprint 2: Connection Pool Management
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone)]
pub struct ConnectionPoolManager {
    max_connections: usize,
    active_connections: Arc<RwLock<usize>>,
    pool_stats: Arc<RwLock<PoolStats>>,
}

#[derive(Debug, Default)]
pub struct PoolStats {
    pub total_created: usize,
    pub total_closed: usize,
    pub peak_usage: usize,
    pub average_lifetime_ms: u64,
}

impl ConnectionPoolManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            active_connections: Arc::new(RwLock::new(0)),
            pool_stats: Arc::new(RwLock::new(PoolStats::default())),
        }
    }
    
    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }
    
    pub async fn acquire_connection(&self) -> Result<bool> {
        let mut active = self.active_connections.write().await;
        
        if *active >= self.max_connections {
            warn!("Connection pool exhausted: {}/{}", *active, self.max_connections);
            return Ok(false);
        }
        
        *active += 1;
        
        // Update stats
        {
            let mut stats = self.pool_stats.write().await;
            stats.total_created += 1;
            if *active > stats.peak_usage {
                stats.peak_usage = *active;
            }
        }
        
        Ok(true)
    }
    
    pub async fn release_connection(&self) {
        let mut active = self.active_connections.write().await;
        if *active > 0 {
            *active -= 1;
            
            // Update stats
            let mut stats = self.pool_stats.write().await;
            stats.total_closed += 1;
        }
    }
    
    pub async fn reduce_connections(&self) -> Result<()> {
        info!("Reducing connection pool size due to high usage");
        
        let current = *self.active_connections.read().await;
        let target_reduction = (current as f64 * 0.2) as usize; // Reduce by 20%
        
        for _ in 0..target_reduction {
            self.release_connection().await;
        }
        
        info!("Reduced connections by {}", target_reduction);
        Ok(())
    }
    
    pub async fn get_stats(&self) -> PoolStats {
        self.pool_stats.read().await.clone()
    }
}