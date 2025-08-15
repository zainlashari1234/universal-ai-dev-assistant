// Sprint 2: Memory Management
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone)]
pub struct MemoryManager {
    max_memory_mb: usize,
    current_usage: Arc<RwLock<usize>>,
    gc_threshold: f64,
}

impl MemoryManager {
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            max_memory_mb,
            current_usage: Arc::new(RwLock::new(0)),
            gc_threshold: 0.8, // Trigger GC at 80% usage
        }
    }
    
    pub async fn get_memory_usage(&self) -> f64 {
        let current = *self.current_usage.read().await;
        (current as f64 / self.max_memory_mb as f64) * 100.0
    }
    
    pub async fn allocate(&self, size_mb: usize) -> Result<bool> {
        let mut current = self.current_usage.write().await;
        
        if *current + size_mb > self.max_memory_mb {
            warn!("Memory allocation failed: {} MB requested, {} MB available", 
                  size_mb, self.max_memory_mb - *current);
            return Ok(false);
        }
        
        *current += size_mb;
        Ok(true)
    }
    
    pub async fn deallocate(&self, size_mb: usize) {
        let mut current = self.current_usage.write().await;
        *current = current.saturating_sub(size_mb);
    }
    
    pub async fn free_memory(&self) -> Result<()> {
        info!("Triggering memory cleanup");
        
        // Simulate garbage collection
        let current = *self.current_usage.read().await;
        let freed = (current as f64 * 0.2) as usize; // Free 20%
        
        self.deallocate(freed).await;
        
        info!("Memory cleanup completed, freed {} MB", freed);
        Ok(())
    }
    
    pub async fn should_trigger_gc(&self) -> bool {
        let usage_percent = self.get_memory_usage().await / 100.0;
        usage_percent > self.gc_threshold
    }
}