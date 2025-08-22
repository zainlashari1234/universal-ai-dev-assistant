use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use serde_json;

use crate::providers::{ProviderRouter, CompletionRequest};
use super::{
    EmbeddingRequest, EmbeddingResponse, EmbeddingType, SimilarityRequest, 
    SimilarityResponse, SimilarityMetric
};

pub struct EmbeddingManager {
    provider_router: Arc<ProviderRouter>,
    embedding_cache: Arc<RwLock<HashMap<String, CachedEmbedding>>>,
    model_config: EmbeddingModelConfig,
}

#[derive(Debug, Clone)]
struct CachedEmbedding {
    embedding: Vec<f32>,
    created_at: chrono::DateTime<chrono::Utc>,
    access_count: usize,
    last_accessed: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingModelConfig {
    pub code_model: String,
    pub text_model: String,
    pub dimension: usize,
    pub max_tokens: usize,
    pub batch_size: usize,
    pub cache_ttl_hours: i64,
}

impl EmbeddingManager {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self {
            provider_router,
            embedding_cache: Arc::new(RwLock::new(HashMap::new())),
            model_config: EmbeddingModelConfig::default(),
        }
    }

    pub async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let start_time = std::time::Instant::now();
        
        // Cache key oluştur
        let cache_key = self.create_cache_key(&request);
        
        // Cache'den kontrol et
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            debug!("Embedding cache hit for key: {}", cache_key);
            return Ok(EmbeddingResponse {
                embedding: cached.embedding,
                dimension: cached.embedding.len(),
                model_used: self.get_model_for_type(&request.embedding_type),
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Yeni embedding oluştur
        let embedding = self.create_embedding(&request).await?;
        
        // Cache'e kaydet
        self.cache_embedding(cache_key, &embedding).await;

        Ok(EmbeddingResponse {
            embedding: embedding.clone(),
            dimension: embedding.len(),
            model_used: self.get_model_for_type(&request.embedding_type),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    pub async fn generate_batch_embeddings(
        &self,
        requests: Vec<EmbeddingRequest>,
    ) -> Result<Vec<EmbeddingResponse>> {
        info!("Generating {} embeddings in batch", requests.len());
        
        let mut responses = Vec::new();
        let batch_size = self.model_config.batch_size;
        
        for chunk in requests.chunks(batch_size) {
            let mut chunk_responses = Vec::new();
            
            for request in chunk {
                match self.generate_embedding(request.clone()).await {
                    Ok(response) => chunk_responses.push(response),
                    Err(e) => {
                        error!("Failed to generate embedding: {}", e);
                        // Boş embedding ile devam et
                        chunk_responses.push(EmbeddingResponse {
                            embedding: vec![0.0; self.model_config.dimension],
                            dimension: self.model_config.dimension,
                            model_used: "fallback".to_string(),
                            processing_time_ms: 0,
                        });
                    }
                }
            }
            
            responses.extend(chunk_responses);
            
            // Rate limiting için kısa bekleme
            if chunk.len() == batch_size {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Ok(responses)
    }

    pub async fn calculate_similarity(&self, request: SimilarityRequest) -> Result<SimilarityResponse> {
        let query_embedding = &request.query_embedding;
        let mut scores = Vec::new();
        
        for candidate in &request.candidate_embeddings {
            let score = match request.similarity_metric {
                SimilarityMetric::Cosine => self.cosine_similarity(query_embedding, candidate),
                SimilarityMetric::Euclidean => self.euclidean_distance(query_embedding, candidate),
                SimilarityMetric::DotProduct => self.dot_product(query_embedding, candidate),
                SimilarityMetric::Manhattan => self.manhattan_distance(query_embedding, candidate),
                SimilarityMetric::Jaccard => self.jaccard_similarity(query_embedding, candidate),
            };
            scores.push(score);
        }

        // Skorlara göre sırala
        let mut indexed_scores: Vec<(usize, f32)> = scores
            .iter()
            .enumerate()
            .map(|(i, &score)| (i, score))
            .collect();
        
        // Cosine similarity için büyükten küçüğe, distance metrikleri için küçükten büyüğe
        match request.similarity_metric {
            SimilarityMetric::Cosine | SimilarityMetric::DotProduct | SimilarityMetric::Jaccard => {
                indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            }
            SimilarityMetric::Euclidean | SimilarityMetric::Manhattan => {
                indexed_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            }
        }

        let ranked_indices: Vec<usize> = indexed_scores.iter().map(|(i, _)| *i).collect();
        
        // Threshold üstündeki sonuçları filtrele
        let above_threshold = if let Some(threshold) = request.threshold {
            match request.similarity_metric {
                SimilarityMetric::Cosine | SimilarityMetric::DotProduct | SimilarityMetric::Jaccard => {
                    indexed_scores.iter()
                        .filter(|(_, score)| *score >= threshold)
                        .map(|(i, _)| *i)
                        .collect()
                }
                SimilarityMetric::Euclidean | SimilarityMetric::Manhattan => {
                    indexed_scores.iter()
                        .filter(|(_, score)| *score <= threshold)
                        .map(|(i, _)| *i)
                        .collect()
                }
            }
        } else {
            ranked_indices.clone()
        };

        Ok(SimilarityResponse {
            scores,
            ranked_indices,
            above_threshold,
        })
    }

    async fn create_embedding(&self, request: &EmbeddingRequest) -> Result<Vec<f32>> {
        let model = self.get_model_for_type(&request.embedding_type);
        let processed_text = self.preprocess_text(&request.text, &request.embedding_type);
        
        // OpenAI embedding API kullan
        let embedding_request = serde_json::json!({
            "input": processed_text,
            "model": model,
            "encoding_format": "float"
        });

        // Provider router üzerinden embedding API'sini çağır
        let response = self.call_embedding_api(&embedding_request).await?;
        
        // Response'dan embedding'i çıkar
        self.parse_embedding_response(&response)
    }

    async fn call_embedding_api(&self, request: &serde_json::Value) -> Result<serde_json::Value> {
        // Bu fonksiyon provider router'ı kullanarak embedding API'sini çağırır
        // Şimdilik basit bir implementasyon yapıyoruz
        
        let prompt = format!(
            "Generate a semantic embedding for this text: {}",
            request["input"].as_str().unwrap_or("")
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-3.5-turbo".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(10),
            temperature: Some(0.0),
            system_prompt: Some("Return only a JSON array of 1536 floating point numbers representing the embedding.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        
        // Gerçek implementasyonda burada OpenAI embedding API'si çağrılacak
        // Şimdilik mock embedding döndürüyoruz
        Ok(serde_json::json!({
            "data": [{
                "embedding": self.generate_mock_embedding(self.model_config.dimension)
            }]
        }))
    }

    fn parse_embedding_response(&self, response: &serde_json::Value) -> Result<Vec<f32>> {
        if let Some(data) = response["data"].as_array() {
            if let Some(first_item) = data.first() {
                if let Some(embedding_array) = first_item["embedding"].as_array() {
                    let embedding: Result<Vec<f32>, _> = embedding_array
                        .iter()
                        .map(|v| v.as_f64().map(|f| f as f32).ok_or_else(|| anyhow::anyhow!("Invalid embedding value")))
                        .collect();
                    return embedding;
                }
            }
        }
        
        Err(anyhow::anyhow!("Invalid embedding response format"))
    }

    fn generate_mock_embedding(&self, dimension: usize) -> Vec<f32> {
        // Mock embedding oluştur (gerçek implementasyonda kaldırılacak)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..dimension).map(|_| rng.gen_range(-1.0..1.0)).collect()
    }

    fn preprocess_text(&self, text: &str, embedding_type: &EmbeddingType) -> String {
        match embedding_type {
            EmbeddingType::Code => {
                // Kod için preprocessing
                self.preprocess_code(text)
            }
            EmbeddingType::Documentation => {
                // Dokümantasyon için preprocessing
                self.preprocess_documentation(text)
            }
            EmbeddingType::Query => {
                // Query için preprocessing
                self.preprocess_query(text)
            }
            EmbeddingType::Symbol => {
                // Sembol için preprocessing
                self.preprocess_symbol(text)
            }
            EmbeddingType::Comment => {
                // Yorum için preprocessing
                self.preprocess_comment(text)
            }
        }
    }

    fn preprocess_code(&self, code: &str) -> String {
        // Kod preprocessing
        let mut processed = code.to_string();
        
        // Fazla whitespace'leri temizle
        processed = regex::Regex::new(r"\s+").unwrap().replace_all(&processed, " ").to_string();
        
        // Yorumları kaldır (opsiyonel)
        processed = regex::Regex::new(r"//.*$").unwrap().replace_all(&processed, "").to_string();
        processed = regex::Regex::new(r"/\*.*?\*/").unwrap().replace_all(&processed, "").to_string();
        
        // String literallerini normalize et
        processed = regex::Regex::new(r#""[^"]*""#).unwrap().replace_all(&processed, "\"STRING\"").to_string();
        
        processed.trim().to_string()
    }

    fn preprocess_documentation(&self, doc: &str) -> String {
        // Dokümantasyon preprocessing
        let mut processed = doc.to_string();
        
        // Markdown syntax'ını temizle
        processed = regex::Regex::new(r"[#*`_]").unwrap().replace_all(&processed, "").to_string();
        
        // Fazla whitespace'leri temizle
        processed = regex::Regex::new(r"\s+").unwrap().replace_all(&processed, " ").to_string();
        
        processed.trim().to_string()
    }

    fn preprocess_query(&self, query: &str) -> String {
        // Query preprocessing
        let mut processed = query.to_lowercase();
        
        // Stop words'leri kaldır (basit liste)
        let stop_words = ["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by"];
        for stop_word in stop_words {
            processed = processed.replace(&format!(" {} ", stop_word), " ");
        }
        
        // Fazla whitespace'leri temizle
        processed = regex::Regex::new(r"\s+").unwrap().replace_all(&processed, " ").to_string();
        
        processed.trim().to_string()
    }

    fn preprocess_symbol(&self, symbol: &str) -> String {
        // Sembol preprocessing
        symbol.trim().to_string()
    }

    fn preprocess_comment(&self, comment: &str) -> String {
        // Yorum preprocessing
        let mut processed = comment.to_string();
        
        // Comment syntax'ını kaldır
        processed = processed.replace("//", "").replace("/*", "").replace("*/", "");
        processed = processed.replace("#", "").replace("<!--", "").replace("-->", "");
        
        processed.trim().to_string()
    }

    fn get_model_for_type(&self, embedding_type: &EmbeddingType) -> String {
        match embedding_type {
            EmbeddingType::Code | EmbeddingType::Symbol => self.model_config.code_model.clone(),
            _ => self.model_config.text_model.clone(),
        }
    }

    fn create_cache_key(&self, request: &EmbeddingRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        request.text.hash(&mut hasher);
        format!("{:?}", request.embedding_type).hash(&mut hasher);
        if let Some(context) = &request.context {
            context.hash(&mut hasher);
        }
        
        format!("{:x}", hasher.finish())
    }

    async fn get_from_cache(&self, cache_key: &str) -> Option<CachedEmbedding> {
        let cache = self.embedding_cache.read().await;
        if let Some(cached) = cache.get(cache_key) {
            // TTL kontrolü
            let now = chrono::Utc::now();
            let age = now.signed_duration_since(cached.created_at);
            
            if age.num_hours() < self.model_config.cache_ttl_hours {
                let mut updated_cached = cached.clone();
                updated_cached.access_count += 1;
                updated_cached.last_accessed = now;
                return Some(updated_cached);
            }
        }
        None
    }

    async fn cache_embedding(&self, cache_key: String, embedding: &[f32]) {
        let mut cache = self.embedding_cache.write().await;
        
        // Cache boyutunu kontrol et (max 10000 entry)
        if cache.len() >= 10000 {
            // En az kullanılan entry'leri temizle
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, cached)| cached.access_count);
            
            for (key, _) in entries.iter().take(1000) {
                cache.remove(*key);
            }
        }
        
        cache.insert(cache_key, CachedEmbedding {
            embedding: embedding.to_vec(),
            created_at: chrono::Utc::now(),
            access_count: 1,
            last_accessed: chrono::Utc::now(),
        });
    }

    // Similarity calculation methods
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    fn euclidean_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::INFINITY;
        }
        
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    fn dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    fn manhattan_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::INFINITY;
        }
        
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    }

    fn jaccard_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        // Binary vectors için Jaccard similarity
        let threshold = 0.5;
        let a_binary: Vec<bool> = a.iter().map(|&x| x > threshold).collect();
        let b_binary: Vec<bool> = b.iter().map(|&x| x > threshold).collect();
        
        let intersection: usize = a_binary.iter()
            .zip(b_binary.iter())
            .map(|(x, y)| if *x && *y { 1 } else { 0 })
            .sum();
        
        let union: usize = a_binary.iter()
            .zip(b_binary.iter())
            .map(|(x, y)| if *x || *y { 1 } else { 0 })
            .sum();
        
        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    pub async fn cleanup_cache(&self) {
        let mut cache = self.embedding_cache.write().await;
        let now = chrono::Utc::now();
        
        cache.retain(|_, cached| {
            let age = now.signed_duration_since(cached.created_at);
            age.num_hours() < self.model_config.cache_ttl_hours
        });
        
        info!("Cache cleanup completed. Remaining entries: {}", cache.len());
    }

    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.embedding_cache.read().await;
        
        let total_entries = cache.len();
        let total_access_count: usize = cache.values().map(|c| c.access_count).sum();
        let avg_access_count = if total_entries > 0 {
            total_access_count as f32 / total_entries as f32
        } else {
            0.0
        };
        
        CacheStats {
            total_entries,
            total_access_count,
            avg_access_count,
            memory_usage_mb: total_entries * self.model_config.dimension * 4 / 1024 / 1024, // Rough estimate
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_access_count: usize,
    pub avg_access_count: f32,
    pub memory_usage_mb: usize,
}

impl Default for EmbeddingModelConfig {
    fn default() -> Self {
        Self {
            code_model: "text-embedding-ada-002".to_string(),
            text_model: "text-embedding-ada-002".to_string(),
            dimension: 1536,
            max_tokens: 8192,
            batch_size: 10,
            cache_ttl_hours: 24,
        }
    }
}