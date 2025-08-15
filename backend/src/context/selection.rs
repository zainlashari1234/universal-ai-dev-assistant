use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

use super::{CodeSpan, ContextPackage, FileContext, Symbol};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionConfig {
    pub mmr_weight: f32,          // Weight for MMR diversity
    pub recency_weight: f32,      // Weight for file recency
    pub centrality_weight: f32,   // Weight for symbol centrality
    pub test_proximity_weight: f32, // Weight for test file proximity
    pub max_tokens: usize,        // Maximum total tokens
    pub max_files: usize,         // Maximum number of files
    pub token_per_file_limit: usize, // Token limit per file
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            mmr_weight: 0.7,
            recency_weight: 0.1,
            centrality_weight: 0.15,
            test_proximity_weight: 0.05,
            max_tokens: 8000,
            max_files: 20,
            token_per_file_limit: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionScore {
    pub relevance: f32,
    pub recency: f32,
    pub centrality: f32,
    pub test_proximity: f32,
    pub diversity: f32,
    pub final_score: f32,
}

pub struct ContextSelector {
    config: SelectionConfig,
}

impl ContextSelector {
    pub fn new() -> Self {
        Self {
            config: SelectionConfig::default(),
        }
    }

    pub fn with_config(config: SelectionConfig) -> Self {
        Self { config }
    }

    /// Select optimal context within token limit using MMR + multiple criteria
    pub async fn select_context(
        &self,
        files: Vec<FileContext>,
        spans: Vec<CodeSpan>,
        symbols: Vec<Symbol>,
        related_tests: Vec<PathBuf>,
        max_tokens: usize,
    ) -> Result<ContextPackage> {
        info!("Selecting context from {} files, {} spans, {} symbols", 
              files.len(), spans.len(), symbols.len());

        let effective_max_tokens = max_tokens.min(self.config.max_tokens);
        
        // Score all files
        let scored_files = self.score_files(&files, &related_tests).await?;
        
        // Score all spans
        let scored_spans = self.score_spans(&spans, &files).await?;
        
        // Score all symbols
        let scored_symbols = self.score_symbols(&symbols, &files).await?;
        
        // Select files using MMR approach
        let selected_files = self.select_files_mmr(scored_files, effective_max_tokens).await?;
        
        // Select spans within token budget
        let selected_spans = self.select_spans_within_budget(&scored_spans, &selected_files, effective_max_tokens / 4).await?;
        
        // Select symbols within remaining budget
        let selected_symbols = self.select_symbols_within_budget(&scored_symbols, &selected_files, effective_max_tokens / 6).await?;
        
        // Calculate total tokens
        let total_tokens = self.calculate_total_tokens(&selected_files, &selected_spans, &selected_symbols);
        
        let context = ContextPackage {
            files: selected_files,
            spans: selected_spans,
            symbols: selected_symbols,
            related_tests,
            total_tokens,
            selection_strategy: format!(
                "MMR(mmr_w={}, rec_w={}, cent_w={}, test_w={})", 
                self.config.mmr_weight,
                self.config.recency_weight, 
                self.config.centrality_weight,
                self.config.test_proximity_weight
            ),
        };
        
        info!("Selected context: {} files, {} spans, {} symbols, {} tokens",
              context.files.len(), context.spans.len(), context.symbols.len(), context.total_tokens);
        
        Ok(context)
    }

    /// Score files based on multiple criteria
    async fn score_files(&self, files: &[FileContext], related_tests: &[PathBuf]) -> Result<Vec<(FileContext, SelectionScore)>> {
        let mut scored_files = Vec::new();
        
        for file in files {
            let relevance = file.relevance_score;
            let recency = self.calculate_recency_score(&file.last_modified);
            let centrality = self.calculate_centrality_score(file, files);
            let test_proximity = self.calculate_test_proximity_score(&file.path, related_tests);
            
            let final_score = self.config.mmr_weight * relevance +
                             self.config.recency_weight * recency +
                             self.config.centrality_weight * centrality +
                             self.config.test_proximity_weight * test_proximity;
            
            let score = SelectionScore {
                relevance,
                recency,
                centrality,
                test_proximity,
                diversity: 0.0, // Will be calculated during MMR selection
                final_score,
            };
            
            scored_files.push((file.clone(), score));
        }
        
        // Sort by final score (descending)
        scored_files.sort_by(|a, b| b.1.final_score.partial_cmp(&a.1.final_score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(scored_files)
    }

    /// Score code spans
    async fn score_spans(&self, spans: &[CodeSpan], files: &[FileContext]) -> Result<Vec<(CodeSpan, SelectionScore)>> {
        let mut scored_spans = Vec::new();
        
        for span in spans {
            let relevance = span.relevance_score;
            
            // Find the file this span belongs to
            let file_context = files.iter().find(|f| f.path == span.file_path);
            let recency = if let Some(file) = file_context {
                self.calculate_recency_score(&file.last_modified)
            } else {
                0.5 // Default recency if file not found
            };
            
            let centrality = 0.5; // Simple default for spans
            let test_proximity = if self.is_test_span(span) { 1.0 } else { 0.0 };
            
            let final_score = self.config.mmr_weight * relevance +
                             self.config.recency_weight * recency +
                             self.config.centrality_weight * centrality +
                             self.config.test_proximity_weight * test_proximity;
            
            let score = SelectionScore {
                relevance,
                recency,
                centrality,
                test_proximity,
                diversity: 0.0,
                final_score,
            };
            
            scored_spans.push((span.clone(), score));
        }
        
        // Sort by final score (descending)
        scored_spans.sort_by(|a, b| b.1.final_score.partial_cmp(&a.1.final_score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(scored_spans)
    }

    /// Score symbols
    async fn score_symbols(&self, symbols: &[Symbol], files: &[FileContext]) -> Result<Vec<(Symbol, SelectionScore)>> {
        let mut scored_symbols = Vec::new();
        
        // Calculate symbol reference counts for centrality
        let mut reference_counts = HashMap::new();
        for symbol in symbols {
            reference_counts.insert(symbol.name.clone(), symbol.references.len());
        }
        
        let max_references = reference_counts.values().max().copied().unwrap_or(1) as f32;
        
        for symbol in symbols {
            let relevance = 0.8; // Base relevance for symbols
            
            // Find the file this symbol belongs to
            let file_context = files.iter().find(|f| f.path == symbol.file_path);
            let recency = if let Some(file) = file_context {
                self.calculate_recency_score(&file.last_modified)
            } else {
                0.5
            };
            
            let centrality = if max_references > 0.0 {
                reference_counts.get(&symbol.name).copied().unwrap_or(0) as f32 / max_references
            } else {
                0.5
            };
            
            let test_proximity = if self.is_test_symbol(symbol) { 1.0 } else { 0.0 };
            
            let final_score = self.config.mmr_weight * relevance +
                             self.config.recency_weight * recency +
                             self.config.centrality_weight * centrality +
                             self.config.test_proximity_weight * test_proximity;
            
            let score = SelectionScore {
                relevance,
                recency,
                centrality,
                test_proximity,
                diversity: 0.0,
                final_score,
            };
            
            scored_symbols.push((symbol.clone(), score));
        }
        
        // Sort by final score (descending)
        scored_symbols.sort_by(|a, b| b.1.final_score.partial_cmp(&a.1.final_score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(scored_symbols)
    }

    /// Select files using Maximal Marginal Relevance (MMR)
    async fn select_files_mmr(&self, mut scored_files: Vec<(FileContext, SelectionScore)>, max_tokens: usize) -> Result<Vec<FileContext>> {
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        // Start with the highest scoring file
        if let Some((first_file, _)) = scored_files.remove(0) {
            let file_tokens = self.estimate_file_tokens(&first_file);
            if file_tokens <= max_tokens {
                current_tokens += file_tokens;
                selected.push(first_file);
            }
        }
        
        // Select remaining files using MMR
        while selected.len() < self.config.max_files && !scored_files.is_empty() && current_tokens < max_tokens {
            let mut best_idx = 0;
            let mut best_mmr_score = f32::NEG_INFINITY;
            
            for (idx, (candidate_file, candidate_score)) in scored_files.iter().enumerate() {
                // Check if adding this file would exceed token limit
                let file_tokens = self.estimate_file_tokens(candidate_file);
                if current_tokens + file_tokens > max_tokens {
                    continue;
                }
                
                // Calculate diversity (1 - max_similarity to selected files)
                let max_similarity = selected.iter()
                    .map(|sel_file| self.calculate_file_similarity(candidate_file, sel_file))
                    .fold(0.0f32, |acc, sim| acc.max(sim));
                
                let diversity = 1.0 - max_similarity;
                
                // MMR score: λ * relevance + (1-λ) * diversity
                let mmr_score = self.config.mmr_weight * candidate_score.final_score + 
                               (1.0 - self.config.mmr_weight) * diversity;
                
                if mmr_score > best_mmr_score {
                    best_mmr_score = mmr_score;
                    best_idx = idx;
                }
            }
            
            if best_idx < scored_files.len() {
                let (selected_file, _) = scored_files.remove(best_idx);
                let file_tokens = self.estimate_file_tokens(&selected_file);
                current_tokens += file_tokens;
                selected.push(selected_file);
            } else {
                break; // No more files can fit
            }
        }
        
        debug!("Selected {} files using MMR (total tokens: {})", selected.len(), current_tokens);
        Ok(selected)
    }

    /// Select spans within token budget
    async fn select_spans_within_budget(
        &self, 
        scored_spans: &[(CodeSpan, SelectionScore)], 
        selected_files: &[FileContext],
        max_tokens: usize
    ) -> Result<Vec<CodeSpan>> {
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        // Only include spans from selected files
        let relevant_spans: Vec<_> = scored_spans.iter()
            .filter(|(span, _)| selected_files.iter().any(|f| f.path == span.file_path))
            .collect();
        
        for (span, _score) in relevant_spans {
            let span_tokens = self.estimate_span_tokens(span);
            
            if current_tokens + span_tokens <= max_tokens {
                current_tokens += span_tokens;
                selected.push(span.clone());
            }
            
            if current_tokens >= max_tokens {
                break;
            }
        }
        
        debug!("Selected {} spans (total tokens: {})", selected.len(), current_tokens);
        Ok(selected)
    }

    /// Select symbols within token budget
    async fn select_symbols_within_budget(
        &self,
        scored_symbols: &[(Symbol, SelectionScore)],
        selected_files: &[FileContext],
        max_tokens: usize
    ) -> Result<Vec<Symbol>> {
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        // Only include symbols from selected files
        let relevant_symbols: Vec<_> = scored_symbols.iter()
            .filter(|(symbol, _)| selected_files.iter().any(|f| f.path == symbol.file_path))
            .collect();
        
        for (symbol, _score) in relevant_symbols {
            let symbol_tokens = self.estimate_symbol_tokens(symbol);
            
            if current_tokens + symbol_tokens <= max_tokens {
                current_tokens += symbol_tokens;
                selected.push(symbol.clone());
            }
            
            if current_tokens >= max_tokens {
                break;
            }
        }
        
        debug!("Selected {} symbols (total tokens: {})", selected.len(), current_tokens);
        Ok(selected)
    }

    /// Calculate recency score based on last modified time
    fn calculate_recency_score(&self, last_modified: &chrono::DateTime<chrono::Utc>) -> f32 {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(*last_modified);
        let days_old = age.num_days() as f32;
        
        // Exponential decay: more recent files get higher scores
        if days_old <= 0.0 {
            1.0
        } else {
            (-days_old / 30.0).exp() // Decay over 30 days
        }
    }

    /// Calculate centrality score (how "central" a file is to the codebase)
    fn calculate_centrality_score(&self, file: &FileContext, all_files: &[FileContext]) -> f32 {
        // Simple heuristic: files with common names or in common directories are more central
        let file_name = file.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Boost for common entry point files
        if matches!(file_name.as_str(), "main.rs" | "main.py" | "index.js" | "app.py" | "lib.rs") {
            return 0.9;
        }
        
        // Boost for configuration files
        if matches!(file_name.as_str(), "cargo.toml" | "package.json" | "requirements.txt") {
            return 0.7;
        }
        
        // Calculate based on directory depth (shallower = more central)
        let depth = file.path.components().count();
        let max_depth = all_files.iter()
            .map(|f| f.path.components().count())
            .max()
            .unwrap_or(1);
        
        if max_depth > 1 {
            1.0 - (depth as f32 - 1.0) / (max_depth as f32 - 1.0)
        } else {
            0.5
        }
    }

    /// Calculate test proximity score
    fn calculate_test_proximity_score(&self, file_path: &PathBuf, related_tests: &[PathBuf]) -> f32 {
        if self.is_test_file(file_path) {
            return 1.0;
        }
        
        // Check if file has related tests
        let file_stem = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        let has_related_test = related_tests.iter().any(|test_path| {
            let test_name = test_path.to_string_lossy();
            test_name.contains(file_stem)
        });
        
        if has_related_test { 0.8 } else { 0.0 }
    }

    /// Calculate similarity between two files
    fn calculate_file_similarity(&self, file1: &FileContext, file2: &FileContext) -> f32 {
        // Language similarity
        let language_sim = if file1.language == file2.language { 0.3 } else { 0.0 };
        
        // Directory similarity
        let dir1 = file1.path.parent().unwrap_or_else(|| std::path::Path::new(""));
        let dir2 = file2.path.parent().unwrap_or_else(|| std::path::Path::new(""));
        let dir_sim = if dir1 == dir2 { 0.4 } else { 0.0 };
        
        // Name similarity (simple heuristic)
        let name1 = file1.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let name2 = file2.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let name_sim = if name1 == name2 { 0.3 } else {
            // Check for partial matches
            if name1.contains(name2) || name2.contains(name1) { 0.15 } else { 0.0 }
        };
        
        language_sim + dir_sim + name_sim
    }

    /// Estimate tokens for a file
    fn estimate_file_tokens(&self, file: &FileContext) -> usize {
        // Simple estimation: ~4 characters per token
        (file.content.len() / 4).min(self.config.token_per_file_limit)
    }

    /// Estimate tokens for a code span
    fn estimate_span_tokens(&self, span: &CodeSpan) -> usize {
        // Simple estimation: ~4 characters per token
        span.content.len() / 4
    }

    /// Estimate tokens for a symbol
    fn estimate_symbol_tokens(&self, symbol: &Symbol) -> usize {
        // Symbols are usually small, but include some context
        50 + symbol.references.len() * 10
    }

    /// Calculate total tokens for selected context
    fn calculate_total_tokens(&self, files: &[FileContext], spans: &[CodeSpan], symbols: &[Symbol]) -> usize {
        let file_tokens: usize = files.iter().map(|f| self.estimate_file_tokens(f)).sum();
        let span_tokens: usize = spans.iter().map(|s| self.estimate_span_tokens(s)).sum();
        let symbol_tokens: usize = symbols.iter().map(|s| self.estimate_symbol_tokens(s)).sum();
        
        file_tokens + span_tokens + symbol_tokens
    }

    /// Check if a file is a test file
    fn is_test_file(&self, path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        path_str.contains("test") || path_str.contains("spec")
    }

    /// Check if a span is from a test
    fn is_test_span(&self, span: &CodeSpan) -> bool {
        self.is_test_file(&span.file_path) || 
        span.content.to_lowercase().contains("test") ||
        matches!(span.span_type, super::SpanType::Test)
    }

    /// Check if a symbol is test-related
    fn is_test_symbol(&self, symbol: &Symbol) -> bool {
        self.is_test_file(&symbol.file_path) ||
        symbol.name.to_lowercase().contains("test")
    }
}

impl Default for ContextSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_file_context(name: &str, content: &str, relevance: f32) -> FileContext {
        FileContext {
            path: PathBuf::from(name),
            content: content.to_string(),
            language: "python".to_string(),
            relevance_score: relevance,
            last_modified: chrono::Utc::now(),
            size_bytes: content.len(),
        }
    }

    #[tokio::test]
    async fn test_context_selection() -> Result<()> {
        let selector = ContextSelector::new();
        
        let files = vec![
            create_test_file_context("main.py", "def main(): pass", 0.9),
            create_test_file_context("utils.py", "def helper(): pass", 0.7),
            create_test_file_context("test_main.py", "def test_main(): assert True", 0.8),
        ];
        
        let spans = vec![];
        let symbols = vec![];
        let related_tests = vec![PathBuf::from("test_main.py")];
        
        let context = selector.select_context(files, spans, symbols, related_tests, 1000).await?;
        
        assert!(!context.files.is_empty());
        assert!(context.total_tokens > 0);
        assert!(context.total_tokens <= 1000);
        
        Ok(())
    }

    #[test]
    fn test_recency_score() {
        let selector = ContextSelector::new();
        
        let now = chrono::Utc::now();
        let old_time = now - chrono::Duration::days(10);
        let very_old_time = now - chrono::Duration::days(100);
        
        let recent_score = selector.calculate_recency_score(&now);
        let old_score = selector.calculate_recency_score(&old_time);
        let very_old_score = selector.calculate_recency_score(&very_old_time);
        
        assert!(recent_score > old_score);
        assert!(old_score > very_old_score);
        assert!(recent_score <= 1.0);
        assert!(very_old_score >= 0.0);
    }

    #[test]
    fn test_file_similarity() {
        let selector = ContextSelector::new();
        
        let file1 = create_test_file_context("src/main.py", "content", 1.0);
        let file2 = create_test_file_context("src/utils.py", "content", 1.0);
        let file3 = create_test_file_context("tests/test.py", "content", 1.0);
        
        let sim_same_dir = selector.calculate_file_similarity(&file1, &file2);
        let sim_diff_dir = selector.calculate_file_similarity(&file1, &file3);
        
        assert!(sim_same_dir > sim_diff_dir);
    }
}