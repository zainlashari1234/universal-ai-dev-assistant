use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use super::{openrouter::OpenRouterProvider, openai::OpenAIProvider, ollama::OllamaProvider};
use crate::config::Config;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct ProviderRouter {
    providers: HashMap<String, Box<dyn AIProvider>>,
    config: Arc<Config>,
    metrics: Arc<RwLock<HashMap<String, ProviderMetrics>>>,
    health_cache: Arc<RwLock<HashMap<String, (HealthCheck, std::time::Instant)>>>,
}

#[derive(Debug, Clone)]
pub struct ProviderMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_response_time_ms: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

impl ProviderMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_response_time_ms: 0,
            total_tokens: 0,
            total_cost_usd: 0.0,
        }
    }

    pub fn average_response_time_ms(&self) -> f64 {
        if self.total_requests > 0 {
            self.total_response_time_ms as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    Priority,        // Use highest priority available provider
    LoadBalance,     // Round-robin between available providers
    CostOptimized,   // Use cheapest available provider
    PerformanceOptimized, // Use fastest available provider
    Failover,        // Try providers in order until one succeeds
}

impl ProviderRouter {
    pub async fn new(config: Arc<Config>) -> Result<Self, ProviderError> {
        let mut providers: HashMap<String, Box<dyn AIProvider>> = HashMap::new();
        
        // Initialize OpenRouter if configured
        if config.providers.openrouter.enabled {
            match OpenRouterProvider::new(config.providers.openrouter.clone()) {
                Ok(provider) => {
                    providers.insert("openrouter".to_string(), Box::new(provider));
                    info!("OpenRouter provider initialized");
                }
                Err(e) => warn!("Failed to initialize OpenRouter provider: {}", e),
            }
        }

        // Initialize OpenAI if configured
        if config.providers.openai.enabled {
            match OpenAIProvider::new(config.providers.openai.clone()) {
                Ok(provider) => {
                    providers.insert("openai".to_string(), Box::new(provider));
                    info!("OpenAI provider initialized");
                }
                Err(e) => warn!("Failed to initialize OpenAI provider: {}", e),
            }
        }

        // Initialize Ollama (always available as fallback)
        match OllamaProvider::new(config.providers.ollama.clone()) {
            Ok(provider) => {
                providers.insert("ollama".to_string(), Box::new(provider));
                info!("Ollama provider initialized");
            }
            Err(e) => warn!("Failed to initialize Ollama provider: {}", e),
        }

        // TODO: Add other providers (Anthropic, Google, etc.)

        let metrics = Arc::new(RwLock::new(HashMap::new()));
        let health_cache = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            providers,
            config,
            metrics,
            health_cache,
        })
    }

    pub async fn get_available_providers(&self) -> Vec<String> {
        let mut available = Vec::new();
        
        for (name, provider) in &self.providers {
            match self.get_cached_health(name).await {
                Some(health) if health.is_available => available.push(name.clone()),
                _ => {
                    // Check health if not cached or cached result shows unavailable
                    if let Ok(health) = provider.health_check().await {
                        self.cache_health(name.clone(), health.clone()).await;
                        if health.is_available {
                            available.push(name.clone());
                        }
                    }
                }
            }
        }
        
        available
    }

    pub async fn select_provider(&self, strategy: RoutingStrategy, model_preference: Option<String>) -> Option<String> {
        let available_providers = self.get_available_providers().await;
        
        if available_providers.is_empty() {
            return None;
        }

        match strategy {
            RoutingStrategy::Priority => {
                self.select_by_priority(&available_providers).await
            }
            RoutingStrategy::LoadBalance => {
                self.select_by_load_balance(&available_providers).await
            }
            RoutingStrategy::CostOptimized => {
                self.select_by_cost(&available_providers).await
            }
            RoutingStrategy::PerformanceOptimized => {
                self.select_by_performance(&available_providers).await
            }
            RoutingStrategy::Failover => {
                self.select_by_priority(&available_providers).await // Same as priority for now
            }
        }
    }

    async fn select_by_priority(&self, available_providers: &[String]) -> Option<String> {
        let mut best_provider = None;
        let mut highest_priority = 0;

        for provider_name in available_providers {
            let priority = match provider_name.as_str() {
                "openrouter" => self.config.providers.openrouter.priority,
                "openai" => self.config.providers.openai.priority,
                "anthropic" => self.config.providers.anthropic.priority,
                "google" => self.config.providers.google.priority,
                "groq" => self.config.providers.groq.priority,
                "together" => self.config.providers.together.priority,
                "cohere" => self.config.providers.cohere.priority,
                "ollama" => self.config.providers.ollama.priority,
                _ => 1,
            };

            if priority > highest_priority {
                highest_priority = priority;
                best_provider = Some(provider_name.clone());
            }
        }

        best_provider
    }

    async fn select_by_load_balance(&self, available_providers: &[String]) -> Option<String> {
        let metrics = self.metrics.read().await;
        
        // Select provider with lowest request count
        let mut best_provider = None;
        let mut lowest_requests = u64::MAX;

        for provider_name in available_providers {
            let request_count = metrics
                .get(provider_name)
                .map(|m| m.total_requests)
                .unwrap_or(0);

            if request_count < lowest_requests {
                lowest_requests = request_count;
                best_provider = Some(provider_name.clone());
            }
        }

        best_provider.or_else(|| available_providers.first().cloned())
    }

    async fn select_by_cost(&self, available_providers: &[String]) -> Option<String> {
        // For now, prioritize free providers (Ollama) then cheapest
        if available_providers.contains(&"ollama".to_string()) {
            return Some("ollama".to_string());
        }

        // TODO: Implement actual cost comparison based on model pricing
        available_providers.first().cloned()
    }

    async fn select_by_performance(&self, available_providers: &[String]) -> Option<String> {
        let metrics = self.metrics.read().await;
        
        let mut best_provider = None;
        let mut best_response_time = f64::MAX;

        for provider_name in available_providers {
            let avg_response_time = metrics
                .get(provider_name)
                .map(|m| m.average_response_time_ms())
                .unwrap_or(1000.0); // Default to 1 second if no metrics

            if avg_response_time < best_response_time {
                best_response_time = avg_response_time;
                best_provider = Some(provider_name.clone());
            }
        }

        best_provider.or_else(|| available_providers.first().cloned())
    }

    async fn get_cached_health(&self, provider_name: &str) -> Option<HealthCheck> {
        let cache = self.health_cache.read().await;
        if let Some((health, timestamp)) = cache.get(provider_name) {
            // Cache for 5 minutes
            if timestamp.elapsed().as_secs() < 300 {
                return Some(health.clone());
            }
        }
        None
    }

    async fn cache_health(&self, provider_name: String, health: HealthCheck) {
        let mut cache = self.health_cache.write().await;
        cache.insert(provider_name, (health, std::time::Instant::now()));
    }

    async fn record_metrics(&self, provider_name: &str, success: bool, response_time_ms: u64, tokens: u32, cost: f64) {
        let mut metrics = self.metrics.write().await;
        let provider_metrics = metrics.entry(provider_name.to_string()).or_insert_with(ProviderMetrics::new);
        
        provider_metrics.total_requests += 1;
        provider_metrics.total_response_time_ms += response_time_ms;
        provider_metrics.total_tokens += tokens as u64;
        provider_metrics.total_cost_usd += cost;

        if success {
            provider_metrics.successful_requests += 1;
        } else {
            provider_metrics.failed_requests += 1;
        }
    }

    pub async fn get_metrics(&self) -> HashMap<String, ProviderMetrics> {
        self.metrics.read().await.clone()
    }

    pub async fn complete_with_fallback(&self, mut request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let strategy = RoutingStrategy::Failover;
        let available_providers = self.get_available_providers().await;
        
        if available_providers.is_empty() {
            return Err(ProviderError::Unavailable("No providers available".to_string()));
        }

        // Try providers in priority order
        let mut last_error = None;
        
        for provider_name in &available_providers {
            if let Some(provider) = self.providers.get(provider_name) {
                let start_time = std::time::Instant::now();
                
                match provider.complete(request.clone()).await {
                    Ok(response) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        let tokens = response.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0);
                        let cost = response.usage.as_ref().and_then(|u| u.cost_usd).unwrap_or(0.0);
                        
                        self.record_metrics(provider_name, true, response_time, tokens, cost).await;
                        
                        info!("Successful completion from provider: {}", provider_name);
                        return Ok(response);
                    }
                    Err(e) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        self.record_metrics(provider_name, false, response_time, 0, 0.0).await;
                        
                        warn!("Provider {} failed: {}", provider_name, e);
                        last_error = Some(e);
                        continue;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::Unavailable("All providers failed".to_string())))
    }
}

#[async_trait]
impl AIProvider for ProviderRouter {
    fn name(&self) -> &str {
        "router"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let available_providers = self.get_available_providers().await;
        
        Ok(HealthCheck {
            is_available: !available_providers.is_empty(),
            response_time_ms: 0,
            supported_models: vec![], // TODO: Aggregate from all providers
            rate_limit_remaining: None,
            error_message: if available_providers.is_empty() {
                Some("No providers available".to_string())
            } else {
                None
            },
        })
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let mut all_models = Vec::new();
        
        for (_, provider) in &self.providers {
            if let Ok(models) = provider.list_models().await {
                all_models.extend(models);
            }
        }
        
        all_models.sort();
        all_models.dedup();
        Ok(all_models)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        self.complete_with_fallback(request).await
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        Err(ProviderError::ApiError("Streaming not yet implemented for router".to_string()))
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let completion_request = CompletionRequest::new(format!("Analyze this code: {}", request.code));
        let response = self.complete(completion_request).await?;
        
        Ok(AnalysisResponse {
            analysis_type: request.analysis_type,
            findings: vec![],
            summary: response.choices.first().map(|c| c.text.clone()).unwrap_or_default(),
            confidence_score: 0.8,
            suggestions: vec![],
        })
    }

    async fn generate_documentation(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!("Generate documentation for this {} code:\n\n{}", language, code));
        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn generate_tests(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!("Generate tests for this {} code:\n\n{}", language, code));
        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn explain_code(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!("Explain this {} code:\n\n{}", language, code));
        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!("Refactor this {} code: {}\n\n{}", language, instructions, code));
        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn translate_code(&self, code: &str, from_language: &str, to_language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!("Translate this {} code to {}:\n\n{}", from_language, to_language, code));
        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    fn get_config(&self) -> &crate::config::ProviderConfig {
        &self.config.providers.openrouter // Return a default config
    }

    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64> {
        // Return estimate from the cheapest available provider
        Some(0.001) // Default estimate
    }
}