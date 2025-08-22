use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

use super::{
    SearchResult, SearchRequest, ProcessedQuery, MatchType, QueryIntent, 
    SymbolType, Highlight, HighlightType, SearchAggregations,
    ComplexityBucket, TemporalBucket
};

pub struct ResultRanker {
    ranking_weights: RankingWeights,
}

#[derive(Debug, Clone)]
pub struct RankingWeights {
    pub relevance_score: f32,
    pub match_type: f32,
    pub symbol_type: f32,
    pub file_recency: f32,
    pub code_quality: f32,
    pub complexity_penalty: f32,
    pub boost_terms: f32,
    pub user_preferences: f32,
}

#[derive(Debug, Clone)]
pub struct RankingContext {
    pub query_intent: QueryIntent,
    pub user_preferences: UserPreferences,
    pub workspace_context: WorkspaceContext,
    pub search_history: Vec<SearchHistoryItem>,
}

#[derive(Debug, Clone)]
pub struct UserPreferences {
    pub preferred_languages: Vec<String>,
    pub preferred_complexity: ComplexityPreference,
    pub preferred_file_types: Vec<String>,
    pub boost_recent_files: bool,
    pub boost_frequently_accessed: bool,
}

#[derive(Debug, Clone)]
pub enum ComplexityPreference {
    Simple,
    Moderate,
    Complex,
    Any,
}

#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub current_project_languages: Vec<String>,
    pub recently_modified_files: Vec<String>,
    pub frequently_accessed_files: Vec<String>,
    pub project_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SearchHistoryItem {
    pub query: String,
    pub clicked_results: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ResultRanker {
    pub fn new() -> Self {
        Self {
            ranking_weights: RankingWeights::default(),
        }
    }

    pub fn with_weights(weights: RankingWeights) -> Self {
        Self {
            ranking_weights: weights,
        }
    }

    pub fn rank_results(
        &self,
        mut results: Vec<SearchResult>,
        request: &SearchRequest,
        processed_query: &ProcessedQuery,
        context: Option<&RankingContext>,
    ) -> Result<Vec<SearchResult>> {
        info!("Ranking {} search results", results.len());

        // Her result için final score hesapla
        for result in &mut results {
            result.relevance_score = self.calculate_final_score(
                result,
                request,
                processed_query,
                context,
            )?;
        }

        // Score'a göre sırala (yüksekten düşüğe)
        results.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Highlights ekle
        for result in &mut results {
            result.highlights = self.generate_highlights(result, processed_query)?;
        }

        // Diversity sağla (aynı dosyadan çok fazla sonuç varsa)
        let diversified_results = self.apply_diversity_filter(results, request.max_results.unwrap_or(50))?;

        debug!("Ranked and filtered {} results", diversified_results.len());
        Ok(diversified_results)
    }

    fn calculate_final_score(
        &self,
        result: &SearchResult,
        request: &SearchRequest,
        processed_query: &ProcessedQuery,
        context: Option<&RankingContext>,
    ) -> Result<f32> {
        let mut score = result.relevance_score * self.ranking_weights.relevance_score;

        // Match type boost
        let match_type_boost = match result.match_type {
            MatchType::ExactMatch => 1.0,
            MatchType::SemanticMatch => 0.9,
            MatchType::PatternMatch => 0.8,
            MatchType::FuzzyMatch => 0.7,
            MatchType::ContextualMatch => 0.6,
        };
        score *= match_type_boost * self.ranking_weights.match_type;

        // Symbol type boost (intent'e göre)
        if let Some(symbol_info) = &result.symbol_info {
            let symbol_boost = self.calculate_symbol_type_boost(
                &symbol_info.symbol_type,
                &processed_query.intent,
            );
            score *= symbol_boost * self.ranking_weights.symbol_type;
        }

        // Code quality boost
        if let Some(symbol_info) = &result.symbol_info {
            let quality_boost = 1.0 + (symbol_info.complexity_score / 10.0) * 0.2;
            score *= quality_boost * self.ranking_weights.code_quality;
        }

        // Complexity penalty/boost
        if let Some(symbol_info) = &result.symbol_info {
            let complexity_factor = self.calculate_complexity_factor(
                symbol_info.complexity_score,
                context.map(|c| &c.user_preferences.preferred_complexity),
            );
            score *= complexity_factor;
        }

        // Boost terms matching
        let boost_score = self.calculate_boost_score(result, processed_query);
        score *= 1.0 + (boost_score * self.ranking_weights.boost_terms);

        // User preferences
        if let Some(ctx) = context {
            let preference_boost = self.calculate_preference_boost(result, &ctx.user_preferences);
            score *= preference_boost * self.ranking_weights.user_preferences;
        }

        // File recency boost
        let recency_boost = self.calculate_recency_boost(result, context);
        score *= recency_boost * self.ranking_weights.file_recency;

        // Language filter boost
        if !request.language_filters.is_empty() {
            if request.language_filters.contains(&result.language) {
                score *= 1.5; // Language match boost
            } else {
                score *= 0.7; // Language mismatch penalty
            }
        }

        Ok(score.max(0.0).min(10.0)) // Clamp between 0 and 10
    }

    fn calculate_symbol_type_boost(&self, symbol_type: &SymbolType, intent: &QueryIntent) -> f32 {
        match (intent, symbol_type) {
            (QueryIntent::FindFunction, SymbolType::Function) => 1.5,
            (QueryIntent::FindFunction, SymbolType::Method) => 1.4,
            (QueryIntent::FindClass, SymbolType::Class) => 1.5,
            (QueryIntent::FindClass, SymbolType::Struct) => 1.4,
            (QueryIntent::FindClass, SymbolType::Interface) => 1.3,
            (QueryIntent::FindDefinition, SymbolType::Function) => 1.3,
            (QueryIntent::FindDefinition, SymbolType::Class) => 1.3,
            (QueryIntent::FindDefinition, SymbolType::Variable) => 1.2,
            (QueryIntent::FindUsage, _) => 1.1, // Usage can be any symbol type
            _ => 1.0,
        }
    }

    fn calculate_complexity_factor(
        &self,
        complexity_score: f32,
        preferred_complexity: Option<&ComplexityPreference>,
    ) -> f32 {
        match preferred_complexity {
            Some(ComplexityPreference::Simple) => {
                if complexity_score <= 3.0 {
                    1.2
                } else if complexity_score <= 6.0 {
                    1.0
                } else {
                    0.8
                }
            }
            Some(ComplexityPreference::Moderate) => {
                if complexity_score >= 3.0 && complexity_score <= 7.0 {
                    1.1
                } else {
                    0.9
                }
            }
            Some(ComplexityPreference::Complex) => {
                if complexity_score >= 6.0 {
                    1.2
                } else {
                    0.9
                }
            }
            Some(ComplexityPreference::Any) | None => 1.0,
        }
    }

    fn calculate_boost_score(&self, result: &SearchResult, processed_query: &ProcessedQuery) -> f32 {
        let mut boost_score = 0.0;
        let content_lower = result.content.to_lowercase();
        
        for boost_term in &processed_query.boost_terms {
            if content_lower.contains(&boost_term.term.to_lowercase()) {
                boost_score += boost_term.boost_factor * 0.1; // Scale down the boost
            }
        }
        
        boost_score.min(1.0) // Cap at 100% boost
    }

    fn calculate_preference_boost(&self, result: &SearchResult, preferences: &UserPreferences) -> f32 {
        let mut boost = 1.0;
        
        // Language preference
        if preferences.preferred_languages.contains(&result.language) {
            boost *= 1.2;
        }
        
        // File type preference
        let file_extension = std::path::Path::new(&result.file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if preferences.preferred_file_types.contains(&file_extension.to_string()) {
            boost *= 1.1;
        }
        
        boost
    }

    fn calculate_recency_boost(&self, result: &SearchResult, context: Option<&RankingContext>) -> f32 {
        if let Some(ctx) = context {
            if ctx.workspace_context.recently_modified_files.contains(&result.file_path) {
                return 1.3;
            }
            
            if ctx.workspace_context.frequently_accessed_files.contains(&result.file_path) {
                return 1.2;
            }
        }
        
        1.0
    }

    fn generate_highlights(&self, result: &SearchResult, processed_query: &ProcessedQuery) -> Result<Vec<Highlight>> {
        let mut highlights = Vec::new();
        let content_lower = result.content.to_lowercase();
        
        // Exact keyword matches
        for keyword in &processed_query.keywords {
            let keyword_lower = keyword.to_lowercase();
            let mut start = 0;
            
            while let Some(pos) = content_lower[start..].find(&keyword_lower) {
                let actual_pos = start + pos;
                highlights.push(Highlight {
                    start_offset: actual_pos,
                    end_offset: actual_pos + keyword.len(),
                    highlight_type: HighlightType::KeywordMatch,
                    explanation: Some(format!("Keyword match: {}", keyword)),
                });
                start = actual_pos + keyword.len();
            }
        }
        
        // Entity matches
        for entity in &processed_query.entities {
            let entity_lower = entity.text.to_lowercase();
            if let Some(pos) = content_lower.find(&entity_lower) {
                highlights.push(Highlight {
                    start_offset: pos,
                    end_offset: pos + entity.text.len(),
                    highlight_type: HighlightType::SymbolMatch,
                    explanation: Some(format!("Entity match: {:?}", entity.entity_type)),
                });
            }
        }
        
        // Boost term matches
        for boost_term in &processed_query.boost_terms {
            let term_lower = boost_term.term.to_lowercase();
            if let Some(pos) = content_lower.find(&term_lower) {
                highlights.push(Highlight {
                    start_offset: pos,
                    end_offset: pos + boost_term.term.len(),
                    highlight_type: HighlightType::SemanticMatch,
                    explanation: Some(format!("Boost term: {}", boost_term.reason)),
                });
            }
        }
        
        // Remove overlapping highlights
        highlights.sort_by_key(|h| h.start_offset);
        self.remove_overlapping_highlights(highlights)
    }

    fn remove_overlapping_highlights(&self, mut highlights: Vec<Highlight>) -> Result<Vec<Highlight>> {
        highlights.sort_by_key(|h| (h.start_offset, h.end_offset));
        let mut result = Vec::new();
        
        for highlight in highlights {
            let overlaps = result.iter().any(|existing: &Highlight| {
                highlight.start_offset < existing.end_offset && highlight.end_offset > existing.start_offset
            });
            
            if !overlaps {
                result.push(highlight);
            }
        }
        
        Ok(result)
    }

    fn apply_diversity_filter(&self, results: Vec<SearchResult>, max_results: usize) -> Result<Vec<SearchResult>> {
        let mut diversified = Vec::new();
        let mut file_counts: HashMap<String, usize> = HashMap::new();
        let max_per_file = 3; // Maximum results per file
        
        for result in results {
            let file_count = file_counts.get(&result.file_path).unwrap_or(&0);
            
            if *file_count < max_per_file && diversified.len() < max_results {
                diversified.push(result.clone());
                file_counts.insert(result.file_path.clone(), file_count + 1);
            } else if diversified.len() >= max_results {
                break;
            }
        }
        
        // If we haven't reached max_results, add remaining results
        if diversified.len() < max_results {
            for result in results {
                if !diversified.iter().any(|r| r.id == result.id) && diversified.len() < max_results {
                    diversified.push(result);
                }
            }
        }
        
        Ok(diversified)
    }

    pub fn generate_aggregations(&self, results: &[SearchResult]) -> SearchAggregations {
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut file_types: HashMap<String, usize> = HashMap::new();
        let mut projects: HashMap<String, usize> = HashMap::new();
        let mut symbol_types: HashMap<String, usize> = HashMap::new();
        
        for result in results {
            // Language aggregation
            *languages.entry(result.language.clone()).or_insert(0) += 1;
            
            // File type aggregation
            let file_extension = std::path::Path::new(&result.file_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("unknown")
                .to_string();
            *file_types.entry(file_extension).or_insert(0) += 1;
            
            // Project aggregation (from file path)
            let project_name = result.file_path
                .split('/')
                .nth(1)
                .unwrap_or("unknown")
                .to_string();
            *projects.entry(project_name).or_insert(0) += 1;
            
            // Symbol type aggregation
            if let Some(symbol_info) = &result.symbol_info {
                let symbol_type_str = format!("{:?}", symbol_info.symbol_type);
                *symbol_types.entry(symbol_type_str).or_insert(0) += 1;
            }
        }
        
        // Complexity distribution
        let complexity_distribution = self.calculate_complexity_distribution(results);
        
        // Temporal distribution (simplified)
        let temporal_distribution = vec![
            TemporalBucket {
                period: "recent".to_string(),
                count: results.len(),
                last_modified: chrono::Utc::now(),
            }
        ];
        
        SearchAggregations {
            languages,
            file_types,
            projects,
            symbol_types,
            complexity_distribution,
            temporal_distribution,
        }
    }

    fn calculate_complexity_distribution(&self, results: &[SearchResult]) -> Vec<ComplexityBucket> {
        let mut low_complexity = 0;
        let mut medium_complexity = 0;
        let mut high_complexity = 0;
        let mut total_relevance = 0.0;
        let mut low_relevance = 0.0;
        let mut medium_relevance = 0.0;
        let mut high_relevance = 0.0;
        
        for result in results {
            if let Some(symbol_info) = &result.symbol_info {
                total_relevance += result.relevance_score;
                
                if symbol_info.complexity_score <= 3.0 {
                    low_complexity += 1;
                    low_relevance += result.relevance_score;
                } else if symbol_info.complexity_score <= 7.0 {
                    medium_complexity += 1;
                    medium_relevance += result.relevance_score;
                } else {
                    high_complexity += 1;
                    high_relevance += result.relevance_score;
                }
            }
        }
        
        vec![
            ComplexityBucket {
                range: "low (0-3)".to_string(),
                count: low_complexity,
                avg_relevance: if low_complexity > 0 { low_relevance / low_complexity as f32 } else { 0.0 },
            },
            ComplexityBucket {
                range: "medium (3-7)".to_string(),
                count: medium_complexity,
                avg_relevance: if medium_complexity > 0 { medium_relevance / medium_complexity as f32 } else { 0.0 },
            },
            ComplexityBucket {
                range: "high (7+)".to_string(),
                count: high_complexity,
                avg_relevance: if high_complexity > 0 { high_relevance / high_complexity as f32 } else { 0.0 },
            },
        ]
    }

    pub fn update_weights_from_feedback(&mut self, feedback: &SearchFeedback) {
        // Machine learning-like weight adjustment based on user feedback
        match feedback.feedback_type {
            FeedbackType::Clicked => {
                // Increase weights for factors that contributed to this result
                if feedback.result.match_type == MatchType::SemanticMatch {
                    self.ranking_weights.match_type *= 1.05;
                }
            }
            FeedbackType::NotRelevant => {
                // Decrease weights for factors that contributed to this result
                if feedback.result.match_type == MatchType::FuzzyMatch {
                    self.ranking_weights.match_type *= 0.95;
                }
            }
            FeedbackType::VeryRelevant => {
                // Significantly increase weights
                self.ranking_weights.relevance_score *= 1.1;
            }
        }
        
        // Normalize weights to prevent drift
        self.normalize_weights();
    }

    fn normalize_weights(&mut self) {
        let total = self.ranking_weights.relevance_score +
                   self.ranking_weights.match_type +
                   self.ranking_weights.symbol_type +
                   self.ranking_weights.file_recency +
                   self.ranking_weights.code_quality +
                   self.ranking_weights.boost_terms +
                   self.ranking_weights.user_preferences;
        
        if total > 0.0 {
            let scale = 7.0 / total; // Target sum of 7.0
            self.ranking_weights.relevance_score *= scale;
            self.ranking_weights.match_type *= scale;
            self.ranking_weights.symbol_type *= scale;
            self.ranking_weights.file_recency *= scale;
            self.ranking_weights.code_quality *= scale;
            self.ranking_weights.boost_terms *= scale;
            self.ranking_weights.user_preferences *= scale;
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchFeedback {
    pub result: SearchResult,
    pub feedback_type: FeedbackType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum FeedbackType {
    Clicked,
    NotRelevant,
    VeryRelevant,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            relevance_score: 1.0,
            match_type: 0.8,
            symbol_type: 0.7,
            file_recency: 0.5,
            code_quality: 0.6,
            complexity_penalty: 0.3,
            boost_terms: 0.9,
            user_preferences: 0.4,
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            preferred_languages: vec!["rust".to_string(), "javascript".to_string()],
            preferred_complexity: ComplexityPreference::Moderate,
            preferred_file_types: vec!["rs".to_string(), "js".to_string(), "ts".to_string()],
            boost_recent_files: true,
            boost_frequently_accessed: true,
        }
    }
}

impl Default for WorkspaceContext {
    fn default() -> Self {
        Self {
            current_project_languages: Vec::new(),
            recently_modified_files: Vec::new(),
            frequently_accessed_files: Vec::new(),
            project_patterns: Vec::new(),
        }
    }
}