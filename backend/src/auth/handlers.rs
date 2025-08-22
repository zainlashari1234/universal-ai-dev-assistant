use super::{user_service, api_key_manager, jwt, User, Role, Permission, AuthContext};
use crate::AppState;
use axum::{
    extract::{State, Json},
    http::{StatusCode, HeaderMap},
    response::Json as ResponseJson,
};
use serde_json::json;
use uuid::Uuid;

pub async fn register_handler(
    State(state): State<crate::AppState>,
    Json(request): Json<user_service::RegisterRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.user_service.register(request).await {
        Ok(user) => Ok(ResponseJson(json!({
            "success": true,
            "message": "User registered successfully",
            "user": user
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

pub async fn login_handler(
    State(state): State<crate::AppState>,
    Json(request): Json<user_service::LoginRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.user_service.login(request).await {
        Ok(response) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Login successful",
            "user": response.user,
            "tokens": response.tokens,
            "session_id": response.session_id
        }))),
        Err(e) => Err((
            StatusCode::UNAUTHORIZED,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn logout_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.user_service.logout(auth_context.session_id).await {
        Ok(_) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Logout successful"
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

pub async fn refresh_token_handler(
    State(state): State<crate::AppState>,
    Json(request): Json<jwt::RefreshRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    // Validate refresh token and get user info
    let claims = match state.jwt_manager.validate_token(&request.refresh_token, jwt::TokenType::Refresh) {
        Ok(claims) => claims,
        Err(_) => return Err((
            StatusCode::UNAUTHORIZED,
            ResponseJson(json!({
                "success": false,
                "error": "Invalid refresh token"
            }))
        ))
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(json!({
                "success": false,
                "error": "Invalid user ID in token"
            }))
        ))
    };

    let session_id = match Uuid::parse_str(&claims.session_id) {
        Ok(id) => id,
        Err(_) => return Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(json!({
                "success": false,
                "error": "Invalid session ID in token"
            }))
        ))
    };

    // Validate session is still active
    match state.user_service.validate_session(session_id).await {
        Ok(true) => {},
        Ok(false) => return Err((
            StatusCode::UNAUTHORIZED,
            ResponseJson(json!({
                "success": false,
                "error": "Session expired or invalid"
            }))
        )),
        Err(e) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    };

    // Get user for token generation
    let user = match state.user_service.get_user(user_id).await {
        Ok(Some(user_response)) => {
            // Convert UserResponse to User for JWT generation
            User {
                id: user_response.id,
                email: user_response.email,
                name: user_response.full_name.unwrap_or_else(|| user_response.username),
                organization_id: Uuid::new_v4(), // TODO: Implement organizations
                roles: vec![Role::developer_role(Uuid::new_v4())], // TODO: Get actual roles
                permissions: vec![Permission::ApiAccess, Permission::CreatePlan], // TODO: Get actual permissions
                created_at: user_response.created_at,
                last_login: user_response.last_login_at,
                is_active: user_response.is_active,
            }
        },
        Ok(None) => return Err((
            StatusCode::NOT_FOUND,
            ResponseJson(json!({
                "success": false,
                "error": "User not found"
            }))
        )),
        Err(e) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    };

    // Generate new token pair
    match state.jwt_manager.refresh_access_token(&request.refresh_token, &user, session_id) {
        Ok(tokens) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Tokens refreshed successfully",
            "tokens": tokens
        }))),
        Err(e) => Err((
            StatusCode::UNAUTHORIZED,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn get_profile_handler(
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    Ok(ResponseJson(json!({
        "success": true,
        "user": {
            "id": auth_context.user.id,
            "email": auth_context.user.email,
            "name": auth_context.user.name,
            "permissions": auth_context.permissions,
            "last_login": auth_context.user.last_login,
            "is_active": auth_context.user.is_active
        }
    })))
}

pub async fn update_profile_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
    Json(request): Json<user_service::UpdateUserRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.user_service.update_user(auth_context.user.id, request).await {
        Ok(user) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Profile updated successfully",
            "user": user
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

pub async fn change_password_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
    Json(request): Json<user_service::ChangePasswordRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.user_service.change_password(auth_context.user.id, request).await {
        Ok(_) => Ok(ResponseJson(json!({
            "success": true,
            "message": "Password changed successfully"
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

// API Key Management Handlers
pub async fn create_api_key_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
    Json(request): Json<api_key_manager::CreateApiKeyRequest>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.api_key_manager.store_api_key(auth_context.user.id, request).await {
        Ok(api_key) => Ok(ResponseJson(json!({
            "success": true,
            "message": "API key stored successfully",
            "api_key": {
                "id": api_key.id,
                "provider": api_key.provider,
                "key_name": api_key.key_name,
                "created_at": api_key.created_at
            }
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

pub async fn get_api_keys_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.api_key_manager.get_user_api_keys(auth_context.user.id).await {
        Ok(keys) => Ok(ResponseJson(json!({
            "success": true,
            "api_keys": keys
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

pub async fn delete_api_key_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
    axum::extract::Path(key_id): axum::extract::Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.api_key_manager.delete_api_key(auth_context.user.id, key_id).await {
        Ok(true) => Ok(ResponseJson(json!({
            "success": true,
            "message": "API key deleted successfully"
        }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            ResponseJson(json!({
                "success": false,
                "error": "API key not found"
            }))
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "success": false,
                "error": e.to_string()
            }))
        ))
    }
}

pub async fn get_api_key_usage_handler(
    State(state): State<crate::AppState>,
    auth_context: AuthContext,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match state.api_key_manager.get_usage_stats(auth_context.user.id).await {
        Ok(stats) => Ok(ResponseJson(json!({
            "success": true,
            "usage_stats": stats
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