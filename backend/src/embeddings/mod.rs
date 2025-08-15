// Sprint 2: Embeddings + sqlite-vss implementation
pub mod embedding_engine;
pub mod vector_store;
pub mod similarity;

pub use embedding_engine::*;
pub use vector_store::*;
pub use similarity::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub id: String,
    pub content: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingQuery {
    pub text: String,
    pub limit: usize,
    pub threshold: f32,
    pub filters: Option<HashMap<String, String>>,
}

/// Main embedding service that coordinates all embedding operations
pub struct EmbeddingService {
    engine: EmbeddingEngine,
    vector_store: VectorStore,
}

impl EmbeddingService {
    pub async fn new(model_name: &str, db_path: &str) -> Result<Self> {
        let engine = EmbeddingEngine::new(model_name).await?;
        let vector_store = VectorStore::new(db_path).await?;
        
        Ok(Self {
            engine,
            vector_store,
        })
    }
    
    /// Add content to the vector store
    pub async fn add_content(&self, content: &str, metadata: HashMap<String, String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let vector = self.engine.embed(content).await?;
        
        let embedding = EmbeddingVector {
            id: id.clone(),
            content: content.to_string(),
            vector,
            metadata,
            created_at: chrono::Utc::now(),
        };
        
        self.vector_store.store_embedding(&embedding).await?;
        Ok(id)
    }
    
    /// Search for similar content
    pub async fn search(&self, query: &EmbeddingQuery) -> Result<Vec<SimilarityResult>> {
        let query_vector = self.engine.embed(&query.text).await?;
        let results = self.vector_store.search_similar(
            &query_vector,
            query.limit,
            query.threshold,
            query.filters.as_ref(),
        ).await?;
        
        Ok(results)
    }
    
    /// Batch add multiple contents
    pub async fn add_batch(&self, contents: Vec<(String, HashMap<String, String>)>) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        
        for (content, metadata) in contents {
            let id = self.add_content(&content, metadata).await?;
            ids.push(id);
        }
        
        Ok(ids)
    }
    
    /// Update existing content
    pub async fn update_content(&self, id: &str, content: &str, metadata: HashMap<String, String>) -> Result<()> {
        let vector = self.engine.embed(content).await?;
        
        let embedding = EmbeddingVector {
            id: id.to_string(),
            content: content.to_string(),
            vector,
            metadata,
            created_at: chrono::Utc::now(),
        };
        
        self.vector_store.update_embedding(&embedding).await?;
        Ok(())
    }
    
    /// Delete content by ID
    pub async fn delete_content(&self, id: &str) -> Result<()> {
        self.vector_store.delete_embedding(id).await?;
        Ok(())
    }
    
    /// Get content by ID
    pub async fn get_content(&self, id: &str) -> Result<Option<EmbeddingVector>> {
        self.vector_store.get_embedding(id).await
    }
    
    /// Clear all embeddings
    pub async fn clear_all(&self) -> Result<()> {
        self.vector_store.clear_all().await?;
        Ok(())
    }
    
    /// Get statistics about the vector store
    pub async fn get_stats(&self) -> Result<VectorStoreStats> {
        self.vector_store.get_stats().await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorStoreStats {
    pub total_embeddings: usize,
    pub total_size_bytes: usize,
    pub average_vector_dimension: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_embedding_service() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db").to_string_lossy().to_string();
        
        let service = EmbeddingService::new("all-MiniLM-L6-v2", &db_path).await.unwrap();
        
        // Add content
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "function".to_string());
        
        let id = service.add_content("def hello_world(): print('Hello, World!')", metadata).await.unwrap();
        assert!(!id.is_empty());
        
        // Search for similar content
        let query = EmbeddingQuery {
            text: "print hello world function".to_string(),
            limit: 5,
            threshold: 0.5,
            filters: None,
        };
        
        let results = service.search(&query).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].score > 0.5);
    }
}