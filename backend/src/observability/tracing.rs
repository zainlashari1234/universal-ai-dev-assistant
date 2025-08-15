// P0 Task #2: OpenTelemetry tracing IMPLEMENTATION
use opentelemetry::{
    global,
    trace::{TraceId, SpanId, TraceError},
    KeyValue,
};
use opentelemetry_jaeger::JaegerPipeline;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_semantic_conventions::trace;
use tracing::{info, warn, Span};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;
use std::env;

/// Initialize OpenTelemetry tracing system with OTLP/Jaeger exporters
pub fn init_tracing() -> anyhow::Result<()> {
    // Get tracing configuration from environment
    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "uaida-backend".to_string());
    let jaeger_endpoint = env::var("JAEGER_ENDPOINT").unwrap_or_else(|_| "http://localhost:14268/api/traces".to_string());
    let otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
    let enable_tracing = env::var("ENABLE_TRACING").unwrap_or_else(|_| "true".to_string()) == "true";
    
    if !enable_tracing {
        info!("OpenTelemetry tracing disabled via ENABLE_TRACING=false");
        // Initialize basic tracing without OpenTelemetry
        tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
            .with(tracing_subscriber::fmt::layer())
            .try_init()?;
        return Ok(());
    }

    // Initialize Jaeger tracer
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(&service_name)
        .with_endpoint(&jaeger_endpoint)
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Create OpenTelemetry layer
    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    // Set up tracing subscriber with OpenTelemetry support
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .compact(),
        )
        .with(telemetry_layer)
        .try_init()?;
    
    info!("OpenTelemetry tracing system initialized with service: {}", service_name);
    info!("Jaeger endpoint: {}", jaeger_endpoint);
    info!("OTLP endpoint: {}", otlp_endpoint);
    
    Ok(())
}

/// Shutdown tracing and flush pending spans
pub fn shutdown_tracing() {
    info!("Shutting down OpenTelemetry tracing");
    global::shutdown_tracer_provider();
}

/// Generate correlation ID for request tracing
pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// Create span with correlation ID for API requests
pub fn create_request_span(operation: &str, request_id: &str) -> tracing::Span {
    tracing::info_span!(
        "request",
        operation = operation,
        request_id = request_id,
        otel.kind = "server"
    )
}

/// Create span for agent operations
pub fn create_agent_span(agent: &str, step: &str, plan_id: Option<&str>) -> tracing::Span {
    match plan_id {
        Some(id) => tracing::info_span!(
            "agent_step",
            agent = agent,
            step = step,
            plan_id = id,
            otel.kind = "internal"
        ),
        None => tracing::info_span!(
            "agent_step",
            agent = agent,
            step = step,
            otel.kind = "internal"
        )
    }
}

/// Create span for provider operations
pub fn create_provider_span(provider: &str, operation: &str) -> tracing::Span {
    tracing::info_span!(
        "provider_operation",
        provider = provider,
        operation = operation,
        otel.kind = "client"
    )
}

/// Add correlation attributes to existing span
pub fn add_correlation_id(span: &Span, request_id: &str) {
    span.record("request_id", request_id);
}

/// Record span attributes for plan operations
pub fn record_plan_attributes(span: &Span, plan_id: &str, goal: &str, steps_count: usize) {
    span.record("plan_id", plan_id);
    span.record("goal", goal);
    span.record("steps_count", steps_count);
}

/// Record span attributes for patch operations  
pub fn record_patch_attributes(span: &Span, patch_id: &str, files_count: usize) {
    span.record("patch_id", patch_id);
    span.record("files_count", files_count);
}

/// Record span attributes for test run operations
pub fn record_run_attributes(span: &Span, run_id: &str, language: &str, test_count: usize) {
    span.record("run_id", run_id);
    span.record("language", language);
    span.record("test_count", test_count);
}