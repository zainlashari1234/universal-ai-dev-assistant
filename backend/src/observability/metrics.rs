use prometheus::{
    Counter, Histogram, IntCounter, IntGauge, Registry, 
    HistogramOpts, Opts, register_counter, register_histogram, 
    register_int_counter, register_int_gauge, IntCounterVec, HistogramVec,
    register_int_counter_vec, register_histogram_vec,
};
use std::sync::OnceLock;
use tracing::warn;

pub struct Metrics {
    // HTTP metrics as specified in the plan
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_ms: HistogramVec,
    
    // Provider metrics as specified in the plan
    pub provider_requests_total: IntCounterVec,
    pub provider_request_duration_ms: HistogramVec,
    
    // Agent metrics as specified in the plan
    pub agent_step_duration_ms: HistogramVec,
    
    // Additional metrics
    pub suggestion_acceptance_total: IntCounterVec,
    pub active_executions: IntGauge,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn init_metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        // HTTP metrics as specified in the plan
        let http_requests_total = register_int_counter_vec!(
            "http_requests_total",
            "Total number of HTTP requests",
            &["route", "method", "status"]
        ).expect("Failed to register http_requests_total metric");

        let http_request_duration_ms = register_histogram_vec!(
            HistogramOpts::new(
                "http_request_duration_ms_bucket",
                "HTTP request duration in milliseconds"
            ).buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0]),
            &["route", "method"]
        ).expect("Failed to register http_request_duration_ms metric");

        // Provider metrics as specified in the plan
        let provider_requests_total = register_int_counter_vec!(
            "provider_requests_total",
            "Total number of AI provider requests",
            &["provider", "op"]
        ).expect("Failed to register provider_requests_total metric");

        let provider_request_duration_ms = register_histogram_vec!(
            HistogramOpts::new(
                "provider_request_duration_ms_bucket",
                "AI provider request duration in milliseconds"
            ).buckets(vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 30000.0]),
            &["provider", "op"]
        ).expect("Failed to register provider_request_duration_ms metric");

        // Agent metrics as specified in the plan
        let agent_step_duration_ms = register_histogram_vec!(
            HistogramOpts::new(
                "agent_step_duration_ms_bucket",
                "Agent step duration in milliseconds"
            ).buckets(vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0]),
            &["agent", "step"]
        ).expect("Failed to register agent_step_duration_ms metric");

        // Additional metrics
        let suggestion_acceptance_total = register_int_counter_vec!(
            "suggestion_acceptance_total",
            "Total number of accepted suggestions",
            &["language"]
        ).expect("Failed to register suggestion_acceptance_total metric");

        let active_executions = register_int_gauge!(
            "active_executions",
            "Number of currently active agent executions"
        ).expect("Failed to register active_executions metric");

        Metrics {
            http_requests_total,
            http_request_duration_ms,
            provider_requests_total,
            provider_request_duration_ms,
            agent_step_duration_ms,
            suggestion_acceptance_total,
            active_executions,
        }
    })
}

pub fn get_metrics() -> &'static Metrics {
    METRICS.get().expect("Metrics not initialized")
}