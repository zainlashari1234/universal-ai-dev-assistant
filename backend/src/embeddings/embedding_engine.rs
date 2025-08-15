// Sprint 2: Embedding engine implementation
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub input: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: usize,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub total_tokens: usize,
}

pub struct EmbeddingEngine {
    model_name: String,
    client: Client,
    cache: RwLock<lru::LruCache<String, Vec<f32>>>,
    ollama_url: String,
}

impl EmbeddingEngine {
    pub async fn new(model_name: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        let cache = RwLock::new(lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()));
        
        let engine = Self {
            model_name: model_name.to_string(),
            client,
            cache,
            ollama_url: "http://localhost:11434".to_string(),
        };
        
        // Verify model availability
        engine.verify_model().await?;
        
        info!("Embedding engine initialized with model: {}", model_name);
        Ok(engine)
    }
    
    async fn verify_model(&self) -> Result<()> {
        // Try to get a test embedding
        match self.embed_uncached("test").await {
            Ok(_) => {
                info!("Model {} is available and working", self.model_name);
                Ok(())
            }
            Err(e) => {
                warn!("Model {} verification failed: {}", self.model_name, e);
                // Try to pull the model
                self.pull_model().await?;
                Ok(())
            }
        }
    }
    
    async fn pull_model(&self) -> Result<()> {
        info!("Pulling embedding model: {}", self.model_name);
        
        let response = self.client
            .post(&format!("{}/api/pull", self.ollama_url))
            .json(&serde_json::json!({
                "name": self.model_name,
                "stream": false
            }))
            .send()
            .await?;
        
        if response.status().is_success() {
            info!("Model {} pulled successfully", self.model_name);
        } else {
            warn!("Failed to pull model {}: {}", self.model_name, response.status());
        }
        
        Ok(())
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.peek(text) {
                return Ok(cached.clone());
            }
        }
        
        // Generate embedding
        let embedding = self.embed_uncached(text).await?;
        
        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.put(text.to_string(), embedding.clone());
        }
        
        Ok(embedding)
    }
    
    async fn embed_uncached(&self, text: &str) -> Result<Vec<f32>> {
        // Try Ollama first
        if let Ok(embedding) = self.embed_ollama(text).await {
            return Ok(embedding);
        }
        
        // Fallback to local embedding
        self.embed_local(text).await
    }
    
    async fn embed_ollama(&self, text: &str) -> Result<Vec<f32>> {
        let request = serde_json::json!({
            "model": self.model_name,
            "prompt": text
        });
        
        let response = self.client
            .post(&format!("{}/api/embeddings", self.ollama_url))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Ollama embedding failed: {}", response.status()));
        }
        
        let result: serde_json::Value = response.json().await?;
        
        if let Some(embedding) = result.get("embedding") {
            let vector: Vec<f32> = serde_json::from_value(embedding.clone())?;
            Ok(vector)
        } else {
            Err(anyhow::anyhow!("No embedding in response"))
        }
    }
    
    async fn embed_local(&self, text: &str) -> Result<Vec<f32>> {
        // Simple local embedding using character-based features
        // This is a fallback when no proper embedding model is available
        
        let chars: Vec<char> = text.chars().collect();
        let mut features = vec![0.0; 384]; // Standard embedding dimension
        
        // Character frequency features
        for (i, &ch) in chars.iter().enumerate() {
            let idx = (ch as u32 % 384) as usize;
            features[idx] += 1.0 / (chars.len() as f32);
        }
        
        // Length features
        features[0] = (chars.len() as f32).ln() / 10.0;
        
        // Word count features
        let word_count = text.split_whitespace().count() as f32;
        features[1] = word_count.ln() / 5.0;
        
        // Normalize
        let norm: f32 = features.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for feature in &mut features {
                *feature /= norm;
            }
        }
        
        Ok(features)
    }
    
    pub async fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        
        for text in texts {
            let embedding = self.embed(text).await?;
            results.push(embedding);
        }
        
        Ok(results)
    }
    
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Embedding cache cleared");
    }
    
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), cache.cap().get())
    }
    
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
    
    pub fn set_ollama_url(&mut self, url: String) {
        self.ollama_url = url;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_embedding_engine() {
        let engine = EmbeddingEngine::new("all-MiniLM-L6-v2").await.unwrap();
        
        let text = "Hello, world!";
        let embedding = engine.embed(text).await.unwrap();
        
        assert_eq!(embedding.len(), 384);
        
        // Test that embeddings are consistent
        let embedding2 = engine.embed(text).await.unwrap();
        assert_eq!(embedding, embedding2);
    }
    
    #[tokio::test]
    async fn test_local_embedding() {
        let engine = EmbeddingEngine::new("test-model").await.unwrap();
        
        let embedding = engine.embed_local("test text").await.unwrap();
        assert_eq!(embedding.len(), 384);
        
        // Test that different texts produce different embeddings
        let embedding2 = engine.embed_local("different text").await.unwrap();
        assert_ne!(embedding, embedding2);
    }
    
    #[tokio::test]
    async fn test_batch_embedding() {
        let engine = EmbeddingEngine::new("test-model").await.unwrap();
        
        let texts = vec!["text 1", "text 2", "text 3"];
        let embeddings = engine.embed_batch(texts).await.unwrap();
        
        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 384);
        }
    }
}