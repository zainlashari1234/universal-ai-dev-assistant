use anyhow::Result;
use serde_json::Value;
use std::time::Duration;

/// Core trait that all AI providers must implement
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Generate code completion suggestions
    async fn complete(&self, prompt: &str, context: Option<&str>) -> Result<Vec<String>>;
    
    /// Analyze code and return structured analysis
    async fn analyze(&self, code: &str, language: &str) -> Result<Value>;
    
    /// Check if the provider is healthy and available
    async fn health(&self) -> Result<ProviderHealth>;
    
    /// Get provider-specific metrics
    async fn metrics(&self) -> ProviderMetrics;
    
    /// Get provider name for identification
    fn name(&self) -> &'static str;
    
    /// Get provider priority (higher = preferred)
    fn priority(&self) -> u8;
}

#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub is_available: bool,
    pub latency_ms: Option<u64>,
    pub error_message: Option<String>,
    pub model_loaded: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
}

impl ProviderMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }
    
    pub fn record_success(&mut self, latency_ms: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_success = Some(chrono::Utc::now());
        
        // Update rolling average
        let new_latency = latency_ms as f64;
        if self.total_requests == 1 {
            self.average_latency_ms = new_latency;
        } else {
            self.average_latency_ms = (self.average_latency_ms * (self.total_requests - 1) as f64 + new_latency) / self.total_requests as f64;
        }
    }
    
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_failure = Some(chrono::Utc::now());
    }
}