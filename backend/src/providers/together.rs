use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use futures_util::StreamExt;
use crate::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct TogetherProvider {
    client: Client,
    config: ProviderConfig,
}

impl TogetherProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    async fn make_request(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("Together AI API key not configured".to_string()))?;

        let model = request.model.as_ref()
            .unwrap_or(&"meta-llama/Llama-2-70b-chat-hf".to_string());

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
            "repetition_penalty": 1.0,
            "stream": false
        });

        debug!("Together AI request: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

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

        debug!("Together AI response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

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
            cost_usd: Some(0.0008), // Together AI pricing
        });

        Ok(CompletionResponse {
            id: response_json["id"].as_str().unwrap_or("unknown").to_string(),
            choices: parsed_choices,
            usage,
            model: model.clone(),
            provider: "together".to_string(),
            created_at: chrono::Utc::now(),
            metadata: None,
        })
    }
}

#[async_trait]
impl AIProvider for TogetherProvider {
    fn name(&self) -> &str {
        "together"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let start = Instant::now();
        
        if self.config.api_key.is_none() {
            return Ok(HealthCheck {
                is_available: false,
                response_time_ms: start.elapsed().as_millis() as u64,
                supported_models: vec![],
                rate_limit_remaining: None,
                error_message: Some("Together AI API key not configured".to_string()),
            });
        }

        // Simple health check
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
            "meta-llama/Llama-2-70b-chat-hf".to_string(),
            "meta-llama/Llama-2-13b-chat-hf".to_string(),
            "meta-llama/CodeLlama-34b-Instruct-hf".to_string(),
            "mistralai/Mixtral-8x7B-Instruct-v0.1".to_string(),
            "NousResearch/Nous-Hermes-2-Mixtral-8x7B-DPO".to_string(),
            "teknium/OpenHermes-2.5-Mistral-7B".to_string(),
        ])
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        info!("Together AI completion request for model: {:?}", request.model);
        self.make_request(request).await
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("Together AI API key not configured".to_string()))?
            .clone();
        
        let client = self.client.clone();
        let base_url = self.config.base_url.clone();
        
        tokio::spawn(async move {
            let model = request.model.as_ref()
                .unwrap_or(&"meta-llama/Llama-2-70b-chat-hf".to_string());

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
                "stream": true
            });

            match client
                .post(&format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        // Parse SSE streaming response
                        let mut stream = response.bytes_stream();
                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    for line in text.lines() {
                                        if line.starts_with("data: ") {
                                            let data = &line[6..];
                                            if data == "[DONE]" {
                                                break;
                                            }
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                                    if tx.send(Ok(content.to_string())).await.is_err() {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(ProviderError::NetworkError(e.to_string()))).await;
                                    break;
                                }
                            }
                        }
                    } else {
                        let _ = tx.send(Err(ProviderError::ApiError("Streaming failed".to_string()))).await;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(ProviderError::NetworkError(e.to_string()))).await;
                }
            }
        });

        Ok(rx)
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let system_prompt = "You are an expert code analyzer. Provide detailed analysis with specific findings.".to_string();
        
        let completion_request = CompletionRequest::new(format!(
            "Analyze this {} code for {}:\n\n```{}\n{}\n```",
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
            confidence_score: 0.8,
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
            "Generate comprehensive unit tests for this {} code:\n\n```{}\n{}\n```",
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
        let model = request.model.as_ref().unwrap_or(&"meta-llama/Llama-2-70b-chat-hf".to_string());
        
        let cost_per_1k_tokens = match model.as_str() {
            "meta-llama/Llama-2-70b-chat-hf" => 0.0009,
            "meta-llama/CodeLlama-34b-Instruct-hf" => 0.0008,
            "mistralai/Mixtral-8x7B-Instruct-v0.1" => 0.0006,
            _ => 0.0008,
        };
        
        Some((tokens as f64 / 1000.0) * cost_per_1k_tokens)
    }
}