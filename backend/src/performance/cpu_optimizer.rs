// Sprint 2: CPU Optimization
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn};

#[derive(Clone)]
pub struct CpuOptimizer {
    max_cpu_percent: f64,
    thread_pool: Arc<rayon::ThreadPool>,
    task_semaphore: Arc<Semaphore>,
    current_load: Arc<RwLock<f64>>,
}

impl CpuOptimizer {
    pub fn new(max_cpu_percent: f64) -> Self {
        let num_cpus = num_cpus::get();
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus)
            .build()
            .expect("Failed to create thread pool");
        
        Self {
            max_cpu_percent,
            thread_pool: Arc::new(thread_pool),
            task_semaphore: Arc::new(Semaphore::new(num_cpus * 2)),
            current_load: Arc::new(RwLock::new(0.0)),
        }
    }
    
    pub async fn get_cpu_usage(&self) -> f64 {
        // Simulate CPU usage calculation
        let load = *self.current_load.read().await;
        load.min(100.0)
    }
    
    pub async fn reduce_load(&self) -> Result<()> {
        warn!("High CPU usage detected, reducing load");
        
        // Reduce concurrent tasks
        let current_permits = self.task_semaphore.available_permits();
        if current_permits > 1 {
            // Temporarily reduce available permits
            let _permits = self.task_semaphore.acquire_many(current_permits as u32 / 2).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        
        info!("CPU load reduction applied");
        Ok(())
    }
    
    pub async fn execute_cpu_intensive_task<F, T>(&self, task: F) -> Result<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let _permit = self.task_semaphore.acquire().await?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        self.thread_pool.spawn(move || {
            let result = task();
            let _ = tx.send(result);
        });
        
        Ok(rx.await?)
    }
    
    pub async fn set_load(&self, load: f64) {
        *self.current_load.write().await = load;
    }
}