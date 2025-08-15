use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tracing::{info, warn};

pub struct GroqProvider {
    client: Client,
    config: ProviderConfig,
}

impl GroqProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    async fn make_request(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("Groq API key not configured".to_string()))?;

        let model = request.model.as_ref()
            .unwrap_or(&"llama-3.1-70b-versatile".to_string());

        let mut messages = Vec::new();
        
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        messages.push(json!({
            "role": "user",
            "content": request.prompt
        }));

        let payload = json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(1000),
            "temperature": request.temperature.unwrap_or(0.7),
            "top_p": request.top_p.unwrap_or(0.9),
            "stream": false
        });

        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
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

        let choices = response_json["choices"].as_array()
            .ok_or_else(|| ProviderError::ApiError("No choices in response".to_string()))?;

        let parsed_choices = choices.iter().enumerate().map(|(index, choice)| {
            super::traits::Choice {
                index: index as u32,
                text: choice["message"]["content"].as_str().unwrap_or("").to_string(),
                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                logprobs: None,
                tool_calls: None,
            }
        }).collect();

        let usage = response_json.get("usage").map(|u| super::traits::Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            cost_usd: Some(0.0), // Groq is currently free
        });

        Ok(CompletionResponse {
            id: response_json["id"].as_str().unwrap_or("unknown").to_string(),
            choices: parsed_choices,
            usage,
            model: model.clone(),
            provider: "groq".to_string(),
            created_at: chrono::Utc::now(),
            metadata: None,
        })
    }
}

#[async_trait]
impl AIProvider for GroqProvider {
    fn name(&self) -> &str {
        "groq"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let start = Instant::now();
        
        match self.list_models().await {
            Ok(models) => {
                Ok(HealthCheck {
                    is_available: true,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    supported_models: models,
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
        Ok(vec![
            "llama-3.1-70b-versatile".to_string(),
            "llama-3.1-8b-instant".to_string(),
            "mixtral-8x7b-32768".to_string(),
            "gemma-7b-it".to_string(),
        ])
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        info!("Groq completion request for model: {:?}", request.model);
        self.make_request(request).await
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        Err(ProviderError::ApiError("Streaming not yet implemented for Groq".to_string()))
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let system_prompt = "You are an expert code analyzer. Provide detailed analysis.".to_string();
        
        let completion_request = CompletionRequest::new(format!(
            "Analyze this {} code:\n\n```{}\n{}\n```",
            request.language, request.language, request.code
        ))
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

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
        let request = CompletionRequest::new(format!(
            "Generate documentation for this {} code:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn generate_tests(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate unit tests for this {} code:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn explain_code(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Explain this {} code:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.4);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Refactor this {} code: {}\n\n```{}\n{}\n```",
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

    fn estimate_cost(&self, _request: &CompletionRequest) -> Option<f64> {
        Some(0.0) // Groq is currently free
    }
}