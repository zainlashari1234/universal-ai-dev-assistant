use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct OllamaProvider {
    client: Client,
    config: ProviderConfig,
}

impl OllamaProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    async fn make_request(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let model = request.model.as_ref()
            .or_else(|| self.config.models.first())
            .unwrap_or(&"qwen2.5-coder:7b".to_string());

        let prompt = if let Some(system_prompt) = &request.system_prompt {
            format!("System: {}\n\nUser: {}", system_prompt, request.prompt)
        } else {
            request.prompt.clone()
        };

        let payload = json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
                "top_p": request.top_p.unwrap_or(0.9),
                "num_predict": request.max_tokens.unwrap_or(1000)
            }
        });

        debug!("Ollama request: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        let response = self.client
            .post(&format!("{}/api/generate", self.config.base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("Ollama API error: {} - {}", status, error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProviderError::ApiError(format!("Failed to parse Ollama response: {}", e)))?;

        let text = response_json["response"].as_str()
            .ok_or_else(|| ProviderError::ApiError("No response text from Ollama".to_string()))?
            .to_string();

        let choices = vec![super::traits::Choice {
            index: 0,
            text,
            finish_reason: Some("stop".to_string()),
            logprobs: None,
            tool_calls: None,
        }];

        // Ollama doesn't provide token usage, so we estimate
        let prompt_tokens = request.prompt.len() / 4;
        let completion_tokens = choices[0].text.len() / 4;
        
        let usage = Some(super::traits::Usage {
            prompt_tokens: prompt_tokens as u32,
            completion_tokens: completion_tokens as u32,
            total_tokens: (prompt_tokens + completion_tokens) as u32,
            cost_usd: Some(0.0), // Ollama is free
        });

        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            choices,
            usage,
            model: model.clone(),
            provider: "ollama".to_string(),
            created_at: chrono::Utc::now(),
            metadata: None,
        })
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let start = Instant::now();
        
        // Check if Ollama is running by hitting the tags endpoint
        match self.client
            .get(&format!("{}/api/tags", self.config.base_url))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let models = self.list_models().await.unwrap_or_default();
                Ok(HealthCheck {
                    is_available: true,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    supported_models: models,
                    rate_limit_remaining: None,
                    error_message: None,
                })
            }
            Ok(response) => {
                Ok(HealthCheck {
                    is_available: false,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    supported_models: vec![],
                    rate_limit_remaining: None,
                    error_message: Some(format!("Ollama returned status: {}", response.status())),
                })
            }
            Err(e) => {
                Ok(HealthCheck {
                    is_available: false,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    supported_models: vec![],
                    rate_limit_remaining: None,
                    error_message: Some(format!("Failed to connect to Ollama: {}", e)),
                })
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.client
            .get(&format!("{}/api/tags", self.config.base_url))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError("Failed to list Ollama models".to_string()));
        }

        let tags_response: serde_json::Value = response.json().await
            .map_err(|e| ProviderError::ApiError(format!("Failed to parse Ollama tags: {}", e)))?;

        let models = tags_response["models"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|model| model["name"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(models)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        info!("Ollama completion request for model: {:?}", request.model);
        self.make_request(request).await
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        Err(ProviderError::ApiError("Streaming not yet implemented for Ollama".to_string()))
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let system_prompt = "You are a code analysis expert. Analyze the code and provide detailed feedback.".to_string();
        
        let completion_request = CompletionRequest::new(format!(
            "Analyze this {} code for issues and improvements:\n\n```{}\n{}\n```",
            request.language, request.language, request.code
        ))
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

        let response = self.complete(completion_request).await?;
        
        Ok(AnalysisResponse {
            analysis_type: request.analysis_type,
            findings: vec![],
            summary: response.choices.first().map(|c| c.text.clone()).unwrap_or_default(),
            confidence_score: 0.7,
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
            "Explain what this {} code does:\n\n```{}\n{}\n```",
            language, language, code
        )).with_temperature(0.4);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Refactor this {} code according to: {}\n\n```{}\n{}\n```",
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
        Some(0.0) // Ollama is free
    }
}