// P0 Task #2: Security guardrails implementation
use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use headers::{Header, HeaderMapExt};
use std::time::Duration;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tracing::{info, warn};

/// Security headers middleware for P0 compliance
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // X-Content-Type-Options: nosniff
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    
    // X-Frame-Options: DENY
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    
    // Referrer-Policy: no-referrer
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    
    // Content-Security-Policy: minimal CSP
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; object-src 'none';"),
    );
    
    // X-Permitted-Cross-Domain-Policies: none
    headers.insert(
        HeaderName::from_static("x-permitted-cross-domain-policies"),
        HeaderValue::from_static("none"),
    );
    
    // Permissions-Policy: minimal permissions
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    
    info!("Security headers applied to response");
    Ok(response)
}

/// Create rate limiting layer for API protection
pub fn create_rate_limit_layer() -> GovernorLayer<SmartIpKeyExtractor> {
    // Configure rate limiting: 100 requests per minute per IP
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(2) // 2 requests per second
            .burst_size(10) // Allow bursts up to 10 requests
            .finish()
            .expect("Failed to create rate limiter configuration"),
    );
    
    info!("Rate limiting configured: 2 req/s, burst 10");
    GovernorLayer {
        config: governor_conf,
        key_extractor: SmartIpKeyExtractor::default(),
    }
}

/// CORS configuration for strict security
pub fn create_cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};
    
    CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
            "https://localhost:3000".parse().unwrap(),
        ])
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(false)
        .max_age(Duration::from_secs(3600))
}

/// Security audit logging middleware
pub async fn security_audit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_agent = request.headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");
    
    // Log security-relevant request details
    info!(
        method = %method,
        uri = %uri,
        user_agent = %user_agent,
        "Security audit: API request"
    );
    
    let response = next.run(request).await;
    let status = response.status();
    
    // Log suspicious activity
    if status.is_client_error() || status.is_server_error() {
        warn!(
            method = %method,
            uri = %uri,
            status = %status,
            user_agent = %user_agent,
            "Security audit: Failed request"
        );
    }
    
    Ok(response)
}