use crate::auth::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub organization_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user: UserInfo,
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub organization_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SsoInitiateResponse {
    pub auth_url: String,
    pub state: String,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuditSearchRequest {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub user_id: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub async fn login(
    State(auth_state): State<AuthState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    info!("Login attempt for email: {}", request.email);

    // In a real implementation, this would:
    // 1. Validate credentials against database
    // 2. Check password policy
    // 3. Handle MFA if required
    // 4. Create session

    // For demo purposes, create a mock user
    let org_id = if let Some(org_str) = request.organization_id {
        Uuid::parse_str(&org_str).map_err(|_| StatusCode::BAD_REQUEST)?
    } else {
        Uuid::new_v4()
    };

    let user = User {
        id: Uuid::new_v4(),
        email: request.email.clone(),
        name: request.email.split('@').next().unwrap_or("User").to_string(),
        organization_id: org_id,
        roles: vec![Role::developer_role(org_id)],
        permissions: vec![
            Permission::ApiAccess,
            Permission::CreatePlan,
            Permission::ExecutePlan,
            Permission::ViewPlan,
            Permission::GenerateCode,
            Permission::ReviewCode,
            Permission::ViewSecurityReports,
            Permission::ViewEvaluations,
        ],
        created_at: chrono::Utc::now(),
        last_login: Some(chrono::Utc::now()),
        is_active: true,
    };

    // Generate JWT tokens
    let session_id = Uuid::new_v4();
    let token_pair = auth_state.jwt_manager
        .generate_token_pair(&user, session_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log authentication event
    let auth_context = AuthContext {
        user: user.clone(),
        session_id,
        ip_address: "127.0.0.1".to_string(),
        user_agent: "api_client".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        permissions: user.permissions.clone(),
    };

    if let Err(e) = auth_state.audit_service.log_authentication(&auth_context).await {
        warn!("Failed to log authentication: {}", e);
    }

    let response = LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user: UserInfo {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            organization_id: user.organization_id.to_string(),
            roles: user.roles.iter().map(|r| r.name.clone()).collect(),
            permissions: user.permissions.iter().map(|p| format!("{:?}", p)).collect(),
        },
    };

    Ok(Json(response))
}

pub async fn refresh_token(
    State(auth_state): State<AuthState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<TokenPair>, StatusCode> {
    info!("Token refresh attempt");

    // Validate refresh token
    let claims = auth_state.jwt_manager
        .validate_token(&request.refresh_token, TokenType::Refresh)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get user from database
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user = auth_state.user_service
        .get_user_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let session_id = Uuid::parse_str(&claims.session_id).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Generate new token pair
    let new_tokens = auth_state.jwt_manager
        .refresh_access_token(&request.refresh_token, &user, session_id)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(new_tokens))
}

pub async fn initiate_sso(
    State(auth_state): State<AuthState>,
    Path(organization_id): Path<String>,
    Json(request): Json<SsoAuthRequest>,
) -> Result<Json<SsoInitiateResponse>, StatusCode> {
    info!("SSO initiation for organization: {}", organization_id);

    // This would use the SsoManager from auth_state
    // For demo purposes, return a mock response
    let response = SsoInitiateResponse {
        auth_url: format!("https://sso.example.com/auth?org={}", organization_id),
        state: Uuid::new_v4().to_string(),
    };

    Ok(Json(response))
}

pub async fn sso_callback(
    State(auth_state): State<AuthState>,
    Path(organization_id): Path<String>,
    Json(callback): Json<SsoCallbackRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    info!("SSO callback for organization: {}", organization_id);

    // This would use the SsoManager to handle the callback
    // For demo purposes, create a mock successful response
    let org_id = Uuid::parse_str(&organization_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let user = User {
        id: Uuid::new_v4(),
        email: "sso.user@example.com".to_string(),
        name: "SSO User".to_string(),
        organization_id: org_id,
        roles: vec![Role::developer_role(org_id)],
        permissions: vec![Permission::ApiAccess, Permission::CreatePlan],
        created_at: chrono::Utc::now(),
        last_login: Some(chrono::Utc::now()),
        is_active: true,
    };

    let session_id = Uuid::new_v4();
    let token_pair = auth_state.jwt_manager
        .generate_token_pair(&user, session_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = LoginResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        expires_in: token_pair.expires_in,
        token_type: token_pair.token_type,
        user: UserInfo {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            organization_id: user.organization_id.to_string(),
            roles: user.roles.iter().map(|r| r.name.clone()).collect(),
            permissions: user.permissions.iter().map(|p| format!("{:?}", p)).collect(),
        },
    };

    Ok(Json(response))
}

pub async fn get_user_profile(
    auth_context: AuthContext,
) -> Result<Json<UserInfo>, StatusCode> {
    let user_info = UserInfo {
        id: auth_context.user.id.to_string(),
        email: auth_context.user.email,
        name: auth_context.user.name,
        organization_id: auth_context.user.organization_id.to_string(),
        roles: auth_context.user.roles.iter().map(|r| r.name.clone()).collect(),
        permissions: auth_context.user.permissions.iter().map(|p| format!("{:?}", p)).collect(),
    };

    Ok(Json(user_info))
}

pub async fn search_audit_logs(
    State(auth_state): State<AuthState>,
    auth_context: AuthContext,
    Query(params): Query<AuditSearchRequest>,
) -> Result<Json<Vec<AuditEvent>>, StatusCode> {
    // Check if user has permission to view audit logs
    if !auth_context.user.has_permission(&Permission::ViewAuditLogs) {
        return Err(StatusCode::FORBIDDEN);
    }

    let criteria = AuditSearchCriteria {
        start_time: params.start_time
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        end_time: params.end_time
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        user_id: params.user_id
            .and_then(|s| Uuid::parse_str(&s).ok()),
        organization_id: Some(auth_context.user.organization_id),
        event_types: params.event_types
            .map(|types| types.into_iter().filter_map(|t| match t.as_str() {
                "Authentication" => Some(AuditEventType::Authentication),
                "Authorization" => Some(AuditEventType::Authorization),
                "DataAccess" => Some(AuditEventType::DataAccess),
                "DataModification" => Some(AuditEventType::DataModification),
                "SystemConfiguration" => Some(AuditEventType::SystemConfiguration),
                "SecurityEvent" => Some(AuditEventType::SecurityEvent),
                "AgentExecution" => Some(AuditEventType::AgentExecution),
                "CodeGeneration" => Some(AuditEventType::CodeGeneration),
                "PolicyViolation" => Some(AuditEventType::PolicyViolation),
                "AdminAction" => Some(AuditEventType::AdminAction),
                _ => None,
            }).collect()),
        outcomes: None,
        resource_type: None,
        action: None,
        limit: params.limit,
        offset: params.offset,
    };

    let events = auth_state.audit_service
        .search_events(criteria)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(events))
}

pub async fn get_user_activity(
    State(auth_state): State<AuthState>,
    auth_context: AuthContext,
    Path(user_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<AuditEvent>>, StatusCode> {
    // Check if user can view this user's activity
    let target_user_id = Uuid::parse_str(&user_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if target_user_id != auth_context.user.id && !auth_context.user.has_permission(&Permission::ViewAuditLogs) {
        return Err(StatusCode::FORBIDDEN);
    }

    let days = params.get("days")
        .and_then(|d| d.parse().ok())
        .unwrap_or(30);

    let events = auth_state.audit_service
        .get_user_activity(target_user_id, days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(events))
}

pub async fn logout(
    State(auth_state): State<AuthState>,
    auth_context: AuthContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("User logout: {}", auth_context.user.email);

    // In a real implementation, this would:
    // 1. Invalidate the session
    // 2. Add token to blacklist
    // 3. Log the logout event

    // Log logout event
    let logout_event = AuditEvent {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        event_type: AuditEventType::Authentication,
        user_id: Some(auth_context.user.id),
        session_id: Some(auth_context.session_id),
        organization_id: auth_context.user.organization_id,
        ip_address: auth_context.ip_address,
        user_agent: auth_context.user_agent,
        resource_type: None,
        resource_id: None,
        action: "logout".to_string(),
        outcome: AuditOutcome::Success,
        details: HashMap::new(),
        risk_score: Some(0.1),
    };

    if let Err(e) = auth_state.audit_service.log_event(logout_event).await {
        warn!("Failed to log logout event: {}", e);
    }

    Ok(Json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}

pub fn auth_routes() -> Router<AuthState> {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/sso/:organization_id/initiate", post(initiate_sso))
        .route("/sso/:organization_id/callback", post(sso_callback))
        .route("/profile", get(get_user_profile))
        .route("/audit/search", get(search_audit_logs))
        .route("/audit/user/:user_id", get(get_user_activity))
        .route("/logout", post(logout))
}