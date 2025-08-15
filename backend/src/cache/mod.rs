// Sprint 2: Advanced Caching System
pub mod redis_cache;
pub mod memory_cache;
pub mod cache_manager;

pub use redis_cache::*;
pub use memory_cache::*;
pub use cache_manager::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub value: T,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub access_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub ttl: Duration,
    pub max_size: usize,
    pub enable_redis: bool,
    pub redis_url: String,
}

#[async_trait::async_trait]
pub trait Cache: Send + Sync {
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send;
    
    async fn set<T>(&self, key: &str, value: T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send;
    
    async fn delete(&self, key: &str) -> Result<()>;
    
    async fn exists(&self, key: &str) -> Result<bool>;
    
    async fn clear(&self) -> Result<()>;
    
    async fn size(&self) -> Result<usize>;
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(3600),
            max_size: 1000,
            enable_redis: false,
            redis_url: "redis://localhost:6379".to_string(),
        }
    }
}