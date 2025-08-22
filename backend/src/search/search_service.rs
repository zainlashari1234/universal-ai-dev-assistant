use anyhow::Result;
use std::sync::Arc;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::{info, debug, error};

use crate::providers::ProviderRouter;
use super::{
    SearchRequest, SearchResponse, SearchAnalytics, QueryRefinement, RefinementType,
    embedding_manager::EmbeddingManager,
    query_processor::QueryProcessor,
    result_ranker::ResultRanker,
    code_indexer::CodeIndexer,
    semantic_engine::{SemanticSearchEngine, IndexStats},
};

pub struct SearchService {
    semantic_engine: Arc<SemanticSearchEngine>,
    pool: Arc<PgPool>,
    analytics_enabled: bool,
}

impl SearchService {
    pub fn new(
        provider_router: Arc<ProviderRouter>,
        pool: Arc<PgPool>,
    ) -> Self {
        // Initialize all components
        let embedding_manager = Arc::new(EmbeddingManager::new(provider_router.clone()));
        let query_processor = Arc::new(QueryProcessor::new(
            provider_router.clone(),
            embedding_manager.clone(),
        ));
        let result_ranker = Arc::new(ResultRanker::new());
        let code_indexer = Arc::new(CodeIndexer::new(embedding_manager.clone()));
        
        let semantic_engine = Arc::new(SemanticSearchEngine::new(
            embedding_manager,
            query_processor,
            result_ranker,
            code_indexer,
            pool.clone(),
        ));

        Self {
            semantic_engine,
            pool,
            analytics_enabled: true,
        }
    }

    pub async fn search(&self, mut request: SearchRequest, user_id: Uuid) -> Result<SearchResponse> {
        info!("Processing search request for user: {}", user_id);
        
        // Validate request
        self.validate_request(&request)?;
        
        // Apply user preferences
        self.apply_user_preferences(&mut request, user_id).await?;
        
        // Perform search
        let response = self.semantic_engine.search(request.clone()).await?;
        
        // Log analytics
        if self.analytics_enabled {
            self.log_search_analytics(&request, &response, user_id).await?;
        }
        
        // Update search history
        self.update_search_history(&request, user_id).await?;
        
        Ok(response)
    }

    pub async fn search_similar_code(&self, code_snippet: &str, workspace_paths: Vec<String>, user_id: Uuid) -> Result<SearchResponse> {
        info!("Searching for similar code for user: {}", user_id);
        
        let request = SearchRequest {
            query: format!("similar code: {}", code_snippet),
            query_type: super::SearchQueryType::Semantic,
            workspace_paths,
            file_filters: Vec::new(),
            language_filters: Vec::new(),
            max_results: Some(20),
            similarity_threshold: Some(0.8),
            include_context: true,
        };
        
        self.search(request, user_id).await
    }

    pub async fn search_by_symbol(&self, symbol_name: &str, symbol_type: Option<super::SymbolType>, workspace_paths: Vec<String>, user_id: Uuid) -> Result<SearchResponse> {
        info!("Searching for symbol: {} for user: {}", symbol_name, user_id);
        
        let query = if let Some(sym_type) = symbol_type {
            format!("{:?} {}", sym_type, symbol_name)
        } else {
            symbol_name.to_string()
        };
        
        let request = SearchRequest {
            query,
            query_type: super::SearchQueryType::SymbolName,
            workspace_paths,
            file_filters: Vec::new(),
            language_filters: Vec::new(),
            max_results: Some(50),
            similarity_threshold: Some(0.7),
            include_context: true,
        };
        
        self.search(request, user_id).await
    }

    pub async fn search_documentation(&self, query: &str, workspace_paths: Vec<String>, user_id: Uuid) -> Result<SearchResponse> {
        info!("Searching documentation for: {} for user: {}", query, user_id);
        
        let request = SearchRequest {
            query: query.to_string(),
            query_type: super::SearchQueryType::Documentation,
            workspace_paths,
            file_filters: vec![
                super::FileFilter {
                    pattern: "*.md".to_string(),
                    include: true,
                },
                super::FileFilter {
                    pattern: "README*".to_string(),
                    include: true,
                },
                super::FileFilter {
                    pattern: "docs/*".to_string(),
                    include: true,
                },
            ],
            language_filters: Vec::new(),
            max_results: Some(30),
            similarity_threshold: Some(0.6),
            include_context: true,
        };
        
        self.search(request, user_id).await
    }

    pub async fn search_errors(&self, error_message: &str, workspace_paths: Vec<String>, user_id: Uuid) -> Result<SearchResponse> {
        info!("Searching for error solutions: {} for user: {}", error_message, user_id);
        
        let request = SearchRequest {
            query: format!("error: {}", error_message),
            query_type: super::SearchQueryType::ErrorMessage,
            workspace_paths,
            file_filters: Vec::new(),
            language_filters: Vec::new(),
            max_results: Some(25),
            similarity_threshold: Some(0.6),
            include_context: true,
        };
        
        self.search(request, user_id).await
    }

    pub async fn get_search_suggestions(&self, partial_query: &str, user_id: Uuid) -> Result<Vec<String>> {
        info!("Getting search suggestions for: {} for user: {}", partial_query, user_id);
        
        // Get suggestions from search history
        let history_suggestions = self.get_suggestions_from_history(partial_query, user_id).await?;
        
        // Get popular searches
        let popular_suggestions = self.get_popular_searches(partial_query).await?;
        
        // Combine and deduplicate
        let mut all_suggestions = history_suggestions;
        all_suggestions.extend(popular_suggestions);
        all_suggestions.sort();
        all_suggestions.dedup();
        
        Ok(all_suggestions.into_iter().take(10).collect())
    }

    pub async fn index_workspace(&self, workspace_path: &str, user_id: Uuid) -> Result<IndexStats> {
        info!("Indexing workspace: {} for user: {}", workspace_path, user_id);
        
        // Check if user has permission to index this workspace
        self.check_workspace_permission(workspace_path, user_id).await?;
        
        // Perform indexing
        self.semantic_engine.reindex_workspace(workspace_path).await?;
        
        // Get stats
        let stats = self.semantic_engine.get_index_stats(workspace_path).await?;
        
        // Log indexing activity
        self.log_indexing_activity(workspace_path, user_id, &stats).await?;
        
        Ok(stats)
    }

    pub async fn get_workspace_stats(&self, workspace_path: &str, user_id: Uuid) -> Result<IndexStats> {
        info!("Getting workspace stats: {} for user: {}", workspace_path, user_id);
        
        self.check_workspace_permission(workspace_path, user_id).await?;
        self.semantic_engine.get_index_stats(workspace_path).await
    }

    pub async fn get_user_search_analytics(&self, user_id: Uuid, days: i32) -> Result<UserSearchAnalytics> {
        let start_date = chrono::Utc::now() - chrono::Duration::days(days as i64);
        
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_searches,
                COUNT(DISTINCT query) as unique_queries,
                AVG(results_count) as avg_results,
                AVG(search_time_ms) as avg_search_time,
                AVG(user_satisfaction) as avg_satisfaction
            FROM search_analytics 
            WHERE user_id = $1 AND timestamp >= $2
            "#,
            user_id,
            start_date
        )
        .fetch_one(&*self.pool)
        .await?;

        let popular_queries = sqlx::query!(
            r#"
            SELECT query, COUNT(*) as count
            FROM search_analytics 
            WHERE user_id = $1 AND timestamp >= $2
            GROUP BY query
            ORDER BY count DESC
            LIMIT 10
            "#,
            user_id,
            start_date
        )
        .fetch_all(&*self.pool)
        .await?;

        let popular_queries: Vec<(String, i64)> = popular_queries
            .into_iter()
            .map(|row| (row.query, row.count.unwrap_or(0)))
            .collect();

        Ok(UserSearchAnalytics {
            total_searches: stats.total_searches.unwrap_or(0),
            unique_queries: stats.unique_queries.unwrap_or(0),
            avg_results: stats.avg_results.unwrap_or(0.0) as f32,
            avg_search_time_ms: stats.avg_search_time.unwrap_or(0.0) as u64,
            avg_satisfaction: stats.avg_satisfaction.unwrap_or(0.0) as f32,
            popular_queries,
        })
    }

    pub async fn provide_search_feedback(&self, search_id: Uuid, feedback: SearchFeedback, user_id: Uuid) -> Result<()> {
        info!("Received search feedback from user: {}", user_id);
        
        // Update analytics
        sqlx::query!(
            r#"
            UPDATE search_analytics 
            SET user_satisfaction = $1
            WHERE query_id = $2 AND user_id = $3
            "#,
            feedback.satisfaction_score,
            search_id,
            user_id
        )
        .execute(&*self.pool)
        .await?;

        // Log feedback for ML training
        sqlx::query!(
            r#"
            INSERT INTO search_feedback (search_id, user_id, feedback_type, satisfaction_score, comments, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            search_id,
            user_id,
            serde_json::to_string(&feedback.feedback_type)?,
            feedback.satisfaction_score,
            feedback.comments,
            chrono::Utc::now()
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    async fn validate_request(&self, request: &SearchRequest) -> Result<()> {
        if request.query.trim().is_empty() {
            return Err(anyhow::anyhow!("Search query cannot be empty"));
        }

        if request.query.len() > 1000 {
            return Err(anyhow::anyhow!("Search query too long (max 1000 characters)"));
        }

        if request.workspace_paths.is_empty() {
            return Err(anyhow::anyhow!("At least one workspace path is required"));
        }

        if let Some(max_results) = request.max_results {
            if max_results > 1000 {
                return Err(anyhow::anyhow!("Max results cannot exceed 1000"));
            }
        }

        Ok(())
    }

    async fn apply_user_preferences(&self, request: &mut SearchRequest, user_id: Uuid) -> Result<()> {
        // Load user preferences from database
        let preferences = self.load_user_search_preferences(user_id).await?;
        
        // Apply language preferences
        if request.language_filters.is_empty() && !preferences.preferred_languages.is_empty() {
            request.language_filters = preferences.preferred_languages;
        }
        
        // Apply default similarity threshold
        if request.similarity_threshold.is_none() {
            request.similarity_threshold = Some(preferences.default_similarity_threshold);
        }
        
        // Apply max results preference
        if request.max_results.is_none() {
            request.max_results = Some(preferences.default_max_results);
        }
        
        Ok(())
    }

    async fn load_user_search_preferences(&self, user_id: Uuid) -> Result<UserSearchPreferences> {
        let row = sqlx::query!(
            r#"
            SELECT search_preferences 
            FROM user_preferences 
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            if let Some(prefs_json) = row.search_preferences {
                return Ok(serde_json::from_value(prefs_json)?);
            }
        }

        Ok(UserSearchPreferences::default())
    }

    async fn log_search_analytics(&self, request: &SearchRequest, response: &SearchResponse, user_id: Uuid) -> Result<()> {
        let analytics = SearchAnalytics {
            query_id: Uuid::new_v4(),
            user_id,
            query: request.query.clone(),
            results_count: response.results.len(),
            clicked_results: Vec::new(), // Will be updated when user clicks
            search_time_ms: response.search_time_ms,
            user_satisfaction: None, // Will be updated with feedback
            refinements: Vec::new(),
            timestamp: chrono::Utc::now(),
        };

        sqlx::query!(
            r#"
            INSERT INTO search_analytics (query_id, user_id, query, results_count, search_time_ms, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            analytics.query_id,
            analytics.user_id,
            analytics.query,
            analytics.results_count as i32,
            analytics.search_time_ms as i64,
            analytics.timestamp
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    async fn update_search_history(&self, request: &SearchRequest, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO search_history (user_id, query, query_type, timestamp)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, query) DO UPDATE SET
                timestamp = EXCLUDED.timestamp,
                search_count = search_count + 1
            "#,
            user_id,
            request.query,
            serde_json::to_string(&request.query_type)?,
            chrono::Utc::now()
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    async fn get_suggestions_from_history(&self, partial_query: &str, user_id: Uuid) -> Result<Vec<String>> {
        let rows = sqlx::query!(
            r#"
            SELECT query 
            FROM search_history 
            WHERE user_id = $1 AND query ILIKE $2
            ORDER BY timestamp DESC
            LIMIT 5
            "#,
            user_id,
            format!("{}%", partial_query)
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.query).collect())
    }

    async fn get_popular_searches(&self, partial_query: &str) -> Result<Vec<String>> {
        let rows = sqlx::query!(
            r#"
            SELECT query, COUNT(*) as count
            FROM search_analytics 
            WHERE query ILIKE $1
            GROUP BY query
            ORDER BY count DESC
            LIMIT 5
            "#,
            format!("{}%", partial_query)
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.query).collect())
    }

    async fn check_workspace_permission(&self, workspace_path: &str, user_id: Uuid) -> Result<()> {
        // TODO: Implement proper workspace permission checking
        // For now, allow all users to access all workspaces
        debug!("Checking workspace permission for user {} on path {}", user_id, workspace_path);
        Ok(())
    }

    async fn log_indexing_activity(&self, workspace_path: &str, user_id: Uuid, stats: &IndexStats) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO indexing_activity (user_id, workspace_path, files_indexed, symbols_indexed, timestamp)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            user_id,
            workspace_path,
            stats.total_files as i32,
            stats.total_symbols as i32,
            chrono::Utc::now()
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_old_analytics(&self, days_to_keep: i32) -> Result<u64> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_to_keep as i64);
        
        let result = sqlx::query!(
            "DELETE FROM search_analytics WHERE timestamp < $1",
            cutoff_date
        )
        .execute(&*self.pool)
        .await?;

        info!("Cleaned up {} old search analytics records", result.rows_affected());
        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone)]
pub struct UserSearchPreferences {
    pub preferred_languages: Vec<String>,
    pub default_similarity_threshold: f32,
    pub default_max_results: usize,
    pub enable_semantic_search: bool,
    pub boost_recent_files: bool,
}

impl Default for UserSearchPreferences {
    fn default() -> Self {
        Self {
            preferred_languages: Vec::new(),
            default_similarity_threshold: 0.7,
            default_max_results: 50,
            enable_semantic_search: true,
            boost_recent_files: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserSearchAnalytics {
    pub total_searches: i64,
    pub unique_queries: i64,
    pub avg_results: f32,
    pub avg_search_time_ms: u64,
    pub avg_satisfaction: f32,
    pub popular_queries: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct SearchFeedback {
    pub feedback_type: SearchFeedbackType,
    pub satisfaction_score: f32,
    pub comments: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchFeedbackType {
    Helpful,
    NotHelpful,
    Irrelevant,
    Perfect,
}