use anyhow::Result;
use crate::observability::tracing::{init_tracing, shutdown_tracing};
use crate::security::{security_headers_middleware, security_audit_middleware, create_rate_limit_layer, create_cors_layer};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

mod ai_engine;
mod api;
mod collaboration;
mod config;
mod language_server;
mod models;
mod services;
mod context;
mod sandbox;
mod agents;
mod observability;
mod auth;
mod security;
mod database;
mod risk;
mod evals;

use ai_engine::AIEngine;
use ai_engine::providers::{ProviderRouter, OllamaProvider, HeuristicProvider};
use config::Config;
use context::ContextManager;
use sandbox::SandboxConfig;
use agents::{AgentOrchestrator, PlannerAgent, AgentConstraints};
use observability::{init_metrics, get_metrics, observability_routes, init_tracing};
use database::DatabaseManager;

#[derive(Clone)]
pub struct AppState {
    ai_engine: Arc<AIEngine>,
    provider_router: Arc<ProviderRouter>,
    context_manager: Arc<tokio::sync::RwLock<ContextManager>>,
    agent_orchestrator: Arc<AgentOrchestrator>,
    config: Arc<Config>,
    database: Arc<DatabaseManager>, // P0 Day-3: Database integration
}

#[derive(Serialize, Deserialize)]
pub struct CompletionRequest {
    pub code: String,
    pub language: String,
    pub cursor_position: usize,
    pub context: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CompletionResponse {
    pub suggestions: Vec<String>,
    pub confidence: f32,
    pub processing_time_ms: u64,
}

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub ai_model_loaded: bool,
    pub supported_languages: Vec<String>,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let provider_health = state.provider_router.health().await.unwrap_or_else(|_| {
        crate::ai_engine::providers::ProviderHealth {
            available: false,
            latency_ms: None,
            model_loaded: false,
            error: Some("Provider router unavailable".to_string()),
            capabilities: vec![],
        }
    });

    Json(HealthResponse {
        status: if provider_health.available { "healthy".to_string() } else { "degraded".to_string() },
        version: env!("CARGO_PKG_VERSION").to_string(),
        ai_model_loaded: provider_health.model_loaded,
        supported_languages: vec![
            "python".to_string(),
            "javascript".to_string(),
            "typescript".to_string(),
            "rust".to_string(),
            "go".to_string(),
            "java".to_string(),
            "cpp".to_string(),
            "c".to_string(),
        ],
    })
}

async fn complete_code(
    State(state): State<AppState>,
    Json(request): Json<CompletionRequest>,
) -> Result<Json<CompletionResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    match state.ai_engine.complete_code(&request).await {
        Ok(suggestions) => {
            let processing_time = start_time.elapsed().as_millis() as u64;
            
            let confidence = calculate_confidence(&suggestions, &request);
            
            Ok(Json(CompletionResponse {
                suggestions,
                confidence,
                processing_time_ms: processing_time,
            }))
        }
        Err(e) => {
            warn!("Code completion failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn analyze_code(
    State(state): State<AppState>,
    Json(request): Json<CompletionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.ai_engine.analyze_code(&request).await {
        Ok(analysis) => Ok(Json(analysis)),
        Err(e) => {
            warn!("Code analysis failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn calculate_confidence(suggestions: &[String], request: &CompletionRequest) -> f32 {
    if suggestions.is_empty() {
        return 0.0;
    }
    
    let mut confidence = 0.5; // Base confidence
    
    // Increase confidence based on context quality
    if !request.context.as_ref().unwrap_or(&String::new()).is_empty() {
        confidence += 0.2;
    }
    
    // Increase confidence based on suggestion quality
    let avg_length = suggestions.iter().map(|s| s.len()).sum::<usize>() as f32 / suggestions.len() as f32;
    if avg_length > 10.0 && avg_length < 100.0 {
        confidence += 0.2;
    }
    
    // Increase confidence if suggestions are diverse
    let unique_suggestions = suggestions.iter().collect::<std::collections::HashSet<_>>().len();
    if unique_suggestions == suggestions.len() {
        confidence += 0.1;
    }
    
    // Cap confidence at 0.95
    confidence.min(0.95)
}

#[tokio::main]
async fn main() -> Result<()> {
    // P0 Task #2: Initialize OpenTelemetry tracing
    init_tracing()?;
    info!("OpenTelemetry tracing initialized");

    info!("Starting Universal AI Development Assistant v{}", env!("CARGO_PKG_VERSION"));
    
    // P0 Day-3: Initialize database connection and migrations
    let database = Arc::new(DatabaseManager::new().await?);
    info!("Database initialized and migrations completed");

    // Initialize metrics
    init_metrics();
    info!("Metrics initialized");

    // Load configuration
    let config = Arc::new(Config::load())?;
    info!("Configuration loaded");

    // Initialize Provider Router
    let mut provider_router = ProviderRouter::new();
    
    // Add Ollama provider
    let ollama_provider = OllamaProvider::new(
        "http://localhost:11434".to_string(),
        "qwen2.5-coder:7b-instruct".to_string(),
    );
    provider_router.add_provider(Box::new(ollama_provider));
    
    // Add heuristic fallback
    let heuristic_provider = HeuristicProvider::new();
    provider_router.add_provider(Box::new(heuristic_provider));
    
    let provider_router = Arc::new(provider_router);
    info!("Provider router initialized");

    // Initialize AI engine
    let ai_engine = Arc::new(AIEngine::new(&config).await?);
    info!("AI engine initialized");

    // Initialize Context Manager
    let repo_path = std::env::current_dir()?;
    let mut context_manager = ContextManager::new(repo_path)?;
    context_manager.scan_repository().await?;
    let context_manager = Arc::new(tokio::sync::RwLock::new(context_manager));
    info!("Context manager initialized");

    // Initialize Sandbox and Agent Orchestrator
    let sandbox_config = SandboxConfig::default();
    let agent_orchestrator = Arc::new(AgentOrchestrator::new(
        provider_router.clone(),
        context_manager.clone(),
        sandbox_config,
    ));
    info!("Agent orchestrator initialized");

    // Create application state
    let state = AppState {
        ai_engine,
        provider_router,
        context_manager,
        agent_orchestrator,
        config: config.clone(),
        database: database.clone(), // P0 Day-3: Add database to app state
    };

    // Initialize collaboration services
    let team_sync = Arc::new(tokio::sync::RwLock::new(collaboration::TeamSyncManager::new()));
    let collaboration_engine = Arc::new(tokio::sync::RwLock::new(collaboration::RealTimeCollaborationEngine::new()));
    let code_reviewer = Arc::new(tokio::sync::RwLock::new(collaboration::AICodeReviewer::new(None)));
    
    let collaboration_state = Arc::new(api::CollaborationState {
        team_sync,
        collaboration_engine,
        code_reviewer,
    });

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/complete", post(complete_code))
        .route("/api/v1/analyze", post(analyze_code))
        .nest("/api/v1/collaboration", api::collaboration_routes().with_state(collaboration_state))
        .nest("/api/v1", api::agent_routes())
        .nest("/api/v1", api::v1_routes())
        .merge(observability::observability_routes())
        // P0 Day-2: Security guardrails
        .layer(axum::middleware::from_fn(security::security_headers_middleware))
        .layer(axum::middleware::from_fn(security::security_audit_middleware))
        .layer(security::create_rate_limit_layer())
        .layer(security::create_cors_layer())
        .with_state(state);

    // Start the server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("Server listening on {}", addr);
    info!("API documentation available at http://{}/docs", addr);
    
    axum::serve(listener, app).await?;
    
    // Shutdown tracing on exit
    shutdown_tracing();

    Ok(())
}