use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::traits::{Provider, ProviderHealth, ProviderMetrics};
use super::ProviderConfig;

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaOptions {
    temperature: f64,
    top_p: f64,
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
    model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaListResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaModel {
    name: String,
    size: u64,
    digest: String,
}

pub struct OllamaProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    metrics: Arc<RwLock<ProviderMetrics>>,
}

impl OllamaProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
        }
    }

    async fn generate_with_ollama(&self, prompt: &str) -> Result<String> {
        let endpoint = self.config.endpoint.as_ref()
            .ok_or_else(|| anyhow!("Ollama endpoint not configured"))?;
        
        let model = self.config.model.as_ref()
            .ok_or_else(|| anyhow!("Ollama model not configured"))?;

        let request = OllamaRequest {
            model: model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: 0.1,
                top_p: 0.9,
                max_tokens: Some(512),
            }),
        };

        let url = format!("{}/api/generate", endpoint);
        debug!("Sending request to Ollama: {}", url);

        let start_time = Instant::now();
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama request failed with status {}: {}", status, error_text));
        }

        let ollama_response: OllamaResponse = response.json().await?;
        let latency = start_time.elapsed().as_millis() as u64;

        // Record successful request
        {
            let mut metrics = self.metrics.write().await;
            metrics.record_success(latency);
        }

        debug!("Ollama response received in {}ms", latency);
        Ok(ollama_response.response)
    }

    async fn check_model_availability(&self) -> Result<bool> {
        let endpoint = self.config.endpoint.as_ref()
            .ok_or_else(|| anyhow!("Ollama endpoint not configured"))?;

        let model = self.config.model.as_ref()
            .ok_or_else(|| anyhow!("Ollama model not configured"))?;

        let url = format!("{}/api/tags", endpoint);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let list_response: OllamaListResponse = response.json().await?;
                    let model_exists = list_response.models.iter()
                        .any(|m| m.name.starts_with(model));
                    Ok(model_exists)
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }
}

#[async_trait::async_trait]
impl Provider for OllamaProvider {
    async fn complete(&self, prompt: &str, context: Option<&str>) -> Result<Vec<String>> {
        let full_prompt = if let Some(ctx) = context {
            format!("{}\n\nContext:\n{}\n\nComplete the code:", ctx, prompt)
        } else {
            format!("Complete the following code:\n{}", prompt)
        };

        match self.generate_with_ollama(&full_prompt).await {
            Ok(response) => {
                // Parse response and extract code completions
                let completions = self.parse_completions(&response);
                Ok(completions)
            }
            Err(e) => {
                // Record failure
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.record_failure();
                }
                error!("Ollama completion failed: {}", e);
                Err(e)
            }
        }
    }

    async fn analyze(&self, code: &str, language: &str) -> Result<Value> {
        let prompt = format!(
            "Analyze the following {} code for issues, improvements, and suggestions. \
            Return your analysis in JSON format with fields: issues, suggestions, complexity, security_concerns.\n\n\
            Code:\n{}", 
            language, code
        );

        match self.generate_with_ollama(&prompt).await {
            Ok(response) => {
                // Try to parse as JSON, fall back to structured text
                match serde_json::from_str::<Value>(&response) {
                    Ok(json) => Ok(json),
                    Err(_) => {
                        // Create structured response from text
                        Ok(serde_json::json!({
                            "analysis": response,
                            "provider": "ollama",
                            "formatted": false
                        }))
                    }
                }
            }
            Err(e) => {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.record_failure();
                }
                error!("Ollama analysis failed: {}", e);
                Err(e)
            }
        }
    }

    async fn health(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();
        
        match self.check_model_availability().await {
            Ok(model_loaded) => {
                let latency = start_time.elapsed().as_millis() as u64;
                Ok(ProviderHealth {
                    is_available: true,
                    latency_ms: Some(latency),
                    error_message: None,
                    model_loaded,
                })
            }
            Err(e) => {
                warn!("Ollama health check failed: {}", e);
                Ok(ProviderHealth {
                    is_available: false,
                    latency_ms: None,
                    error_message: Some(e.to_string()),
                    model_loaded: false,
                })
            }
        }
    }

    async fn metrics(&self) -> ProviderMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    fn name(&self) -> &'static str {
        "ollama"
    }

    fn priority(&self) -> u8 {
        100 // High priority for local model
    }
}

impl OllamaProvider {
    fn parse_completions(&self, response: &str) -> Vec<String> {
        // Split response into potential completions
        let mut completions = Vec::new();
        
        // Clean up the response
        let cleaned = response.trim()
            .replace("```", "")
            .replace("```rust", "")
            .replace("```python", "")
            .replace("```javascript", "")
            .replace("```typescript", "");
        
        // Split by common delimiters and filter
        for line in cleaned.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.len() > 2 && trimmed.len() < 200 {
                completions.push(trimmed.to_string());
            }
        }
        
        // If no good completions found, return the whole response
        if completions.is_empty() && !cleaned.trim().is_empty() {
            completions.push(cleaned.trim().to_string());
        }
        
        // Limit to top 3 completions
        completions.truncate(3);
        completions
    }
}