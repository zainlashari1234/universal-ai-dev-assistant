use super::*;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct AuthState {
    pub jwt_manager: Arc<JwtManager>,
    pub user_service: Arc<dyn UserService>,
    pub audit_service: Arc<dyn AuditService>,
}

#[async_trait::async_trait]
pub trait UserService: Send + Sync {
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn is_session_valid(&self, user_id: Uuid, session_id: Uuid) -> Result<bool>;
    async fn update_last_login(&self, user_id: Uuid) -> Result<()>;
}

pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract bearer token
    let token = JwtManager::extract_bearer_token(auth_header)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate JWT token
    let claims = auth_state
        .jwt_manager
        .validate_token(token, TokenType::Access)
        .map_err(|e| {
            warn!("JWT validation failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // Check if token is expired
    if auth_state.jwt_manager.is_token_expired(&claims) {
        warn!("Token expired for user: {}", claims.email);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Get user from database to ensure they still exist and are active
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user = auth_state
        .user_service
        .get_user_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !user.is_active {
        warn!("Inactive user attempted access: {}", user.email);
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate session
    let session_id = Uuid::parse_str(&claims.session_id).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let session_valid = auth_state
        .user_service
        .is_session_valid(user_id, session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !session_valid {
        warn!("Invalid session for user: {}", user.email);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create auth context
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);
    
    let auth_context = claims
        .to_auth_context(ip_address, user_agent)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add auth context to request extensions
    request.extensions_mut().insert(auth_context.clone());

    // Log authentication event
    if let Err(e) = auth_state.audit_service.log_authentication(&auth_context).await {
        warn!("Failed to log authentication event: {}", e);
    }

    debug!("Authenticated user: {} ({})", user.email, user.id);

    Ok(next.run(request).await)
}

pub async fn require_permission_middleware(
    required_permission: Permission,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |mut request: Request, next: Next| {
        let required_permission = required_permission.clone();
        Box::pin(async move {
            // Get auth context from request extensions
            let auth_context = request
                .extensions()
                .get::<AuthContext>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            // Check if user has required permission
            if !auth_context.user.has_permission(&required_permission) {
                warn!(
                    "User {} lacks required permission: {:?}",
                    auth_context.user.email, required_permission
                );
                return Err(StatusCode::FORBIDDEN);
            }

            Ok(next.run(request).await)
        })
    }
}

pub async fn require_any_permission_middleware(
    required_permissions: Vec<Permission>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |mut request: Request, next: Next| {
        let required_permissions = required_permissions.clone();
        Box::pin(async move {
            // Get auth context from request extensions
            let auth_context = request
                .extensions()
                .get::<AuthContext>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            // Check if user has any of the required permissions
            if !auth_context.user.has_any_permission(&required_permissions) {
                warn!(
                    "User {} lacks any of required permissions: {:?}",
                    auth_context.user.email, required_permissions
                );
                return Err(StatusCode::FORBIDDEN);
            }

            Ok(next.run(request).await)
        })
    }
}

pub async fn require_admin_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get auth context from request extensions
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check if user is admin
    if !auth_context.user.is_admin() {
        warn!("Non-admin user attempted admin action: {}", auth_context.user.email);
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

pub async fn organization_isolation_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get auth context from request extensions
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Add organization context to request
    request.extensions_mut().insert(auth_context.user.organization_id);

    Ok(next.run(request).await)
}

fn extract_ip_address(headers: &HeaderMap) -> String {
    // Try various headers for real IP address
    headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            headers
                .get("cf-connecting-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn extract_user_agent(headers: &HeaderMap) -> String {
    headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

// Helper macro for creating permission-protected routes
#[macro_export]
macro_rules! protected_route {
    ($permission:expr, $handler:expr) => {
        axum::middleware::from_fn(require_permission_middleware($permission))
            .layer(axum::middleware::from_fn(auth_middleware))
            .route_handler($handler)
    };
}

// Helper macro for creating admin-only routes
#[macro_export]
macro_rules! admin_route {
    ($handler:expr) => {
        axum::middleware::from_fn(require_admin_middleware)
            .layer(axum::middleware::from_fn(auth_middleware))
            .route_handler($handler)
    };
}