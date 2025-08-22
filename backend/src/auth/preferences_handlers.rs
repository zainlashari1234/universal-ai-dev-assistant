use super::preferences::*;
use crate::{auth::AuthContext, AppState};
use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::Json as ResponseJson,
};
use serde_json::json;

pub async fn get_preferences_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.preferences_service.get_user_preferences(auth_context.user.id).await {
        Ok(preferences) => Ok(ResponseJson(json!({
            "success": true,
            "preferences": preferences
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn update_preferences_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<UpdatePreferencesRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.preferences_service.update_user_preferences(auth_context.user.id, request).await {
        Ok(preferences) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Preferences updated successfully",
            "preferences": preferences
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn reset_preferences_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.preferences_service.reset_preferences(auth_context.user.id).await {
        Ok(preferences) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Preferences reset to defaults",
            "preferences": preferences
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn export_preferences_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.preferences_service.export_preferences(auth_context.user.id).await {
        Ok(preferences) => Ok(ResponseJson(json!({
            "success": true,
            "preferences": preferences
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn import_preferences_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(preferences_json): Json<serde_json::Value>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.preferences_service.import_preferences(auth_context.user.id, preferences_json).await {
        Ok(preferences) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Preferences imported successfully",
            "preferences": preferences
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}