pub mod metrics;
pub mod tracing;

pub use metrics::*;
pub use tracing::*;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};

pub async fn metrics_handler() -> Result<Response<String>, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(output) => Ok(Response::builder()
            .status(200)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(output)
            .unwrap()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn observability_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/metrics", get(metrics_handler))
}