use anyhow::Result;
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

use ai_engine::AIEngine;
use config::Config;

#[derive(Clone)]
pub struct AppState {
    ai_engine: Arc<AIEngine>,
    config: Arc<Config>,
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
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        ai_model_loaded: state.ai_engine.is_model_loaded().await,
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
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Universal AI Development Assistant v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Arc::new(Config::load()?);
    info!("Configuration loaded");

    // Initialize AI engine
    let ai_engine = Arc::new(AIEngine::new(&config).await?);
    info!("AI engine initialized");

    // Create application state
    let state = AppState {
        ai_engine,
        config: config.clone(),
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
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start the server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("Server listening on {}", addr);
    info!("API documentation available at http://{}/docs", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}