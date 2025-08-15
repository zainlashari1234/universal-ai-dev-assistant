use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct AnthropicProvider {
    client: Client,
    config: ProviderConfig,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    async fn make_request(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("Anthropic API key not configured".to_string()))?;

        let model = request.model.as_ref()
            .unwrap_or(&"claude-3-haiku-20240307".to_string());

        let mut messages = Vec::new();
        
        messages.push(json!({
            "role": "user",
            "content": request.prompt
        }));

        let mut payload = json!({
            "model": model,
            "max_tokens": request.max_tokens.unwrap_or(1000),
            "messages": messages
        });

        if let Some(system_prompt) = &request.system_prompt {
            payload["system"] = json!(system_prompt);
        }

        if let Some(temperature) = request.temperature {
            payload["temperature"] = json!(temperature);
        }

        if let Some(top_p) = request.top_p {
            payload["top_p"] = json!(top_p);
        }

        let response = self.client
            .post(&format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            return Err(match status.as_u16() {
                401 => ProviderError::AuthError(error_text),
                429 => ProviderError::RateLimitError(error_text),
                404 => ProviderError::ModelNotFound(error_text),
                _ => ProviderError::ApiError(format!("HTTP {}: {}", status, error_text)),
            });
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProviderError::ApiError(format!("Failed to parse response: {}", e)))?;

        let content = response_json["content"].as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["text"].as_str())
            .unwrap_or("")
            .to_string();

        let choices = vec![super::traits::Choice {
            index: 0,
            text: content,
            finish_reason: response_json["stop_reason"].as_str().map(|s| s.to_string()),
            logprobs: None,
            tool_calls: None,
        }];

        let usage = response_json.get("usage").map(|u| super::traits::Usage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: (u["input_tokens"].as_u64().unwrap_or(0) + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
            cost_usd: None,
        });

        Ok(CompletionResponse {
            id: response_json["id"].as_str().unwrap_or("unknown").to_string(),
            choices,
            usage,
            model: model.clone(),
            provider: "anthropic".to_string(),
            created_at: chrono::Utc::now(),
            metadata: None,
        })
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let start = Instant::now();
        
        // Simple health check - try to list models or make a minimal request
        let test_request = CompletionRequest::new("Hello".to_string())
            .with_max_tokens(1);
        
        match self.make_request(test_request).await {
            Ok(_) => {
                Ok(HealthCheck {
                    is_available: true,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    supported_models: self.config.models.clone(),
                    rate_limit_remaining: None,
                    error_message: None,
                })
            }
            Err(e) => {
                Ok(HealthCheck {
                    is_available: false,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    supported_models: vec![],
                    rate_limit_remaining: None,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        // Return known Claude models
        Ok(vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-3-opus-20240229".to_string(),
        ])
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        info!("Anthropic completion request for model: {:?}", request.model);
        self.make_request(request).await
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        Err(ProviderError::ApiError("Streaming not yet implemented for Anthropic".to_string()))
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let system_prompt = "You are Claude, an AI assistant created by Anthropic. You are an expert at code analysis.".to_string();
        
        let completion_request = CompletionRequest::new(format!(
            "Analyze this {} code for potential issues:\n\n```{}\n{}\n```",
            request.language, request.language, request.code
        ))
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

        let response = self.complete(completion_request).await?;
        
        Ok(AnalysisResponse {
            analysis_type: request.analysis_type,
            findings: vec![],
            summary: response.choices.first().map(|c| c.text.clone()).unwrap_or_default(),
            confidence_score: 0.9,
            suggestions: vec![],
        })
    }

    async fn generate_documentation(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate comprehensive documentation for this {} code:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn generate_tests(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate thorough unit tests for this {} code:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn explain_code(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Explain what this {} code does in detail:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.4);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Refactor this {} code according to these instructions: {}\n\n```{}\n{}\n```",
            language, instructions, language, code
        )).with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn translate_code(&self, code: &str, from_language: &str, to_language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Translate this {} code to {}:\n\n```{}\n{}\n```",
            from_language, to_language, from_language, code
        )).with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    fn get_config(&self) -> &ProviderConfig {
        &self.config
    }

    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64> {
        let tokens = request.prompt.len() / 4 + request.max_tokens.unwrap_or(1000) as usize;
        let model = request.model.as_ref().unwrap_or(&"claude-3-haiku-20240307".to_string());
        
        let cost_per_1k_tokens = match model.as_str() {
            "claude-3-5-sonnet-20241022" => 0.015,
            "claude-3-opus-20240229" => 0.075,
            "claude-3-haiku-20240307" => 0.0025,
            _ => 0.0025,
        };
        
        Some((tokens as f64 / 1000.0) * cost_per_1k_tokens)
    }
}