use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use crate::observability::get_metrics;

use super::traits::{Provider, ProviderHealth, ProviderMetrics};
use super::{HeuristicProvider, OllamaProvider, ProviderConfig, ProviderType};

#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    pub prefer_local: bool,
    pub max_latency_ms: u64,
    pub min_success_rate: f64,
    pub fallback_enabled: bool,
    pub quality_threshold: f64,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            prefer_local: true,
            max_latency_ms: 5000,
            min_success_rate: 0.8,
            fallback_enabled: true,
            quality_threshold: 0.7,
        }
    }
}

pub struct ProviderRouter {
    providers: Vec<Box<dyn Provider>>,
    policy: RoutingPolicy,
    fallback_provider: Box<dyn Provider>,
    metrics: Arc<RwLock<RouterMetrics>>,
}

#[derive(Debug, Clone, Default)]
struct RouterMetrics {
    total_requests: u64,
    provider_selections: std::collections::HashMap<String, u64>,
    fallback_usage: u64,
    routing_latency_ms: f64,
}

impl ProviderRouter {
    
    /// New improved method with health gating and timeout handling
    async fn select_provider_with_health_check(&self, task_type: &str) -> Result<&dyn Provider> {
        let start_time = Instant::now();
        
        debug!("Selecting provider with health check for task: {}", task_type);
        
        // First, health check all providers in parallel
        let health_futures: Vec<_> = self.providers.iter()
            .map(|provider| async move {
                let health_result = tokio::time::timeout(
                    Duration::from_millis(1000), // 1s timeout for health checks
                    provider.health()
                ).await;
                
                match health_result {
                    Ok(Ok(health)) => (provider.name(), Some(health)),
                    Ok(Err(e)) => {
                        warn!("Health check failed for {}: {}", provider.name(), e);
                        (provider.name(), None)
                    }
                    Err(_) => {
                        warn!("Health check timed out for {}", provider.name());
                        (provider.name(), None)
                    }
                }
            })
            .collect();
            
        let health_results = futures::future::join_all(health_futures).await;
        
        // Score available providers
        let mut provider_scores = Vec::new();
        
        for (idx, provider) in self.providers.iter().enumerate() {
            if let Some((_, Some(health))) = health_results.iter().find(|(name, _)| *name == provider.name()) {
                let metrics = provider.metrics().await;
                let score = self.calculate_provider_score(health, &metrics, task_type).await;
                
                if health.is_available && score >= self.policy.quality_threshold {
                    provider_scores.push((idx, score, provider.name()));
                    debug!("Provider {} qualified with score: {:.2}", provider.name(), score);
                }
            }
        }
        
        // Sort by score (highest first)  
        provider_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Try providers in order until one succeeds
        for (idx, score, name) in provider_scores {
            debug!("Attempting to use provider: {} (score: {:.2})", name, score);
            
            // Record selection metrics
            {
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
                *metrics.provider_selections.entry(name.to_string()).or_insert(0) += 1;
                metrics.routing_latency_ms = start_time.elapsed().as_millis() as f64;
            }
            
            info!("Selected provider: {} (score: {:.2})", name, score);
            return Ok(self.providers[idx].as_ref());
        }
        
        // No provider qualified, use fallback if enabled
        if self.policy.fallback_enabled {
            warn!("No qualified provider found, using fallback");
            {
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
                metrics.fallback_usage += 1;
                metrics.routing_latency_ms = start_time.elapsed().as_millis() as f64;
            }
            return Ok(self.fallback_provider.as_ref());
        }
        
        Err(anyhow!("No provider available and fallback disabled"))
    }
    
    /// Handle provider failure with fallback logic
    async fn handle_provider_failure(&self, provider_name: &str, operation: &str, start_time: Instant) -> Result<Vec<String>> {
        warn!("Provider {} failed for {}", provider_name, operation);
        
        if self.policy.fallback_enabled {
            warn!("Falling back to heuristic provider");
            
            // Record fallback usage
            {
                let mut metrics_guard = self.metrics.write().await;
                metrics_guard.fallback_usage += 1;
            }
            
            // Record metrics for failed attempt
            let metrics = get_metrics();
            metrics.provider_request_duration_ms
                .with_label_values(&[provider_name, operation])
                .observe(start_time.elapsed().as_millis() as f64);
            
            // Attempt fallback with timeout
            let fallback_future = self.fallback_provider.complete("fallback", None);
            let timeout_duration = Duration::from_millis(self.policy.max_latency_ms / 2); // Shorter timeout for fallback
            
            match tokio::time::timeout(timeout_duration, fallback_future).await {
                Ok(Ok(suggestions)) => {
                    info!("Fallback completion successful");
                    Ok(suggestions)
                }
                Ok(Err(fallback_error)) => {
                    error!("Fallback provider also failed: {}", fallback_error);
                    Err(anyhow!("All providers failed. Last error: {}", fallback_error))
                }
                Err(_) => {
                    error!("Fallback provider timed out");
                    Err(anyhow!("All providers failed due to timeout"))
                }
            }
        } else {
            Err(anyhow!("Provider {} failed and fallback disabled", provider_name))
        }
    }
    pub async fn new(configs: Vec<ProviderConfig>, policy: RoutingPolicy) -> Result<Self> {
        let mut providers: Vec<Box<dyn Provider>> = Vec::new();
        
        // Initialize providers based on configs
        for config in configs {
            match config.provider_type {
                ProviderType::Ollama => {
                    let provider = OllamaProvider::new(config);
                    providers.push(Box::new(provider));
                }
                ProviderType::Heuristic => {
                    let provider = HeuristicProvider::new();
                    providers.push(Box::new(provider));
                }
            }
        }
        
        // Always have a heuristic fallback
        let fallback_provider = Box::new(HeuristicProvider::new());
        
        info!("ProviderRouter initialized with {} providers", providers.len());
        
        Ok(Self {
            providers,
            policy,
            fallback_provider,
            metrics: Arc::new(RwLock::new(RouterMetrics::default())),
        })
    }

    pub async fn select_provider(&self, task_type: &str) -> Result<&dyn Provider> {
        let start_time = Instant::now();
        
        debug!("Selecting provider for task: {}", task_type);
        
        // Check health of all providers
        let mut provider_scores = Vec::new();
        
        for (idx, provider) in self.providers.iter().enumerate() {
            let health = provider.health().await?;
            let metrics = provider.metrics().await;
            let score = self.calculate_provider_score(&health, &metrics, task_type).await;
            
            provider_scores.push((idx, score, provider.name()));
            debug!("Provider {} score: {:.2}", provider.name(), score);
        }
        
        // Sort by score (highest first)
        provider_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Select best provider that meets policy requirements
        for (idx, score, name) in provider_scores {
            if score >= self.policy.quality_threshold {
                // Record selection
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.total_requests += 1;
                    *metrics.provider_selections.entry(name.to_string()).or_insert(0) += 1;
                    metrics.routing_latency_ms = start_time.elapsed().as_millis() as f64;
                }
                
                info!("Selected provider: {} (score: {:.2})", name, score);
                return Ok(self.providers[idx].as_ref());
            }
        }
        
        // If no provider meets threshold, use fallback if enabled
        if self.policy.fallback_enabled {
            warn!("No suitable provider found, using fallback");
            {
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
                metrics.fallback_usage += 1;
                metrics.routing_latency_ms = start_time.elapsed().as_millis() as f64;
            }
            return Ok(self.fallback_provider.as_ref());
        }
        
        Err(anyhow!("No suitable provider available"))
    }

    async fn calculate_provider_score(&self, health: &ProviderHealth, metrics: &ProviderMetrics, _task_type: &str) -> f64 {
        let mut score = 0.0;
        
        // Availability check (required)
        if !health.is_available {
            return 0.0;
        }
        
        // Base score for availability
        score += 30.0;
        
        // Model loaded bonus
        if health.model_loaded {
            score += 20.0;
        }
        
        // Latency score (lower is better)
        if let Some(latency) = health.latency_ms {
            let latency_score = if latency <= self.policy.max_latency_ms {
                20.0 * (1.0 - (latency as f64 / self.policy.max_latency_ms as f64))
            } else {
                0.0
            };
            score += latency_score;
        }
        
        // Success rate score
        let success_rate = metrics.success_rate();
        if success_rate >= self.policy.min_success_rate {
            score += 20.0 * success_rate;
        }
        
        // Recent performance bonus
        if let Some(last_success) = metrics.last_success {
            let time_since = chrono::Utc::now().signed_duration_since(last_success);
            if time_since.num_minutes() < 5 {
                score += 10.0;
            }
        }
        
        // Local provider preference
        if self.policy.prefer_local && health.latency_ms.unwrap_or(0) < 100 {
            score += 10.0;
        }
        
        // Normalize to 0-1 scale
        score / 100.0
    }

    pub async fn get_router_metrics(&self) -> RouterMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    pub async fn health_check_all(&self) -> Vec<(String, ProviderHealth)> {
        let mut results = Vec::new();
        
        for provider in &self.providers {
            match provider.health().await {
                Ok(health) => results.push((provider.name().to_string(), health)),
                Err(e) => {
                    results.push((provider.name().to_string(), ProviderHealth {
                        is_available: false,
                        latency_ms: None,
                        error_message: Some(e.to_string()),
                        model_loaded: false,
                    }));
                }
            }
        }
        
        results
    }
}

#[async_trait::async_trait]
impl Provider for ProviderRouter {
    /// Enhanced complete method with health gating and timeout handling
    async fn complete(&self, prompt: &str, context: Option<&str>) -> Result<Vec<String>> {
        let start_time = Instant::now();
        
        // Record HTTP metrics
        let metrics = get_metrics();
        metrics.provider_requests_total
            .with_label_values(&["router", "complete"])
            .inc();

        // Select best provider with health gating
        let provider = self.select_provider_with_health_check("completion").await?;
        let provider_name = provider.name().to_string();

        debug!("Using provider {} for completion", provider_name);

        // Attempt completion with timeout
        let completion_future = provider.complete(prompt, context);
        let timeout_duration = Duration::from_millis(self.policy.max_latency_ms);

        match tokio::time::timeout(timeout_duration, completion_future).await {
            Ok(Ok(suggestions)) => {
                let latency_ms = start_time.elapsed().as_millis() as f64;
                
                // Record successful metrics
                metrics.provider_request_duration_ms
                    .with_label_values(&[&provider_name, "complete"])
                    .observe(latency_ms);

                info!("Completion successful with provider {} in {}ms", provider_name, latency_ms as u64);
                Ok(suggestions)
            }
            Ok(Err(e)) => {
                warn!("Provider {} failed for completion: {}", provider_name, e);
                self.handle_provider_failure(&provider_name, "complete", start_time).await
            }
            Err(_) => {
                warn!("Provider {} timed out after {}ms", provider_name, self.policy.max_latency_ms);
                self.handle_provider_failure(&provider_name, "complete", start_time).await
            }
        }
    }

    /// Enhanced analyze method with health gating and timeout handling
    async fn analyze(&self, code: &str, language: &str) -> Result<Value> {
        let start_time = Instant::now();
        
        // Record HTTP metrics
        let metrics = get_metrics();
        metrics.provider_requests_total
            .with_label_values(&["router", "analyze"])
            .inc();

        // Select best provider with health gating
        let provider = self.select_provider_with_health_check("analysis").await?;
        let provider_name = provider.name().to_string();

        debug!("Using provider {} for analysis", provider_name);

        // Attempt analysis with timeout
        let analysis_future = provider.analyze(code, language);
        let timeout_duration = Duration::from_millis(self.policy.max_latency_ms);

        match tokio::time::timeout(timeout_duration, analysis_future).await {
            Ok(Ok(analysis)) => {
                let latency_ms = start_time.elapsed().as_millis() as f64;
                
                // Record successful metrics
                metrics.provider_request_duration_ms
                    .with_label_values(&[&provider_name, "analyze"])
                    .observe(latency_ms);

                info!("Analysis successful with provider {} in {}ms", provider_name, latency_ms as u64);
                Ok(analysis)
            }
            Ok(Err(e)) => {
                warn!("Provider {} failed for analysis: {}", provider_name, e);
                
                if self.policy.fallback_enabled {
                    warn!("Falling back to heuristic provider for analysis");
                    
                    match self.fallback_provider.analyze(code, language).await {
                        Ok(analysis) => {
                            info!("Fallback analysis successful");
                            Ok(analysis)
                        }
                        Err(fallback_error) => {
                            error!("Fallback provider also failed for analysis: {}", fallback_error);
                            Err(anyhow!("All providers failed for analysis. Last error: {}", fallback_error))
                        }
                    }
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                warn!("Provider {} timed out for analysis after {}ms", provider_name, self.policy.max_latency_ms);
                
                if self.policy.fallback_enabled {
                    match self.fallback_provider.analyze(code, language).await {
                        Ok(analysis) => Ok(analysis),
                        Err(e) => Err(anyhow!("Analysis timed out and fallback failed: {}", e))
                    }
                } else {
                    Err(anyhow!("Analysis timed out for provider {}", provider_name))
                }
            }
        }
    }

    async fn health(&self) -> Result<ProviderHealth> {
        // Return aggregated health status
        let health_checks = self.health_check_all().await;
        
        let available_count = health_checks.iter()
            .filter(|(_, health)| health.is_available)
            .count();
        
        let total_count = health_checks.len();
        let is_available = available_count > 0 || self.policy.fallback_enabled;
        
        let avg_latency = health_checks.iter()
            .filter_map(|(_, health)| health.latency_ms)
            .collect::<Vec<_>>();
        
        let latency_ms = if !avg_latency.is_empty() {
            Some(avg_latency.iter().sum::<u64>() / avg_latency.len() as u64)
        } else {
            None
        };

        Ok(ProviderHealth {
            is_available,
            latency_ms,
            error_message: if is_available { None } else { Some("No providers available".to_string()) },
            model_loaded: available_count > 0,
        })
    }

    async fn metrics(&self) -> ProviderMetrics {
        // Aggregate metrics from all providers
        let mut total_metrics = ProviderMetrics::default();
        
        for provider in &self.providers {
            let metrics = provider.metrics().await;
            total_metrics.total_requests += metrics.total_requests;
            total_metrics.successful_requests += metrics.successful_requests;
            total_metrics.failed_requests += metrics.failed_requests;
            
            if metrics.last_success.is_some() && 
               (total_metrics.last_success.is_none() || 
                metrics.last_success > total_metrics.last_success) {
                total_metrics.last_success = metrics.last_success;
            }
            
            if metrics.last_failure.is_some() && 
               (total_metrics.last_failure.is_none() || 
                metrics.last_failure > total_metrics.last_failure) {
                total_metrics.last_failure = metrics.last_failure;
            }
        }
        
        // Recalculate average latency
        if total_metrics.total_requests > 0 {
            let mut total_latency = 0.0;
            let mut request_count = 0;
            
            for provider in &self.providers {
                let metrics = provider.metrics().await;
                if metrics.total_requests > 0 {
                    total_latency += metrics.average_latency_ms * metrics.total_requests as f64;
                    request_count += metrics.total_requests;
                }
            }
            
            if request_count > 0 {
                total_metrics.average_latency_ms = total_latency / request_count as f64;
            }
        }
        
        total_metrics
    }

    fn name(&self) -> &'static str {
        "router"
    }

    fn priority(&self) -> u8 {
        255 // Highest priority as it coordinates others
    }
}