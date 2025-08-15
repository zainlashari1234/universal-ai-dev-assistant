use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct GoogleProvider {
    client: Client,
    config: ProviderConfig,
}

impl GoogleProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    async fn make_request(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("Google API key not configured".to_string()))?;

        let model = request.model.as_ref()
            .unwrap_or(&"gemini-pro".to_string());

        let mut parts = vec![json!({
            "text": request.prompt
        })];

        if let Some(system_prompt) = &request.system_prompt {
            parts.insert(0, json!({
                "text": format!("System: {}", system_prompt)
            }));
        }

        let payload = json!({
            "contents": [{
                "parts": parts
            }],
            "generationConfig": {
                "temperature": request.temperature.unwrap_or(0.7),
                "topP": request.top_p.unwrap_or(0.9),
                "maxOutputTokens": request.max_tokens.unwrap_or(1000),
                "stopSequences": request.stop_sequences.unwrap_or_default()
            }
        });

        debug!("Google Gemini request: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        let response = self.client
            .post(&format!("{}/models/{}:generateContent?key={}", self.config.base_url, model, api_key))
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

        debug!("Google Gemini response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        let text = response_json["candidates"]
            .as_array()
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part["text"].as_str())
            .unwrap_or("")
            .to_string();

        let choices = vec![super::traits::Choice {
            index: 0,
            text,
            finish_reason: response_json["candidates"]
                .as_array()
                .and_then(|candidates| candidates.first())
                .and_then(|candidate| candidate["finishReason"].as_str())
                .map(|s| s.to_string()),
            logprobs: None,
            tool_calls: None,
        }];

        // Google doesn't provide token usage in the same format, estimate
        let prompt_tokens = request.prompt.len() / 4;
        let completion_tokens = choices[0].text.len() / 4;
        
        let usage = Some(super::traits::Usage {
            prompt_tokens: prompt_tokens as u32,
            completion_tokens: completion_tokens as u32,
            total_tokens: (prompt_tokens + completion_tokens) as u32,
            cost_usd: Some(0.001), // Approximate cost
        });

        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            choices,
            usage,
            model: model.clone(),
            provider: "google".to_string(),
            created_at: chrono::Utc::now(),
            metadata: None,
        })
    }
}

#[async_trait]
impl AIProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let start = Instant::now();
        
        if self.config.api_key.is_none() {
            return Ok(HealthCheck {
                is_available: false,
                response_time_ms: start.elapsed().as_millis() as u64,
                supported_models: vec![],
                rate_limit_remaining: None,
                error_message: Some("Google API key not configured".to_string()),
            });
        }

        // Simple health check with minimal request
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
        Ok(vec![
            "gemini-pro".to_string(),
            "gemini-pro-vision".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ])
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        info!("Google Gemini completion request for model: {:?}", request.model);
        self.make_request(request).await
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        Err(ProviderError::ApiError("Streaming not yet implemented for Google".to_string()))
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let system_prompt = "You are an expert code analyzer. Analyze the code thoroughly and provide detailed insights.".to_string();
        
        let completion_request = CompletionRequest::new(format!(
            "Analyze this {} code for {}:\n\n```{}\n{}\n```\n\nProvide detailed analysis with specific findings.",
            request.language,
            match request.analysis_type {
                super::traits::AnalysisType::Security => "security vulnerabilities",
                super::traits::AnalysisType::Performance => "performance issues",
                super::traits::AnalysisType::Quality => "code quality",
                super::traits::AnalysisType::Bugs => "potential bugs",
                super::traits::AnalysisType::Suggestions => "improvement suggestions",
                super::traits::AnalysisType::Documentation => "documentation needs",
                super::traits::AnalysisType::Testing => "testing requirements",
                super::traits::AnalysisType::Refactoring => "refactoring opportunities",
            },
            request.language,
            request.code
        ))
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

        let response = self.complete(completion_request).await?;
        
        Ok(AnalysisResponse {
            analysis_type: request.analysis_type,
            findings: vec![],
            summary: response.choices.first().map(|c| c.text.clone()).unwrap_or_default(),
            confidence_score: 0.85,
            suggestions: vec![],
        })
    }

    async fn generate_documentation(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate comprehensive documentation for this {} code:\n\n```{}\n{}\n```\n\nInclude function descriptions, parameters, return values, and usage examples.",
            language, language, code
        ))
        .with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn generate_tests(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate comprehensive unit tests for this {} code:\n\n```{}\n{}\n```\n\nInclude edge cases, error conditions, and proper test structure.",
            language, language, code
        ))
        .with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn explain_code(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Explain what this {} code does in detail:\n\n```{}\n{}\n```\n\nProvide a clear, step-by-step explanation.",
            language, language, code
        ))
        .with_temperature(0.4);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Refactor this {} code according to these instructions: {}\n\nOriginal code:\n```{}\n{}\n```\n\nProvide the refactored code with explanations.",
            language, instructions, language, code
        ))
        .with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn translate_code(&self, code: &str, from_language: &str, to_language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Translate this {} code to {}:\n\n```{}\n{}\n```\n\nMaintain functionality while following {} best practices.",
            from_language, to_language, from_language, code, to_language
        ))
        .with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    fn get_config(&self) -> &ProviderConfig {
        &self.config
    }

    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64> {
        let tokens = request.prompt.len() / 4 + request.max_tokens.unwrap_or(1000) as usize;
        let model = request.model.as_ref().unwrap_or(&"gemini-pro".to_string());
        
        let cost_per_1k_tokens = match model.as_str() {
            "gemini-1.5-pro" => 0.0035,
            "gemini-1.5-flash" => 0.00035,
            "gemini-pro" => 0.0005,
            "gemini-pro-vision" => 0.0025,
            _ => 0.0005,
        };
        
        Some((tokens as f64 / 1000.0) * cost_per_1k_tokens)
    }
}