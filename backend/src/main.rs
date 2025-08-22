mod config;
mod providers;
mod database;
mod auth;
mod terminal;
mod conversation;
mod search;
mod streaming;
mod api;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post, put, delete},
    Router, middleware,
};
use config::Config;
use providers::{
    router::ProviderRouter,
    traits::{AIProvider, AnalysisRequest, AnalysisType, CompletionRequest},
    ProviderHealth, ProviderMetrics,
};
use database::DatabaseManager;
use terminal::ai_terminal::AITerminalService;
use terminal::history_manager::HistoryManager;
use conversation::conversation_service::ConversationService;
use conversation::session_manager::SessionManager;
use search::search_service::SearchService;
use auth::{JwtManager, UserService, ApiKeyManager, AuthContext, preferences::PreferencesService};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, env};
use uuid::Uuid;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Application state
#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    provider_router: Arc<ProviderRouter>,
    database: Arc<DatabaseManager>,
    jwt_manager: Arc<JwtManager>,
    user_service: Arc<UserService>,
    api_key_manager: Arc<ApiKeyManager>,
    preferences_service: Arc<PreferencesService>,
    terminal_service: Arc<AITerminalService>,
    conversation_service: Arc<ConversationService>,
    search_service: Arc<SearchService>,
}

// API Request/Response types
#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
    providers: HashMap<String, ProviderHealth>,
    features: Vec<String>,
    database: database::DatabaseHealth,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompletionApiRequest {
    prompt: String,
    model: Option<String>,
    provider: Option<String>,
    language: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    system_prompt: Option<String>,
    stream: Option<bool>,
}

// Terminal API types
#[derive(Debug, Serialize, Deserialize)]
struct TerminalSuggestRequest {
    query: String,
    query_type: String,
    session_id: Option<String>,
    workspace_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TerminalSearchRequest {
    query: String,
    limit: Option<i64>,
}

// Conversation API types
#[derive(Debug, Serialize, Deserialize)]
struct ConversationCreateSessionRequest {
    workspace_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConversationMessageRequest {
    session_id: String,
    message: String,
    current_file: Option<String>,
    selected_text: Option<ConversationTextSelection>,
    context_files: Vec<String>,
    intent_hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConversationTextSelection {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConversationSearchRequest {
    query: String,
    limit: Option<i64>,
}

// Search API types
#[derive(Debug, Serialize, Deserialize)]
struct SearchApiRequest {
    query: String,
    query_type: Option<String>,
    workspace_paths: Vec<String>,
    file_filters: Vec<SearchFileFilter>,
    language_filters: Vec<String>,
    max_results: Option<usize>,
    similarity_threshold: Option<f32>,
    include_context: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchFileFilter {
    pattern: String,
    include: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchSimilarRequest {
    code_snippet: String,
    workspace_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchSymbolRequest {
    symbol_name: String,
    symbol_type: Option<String>,
    workspace_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchDocumentationRequest {
    query: String,
    workspace_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchErrorsRequest {
    error_message: String,
    workspace_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchIndexRequest {
    workspace_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchFeedbackRequest {
    search_id: String,
    feedback_type: String,
    satisfaction_score: f32,
    comments: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "universal_ai_dev_assistant=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting Universal AI Development Assistant Backend v6.2.0");

    // Load configuration
    let config = Arc::new(Config::load()?);
    info!("✅ Configuration loaded");

    // Initialize database
    let database = Arc::new(DatabaseManager::new().await?);
    info!("✅ Database connected and migrations applied");

    // Initialize JWT manager
    let jwt_secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-super-secret-jwt-key-change-in-production".to_string());
    let jwt_manager = Arc::new(JwtManager::new(&jwt_secret, "uaida-backend".to_string()));
    info!("✅ JWT manager initialized");

    // Initialize API key manager
    let encryption_key = env::var("ENCRYPTION_KEY")
        .unwrap_or_else(|_| "your-32-byte-encryption-key-change-this".to_string())
        .as_bytes()[..32]
        .try_into()
        .expect("Encryption key must be 32 bytes");
    let api_key_manager = Arc::new(ApiKeyManager::new(database.pool.clone(), encryption_key));
    info!("✅ API key manager initialized");

    // Initialize user service
    let user_service = Arc::new(UserService::new(database.pool.clone(), (*jwt_manager).clone()));
    info!("✅ User service initialized");

    // Initialize preferences service
    let preferences_service = Arc::new(PreferencesService::new(database.pool.clone()));
    info!("✅ Preferences service initialized");

    // Initialize terminal service
    let history_manager = HistoryManager::new(database.pool.clone());
    let terminal_service = Arc::new(AITerminalService::new(provider_router.clone(), history_manager));
    info!("✅ Terminal service initialized");

    // Initialize conversation service
    let conversation_session_manager = SessionManager::new(database.pool.clone());
    let conversation_service = Arc::new(ConversationService::new(provider_router.clone(), conversation_session_manager));
    info!("✅ Conversation service initialized");

    // Initialize search service
    let search_service = Arc::new(SearchService::new(provider_router.clone(), database.pool.clone()));
    info!("✅ Search service initialized");

    // Initialize provider router
    let provider_router = Arc::new(ProviderRouter::new(&config)?);
    info!("✅ Provider router initialized with {} providers", provider_router.get_available_providers().len());

    // Create application state
    let app_state = AppState {
        config: config.clone(),
        provider_router,
        database,
        jwt_manager,
        user_service,
        api_key_manager,
        preferences_service,
        terminal_service,
        conversation_service,
        search_service,
    };

    // Build router
    let app = Router::new()
        // Public routes (no authentication required)
        .route("/health", get(health_handler))
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/login", post(auth::login_handler))
        .route("/auth/refresh", post(auth::refresh_token_handler))
        
        // Protected routes (authentication required)
        .route("/auth/logout", post(auth::logout_handler))
        .route("/auth/profile", get(auth::get_profile_handler))
        .route("/auth/profile", put(auth::update_profile_handler))
        .route("/auth/change-password", post(auth::change_password_handler))
        
        // API key management
        .route("/api-keys", get(auth::get_api_keys_handler))
        .route("/api-keys", post(auth::create_api_key_handler))
        .route("/api-keys/:key_id", delete(auth::delete_api_key_handler))
        .route("/api-keys/usage", get(auth::get_api_key_usage_handler))
        
        // User preferences
        .route("/preferences", get(auth::preferences_handlers::get_preferences_handler))
        .route("/preferences", put(auth::preferences_handlers::update_preferences_handler))
        .route("/preferences/reset", post(auth::preferences_handlers::reset_preferences_handler))
        .route("/preferences/export", get(auth::preferences_handlers::export_preferences_handler))
        
        // Terminal endpoints
        .route("/terminal/suggest", post(terminal_suggest_handler))
        .route("/terminal/execute", post(terminal_execute_handler))
        .route("/terminal/sessions", get(terminal_sessions_handler))
        .route("/terminal/sessions/:session_id", delete(terminal_delete_session_handler))
        .route("/terminal/history/search", post(terminal_search_history_handler))
        .route("/terminal/stats", get(terminal_stats_handler))
        
        // Conversation endpoints
        .route("/conversation/sessions", post(conversation_create_session_handler))
        .route("/conversation/sessions", get(conversation_get_sessions_handler))
        .route("/conversation/sessions/:session_id", get(conversation_get_session_handler))
        .route("/conversation/sessions/:session_id", delete(conversation_delete_session_handler))
        .route("/conversation/message", post(conversation_process_message_handler))
        .route("/conversation/search", post(conversation_search_handler))
        .route("/conversation/stats", get(conversation_stats_handler))
        
        // Search endpoints
        .route("/search", post(search_handler))
        .route("/search/similar", post(search_similar_handler))
        .route("/search/symbol", post(search_symbol_handler))
        .route("/search/documentation", post(search_documentation_handler))
        .route("/search/errors", post(search_errors_handler))
        .route("/search/suggestions", get(search_suggestions_handler))
        .route("/search/index", post(search_index_workspace_handler))
        .route("/search/stats/:workspace_path", get(search_workspace_stats_handler))
        .route("/search/analytics", get(search_user_analytics_handler))
        .route("/search/feedback", post(search_feedback_handler))
        .route("/preferences/import", post(auth::preferences_handlers::import_preferences_handler))
        
        // AI completion endpoints
        .route("/completion", post(completion_handler))
        .route("/completion/stream", post(streaming::streaming_completion_handler))
        .route("/analysis", post(analysis_handler))
        
        // Provider management
        .route("/providers", get(providers_handler))
        .route("/providers/:provider/health", get(provider_health_handler))
        // Code completion and analysis endpoints
        .route("/api/v1/complete", post(api::completion::complete_code))
        .route("/api/v1/analyze", post(api::completion::analyze_code))
        .route("/providers/:provider/models", get(provider_models_handler))
        
        // System endpoints
        .route("/metrics", get(metrics_handler))
        .route("/database/stats", get(database_stats_handler))
        
        // Add authentication middleware to protected routes
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware_wrapper
        ))
        
        // Add CORS and tracing
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        )
        .with_state(app_state);

    // Start server
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🌐 Server listening on http://0.0.0.0:{}", port);
    info!("📚 API Documentation available at http://0.0.0.0:{}/health", port);

    axum::serve(listener, app).await?;

    Ok(())
}

// Authentication middleware wrapper
async fn auth_middleware_wrapper(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // Skip authentication for public routes
    let path = request.uri().path();
    if path.starts_with("/health") || 
       path.starts_with("/auth/register") || 
       path.starts_with("/auth/login") || 
       path.starts_with("/auth/refresh") {
        return Ok(next.run(request).await);
    }

    // Extract authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract bearer token
    let token = JwtManager::extract_bearer_token(auth_header)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate JWT token
    let claims = state
        .jwt_manager
        .validate_token(token, auth::TokenType::Access)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Create auth context
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let auth_context = claims
        .to_auth_context(ip_address, user_agent)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add auth context to request extensions
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

// API Handlers
async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let providers = state.provider_router.get_provider_health().await;
    let database_health = state.database.health_check().await.unwrap_or_else(|_| {
        database::DatabaseHealth {
            connected: false,
            latency_ms: None,
            pool_size: 0,
            active_connections: 0,
            error: Some("Health check failed".to_string()),
        }
    });

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        providers,
        features: vec![
            "ai-completion".to_string(),
            "code-analysis".to_string(),
            "multi-provider".to_string(),
            "authentication".to_string(),
            "api-key-management".to_string(),
            "user-management".to_string(),
        ],
        database: database_health,
    })
}

async fn completion_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<CompletionApiRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get user's API key for the provider
    let provider = request.provider.as_deref().unwrap_or("openrouter");
    let api_key = state
        .api_key_manager
        .get_api_key(auth_context.user.id, provider)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if api_key.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create completion request
    let completion_request = CompletionRequest {
        prompt: request.prompt,
        model: request.model,
        provider: request.provider,
        language: request.language,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        system_prompt: request.system_prompt,
        stream: Some(request.stream.unwrap_or(false)),
    };

    // Process completion
    match state.provider_router.complete(completion_request).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": response
        }))),
        Err(e) => {
            tracing::error!("Completion failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn completion_stream_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<CompletionApiRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get user's API key for the provider
    let provider = request.provider.as_deref().unwrap_or("openrouter");
    let api_key = state
        .api_key_manager
        .get_api_key(auth_context.user.id, provider)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if api_key.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create completion request
    let completion_request = CompletionRequest {
        prompt: request.prompt,
        model: request.model,
        provider: request.provider,
        language: request.language,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        system_prompt: request.system_prompt,
        stream: Some(true),
    };

    // Process streaming completion
    match state.provider_router.complete_stream(completion_request).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "stream_id": response.stream_id,
            "estimated_tokens": response.estimated_tokens
        }))),
        Err(e) => {
            tracing::error!("Streaming completion failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn analysis_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<AnalysisRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.provider_router.analyze(request).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "analysis": response
        }))),
        Err(e) => {
            tracing::error!("Analysis failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn providers_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let providers = state.provider_router.get_available_providers();
    Json(serde_json::json!({
        "success": true,
        "providers": providers
    }))
}

async fn provider_health_handler(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> Result<Json<ProviderHealth>, StatusCode> {
    match state.provider_router.get_provider_health_by_name(&provider).await {
        Some(health) => Ok(Json(health)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn provider_models_handler(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.provider_router.get_models(&provider).await {
        Ok(models) => Ok(Json(serde_json::json!({
            "success": true,
            "models": models
        }))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn metrics_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let metrics = state.provider_router.get_metrics().await;
    Json(serde_json::json!({
        "success": true,
        "metrics": metrics
    }))
}

async fn database_stats_handler(
    State(state): State<AppState>,
    _auth_context: AuthContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.database.get_stats().await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "success": true,
            "stats": stats
        }))),
        Err(e) => {
            tracing::error!("Failed to get database stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Extract auth context from request extensions
impl axum::extract::FromRequestParts<AppState> for AuthContext {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

// Terminal API Handlers
async fn terminal_suggest_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<TerminalSuggestRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use terminal::{TerminalRequest, QueryType};
    
    let query_type = match request.query_type.as_str() {
        "natural_language" => QueryType::NaturalLanguage,
        "command_explanation" => QueryType::CommandExplanation,
        "history_search" => QueryType::HistorySearch,
        _ => QueryType::NaturalLanguage,
    };

    let session_id = request.session_id
        .and_then(|s| Uuid::parse_str(&s).ok());

    // Session yoksa yeni oluştur
    let session = if let Some(sid) = session_id {
        state.terminal_service.get_session(sid).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .unwrap_or_else(|| {
                // Session bulunamazsa yeni oluştur
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        state.terminal_service.create_session(
                            auth_context.user.id,
                            request.workspace_path.clone()
                        ).await.unwrap_or_else(|_| {
                            terminal::TerminalSession::new(auth_context.user.id, request.workspace_path.clone())
                        })
                    })
                })
            })
    } else {
        state.terminal_service.create_session(
            auth_context.user.id,
            request.workspace_path.clone()
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let terminal_request = TerminalRequest {
        session_id: Some(session.id),
        query: request.query,
        query_type,
        context: Some(session.context.clone()),
    };

    match state.terminal_service.process_request(terminal_request).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "session_id": response.session_id.to_string(),
            "suggestions": response.suggestions,
            "explanation": response.explanation,
            "warnings": response.warnings
        }))),
        Err(e) => {
            tracing::error!("Terminal suggest failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn terminal_execute_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<TerminalSuggestRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use terminal::{TerminalRequest, QueryType};
    
    let session_id = request.session_id
        .and_then(|s| Uuid::parse_str(&s).ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let terminal_request = TerminalRequest {
        session_id: Some(session_id),
        query: request.query,
        query_type: QueryType::CommandExecution,
        context: None,
    };

    match state.terminal_service.process_request(terminal_request).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "session_id": response.session_id.to_string(),
            "execution_result": response.execution_result,
            "warnings": response.warnings
        }))),
        Err(e) => {
            tracing::error!("Terminal execute failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn terminal_sessions_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(10);

    match state.terminal_service.get_user_sessions(auth_context.user.id, limit).await {
        Ok(sessions) => Ok(Json(serde_json::json!({
            "success": true,
            "sessions": sessions
        }))),
        Err(e) => {
            tracing::error!("Failed to get terminal sessions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn terminal_delete_session_handler(
    State(state): State<AppState>,
    _auth_context: AuthContext,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match state.terminal_service.delete_session(session_uuid).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Session deleted successfully"
        }))),
        Err(e) => {
            tracing::error!("Failed to delete terminal session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn terminal_search_history_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<TerminalSearchRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = request.limit.unwrap_or(20);

    match state.terminal_service.search_user_commands(
        auth_context.user.id,
        &request.query,
        limit
    ).await {
        Ok(commands) => Ok(Json(serde_json::json!({
            "success": true,
            "commands": commands
        }))),
        Err(e) => {
            tracing::error!("Failed to search command history: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn terminal_stats_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.terminal_service.get_command_statistics(auth_context.user.id).await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "success": true,
            "stats": {
                "total_commands": stats.total_commands,
                "ai_suggested_count": stats.ai_suggested_count,
                "successful_commands": stats.successful_commands,
                "total_sessions": stats.total_sessions,
                "success_rate": stats.success_rate(),
                "ai_usage_rate": stats.ai_usage_rate(),
                "most_used_commands": stats.most_used_commands
            }
        }))),
        Err(e) => {
            tracing::error!("Failed to get terminal statistics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Conversation API Handlers
async fn conversation_create_session_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<ConversationCreateSessionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.conversation_service.create_session(
        auth_context.user.id,
        request.workspace_path,
    ).await {
        Ok(session) => Ok(Json(serde_json::json!({
            "success": true,
            "session": {
                "id": session.id.to_string(),
                "workspace_context": session.workspace_context,
                "created_at": session.created_at
            }
        }))),
        Err(e) => {
            tracing::error!("Failed to create conversation session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn conversation_get_sessions_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(10);

    match state.conversation_service.get_user_sessions(auth_context.user.id, limit).await {
        Ok(sessions) => Ok(Json(serde_json::json!({
            "success": true,
            "sessions": sessions
        }))),
        Err(e) => {
            tracing::error!("Failed to get conversation sessions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn conversation_get_session_handler(
    State(state): State<AppState>,
    _auth_context: AuthContext,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match state.conversation_service.get_session(session_uuid).await {
        Ok(Some(session)) => Ok(Json(serde_json::json!({
            "success": true,
            "session": session
        }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get conversation session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn conversation_delete_session_handler(
    State(state): State<AppState>,
    _auth_context: AuthContext,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match state.conversation_service.delete_session(session_uuid).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Session deleted successfully"
        }))),
        Err(e) => {
            tracing::error!("Failed to delete conversation session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn conversation_process_message_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<ConversationMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use conversation::{ConversationRequest, MessageIntent, TextSelection, Position};
    
    let session_id = Uuid::parse_str(&request.session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Convert text selection
    let selected_text = request.selected_text.map(|sel| TextSelection {
        start: Position {
            line: sel.start_line,
            column: sel.start_column,
        },
        end: Position {
            line: sel.end_line,
            column: sel.end_column,
        },
        text: sel.text,
    });

    // Convert intent hint
    let intent_hint = request.intent_hint.and_then(|hint| {
        match hint.as_str() {
            "CodeGeneration" => Some(MessageIntent::CodeGeneration),
            "CodeExplanation" => Some(MessageIntent::CodeExplanation),
            "CodeReview" => Some(MessageIntent::CodeReview),
            "Debugging" => Some(MessageIntent::Debugging),
            "Refactoring" => Some(MessageIntent::Refactoring),
            "Testing" => Some(MessageIntent::Testing),
            "Documentation" => Some(MessageIntent::Documentation),
            "FileOperation" => Some(MessageIntent::FileOperation),
            "ProjectSetup" => Some(MessageIntent::ProjectSetup),
            "TerminalCommand" => Some(MessageIntent::TerminalCommand),
            "WorkspaceNavigation" => Some(MessageIntent::WorkspaceNavigation),
            _ => None,
        }
    });

    let conversation_request = ConversationRequest {
        session_id: Some(session_id),
        message: request.message,
        workspace_path: None, // Session'dan alınacak
        current_file: request.current_file,
        selected_text,
        context_files: request.context_files,
        intent_hint,
    };

    match state.conversation_service.process_message(conversation_request).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": {
                "session_id": response.session_id.to_string(),
                "ai_response": response.ai_response,
                "intent": format!("{:?}", response.intent),
                "confidence_score": response.confidence_score,
                "code_changes": response.code_changes,
                "suggested_actions": response.suggested_actions,
                "file_references": response.file_references,
                "follow_up_questions": response.follow_up_questions,
                "execution_time_ms": response.execution_time_ms
            }
        }))),
        Err(e) => {
            tracing::error!("Failed to process conversation message: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn conversation_search_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<ConversationSearchRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = request.limit.unwrap_or(20);

    match state.conversation_service.search_conversations(
        auth_context.user.id,
        &request.query,
        limit
    ).await {
        Ok(conversations) => Ok(Json(serde_json::json!({
            "success": true,
            "conversations": conversations
        }))),
        Err(e) => {
            tracing::error!("Failed to search conversations: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn conversation_stats_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.conversation_service.get_conversation_statistics(auth_context.user.id).await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "success": true,
            "stats": {
                "total_sessions": stats.total_sessions,
                "total_turns": stats.total_turns,
                "average_confidence": stats.average_confidence,
                "turns_with_code_changes": stats.turns_with_code_changes,
                "intent_distribution": stats.intent_distribution,
                "most_common_intent": stats.most_common_intent(),
                "code_generation_rate": stats.code_generation_rate(),
                "average_turns_per_session": stats.average_turns_per_session()
            }
        }))),
        Err(e) => {
            tracing::error!("Failed to get conversation statistics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Search API Handlers
async fn search_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchApiRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use search::{SearchRequest, SearchQueryType, FileFilter};
    
    let query_type = match request.query_type.as_deref() {
        Some("natural_language") => SearchQueryType::NaturalLanguage,
        Some("code_pattern") => SearchQueryType::CodePattern,
        Some("function_signature") => SearchQueryType::FunctionSignature,
        Some("symbol_name") => SearchQueryType::SymbolName,
        Some("documentation") => SearchQueryType::Documentation,
        Some("error_message") => SearchQueryType::ErrorMessage,
        Some("semantic") => SearchQueryType::Semantic,
        _ => SearchQueryType::NaturalLanguage,
    };

    let file_filters: Vec<FileFilter> = request.file_filters.into_iter()
        .map(|f| FileFilter {
            pattern: f.pattern,
            include: f.include,
        })
        .collect();

    let search_request = SearchRequest {
        query: request.query,
        query_type,
        workspace_paths: request.workspace_paths,
        file_filters,
        language_filters: request.language_filters,
        max_results: request.max_results,
        similarity_threshold: request.similarity_threshold,
        include_context: request.include_context.unwrap_or(true),
    };

    match state.search_service.search(search_request, auth_context.user.id).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": response
        }))),
        Err(e) => {
            tracing::error!("Search failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_similar_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchSimilarRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.search_service.search_similar_code(
        &request.code_snippet,
        request.workspace_paths,
        auth_context.user.id
    ).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": response
        }))),
        Err(e) => {
            tracing::error!("Similar code search failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_symbol_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchSymbolRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use search::SymbolType;
    
    let symbol_type = request.symbol_type.and_then(|s| match s.as_str() {
        "Function" => Some(SymbolType::Function),
        "Method" => Some(SymbolType::Method),
        "Class" => Some(SymbolType::Class),
        "Struct" => Some(SymbolType::Struct),
        "Enum" => Some(SymbolType::Enum),
        "Interface" => Some(SymbolType::Interface),
        "Variable" => Some(SymbolType::Variable),
        "Constant" => Some(SymbolType::Constant),
        "Module" => Some(SymbolType::Module),
        "Namespace" => Some(SymbolType::Namespace),
        "Trait" => Some(SymbolType::Trait),
        "Type" => Some(SymbolType::Type),
        _ => None,
    });

    match state.search_service.search_by_symbol(
        &request.symbol_name,
        symbol_type,
        request.workspace_paths,
        auth_context.user.id
    ).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": response
        }))),
        Err(e) => {
            tracing::error!("Symbol search failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_documentation_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchDocumentationRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.search_service.search_documentation(
        &request.query,
        request.workspace_paths,
        auth_context.user.id
    ).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": response
        }))),
        Err(e) => {
            tracing::error!("Documentation search failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_errors_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchErrorsRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.search_service.search_errors(
        &request.error_message,
        request.workspace_paths,
        auth_context.user.id
    ).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "response": response
        }))),
        Err(e) => {
            tracing::error!("Error search failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_suggestions_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let partial_query = params.get("q").cloned().unwrap_or_default();
    
    match state.search_service.get_search_suggestions(&partial_query, auth_context.user.id).await {
        Ok(suggestions) => Ok(Json(serde_json::json!({
            "success": true,
            "suggestions": suggestions
        }))),
        Err(e) => {
            tracing::error!("Failed to get search suggestions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_index_workspace_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchIndexRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.search_service.index_workspace(&request.workspace_path, auth_context.user.id).await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "success": true,
            "stats": stats
        }))),
        Err(e) => {
            tracing::error!("Failed to index workspace: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_workspace_stats_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Path(workspace_path): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.search_service.get_workspace_stats(&workspace_path, auth_context.user.id).await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "success": true,
            "stats": stats
        }))),
        Err(e) => {
            tracing::error!("Failed to get workspace stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_user_analytics_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let days = params.get("days")
        .and_then(|d| d.parse::<i32>().ok())
        .unwrap_or(30);

    match state.search_service.get_user_search_analytics(auth_context.user.id, days).await {
        Ok(analytics) => Ok(Json(serde_json::json!({
            "success": true,
            "analytics": analytics
        }))),
        Err(e) => {
            tracing::error!("Failed to get user analytics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn search_feedback_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<SearchFeedbackRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use search::search_service::{SearchFeedback, SearchFeedbackType};
    
    let search_id = Uuid::parse_str(&request.search_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let feedback_type = match request.feedback_type.as_str() {
        "helpful" => SearchFeedbackType::Helpful,
        "not_helpful" => SearchFeedbackType::NotHelpful,
        "irrelevant" => SearchFeedbackType::Irrelevant,
        "perfect" => SearchFeedbackType::Perfect,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let feedback = SearchFeedback {
        feedback_type,
        satisfaction_score: request.satisfaction_score,
        comments: request.comments,
    };

    match state.search_service.provide_search_feedback(search_id, feedback, auth_context.user.id).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Feedback recorded successfully"
        }))),
        Err(e) => {
            tracing::error!("Failed to record search feedback: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}