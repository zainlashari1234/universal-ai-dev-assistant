use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use sqlx::PgPool;
use tracing::{info, debug, warn, error};
use uuid::Uuid;

use super::{
    SearchRequest, SearchResponse, SearchResult, ProcessedQuery, CodeIndex,
    MatchType, SimilarityRequest, SimilarityMetric, SearchSuggestion, SuggestionType,
    embedding_manager::EmbeddingManager,
    query_processor::QueryProcessor,
    result_ranker::{ResultRanker, RankingContext},
    code_indexer::CodeIndexer,
};

pub struct SemanticSearchEngine {
    embedding_manager: Arc<EmbeddingManager>,
    query_processor: Arc<QueryProcessor>,
    result_ranker: Arc<ResultRanker>,
    code_indexer: Arc<CodeIndexer>,
    pool: Arc<PgPool>,
    index_cache: Arc<tokio::sync::RwLock<HashMap<String, Vec<CodeIndex>>>>,
}

impl SemanticSearchEngine {
    pub fn new(
        embedding_manager: Arc<EmbeddingManager>,
        query_processor: Arc<QueryProcessor>,
        result_ranker: Arc<ResultRanker>,
        code_indexer: Arc<CodeIndexer>,
        pool: Arc<PgPool>,
    ) -> Self {
        Self {
            embedding_manager,
            query_processor,
            result_ranker,
            code_indexer,
            pool,
            index_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let start_time = std::time::Instant::now();
        info!("Starting semantic search for: {}", request.query);

        // Query'yi işle
        let processed_query = self.query_processor.process_query(&request).await?;
        debug!("Processed query: {:?}", processed_query.intent);

        // Query embedding'i oluştur
        let query_embedding = self.query_processor.generate_query_embedding(&processed_query).await?;

        // Workspace'leri index'le (gerekirse)
        let mut all_indices = Vec::new();
        for workspace_path in &request.workspace_paths {
            let indices = self.get_or_create_indices(workspace_path).await?;
            all_indices.extend(indices);
        }

        // Semantic search yap
        let mut search_results = self.perform_semantic_search(
            &query_embedding,
            &all_indices,
            &processed_query,
            &request,
        ).await?;

        // Sonuçları rank'le
        let ranking_context = self.build_ranking_context(&request).await?;
        search_results = self.result_ranker.rank_results(
            search_results,
            &request,
            &processed_query,
            ranking_context.as_ref(),
        )?;

        // Limit uygula
        let max_results = request.max_results.unwrap_or(50);
        search_results.truncate(max_results);

        // Suggestions oluştur
        let suggestions = self.generate_suggestions(&request, &processed_query, &search_results).await?;

        // Related queries oluştur
        let related_queries = self.generate_related_queries(&processed_query).await?;

        // Aggregations oluştur
        let aggregations = self.result_ranker.generate_aggregations(&search_results);

        // Filters applied bilgisi
        let filters_applied = self.extract_applied_filters(&request, &processed_query);

        let search_time_ms = start_time.elapsed().as_millis() as u64;
        info!("Search completed in {}ms, found {} results", search_time_ms, search_results.len());

        Ok(SearchResponse {
            query: request.query,
            results: search_results.clone(),
            total_matches: search_results.len(),
            search_time_ms,
            suggestions,
            related_queries,
            filters_applied,
            aggregations,
        })
    }

    async fn get_or_create_indices(&self, workspace_path: &str) -> Result<Vec<CodeIndex>> {
        // Cache'den kontrol et
        {
            let cache = self.index_cache.read().await;
            if let Some(indices) = cache.get(workspace_path) {
                debug!("Using cached indices for workspace: {}", workspace_path);
                return Ok(indices.clone());
            }
        }

        // Veritabanından kontrol et
        let db_indices = self.load_indices_from_db(workspace_path).await?;
        if !db_indices.is_empty() {
            // Cache'e kaydet
            let mut cache = self.index_cache.write().await;
            cache.insert(workspace_path.to_string(), db_indices.clone());
            return Ok(db_indices);
        }

        // Yeni indexleme yap
        info!("Creating new indices for workspace: {}", workspace_path);
        let indices = self.code_indexer.index_workspace(workspace_path).await?;
        
        // Veritabanına kaydet
        self.save_indices_to_db(&indices).await?;
        
        // Cache'e kaydet
        let mut cache = self.index_cache.write().await;
        cache.insert(workspace_path.to_string(), indices.clone());

        Ok(indices)
    }

    async fn load_indices_from_db(&self, workspace_path: &str) -> Result<Vec<CodeIndex>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, file_path, content_hash, embedding, metadata, indexed_at, last_updated
            FROM code_index 
            WHERE file_path LIKE $1
            ORDER BY last_updated DESC
            "#,
            format!("{}%", workspace_path)
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut indices = Vec::new();
        for row in rows {
            // Embedding'i deserialize et
            let embedding: Vec<f32> = serde_json::from_value(row.embedding)?;
            
            // Metadata'yı deserialize et
            let metadata: super::IndexMetadata = serde_json::from_value(row.metadata)?;

            // Symbols'ları ayrı tabloda yükle
            let symbols = self.load_symbols_from_db(row.id).await?;

            indices.push(CodeIndex {
                id: row.id,
                file_path: row.file_path,
                content_hash: row.content_hash,
                embedding,
                symbols,
                metadata,
                indexed_at: row.indexed_at,
                last_updated: row.last_updated,
            });
        }

        Ok(indices)
    }

    async fn load_symbols_from_db(&self, index_id: Uuid) -> Result<Vec<super::IndexedSymbol>> {
        let rows = sqlx::query!(
            r#"
            SELECT name, symbol_type, line_start, line_end, content, embedding, signature_hash, references
            FROM indexed_symbols 
            WHERE index_id = $1
            "#,
            index_id
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut symbols = Vec::new();
        for row in rows {
            let symbol_type: super::SymbolType = serde_json::from_str(&row.symbol_type)?;
            let embedding: Vec<f32> = serde_json::from_value(row.embedding)?;
            let references: Vec<super::SymbolReference> = serde_json::from_value(row.references)?;

            symbols.push(super::IndexedSymbol {
                name: row.name,
                symbol_type,
                line_start: row.line_start as usize,
                line_end: row.line_end as usize,
                content: row.content,
                embedding,
                signature_hash: row.signature_hash,
                references,
            });
        }

        Ok(symbols)
    }

    async fn save_indices_to_db(&self, indices: &[CodeIndex]) -> Result<()> {
        for index in indices {
            // Ana index'i kaydet
            sqlx::query!(
                r#"
                INSERT INTO code_index (id, file_path, content_hash, embedding, metadata, indexed_at, last_updated)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (id) DO UPDATE SET
                    content_hash = EXCLUDED.content_hash,
                    embedding = EXCLUDED.embedding,
                    metadata = EXCLUDED.metadata,
                    last_updated = EXCLUDED.last_updated
                "#,
                index.id,
                index.file_path,
                index.content_hash,
                serde_json::to_value(&index.embedding)?,
                serde_json::to_value(&index.metadata)?,
                index.indexed_at,
                index.last_updated
            )
            .execute(&*self.pool)
            .await?;

            // Symbols'ları kaydet
            for symbol in &index.symbols {
                sqlx::query!(
                    r#"
                    INSERT INTO indexed_symbols (index_id, name, symbol_type, line_start, line_end, content, embedding, signature_hash, references)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (index_id, signature_hash) DO UPDATE SET
                        name = EXCLUDED.name,
                        symbol_type = EXCLUDED.symbol_type,
                        line_start = EXCLUDED.line_start,
                        line_end = EXCLUDED.line_end,
                        content = EXCLUDED.content,
                        embedding = EXCLUDED.embedding,
                        references = EXCLUDED.references
                    "#,
                    index.id,
                    symbol.name,
                    serde_json::to_string(&symbol.symbol_type)?,
                    symbol.line_start as i32,
                    symbol.line_end as i32,
                    symbol.content,
                    serde_json::to_value(&symbol.embedding)?,
                    symbol.signature_hash,
                    serde_json::to_value(&symbol.references)?
                )
                .execute(&*self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn perform_semantic_search(
        &self,
        query_embedding: &[f32],
        indices: &[CodeIndex],
        processed_query: &ProcessedQuery,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let similarity_threshold = request.similarity_threshold.unwrap_or(0.7);

        // File-level search
        let file_embeddings: Vec<Vec<f32>> = indices.iter()
            .map(|index| index.embedding.clone())
            .collect();

        if !file_embeddings.is_empty() {
            let similarity_request = SimilarityRequest {
                query_embedding: query_embedding.to_vec(),
                candidate_embeddings: file_embeddings,
                similarity_metric: SimilarityMetric::Cosine,
                threshold: Some(similarity_threshold),
            };

            let similarity_response = self.embedding_manager.calculate_similarity(similarity_request).await?;

            // File-level results
            for &index_idx in &similarity_response.above_threshold {
                if index_idx < indices.len() {
                    let index = &indices[index_idx];
                    let score = similarity_response.scores[index_idx];
                    
                    if self.passes_filters(index, request)? {
                        results.push(self.create_file_search_result(index, score, processed_query).await?);
                    }
                }
            }
        }

        // Symbol-level search
        for index in indices {
            if !self.passes_filters(index, request)? {
                continue;
            }

            let symbol_embeddings: Vec<Vec<f32>> = index.symbols.iter()
                .map(|symbol| symbol.embedding.clone())
                .collect();

            if !symbol_embeddings.is_empty() {
                let similarity_request = SimilarityRequest {
                    query_embedding: query_embedding.to_vec(),
                    candidate_embeddings: symbol_embeddings,
                    similarity_metric: SimilarityMetric::Cosine,
                    threshold: Some(similarity_threshold),
                };

                let similarity_response = self.embedding_manager.calculate_similarity(similarity_request).await?;

                for &symbol_idx in &similarity_response.above_threshold {
                    if symbol_idx < index.symbols.len() {
                        let symbol = &index.symbols[symbol_idx];
                        let score = similarity_response.scores[symbol_idx];
                        
                        results.push(self.create_symbol_search_result(index, symbol, score, processed_query).await?);
                    }
                }
            }
        }

        // Pattern-based search (fallback)
        if results.len() < 10 {
            let pattern_results = self.perform_pattern_search(indices, processed_query, request).await?;
            results.extend(pattern_results);
        }

        Ok(results)
    }

    async fn create_file_search_result(
        &self,
        index: &CodeIndex,
        score: f32,
        processed_query: &ProcessedQuery,
    ) -> Result<SearchResult> {
        let content = tokio::fs::read_to_string(&index.file_path).await
            .unwrap_or_else(|_| "Content not available".to_string());

        Ok(SearchResult {
            id: Uuid::new_v4(),
            file_path: index.file_path.clone(),
            content: self.truncate_content(&content, 500),
            start_line: 1,
            end_line: content.lines().count(),
            relevance_score: score,
            match_type: MatchType::SemanticMatch,
            language: index.metadata.language.clone(),
            symbol_info: None,
            context: self.create_search_context(index, &content).await?,
            highlights: Vec::new(), // Will be filled by ranker
        })
    }

    async fn create_symbol_search_result(
        &self,
        index: &CodeIndex,
        symbol: &super::IndexedSymbol,
        score: f32,
        processed_query: &ProcessedQuery,
    ) -> Result<SearchResult> {
        let content = tokio::fs::read_to_string(&index.file_path).await
            .unwrap_or_else(|_| "Content not available".to_string());

        let symbol_info = super::SymbolInfo {
            name: symbol.name.clone(),
            symbol_type: symbol.symbol_type.clone(),
            signature: Some(symbol.content.clone()),
            documentation: None, // TODO: Extract from comments
            parameters: Vec::new(), // TODO: Parse parameters
            return_type: None, // TODO: Extract return type
            visibility: super::Visibility::Public, // TODO: Determine visibility
            complexity_score: self.calculate_symbol_complexity(&symbol.content),
        };

        Ok(SearchResult {
            id: Uuid::new_v4(),
            file_path: index.file_path.clone(),
            content: symbol.content.clone(),
            start_line: symbol.line_start,
            end_line: symbol.line_end,
            relevance_score: score,
            match_type: MatchType::SemanticMatch,
            language: index.metadata.language.clone(),
            symbol_info: Some(symbol_info),
            context: self.create_search_context(index, &content).await?,
            highlights: Vec::new(),
        })
    }

    async fn perform_pattern_search(
        &self,
        indices: &[CodeIndex],
        processed_query: &ProcessedQuery,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        
        for index in indices {
            if !self.passes_filters(index, request)? {
                continue;
            }

            let content = tokio::fs::read_to_string(&index.file_path).await
                .unwrap_or_else(|_| continue);

            // Keyword-based search
            for keyword in &processed_query.keywords {
                if content.to_lowercase().contains(&keyword.to_lowercase()) {
                    let lines: Vec<&str> = content.lines().collect();
                    
                    for (line_num, line) in lines.iter().enumerate() {
                        if line.to_lowercase().contains(&keyword.to_lowercase()) {
                            results.push(SearchResult {
                                id: Uuid::new_v4(),
                                file_path: index.file_path.clone(),
                                content: line.to_string(),
                                start_line: line_num + 1,
                                end_line: line_num + 1,
                                relevance_score: 0.6, // Lower score for pattern match
                                match_type: MatchType::PatternMatch,
                                language: index.metadata.language.clone(),
                                symbol_info: None,
                                context: self.create_search_context(index, &content).await?,
                                highlights: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn passes_filters(&self, index: &CodeIndex, request: &SearchRequest) -> Result<bool> {
        // Language filters
        if !request.language_filters.is_empty() {
            if !request.language_filters.contains(&index.metadata.language) {
                return Ok(false);
            }
        }

        // File filters
        for file_filter in &request.file_filters {
            let matches = if file_filter.pattern.contains('*') {
                // Glob pattern matching (simplified)
                let pattern = file_filter.pattern.replace('*', ".*");
                regex::Regex::new(&pattern)?.is_match(&index.file_path)
            } else {
                index.file_path.contains(&file_filter.pattern)
            };

            if file_filter.include && !matches {
                return Ok(false);
            }
            if !file_filter.include && matches {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn create_search_context(
        &self,
        index: &CodeIndex,
        content: &str,
    ) -> Result<super::SearchContext> {
        let imports = self.extract_imports(content, &index.metadata.language);
        let dependencies = self.extract_dependencies(&index.file_path).await?;
        let related_symbols: Vec<String> = index.symbols.iter()
            .map(|s| s.name.clone())
            .collect();

        let file_summary = format!(
            "{} file with {} symbols, {} lines",
            index.metadata.language,
            index.symbols.len(),
            index.metadata.line_count
        );

        let project_context = super::ProjectContext {
            project_name: self.extract_project_name(&index.file_path),
            project_type: index.metadata.language.clone(),
            main_language: index.metadata.language.clone(),
            framework: self.detect_framework(content),
            version: None,
        };

        Ok(super::SearchContext {
            surrounding_code: self.truncate_content(content, 200),
            imports,
            dependencies,
            related_symbols,
            file_summary,
            project_context,
        })
    }

    fn extract_imports(&self, content: &str, language: &str) -> Vec<String> {
        match language {
            "rust" => {
                regex::Regex::new(r"use\s+([^;]+);").unwrap()
                    .captures_iter(content)
                    .map(|cap| cap[1].to_string())
                    .collect()
            }
            "javascript" | "typescript" => {
                regex::Regex::new(r"import\s+.*\s+from\s+['\"]([^'\"]+)['\"]").unwrap()
                    .captures_iter(content)
                    .map(|cap| cap[1].to_string())
                    .collect()
            }
            "python" => {
                regex::Regex::new(r"(?:from\s+(\S+)\s+)?import\s+([^#\n]+)").unwrap()
                    .captures_iter(content)
                    .map(|cap| {
                        if let Some(from_module) = cap.get(1) {
                            format!("{}.{}", from_module.as_str(), cap[2].trim())
                        } else {
                            cap[2].trim().to_string()
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    async fn extract_dependencies(&self, file_path: &str) -> Result<Vec<String>> {
        // Extract dependencies from project files
        let project_root = std::path::Path::new(file_path)
            .ancestors()
            .find(|p| {
                p.join("Cargo.toml").exists() ||
                p.join("package.json").exists() ||
                p.join("requirements.txt").exists()
            });

        if let Some(root) = project_root {
            if root.join("Cargo.toml").exists() {
                return self.extract_cargo_dependencies(root).await;
            } else if root.join("package.json").exists() {
                return self.extract_npm_dependencies(root).await;
            }
        }

        Ok(Vec::new())
    }

    async fn extract_cargo_dependencies(&self, project_root: &std::path::Path) -> Result<Vec<String>> {
        let cargo_toml = project_root.join("Cargo.toml");
        if let Ok(content) = tokio::fs::read_to_string(cargo_toml).await {
            let deps_regex = regex::Regex::new(r#"(\w+)\s*=\s*"([^"]+)""#)?;
            return Ok(deps_regex.captures_iter(&content)
                .map(|cap| cap[1].to_string())
                .collect());
        }
        Ok(Vec::new())
    }

    async fn extract_npm_dependencies(&self, project_root: &std::path::Path) -> Result<Vec<String>> {
        let package_json = project_root.join("package.json");
        if let Ok(content) = tokio::fs::read_to_string(package_json).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let mut deps = Vec::new();
                if let Some(dependencies) = json["dependencies"].as_object() {
                    deps.extend(dependencies.keys().cloned());
                }
                if let Some(dev_dependencies) = json["devDependencies"].as_object() {
                    deps.extend(dev_dependencies.keys().cloned());
                }
                return Ok(deps);
            }
        }
        Ok(Vec::new())
    }

    fn extract_project_name(&self, file_path: &str) -> String {
        std::path::Path::new(file_path)
            .components()
            .nth(1)
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn detect_framework(&self, content: &str) -> Option<String> {
        if content.contains("React") || content.contains("useState") {
            Some("react".to_string())
        } else if content.contains("Vue") || content.contains("vue") {
            Some("vue".to_string())
        } else if content.contains("Angular") || content.contains("@angular") {
            Some("angular".to_string())
        } else if content.contains("tokio") || content.contains("async fn") {
            Some("tokio".to_string())
        } else {
            None
        }
    }

    fn calculate_symbol_complexity(&self, content: &str) -> f32 {
        let lines = content.lines().count() as f32;
        let control_structures = content.matches("if ").count() + 
                               content.matches("for ").count() + 
                               content.matches("while ").count() + 
                               content.matches("match ").count();
        
        (lines / 10.0 + control_structures as f32 * 0.5).min(10.0)
    }

    fn truncate_content(&self, content: &str, max_chars: usize) -> String {
        if content.len() <= max_chars {
            content.to_string()
        } else {
            format!("{}...", &content[..max_chars])
        }
    }

    async fn build_ranking_context(&self, request: &SearchRequest) -> Result<Option<RankingContext>> {
        // TODO: Build ranking context from user preferences and workspace context
        Ok(None)
    }

    async fn generate_suggestions(
        &self,
        request: &SearchRequest,
        processed_query: &ProcessedQuery,
        results: &[SearchResult],
    ) -> Result<Vec<SearchSuggestion>> {
        let mut suggestions = Vec::new();

        // Query improvement suggestions
        if results.len() < 5 {
            let improvements = self.query_processor.suggest_query_improvements(&request.query).await?;
            for improvement in improvements {
                suggestions.push(SearchSuggestion {
                    suggestion: improvement,
                    suggestion_type: SuggestionType::QueryExpansion,
                    confidence: 0.8,
                    reason: "Low result count - try expanding query".to_string(),
                });
            }
        }

        // Filter suggestions based on aggregations
        if !request.language_filters.is_empty() && results.len() > 20 {
            suggestions.push(SearchSuggestion {
                suggestion: "Remove language filter for more results".to_string(),
                suggestion_type: SuggestionType::FilterSuggestion,
                confidence: 0.7,
                reason: "Many results available in other languages".to_string(),
            });
        }

        Ok(suggestions)
    }

    async fn generate_related_queries(&self, processed_query: &ProcessedQuery) -> Result<Vec<String>> {
        // Generate related queries based on entities and intent
        let mut related = Vec::new();

        for entity in &processed_query.entities {
            related.push(format!("examples of {}", entity.text));
            related.push(format!("how to use {}", entity.text));
            related.push(format!("{} documentation", entity.text));
        }

        match processed_query.intent {
            super::QueryIntent::FindFunction => {
                related.push("function examples".to_string());
                related.push("method implementations".to_string());
            }
            super::QueryIntent::FindClass => {
                related.push("class definitions".to_string());
                related.push("struct examples".to_string());
            }
            _ => {}
        }

        Ok(related.into_iter().take(5).collect())
    }

    fn extract_applied_filters(&self, request: &SearchRequest, processed_query: &ProcessedQuery) -> Vec<String> {
        let mut filters = Vec::new();

        if !request.language_filters.is_empty() {
            filters.push(format!("Languages: {}", request.language_filters.join(", ")));
        }

        if !request.file_filters.is_empty() {
            filters.push(format!("Files: {} filters", request.file_filters.len()));
        }

        if let Some(threshold) = request.similarity_threshold {
            filters.push(format!("Similarity threshold: {:.2}", threshold));
        }

        filters.push(format!("Intent: {:?}", processed_query.intent));

        filters
    }

    pub async fn reindex_workspace(&self, workspace_path: &str) -> Result<()> {
        info!("Reindexing workspace: {}", workspace_path);

        // Cache'den kaldır
        {
            let mut cache = self.index_cache.write().await;
            cache.remove(workspace_path);
        }

        // Veritabanından eski index'leri sil
        sqlx::query!(
            "DELETE FROM code_index WHERE file_path LIKE $1",
            format!("{}%", workspace_path)
        )
        .execute(&*self.pool)
        .await?;

        // Yeniden index'le
        self.get_or_create_indices(workspace_path).await?;

        info!("Workspace reindexing completed: {}", workspace_path);
        Ok(())
    }

    pub async fn get_index_stats(&self, workspace_path: &str) -> Result<IndexStats> {
        let row = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_files,
                COUNT(DISTINCT metadata->>'language') as languages,
                SUM((metadata->>'symbol_count')::int) as total_symbols,
                AVG((metadata->>'complexity_score')::float) as avg_complexity
            FROM code_index 
            WHERE file_path LIKE $1
            "#,
            format!("{}%", workspace_path)
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(IndexStats {
            total_files: row.total_files.unwrap_or(0),
            total_symbols: row.total_symbols.unwrap_or(Some(0)).unwrap_or(0),
            languages_count: row.languages.unwrap_or(0),
            avg_complexity: row.avg_complexity.unwrap_or(0.0) as f32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_files: i64,
    pub total_symbols: i64,
    pub languages_count: i64,
    pub avg_complexity: f32,
}