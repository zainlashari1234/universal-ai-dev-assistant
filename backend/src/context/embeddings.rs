use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

use super::{FileContext, Symbol};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    pub id: String,
    pub file_path: PathBuf,
    pub content: String,
    pub embedding: Embedding,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub entry: EmbeddingEntry,
    pub score: f32,
}

pub struct EmbeddingStore {
    storage_path: PathBuf,
    entries: HashMap<String, EmbeddingEntry>,
    dimension: usize,
}

impl EmbeddingStore {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        // Create storage directory if it doesn't exist
        if let Some(parent) = storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        info!("Initializing EmbeddingStore at: {:?}", storage_path);
        
        let mut store = Self {
            storage_path,
            entries: HashMap::new(),
            dimension: 384, // Default dimension for small models
        };
        
        // Try to load existing embeddings
        if let Err(e) = store.load_from_disk() {
            warn!("Could not load existing embeddings: {}", e);
        }
        
        Ok(store)
    }

    /// Generate embeddings for files using a simple local model
    pub async fn generate_embeddings(&mut self, files: &[FileContext]) -> Result<()> {
        info!("Generating embeddings for {} files", files.len());
        
        for (idx, file) in files.iter().enumerate() {
            if idx % 10 == 0 {
                debug!("Processing file {}/{}: {:?}", idx + 1, files.len(), file.path);
            }
            
            let embedding = self.generate_content_embedding(&file.content).await?;
            
            let entry = EmbeddingEntry {
                id: format!("file_{}", idx),
                file_path: file.path.clone(),
                content: file.content.clone(),
                embedding,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("language".to_string(), file.language.clone());
                    meta.insert("size_bytes".to_string(), file.size_bytes.to_string());
                    meta.insert("relevance_score".to_string(), file.relevance_score.to_string());
                    meta
                },
            };
            
            self.entries.insert(entry.id.clone(), entry);
        }
        
        // Save to disk
        self.save_to_disk().await?;
        
        info!("Generated and stored {} embeddings", self.entries.len());
        Ok(())
    }

    /// Generate embedding for text content using simple heuristics
    /// In a real implementation, this would use a proper embedding model
    async fn generate_content_embedding(&self, content: &str) -> Result<Embedding> {
        // Simple TF-IDF like approach for demonstration
        let words: Vec<&str> = content.split_whitespace().collect();
        let mut word_counts = HashMap::new();
        
        // Count word frequencies
        for word in &words {
            let clean_word = word.to_lowercase()
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>();
            
            if !clean_word.is_empty() && clean_word.len() > 2 {
                *word_counts.entry(clean_word).or_insert(0) += 1;
            }
        }
        
        // Create a simple embedding vector
        let mut vector = vec![0.0; self.dimension];
        
        // Use hash of words to create deterministic embeddings
        for (word, count) in word_counts {
            let hash = self.simple_hash(&word) % self.dimension;
            vector[hash] += count as f32;
        }
        
        // Normalize vector
        let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut vector {
                *val /= magnitude;
            }
        }
        
        // Add some semantic features
        self.add_semantic_features(&mut vector, content);
        
        Ok(Embedding {
            vector,
            dimension: self.dimension,
        })
    }

    /// Add semantic features to embedding based on content analysis
    fn add_semantic_features(&self, vector: &mut Vec<f32>, content: &str) {
        let content_lower = content.to_lowercase();
        
        // Programming language keywords
        let keywords = [
            ("function", 0.1), ("class", 0.15), ("import", 0.05),
            ("def", 0.1), ("async", 0.08), ("await", 0.08),
            ("if", 0.05), ("for", 0.05), ("while", 0.05),
            ("try", 0.07), ("catch", 0.07), ("error", 0.1),
            ("test", 0.12), ("mock", 0.08), ("assert", 0.1),
            ("config", 0.06), ("api", 0.08), ("http", 0.07),
            ("database", 0.09), ("sql", 0.08), ("query", 0.08),
        ];
        
        for (keyword, weight) in keywords {
            if content_lower.contains(keyword) {
                let hash = self.simple_hash(keyword) % self.dimension;
                vector[hash] += weight;
            }
        }
        
        // Code complexity indicators
        let line_count = content.lines().count();
        let complexity_feature = (line_count as f32).ln() / 10.0;
        if let Some(last) = vector.last_mut() {
            *last += complexity_feature;
        }
    }

    /// Simple hash function for consistent word mapping
    fn simple_hash(&self, s: &str) -> usize {
        let mut hash = 0usize;
        for byte in s.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
        }
        hash
    }

    /// Search for similar content using cosine similarity
    pub async fn search_similar(&self, query: &str, limit: usize) -> Result<Vec<SimilarityResult>> {
        let query_embedding = self.generate_content_embedding(query).await?;
        
        let mut results: Vec<SimilarityResult> = self.entries
            .values()
            .map(|entry| {
                let similarity = self.cosine_similarity(&query_embedding.vector, &entry.embedding.vector);
                SimilarityResult {
                    entry: entry.clone(),
                    score: similarity,
                }
            })
            .collect();
        
        // Sort by similarity score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top results
        results.truncate(limit);
        
        debug!("Found {} similar results for query: \"{}\"", results.len(), query);
        
        Ok(results)
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (magnitude_a * magnitude_b)
    }

    /// Store symbols with their embeddings
    pub async fn store_symbols(&mut self, file_path: &Path, symbols: &[Symbol]) -> Result<()> {
        for (idx, symbol) in symbols.iter().enumerate() {
            let symbol_content = format!("{} {} at {}:{}", 
                match symbol.symbol_type {
                    super::SymbolType::Function => "function",
                    super::SymbolType::Class => "class",
                    super::SymbolType::Variable => "variable",
                    super::SymbolType::Constant => "constant",
                    super::SymbolType::Module => "module",
                    super::SymbolType::Interface => "interface",
                    super::SymbolType::Enum => "enum",
                },
                symbol.name,
                symbol.line,
                symbol.column
            );
            
            let embedding = self.generate_content_embedding(&symbol_content).await?;
            
            let entry = EmbeddingEntry {
                id: format!("symbol_{}_{}", file_path.display(), idx),
                file_path: file_path.to_path_buf(),
                content: symbol_content,
                embedding,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("type".to_string(), "symbol".to_string());
                    meta.insert("symbol_name".to_string(), symbol.name.clone());
                    meta.insert("symbol_type".to_string(), format!("{:?}", symbol.symbol_type));
                    meta.insert("line".to_string(), symbol.line.to_string());
                    meta.insert("column".to_string(), symbol.column.to_string());
                    meta
                },
            };
            
            self.entries.insert(entry.id.clone(), entry);
        }
        
        Ok(())
    }

    /// Search for similar symbols
    pub async fn search_similar_symbols(&self, query: &str, limit: usize) -> Result<Vec<SimilarityResult>> {
        let query_embedding = self.generate_content_embedding(query).await?;
        
        let mut results: Vec<SimilarityResult> = self.entries
            .values()
            .filter(|entry| entry.metadata.get("type") == Some(&"symbol".to_string()))
            .map(|entry| {
                let similarity = self.cosine_similarity(&query_embedding.vector, &entry.embedding.vector);
                SimilarityResult {
                    entry: entry.clone(),
                    score: similarity,
                }
            })
            .collect();
        
        // Sort by similarity score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top results
        results.truncate(limit);
        
        debug!("Found {} similar symbols for query: \"{}\"", results.len(), query);
        
        Ok(results)
    }

    /// Get embeddings by file path
    pub fn get_by_file(&self, file_path: &Path) -> Vec<&EmbeddingEntry> {
        self.entries
            .values()
            .filter(|entry| entry.file_path == file_path)
            .collect()
    }

    /// Get all embeddings
    pub fn get_all(&self) -> Vec<&EmbeddingEntry> {
        self.entries.values().collect()
    }

    /// Save embeddings to disk
    async fn save_to_disk(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.storage_path, data).await?;
        debug!("Saved {} embeddings to disk", self.entries.len());
        Ok(())
    }

    /// Load embeddings from disk
    fn load_from_disk(&mut self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(()); // No existing file, that's fine
        }
        
        let data = std::fs::read_to_string(&self.storage_path)?;
        self.entries = serde_json::from_str(&data)?;
        info!("Loaded {} embeddings from disk", self.entries.len());
        Ok(())
    }

    /// Clear all embeddings
    pub async fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        self.save_to_disk().await?;
        info!("Cleared all embeddings");
        Ok(())
    }

    /// Get statistics about the embedding store
    pub fn get_stats(&self) -> EmbeddingStats {
        let total_entries = self.entries.len();
        let file_entries = self.entries.values()
            .filter(|e| e.metadata.get("type") != Some(&"symbol".to_string()))
            .count();
        let symbol_entries = total_entries - file_entries;
        
        let languages: std::collections::HashSet<String> = self.entries
            .values()
            .filter_map(|e| e.metadata.get("language"))
            .cloned()
            .collect();
        
        EmbeddingStats {
            total_entries,
            file_entries,
            symbol_entries,
            dimension: self.dimension,
            languages: languages.into_iter().collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingStats {
    pub total_entries: usize,
    pub file_entries: usize,
    pub symbol_entries: usize,
    pub dimension: usize,
    pub languages: Vec<String>,
}

/// Maximal Marginal Relevance (MMR) selector for diverse results
pub fn select_diverse_results(results: Vec<SimilarityResult>, lambda: f32, max_results: usize) -> Vec<SimilarityResult> {
    if results.is_empty() || max_results == 0 {
        return Vec::new();
    }
    
    let mut selected = Vec::new();
    let mut remaining = results;
    
    // Start with the most relevant result
    if let Some(first) = remaining.remove(0) {
        selected.push(first);
    }
    
    // Select remaining results using MMR
    while selected.len() < max_results && !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_score = f32::NEG_INFINITY;
        
        for (idx, candidate) in remaining.iter().enumerate() {
            // Calculate relevance score
            let relevance = candidate.score;
            
            // Calculate maximum similarity to already selected items
            let max_similarity = selected.iter()
                .map(|sel| cosine_similarity(&candidate.entry.embedding.vector, &sel.entry.embedding.vector))
                .fold(0.0f32, |acc, sim| acc.max(sim));
            
            // MMR score: λ * relevance - (1-λ) * max_similarity
            let mmr_score = lambda * relevance - (1.0 - lambda) * max_similarity;
            
            if mmr_score > best_score {
                best_score = mmr_score;
                best_idx = idx;
            }
        }
        
        selected.push(remaining.remove(best_idx));
    }
    
    selected
}

/// Helper function for cosine similarity (duplicated for MMR function)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_embedding_generation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store_path = temp_dir.path().join("embeddings.json");
        
        let mut store = EmbeddingStore::new(store_path)?;
        
        let content = "def hello_world(): print('Hello, World!')";
        let embedding = store.generate_content_embedding(content).await?;
        
        assert_eq!(embedding.vector.len(), store.dimension);
        assert!(embedding.vector.iter().any(|&x| x != 0.0)); // Should have some non-zero values
        
        Ok(())
    }

    #[tokio::test]
    async fn test_similarity_search() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store_path = temp_dir.path().join("embeddings.json");
        
        let mut store = EmbeddingStore::new(store_path)?;
        
        let files = vec![
            FileContext {
                path: PathBuf::from("test1.py"),
                content: "def calculate_fibonacci(n): return n if n <= 1 else calculate_fibonacci(n-1) + calculate_fibonacci(n-2)".to_string(),
                language: "python".to_string(),
                relevance_score: 1.0,
                last_modified: chrono::Utc::now(),
                size_bytes: 100,
            },
            FileContext {
                path: PathBuf::from("test2.py"),
                content: "def sort_array(arr): return sorted(arr)".to_string(),
                language: "python".to_string(),
                relevance_score: 1.0,
                last_modified: chrono::Utc::now(),
                size_bytes: 50,
            },
        ];
        
        store.generate_embeddings(&files).await?;
        
        let results = store.search_similar("fibonacci calculation", 5).await?;
        
        assert!(!results.is_empty());
        assert!(results[0].entry.content.contains("fibonacci"));
        
        Ok(())
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        
        let similarity_ab = cosine_similarity(&a, &b);
        let similarity_ac = cosine_similarity(&a, &c);
        
        assert!((similarity_ab - 1.0).abs() < 1e-6); // Should be 1.0 (identical)
        assert!((similarity_ac - 0.0).abs() < 1e-6); // Should be 0.0 (orthogonal)
    }

    #[test]
    fn test_mmr_selection() {
        let results = vec![
            SimilarityResult {
                entry: EmbeddingEntry {
                    id: "1".to_string(),
                    file_path: PathBuf::from("test1.py"),
                    content: "similar content A".to_string(),
                    embedding: Embedding { vector: vec![1.0, 0.0], dimension: 2 },
                    metadata: HashMap::new(),
                },
                score: 0.9,
            },
            SimilarityResult {
                entry: EmbeddingEntry {
                    id: "2".to_string(),
                    file_path: PathBuf::from("test2.py"),
                    content: "similar content B".to_string(),
                    embedding: Embedding { vector: vec![0.9, 0.1], dimension: 2 },
                    metadata: HashMap::new(),
                },
                score: 0.8,
            },
            SimilarityResult {
                entry: EmbeddingEntry {
                    id: "3".to_string(),
                    file_path: PathBuf::from("test3.py"),
                    content: "different content".to_string(),
                    embedding: Embedding { vector: vec![0.0, 1.0], dimension: 2 },
                    metadata: HashMap::new(),
                },
                score: 0.7,
            },
        ];
        
        let selected = select_diverse_results(results, 0.7, 2);
        
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].entry.id, "1"); // Should pick the most relevant first
    }
}