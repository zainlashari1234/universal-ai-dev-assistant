pub mod semantic_engine;
pub mod code_indexer;
pub mod embedding_manager;
pub mod query_processor;
pub mod result_ranker;
pub mod search_service;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub query_type: SearchQueryType,
    pub workspace_paths: Vec<String>,
    pub file_filters: Vec<FileFilter>,
    pub language_filters: Vec<String>,
    pub max_results: Option<usize>,
    pub similarity_threshold: Option<f32>,
    pub include_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchQueryType {
    NaturalLanguage,
    CodePattern,
    FunctionSignature,
    SymbolName,
    Documentation,
    ErrorMessage,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilter {
    pub pattern: String,
    pub include: bool, // true for include, false for exclude
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub file_path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub relevance_score: f32,
    pub match_type: MatchType,
    pub language: String,
    pub symbol_info: Option<SymbolInfo>,
    pub context: SearchContext,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    ExactMatch,
    SemanticMatch,
    PatternMatch,
    FuzzyMatch,
    ContextualMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: SymbolType,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub complexity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Variable,
    Constant,
    Module,
    Namespace,
    Trait,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    pub surrounding_code: String,
    pub imports: Vec<String>,
    pub dependencies: Vec<String>,
    pub related_symbols: Vec<String>,
    pub file_summary: String,
    pub project_context: ProjectContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_name: String,
    pub project_type: String,
    pub main_language: String,
    pub framework: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub start_offset: usize,
    pub end_offset: usize,
    pub highlight_type: HighlightType,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HighlightType {
    ExactMatch,
    SemanticMatch,
    KeywordMatch,
    SymbolMatch,
    PatternMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndex {
    pub id: Uuid,
    pub file_path: String,
    pub content_hash: String,
    pub embedding: Vec<f32>,
    pub symbols: Vec<IndexedSymbol>,
    pub metadata: IndexMetadata,
    pub indexed_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSymbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub embedding: Vec<f32>,
    pub signature_hash: String,
    pub references: Vec<SymbolReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub file_path: String,
    pub line_number: usize,
    pub reference_type: ReferenceType,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Definition,
    Usage,
    Import,
    Call,
    Inheritance,
    Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub language: String,
    pub file_size: u64,
    pub line_count: usize,
    pub symbol_count: usize,
    pub complexity_score: f32,
    pub quality_score: f32,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_matches: usize,
    pub search_time_ms: u64,
    pub suggestions: Vec<SearchSuggestion>,
    pub related_queries: Vec<String>,
    pub filters_applied: Vec<String>,
    pub aggregations: SearchAggregations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub suggestion: String,
    pub suggestion_type: SuggestionType,
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    QueryCorrection,
    QueryExpansion,
    AlternativeQuery,
    FilterSuggestion,
    ScopeSuggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAggregations {
    pub languages: HashMap<String, usize>,
    pub file_types: HashMap<String, usize>,
    pub projects: HashMap<String, usize>,
    pub symbol_types: HashMap<String, usize>,
    pub complexity_distribution: Vec<ComplexityBucket>,
    pub temporal_distribution: Vec<TemporalBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityBucket {
    pub range: String,
    pub count: usize,
    pub avg_relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalBucket {
    pub period: String,
    pub count: usize,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub context: Option<String>,
    pub embedding_type: EmbeddingType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingType {
    Code,
    Documentation,
    Query,
    Symbol,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub dimension: usize,
    pub model_used: String,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityRequest {
    pub query_embedding: Vec<f32>,
    pub candidate_embeddings: Vec<Vec<f32>>,
    pub similarity_metric: SimilarityMetric,
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimilarityMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Manhattan,
    Jaccard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResponse {
    pub scores: Vec<f32>,
    pub ranked_indices: Vec<usize>,
    pub above_threshold: Vec<usize>,
}

// Query processing types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedQuery {
    pub original_query: String,
    pub normalized_query: String,
    pub keywords: Vec<String>,
    pub entities: Vec<Entity>,
    pub intent: QueryIntent,
    pub filters: Vec<QueryFilter>,
    pub boost_terms: Vec<BoostTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub text: String,
    pub entity_type: EntityType,
    pub confidence: f32,
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    FunctionName,
    ClassName,
    VariableName,
    FileName,
    Language,
    Framework,
    Concept,
    ErrorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryIntent {
    FindFunction,
    FindClass,
    FindUsage,
    FindDefinition,
    FindSimilar,
    FindExamples,
    FindDocumentation,
    FindBugs,
    FindPattern,
    ExploreCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
    pub boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Range,
    In,
    NotIn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoostTerm {
    pub term: String,
    pub boost_factor: f32,
    pub reason: String,
}

// Search analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalytics {
    pub query_id: Uuid,
    pub user_id: Uuid,
    pub query: String,
    pub results_count: usize,
    pub clicked_results: Vec<usize>,
    pub search_time_ms: u64,
    pub user_satisfaction: Option<f32>,
    pub refinements: Vec<QueryRefinement>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRefinement {
    pub original_query: String,
    pub refined_query: String,
    pub refinement_type: RefinementType,
    pub improvement_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefinementType {
    AddFilter,
    RemoveFilter,
    ExpandQuery,
    NarrowQuery,
    CorrectSpelling,
    ChangeSyntax,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            query_type: SearchQueryType::NaturalLanguage,
            workspace_paths: Vec::new(),
            file_filters: Vec::new(),
            language_filters: Vec::new(),
            max_results: Some(50),
            similarity_threshold: Some(0.7),
            include_context: true,
        }
    }
}

impl SearchResult {
    pub fn calculate_final_score(&self) -> f32 {
        let base_score = self.relevance_score;
        let type_boost = match self.match_type {
            MatchType::ExactMatch => 1.0,
            MatchType::SemanticMatch => 0.9,
            MatchType::PatternMatch => 0.8,
            MatchType::FuzzyMatch => 0.7,
            MatchType::ContextualMatch => 0.6,
        };
        
        let complexity_penalty = if let Some(symbol) = &self.symbol_info {
            1.0 - (symbol.complexity_score * 0.1).min(0.3)
        } else {
            1.0
        };
        
        base_score * type_boost * complexity_penalty
    }
}

impl CodeIndex {
    pub fn is_stale(&self, file_modified_time: DateTime<Utc>) -> bool {
        self.last_updated < file_modified_time
    }
    
    pub fn needs_reindexing(&self, current_hash: &str) -> bool {
        self.content_hash != current_hash
    }
}