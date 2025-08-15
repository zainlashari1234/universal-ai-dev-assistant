mod config;
mod providers;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use config::Config;
use providers::{
    router::ProviderRouter,
    traits::{AIProvider, AnalysisRequest, AnalysisType, CompletionRequest},
    ProviderHealth, ProviderMetrics,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
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
}

// API Request/Response types
#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
    providers: HashMap<String, ProviderHealth>,
    features: Vec<String>,
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

#[derive(Debug, Serialize, Deserialize)]
struct CompletionApiResponse {
    id: String,
    text: String,
    model: String,
    provider: String,
    usage: Option<UsageInfo>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UsageInfo {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    cost_usd: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisApiRequest {
    code: String,
    language: String,
    analysis_type: String, // "security", "performance", "quality", etc.
    context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CodeActionRequest {
    code: String,
    language: String,
    action: String, // "explain", "document", "test", "refactor", "translate"
    instructions: Option<String>,
    target_language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProvidersResponse {
    available_providers: Vec<String>,
    provider_metrics: HashMap<String, ProviderMetrics>,
    recommended_provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
    total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelInfo {
    name: String,
    provider: String,
    description: Option<String>,
    context_length: Option<u32>,
    cost_per_1k_tokens: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    provider: Option<String>,
    model: Option<String>,
    limit: Option<usize>,
}

// API Handlers
async fn health_handler(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let mut provider_health = HashMap::new();
    
    // Check health of all providers
    for provider_name in ["openrouter", "openai", "anthropic", "groq", "ollama"] {
        // This is a simplified health check - in reality we'd check each provider
        provider_health.insert(
            provider_name.to_string(),
            ProviderHealth {
                provider_type: match provider_name {
                    "openrouter" => providers::ProviderType::OpenRouter,
                    "openai" => providers::ProviderType::OpenAI,
                    "anthropic" => providers::ProviderType::Anthropic,
                    "groq" => providers::ProviderType::Groq,
                    "ollama" => providers::ProviderType::Ollama,
                    _ => providers::ProviderType::Ollama,
                },
                is_available: true,
                response_time_ms: Some(100),
                error_message: None,
                models_available: vec!["gpt-4o".to_string(), "claude-3-5-sonnet".to_string()],
            },
        );
    }

    let features = vec![
        "Multi-provider AI support".to_string(),
        "Code completion".to_string(),
        "Code analysis".to_string(),
        "Documentation generation".to_string(),
        "Test generation".to_string(),
        "Code explanation".to_string(),
        "Code refactoring".to_string(),
        "Language translation".to_string(),
        "Real-time streaming".to_string(),
        "Cost optimization".to_string(),
        "Provider failover".to_string(),
    ];

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: "6.2.0".to_string(),
        providers: provider_health,
        features,
    }))
}

async fn complete_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CompletionApiRequest>,
) -> Result<Json<CompletionApiResponse>, StatusCode> {
    info!("Code completion request: {:?}", req);

    let completion_request = CompletionRequest::new(req.prompt)
        .with_model(req.model.unwrap_or_else(|| "gpt-4o-mini".to_string()))
        .with_temperature(req.temperature.unwrap_or(0.7))
        .with_max_tokens(req.max_tokens.unwrap_or(1000))
        .with_language(req.language.unwrap_or_else(|| "unknown".to_string()))
        .with_streaming(req.stream.unwrap_or(false));

    let completion_request = if let Some(system_prompt) = req.system_prompt {
        completion_request.with_system_prompt(system_prompt)
    } else {
        completion_request
    };

    match state.provider_router.complete(completion_request).await {
        Ok(response) => {
            let api_response = CompletionApiResponse {
                id: response.id,
                text: response.choices.first()
                    .map(|c| c.text.clone())
                    .unwrap_or_else(|| "No completion generated".to_string()),
                model: response.model,
                provider: response.provider,
                usage: response.usage.map(|u| UsageInfo {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                    cost_usd: u.cost_usd,
                }),
                metadata: response.metadata,
            };
            Ok(Json(api_response))
        }
        Err(e) => {
            tracing::error!("Completion failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn analyze_handler(
    State(state): State<AppState>,
    Json(req): Json<AnalysisApiRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Code analysis request: {:?}", req);

    let analysis_type = match req.analysis_type.as_str() {
        "security" => AnalysisType::Security,
        "performance" => AnalysisType::Performance,
        "quality" => AnalysisType::Quality,
        "bugs" => AnalysisType::Bugs,
        "suggestions" => AnalysisType::Suggestions,
        "documentation" => AnalysisType::Documentation,
        "testing" => AnalysisType::Testing,
        "refactoring" => AnalysisType::Refactoring,
        _ => AnalysisType::Quality,
    };

    let analysis_request = AnalysisRequest {
        code: req.code,
        language: req.language,
        analysis_type,
        context: req.context,
    };

    match state.provider_router.analyze_code(analysis_request).await {
        Ok(response) => Ok(Json(serde_json::to_value(response).unwrap())),
        Err(e) => {
            tracing::error!("Analysis failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn code_action_handler(
    State(state): State<AppState>,
    Json(req): Json<CodeActionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Code action request: {:?}", req);

    let result = match req.action.as_str() {
        "explain" => state.provider_router.explain_code(&req.code, &req.language).await,
        "document" => state.provider_router.generate_documentation(&req.code, &req.language).await,
        "test" => state.provider_router.generate_tests(&req.code, &req.language).await,
        "refactor" => {
            let instructions = req.instructions.unwrap_or_else(|| "Improve code quality and readability".to_string());
            state.provider_router.refactor_code(&req.code, &req.language, &instructions).await
        }
        "translate" => {
            let target_language = req.target_language.unwrap_or_else(|| "python".to_string());
            state.provider_router.translate_code(&req.code, &req.language, &target_language).await
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match result {
        Ok(response) => Ok(Json(serde_json::json!({
            "action": req.action,
            "result": response,
            "success": true
        }))),
        Err(e) => {
            tracing::error!("Code action failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn providers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<ProvidersResponse>, StatusCode> {
    let available_providers = state.provider_router.get_available_providers().await;
    let provider_metrics = state.provider_router.get_metrics().await;
    
    // Convert internal metrics to API metrics
    let api_metrics: HashMap<String, ProviderMetrics> = provider_metrics
        .into_iter()
        .map(|(k, v)| (k, ProviderMetrics {
            provider_type: providers::ProviderType::OpenRouter, // Placeholder
            total_requests: v.total_requests,
            successful_requests: v.successful_requests,
            failed_requests: v.failed_requests,
            average_response_time_ms: v.average_response_time_ms(),
            tokens_processed: v.total_tokens,
            cost_usd: v.total_cost_usd,
        }))
        .collect();

    let recommended_provider = if !available_providers.is_empty() {
        Some(available_providers[0].clone())
    } else {
        None
    };

    Ok(Json(ProvidersResponse {
        available_providers,
        provider_metrics: api_metrics,
        recommended_provider,
    }))
}

async fn models_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<ModelsResponse>, StatusCode> {
    match state.provider_router.list_models().await {
        Ok(models) => {
            let model_infos: Vec<ModelInfo> = models
                .into_iter()
                .map(|model| ModelInfo {
                    name: model.clone(),
                    provider: "multiple".to_string(), // TODO: Map models to providers
                    description: Some(format!("AI model: {}", model)),
                    context_length: Some(4096), // Default context length
                    cost_per_1k_tokens: Some(0.002), // Default cost
                })
                .collect();

            let total_count = model_infos.len();

            Ok(Json(ModelsResponse {
                models: model_infos,
                total_count,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list models: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn metrics_handler(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let metrics = state.provider_router.get_metrics().await;
    
    let response = serde_json::json!({
        "provider_metrics": metrics,
        "system_info": {
            "version": "6.2.0",
            "uptime_seconds": 0, // TODO: Track actual uptime
            "total_requests": metrics.values().map(|m| m.total_requests).sum::<u64>(),
            "total_cost_usd": metrics.values().map(|m| m.total_cost_usd).sum::<f64>(),
        }
    });

    Ok(Json(response))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "universal_ai_dev_assistant=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting Universal AI Development Assistant v6.2.0");

    // Load configuration
    let config = Arc::new(Config::from_env()?);
    info!("📋 Configuration loaded successfully");

    // Initialize provider router
    let provider_router = Arc::new(ProviderRouter::new(config.clone()).await?);
    info!("🤖 Provider router initialized with multiple AI providers");

    // Create application state
    let app_state = AppState {
        config: config.clone(),
        provider_router,
    };

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(
            config.server.cors_origins
                .iter()
                .map(|origin| origin.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]);

    // Build our application with routes
    let app = Router::new()
        // Health and system endpoints
        .route("/health", get(health_handler))
        .route("/api/v1/providers", get(providers_handler))
        .route("/api/v1/models", get(models_handler))
        .route("/api/v1/metrics", get(metrics_handler))
        
        // Core AI endpoints
        .route("/api/v1/complete", post(complete_handler))
        .route("/api/v1/analyze", post(analyze_handler))
        .route("/api/v1/code/action", post(code_action_handler))
        
        // Legacy compatibility endpoints
        .route("/api/v1/completion", post(complete_handler))
        .route("/api/v1/analysis", post(analyze_handler))
        
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        .with_state(app_state);

    // Start the server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("🌐 Server running on http://{}", addr);
    info!("📚 API Documentation:");
    info!("  - Health: GET /health");
    info!("  - Providers: GET /api/v1/providers");
    info!("  - Models: GET /api/v1/models");
    info!("  - Complete: POST /api/v1/complete");
    info!("  - Analyze: POST /api/v1/analyze");
    info!("  - Code Actions: POST /api/v1/code/action");
    info!("  - Metrics: GET /api/v1/metrics");

    axum::serve(listener, app).await?;

    Ok(())
}