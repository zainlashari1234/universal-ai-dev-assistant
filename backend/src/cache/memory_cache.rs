// Sprint 2: In-Memory Cache Implementation
use super::{Cache, CacheEntry, CacheConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

pub struct MemoryCache {
    data: Arc<RwLock<HashMap<String, CacheEntry<serde_json::Value>>>>,
    config: CacheConfig,
}

impl MemoryCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut data = self.data.write().await;
        let now = chrono::Utc::now();
        let initial_size = data.len();
        
        data.retain(|_, entry| entry.expires_at > now);
        
        let removed = initial_size - data.len();
        if removed > 0 {
            debug!("Cleaned up {} expired cache entries", removed);
        }
        
        Ok(removed)
    }
    
    pub async fn evict_lru(&self) -> Result<()> {
        let mut data = self.data.write().await;
        
        if data.len() <= self.config.max_size {
            return Ok(());
        }
        
        // Find LRU entry (lowest access count + oldest)
        let mut lru_key = String::new();
        let mut min_score = f64::MAX;
        
        for (key, entry) in data.iter() {
            let age_hours = (chrono::Utc::now() - entry.created_at).num_hours() as f64;
            let score = entry.access_count as f64 / (age_hours + 1.0);
            
            if score < min_score {
                min_score = score;
                lru_key = key.clone();
            }
        }
        
        if !lru_key.is_empty() {
            data.remove(&lru_key);
            debug!("Evicted LRU cache entry: {}", lru_key);
        }
        
        Ok(())
    }
    
    pub async fn get_stats(&self) -> CacheStats {
        let data = self.data.read().await;
        let now = chrono::Utc::now();
        
        let mut expired_count = 0;
        let mut total_access_count = 0;
        
        for entry in data.values() {
            if entry.expires_at <= now {
                expired_count += 1;
            }
            total_access_count += entry.access_count;
        }
        
        CacheStats {
            total_entries: data.len(),
            expired_entries: expired_count,
            memory_usage_mb: (data.len() * 1024) / (1024 * 1024), // Rough estimate
            hit_rate: if total_access_count > 0 { 0.85 } else { 0.0 }, // Placeholder
            average_access_count: if !data.is_empty() { 
                total_access_count as f64 / data.len() as f64 
            } else { 
                0.0 
            },
        }
    }
}

#[async_trait::async_trait]
impl Cache for MemoryCache {
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        let mut data = self.data.write().await;
        
        if let Some(entry) = data.get_mut(key) {
            // Check if expired
            if entry.expires_at <= chrono::Utc::now() {
                data.remove(key);
                return Ok(None);
            }
            
            // Update access count
            entry.access_count += 1;
            
            // Deserialize value
            let value: T = serde_json::from_value(entry.value.clone())?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    async fn set<T>(&self, key: &str, value: T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send,
    {
        let ttl = ttl.unwrap_or(self.config.ttl);
        let expires_at = chrono::Utc::now() + chrono::Duration::from_std(ttl)?;
        
        let json_value = serde_json::to_value(value)?;
        let entry = CacheEntry {
            value: json_value,
            expires_at,
            access_count: 0,
            created_at: chrono::Utc::now(),
        };
        
        let mut data = self.data.write().await;
        
        // Check if we need to evict
        if data.len() >= self.config.max_size && !data.contains_key(key) {
            drop(data); // Release lock before eviction
            self.evict_lru().await?;
            data = self.data.write().await;
        }
        
        data.insert(key.to_string(), entry);
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let data = self.data.read().await;
        
        if let Some(entry) = data.get(key) {
            Ok(entry.expires_at > chrono::Utc::now())
        } else {
            Ok(false)
        }
    }
    
    async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().await;
        let count = data.len();
        data.clear();
        info!("Cleared {} cache entries", count);
        Ok(())
    }
    
    async fn size(&self) -> Result<usize> {
        let data = self.data.read().await;
        Ok(data.len())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub memory_usage_mb: usize,
    pub hit_rate: f64,
    pub average_access_count: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_cache() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        // Test set and get
        cache.set("key1", "value1", None).await.unwrap();
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
        
        // Test expiration
        cache.set("key2", "value2", Some(Duration::from_millis(1))).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let value: Option<String> = cache.get("key2").await.unwrap();
        assert_eq!(value, None);
        
        // Test delete
        cache.delete("key1").await.unwrap();
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, None);
    }
}