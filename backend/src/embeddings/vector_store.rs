// Sprint 2: sqlite-vss vector store implementation
use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};
use std::collections::HashMap;
use tracing::{info, warn};

use super::{EmbeddingVector, SimilarityResult, VectorStoreStats};

pub struct VectorStore {
    pool: SqlitePool,
}

impl VectorStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        
        let store = Self { pool };
        store.initialize_schema().await?;
        
        info!("Vector store initialized with sqlite-vss");
        Ok(store)
    }
    
    async fn initialize_schema(&self) -> Result<()> {
        // Enable sqlite-vss extension
        sqlx::query("SELECT load_extension('vss0')")
            .execute(&self.pool)
            .await
            .ok(); // Ignore if extension not available
        
        // Create embeddings table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS embeddings (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#)
        .execute(&self.pool)
        .await?;
        
        // Create vector table with vss
        sqlx::query(r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS embeddings_vss USING vss0(
                embedding(384)
            )
        "#)
        .execute(&self.pool)
        .await
        .unwrap_or_else(|_| {
            // Fallback to regular table if vss not available
            sqlx::query(r#"
                CREATE TABLE IF NOT EXISTS embeddings_vectors (
                    id TEXT PRIMARY KEY,
                    vector BLOB NOT NULL
                )
            "#)
            .execute(&self.pool)
            .await
            .unwrap()
        });
        
        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_embeddings_created_at ON embeddings(created_at)")
            .execute(&self.pool)
            .await?;
        
        info!("Vector store schema initialized");
        Ok(())
    }
    
    pub async fn store_embedding(&self, embedding: &EmbeddingVector) -> Result<()> {
        let metadata_json = serde_json::to_string(&embedding.metadata)?;
        
        // Store metadata
        sqlx::query(r#"
            INSERT OR REPLACE INTO embeddings (id, content, metadata, created_at)
            VALUES (?, ?, ?, ?)
        "#)
        .bind(&embedding.id)
        .bind(&embedding.content)
        .bind(&metadata_json)
        .bind(&embedding.created_at)
        .execute(&self.pool)
        .await?;
        
        // Store vector
        if self.has_vss_support().await {
            self.store_vector_vss(&embedding.id, &embedding.vector).await?;
        } else {
            self.store_vector_fallback(&embedding.id, &embedding.vector).await?;
        }
        
        Ok(())
    }
    
    async fn store_vector_vss(&self, id: &str, vector: &[f32]) -> Result<()> {
        let vector_blob = vector.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>();
        
        sqlx::query("INSERT OR REPLACE INTO embeddings_vss(rowid, embedding) VALUES (?, ?)")
            .bind(id)
            .bind(&vector_blob)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    async fn store_vector_fallback(&self, id: &str, vector: &[f32]) -> Result<()> {
        let vector_blob = bincode::serialize(vector)?;
        
        sqlx::query("INSERT OR REPLACE INTO embeddings_vectors (id, vector) VALUES (?, ?)")
            .bind(id)
            .bind(&vector_blob)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn search_similar(
        &self,
        query_vector: &[f32],
        limit: usize,
        threshold: f32,
        filters: Option<&HashMap<String, String>>,
    ) -> Result<Vec<SimilarityResult>> {
        if self.has_vss_support().await {
            self.search_similar_vss(query_vector, limit, threshold, filters).await
        } else {
            self.search_similar_fallback(query_vector, limit, threshold, filters).await
        }
    }
    
    async fn search_similar_vss(
        &self,
        query_vector: &[f32],
        limit: usize,
        threshold: f32,
        _filters: Option<&HashMap<String, String>>,
    ) -> Result<Vec<SimilarityResult>> {
        let query_blob = query_vector.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>();
        
        let rows = sqlx::query(r#"
            SELECT e.id, e.content, e.metadata, v.distance
            FROM embeddings e
            JOIN (
                SELECT rowid, distance 
                FROM embeddings_vss 
                WHERE vss_search(embedding, ?)
                ORDER BY distance
                LIMIT ?
            ) v ON e.id = v.rowid
            WHERE v.distance <= ?
            ORDER BY v.distance
        "#)
        .bind(&query_blob)
        .bind(limit as i64)
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;
        
        let mut results = Vec::new();
        for row in rows {
            let metadata_json: String = row.get("metadata");
            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)?;
            let distance: f32 = row.get("distance");
            let score = 1.0 - distance; // Convert distance to similarity score
            
            results.push(SimilarityResult {
                id: row.get("id"),
                content: row.get("content"),
                score,
                metadata,
            });
        }
        
        Ok(results)
    }
    
    async fn search_similar_fallback(
        &self,
        query_vector: &[f32],
        limit: usize,
        threshold: f32,
        _filters: Option<&HashMap<String, String>>,
    ) -> Result<Vec<SimilarityResult>> {
        // Fallback: Load all vectors and compute similarity in memory
        let rows = sqlx::query(r#"
            SELECT e.id, e.content, e.metadata, v.vector
            FROM embeddings e
            JOIN embeddings_vectors v ON e.id = v.id
        "#)
        .fetch_all(&self.pool)
        .await?;
        
        let mut results = Vec::new();
        
        for row in rows {
            let vector_blob: Vec<u8> = row.get("vector");
            let vector: Vec<f32> = bincode::deserialize(&vector_blob)?;
            
            let similarity = cosine_similarity(query_vector, &vector);
            
            if similarity >= threshold {
                let metadata_json: String = row.get("metadata");
                let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)?;
                
                results.push(SimilarityResult {
                    id: row.get("id"),
                    content: row.get("content"),
                    score: similarity,
                    metadata,
                });
            }
        }
        
        // Sort by similarity and limit
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        Ok(results)
    }
    
    pub async fn get_embedding(&self, id: &str) -> Result<Option<EmbeddingVector>> {
        let row = sqlx::query(r#"
            SELECT id, content, metadata, created_at
            FROM embeddings
            WHERE id = ?
        "#)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            let metadata_json: String = row.get("metadata");
            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)?;
            
            // Get vector
            let vector = if self.has_vss_support().await {
                self.get_vector_vss(id).await?
            } else {
                self.get_vector_fallback(id).await?
            };
            
            Ok(Some(EmbeddingVector {
                id: row.get("id"),
                content: row.get("content"),
                vector: vector.unwrap_or_default(),
                metadata,
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn get_vector_vss(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let row = sqlx::query("SELECT embedding FROM embeddings_vss WHERE rowid = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        
        if let Some(row) = row {
            let vector_blob: Vec<u8> = row.get("embedding");
            let vector = vector_blob.chunks(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            Ok(Some(vector))
        } else {
            Ok(None)
        }
    }
    
    async fn get_vector_fallback(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let row = sqlx::query("SELECT vector FROM embeddings_vectors WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        
        if let Some(row) = row {
            let vector_blob: Vec<u8> = row.get("vector");
            let vector: Vec<f32> = bincode::deserialize(&vector_blob)?;
            Ok(Some(vector))
        } else {
            Ok(None)
        }
    }
    
    pub async fn update_embedding(&self, embedding: &EmbeddingVector) -> Result<()> {
        self.store_embedding(embedding).await
    }
    
    pub async fn delete_embedding(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM embeddings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        if self.has_vss_support().await {
            sqlx::query("DELETE FROM embeddings_vss WHERE rowid = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("DELETE FROM embeddings_vectors WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        
        Ok(())
    }
    
    pub async fn clear_all(&self) -> Result<()> {
        sqlx::query("DELETE FROM embeddings").execute(&self.pool).await?;
        
        if self.has_vss_support().await {
            sqlx::query("DELETE FROM embeddings_vss").execute(&self.pool).await?;
        } else {
            sqlx::query("DELETE FROM embeddings_vectors").execute(&self.pool).await?;
        }
        
        Ok(())
    }
    
    pub async fn get_stats(&self) -> Result<VectorStoreStats> {
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM embeddings")
            .fetch_one(&self.pool)
            .await?;
        
        let total_embeddings: i64 = count_row.get("count");
        
        Ok(VectorStoreStats {
            total_embeddings: total_embeddings as usize,
            total_size_bytes: total_embeddings as usize * 384 * 4, // Approximate
            average_vector_dimension: 384,
            created_at: chrono::Utc::now(),
        })
    }
    
    async fn has_vss_support(&self) -> bool {
        sqlx::query("SELECT name FROM pragma_module_list WHERE name = 'vss0'")
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.is_some())
            .unwrap_or(false)
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_vector_store() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db").to_string_lossy().to_string();
        
        let store = VectorStore::new(&format!("sqlite://{}", db_path)).await.unwrap();
        
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "test".to_string());
        
        let embedding = EmbeddingVector {
            id: "test-1".to_string(),
            content: "Hello world".to_string(),
            vector: vec![0.1, 0.2, 0.3, 0.4],
            metadata,
            created_at: chrono::Utc::now(),
        };
        
        // Store embedding
        store.store_embedding(&embedding).await.unwrap();
        
        // Retrieve embedding
        let retrieved = store.get_embedding("test-1").await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Hello world");
        
        // Search similar
        let results = store.search_similar(&vec![0.1, 0.2, 0.3, 0.4], 5, 0.5, None).await.unwrap();
        assert!(!results.is_empty());
    }
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);
    }
}