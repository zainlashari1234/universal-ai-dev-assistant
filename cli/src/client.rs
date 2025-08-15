use anyhow::Result;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Client {
    http_client: HttpClient,
    base_url: String,
    config: Config,
}

#[derive(Debug, Serialize)]
pub struct CompletionRequest {
    pub prompt: String,
    pub language: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub text: String,
    pub model: String,
    pub provider: String,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct AnalysisRequest {
    pub code: String,
    pub language: String,
    pub analysis_type: String,
    pub context: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisResponse {
    pub analysis_type: String,
    pub findings: Vec<Finding>,
    pub summary: String,
    pub confidence_score: f32,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Deserialize)]
pub struct Finding {
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub code_snippet: Option<String>,
    pub fix_suggestion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Suggestion {
    pub title: String,
    pub description: String,
    pub code_example: Option<String>,
    pub impact: String,
    pub effort: String,
}

#[derive(Debug, Serialize)]
pub struct CodeActionRequest {
    pub code: String,
    pub language: String,
    pub action: String,
    pub instructions: Option<String>,
    pub target_language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodeActionResponse {
    pub action: String,
    pub result: String,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub providers: HashMap<String, ProviderHealth>,
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderHealth {
    pub provider_type: String,
    pub is_available: bool,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub models_available: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProvidersResponse {
    pub available_providers: Vec<String>,
    pub provider_metrics: HashMap<String, ProviderMetrics>,
    pub recommended_provider: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderMetrics {
    pub provider_type: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub tokens_processed: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
    pub total_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
    pub context_length: Option<u32>,
    pub cost_per_1k_tokens: Option<f64>,
}

impl Client {
    pub fn new(base_url: &str, config: &Config) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(config.server.timeout_seconds))
            .build()?;

        Ok(Self {
            http_client,
            base_url: base_url.to_string(),
            config: config.clone(),
        })
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Health check failed: {}", response.status()));
        }
        
        let health: HealthResponse = response.json().await?;
        Ok(health)
    }

    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/api/v1/complete", self.base_url);
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Completion failed: {}", error_text));
        }
        
        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }

    pub async fn analyze(&self, request: AnalysisRequest) -> Result<AnalysisResponse> {
        let url = format!("{}/api/v1/analyze", self.base_url);
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Analysis failed: {}", error_text));
        }
        
        let analysis: AnalysisResponse = response.json().await?;
        Ok(analysis)
    }

    pub async fn code_action(&self, request: CodeActionRequest) -> Result<CodeActionResponse> {
        let url = format!("{}/api/v1/code/action", self.base_url);
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Code action failed: {}", error_text));
        }
        
        let action: CodeActionResponse = response.json().await?;
        Ok(action)
    }

    pub async fn providers(&self) -> Result<ProvidersResponse> {
        let url = format!("{}/api/v1/providers", self.base_url);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get providers: {}", response.status()));
        }
        
        let providers: ProvidersResponse = response.json().await?;
        Ok(providers)
    }

    pub async fn models(&self) -> Result<ModelsResponse> {
        let url = format!("{}/api/v1/models", self.base_url);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get models: {}", response.status()));
        }
        
        let models: ModelsResponse = response.json().await?;
        Ok(models)
    }

    pub async fn metrics(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/metrics", self.base_url);
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get metrics: {}", response.status()));
        }
        
        let metrics: serde_json::Value = response.json().await?;
        Ok(metrics)
    }
}