use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::collaboration::{TeamSyncManager, RealTimeCollaborationEngine, AICodeReviewer};

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub name: String,
    pub host_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinSessionRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ShareFileRequest {
    pub file_path: String,
    pub content: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub title: String,
    pub author_id: String,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CollaborationResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}

pub struct CollaborationState {
    pub team_sync: Arc<RwLock<TeamSyncManager>>,
    pub collaboration_engine: Arc<RwLock<RealTimeCollaborationEngine>>,
    pub code_reviewer: Arc<RwLock<AICodeReviewer>>,
}

pub fn collaboration_routes() -> Router<Arc<CollaborationState>> {
    Router::new()
        .route("/sessions", post(create_session))
        .route("/sessions/:session_id/join", post(join_session))
        .route("/sessions/:session_id/share", post(share_file))
        .route("/sessions/:session_id/analysis", post(request_analysis))
        .route("/sessions", get(get_active_sessions))
        .route("/sessions/:session_id", get(get_session_info))
        .route("/reviews", post(create_code_review))
        .route("/reviews/:review_id/suggestions", get(get_ai_suggestions))
        .route("/reviews/:review_id/accept/:suggestion_id", post(accept_suggestion))
        .route("/team/insights", get(get_team_insights))
        .route("/team/members", get(get_active_members))
}

async fn create_session(
    State(state): State<Arc<CollaborationState>>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let host_id = Uuid::parse_str(&request.host_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let collaboration_engine = state.collaboration_engine.read().await;
    
    match collaboration_engine.create_session(request.name, host_id).await {
        Ok(session_id) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::json!({"session_id": session_id})),
            message: "Session created successfully".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to create session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn join_session(
    State(state): State<Arc<CollaborationState>>,
    Path(session_id): Path<String>,
    Json(request): Json<JoinSessionRequest>,
) -> Result<Json<CollaborationResponse<()>>, StatusCode> {
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = Uuid::parse_str(&request.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let collaboration_engine = state.collaboration_engine.read().await;
    
    match collaboration_engine.join_session(session_id, user_id).await {
        Ok(_) => Ok(Json(CollaborationResponse {
            success: true,
            data: None,
            message: "Joined session successfully".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to join session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn share_file(
    State(state): State<Arc<CollaborationState>>,
    Path(session_id): Path<String>,
    Json(request): Json<ShareFileRequest>,
) -> Result<Json<CollaborationResponse<()>>, StatusCode> {
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = Uuid::parse_str(&request.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let collaboration_engine = state.collaboration_engine.read().await;
    
    match collaboration_engine.share_file(session_id, request.file_path, request.content, user_id).await {
        Ok(_) => Ok(Json(CollaborationResponse {
            success: true,
            data: None,
            message: "File shared successfully".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to share file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn request_analysis(
    State(state): State<Arc<CollaborationState>>,
    Path(session_id): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = Uuid::parse_str(request["user_id"].as_str().unwrap_or(""))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_path = request["file_path"].as_str().unwrap_or("").to_string();

    let collaboration_engine = state.collaboration_engine.read().await;
    
    match collaboration_engine.request_ai_analysis(session_id, user_id, file_path).await {
        Ok(analysis) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(analysis),
            message: "Analysis completed".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to perform analysis: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_active_sessions(
    State(state): State<Arc<CollaborationState>>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let collaboration_engine = state.collaboration_engine.read().await;
    
    match collaboration_engine.get_active_sessions().await {
        Ok(sessions) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::to_value(sessions).unwrap()),
            message: "Active sessions retrieved".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to get active sessions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_session_info(
    State(state): State<Arc<CollaborationState>>,
    Path(session_id): Path<String>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let collaboration_engine = state.collaboration_engine.read().await;
    
    match collaboration_engine.get_session_info(session_id).await {
        Ok(Some(session)) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::to_value(session).unwrap()),
            message: "Session info retrieved".to_string(),
        })),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            eprintln!("Failed to get session info: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn create_code_review(
    State(state): State<Arc<CollaborationState>>,
    Json(request): Json<CreateReviewRequest>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let author_id = Uuid::parse_str(&request.author_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Create review files (simplified)
    let review_files = request.file_paths.into_iter().map(|path| {
        crate::collaboration::code_review_ai::ReviewFile {
            path: path.clone(),
            original_content: format!("// Original content of {}", path),
            modified_content: format!("// Modified content of {}", path),
            comments: Vec::new(),
            ai_suggestions: Vec::new(),
        }
    }).collect();

    let code_reviewer = state.code_reviewer.read().await;
    
    match code_reviewer.create_review(request.title, author_id, review_files).await {
        Ok(review_id) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::json!({"review_id": review_id})),
            message: "Code review created successfully".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to create code review: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_ai_suggestions(
    State(state): State<Arc<CollaborationState>>,
    Path(review_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let review_id = Uuid::parse_str(&review_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_path = params.get("file_path").cloned().unwrap_or_default();

    let code_reviewer = state.code_reviewer.read().await;
    
    match code_reviewer.get_ai_suggestions(review_id, file_path).await {
        Ok(suggestions) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::to_value(suggestions).unwrap()),
            message: "AI suggestions retrieved".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to get AI suggestions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn accept_suggestion(
    State(state): State<Arc<CollaborationState>>,
    Path((review_id, suggestion_id)): Path<(String, String)>,
) -> Result<Json<CollaborationResponse<()>>, StatusCode> {
    let review_id = Uuid::parse_str(&review_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let suggestion_id = Uuid::parse_str(&suggestion_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let code_reviewer = state.code_reviewer.read().await;
    
    match code_reviewer.accept_ai_suggestion(review_id, suggestion_id).await {
        Ok(_) => Ok(Json(CollaborationResponse {
            success: true,
            data: None,
            message: "AI suggestion accepted".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to accept suggestion: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_team_insights(
    State(state): State<Arc<CollaborationState>>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let team_sync = state.team_sync.read().await;
    
    match team_sync.get_team_insights().await {
        Ok(insights) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::to_value(insights).unwrap()),
            message: "Team insights retrieved".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to get team insights: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_active_members(
    State(state): State<Arc<CollaborationState>>,
) -> Result<Json<CollaborationResponse<serde_json::Value>>, StatusCode> {
    let team_sync = state.team_sync.read().await;
    
    match team_sync.get_active_members().await {
        Ok(members) => Ok(Json(CollaborationResponse {
            success: true,
            data: Some(serde_json::to_value(members).unwrap()),
            message: "Active members retrieved".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to get active members: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}