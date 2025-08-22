use super::traits::{AIProvider, AnalysisRequest, AnalysisResponse, CompletionRequest, CompletionResponse, HealthCheck, ProviderError};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tracing::{debug, error, info, warn};

pub struct OpenRouterProvider {
    client: Client,
    config: ProviderConfig,
}

impl OpenRouterProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    async fn make_request(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("OpenRouter API key not configured".to_string()))?;

        let model = request.model.as_ref()
            .or_else(|| self.config.models.first())
            .ok_or_else(|| ProviderError::ModelNotFound("No model specified".to_string()))?;

        let mut messages = Vec::new();
        
        // Add system prompt if provided
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Add main prompt
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
            "frequency_penalty": request.frequency_penalty.unwrap_or(0.0),
            "presence_penalty": request.presence_penalty.unwrap_or(0.0),
            "stream": request.stream.unwrap_or(false),
            "stop": request.stop_sequences
        });

        debug!("OpenRouter request payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        let response = self.client
            .post(&format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/Tehlikeli107/universal-ai-dev-assistant")
            .header("X-Title", "Universal AI Development Assistant")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("OpenRouter API error: {} - {}", status, error_text);
            
            return Err(match status.as_u16() {
                401 => ProviderError::AuthError(error_text),
                429 => ProviderError::RateLimitError(error_text),
                404 => ProviderError::ModelNotFound(error_text),
                _ => ProviderError::ApiError(format!("HTTP {}: {}", status, error_text)),
            });
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ProviderError::ApiError(format!("Failed to parse response: {}", e)))?;

        debug!("OpenRouter response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        // Parse OpenAI-compatible response
        let choices = response_json["choices"].as_array()
            .ok_or_else(|| ProviderError::ApiError("No choices in response".to_string()))?;

        let parsed_choices = choices.iter().enumerate().map(|(index, choice)| {
            super::traits::Choice {
                index: index as u32,
                text: choice["message"]["content"].as_str().unwrap_or("").to_string(),
                finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                logprobs: choice.get("logprobs").cloned(),
                tool_calls: None, // TODO: Implement tool calls parsing
            }
        }).collect();

        let usage = response_json.get("usage").map(|u| super::traits::Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            cost_usd: None, // OpenRouter doesn't provide cost in response
        });

        Ok(CompletionResponse {
            id: response_json["id"].as_str().unwrap_or("unknown").to_string(),
            choices: parsed_choices,
            usage,
            model: model.clone(),
            provider: "openrouter".to_string(),
            created_at: chrono::Utc::now(),
            metadata: None,
        })
    }

    fn parse_analysis_findings(&self, content: &str, analysis_type: &super::traits::AnalysisType) -> Vec<String> {
        let mut findings = Vec::new();
        
        // Simple pattern-based parsing - could be enhanced with more sophisticated NLP
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines {
            let line_lower = line.to_lowercase();
            
            match analysis_type {
                super::traits::AnalysisType::Security => {
                    if line_lower.contains("vulnerability") || line_lower.contains("security") || 
                       line_lower.contains("injection") || line_lower.contains("xss") {
                        findings.push(line.trim().to_string());
                    }
                }
                super::traits::AnalysisType::Performance => {
                    if line_lower.contains("performance") || line_lower.contains("optimization") || 
                       line_lower.contains("slow") || line_lower.contains("memory") {
                        findings.push(line.trim().to_string());
                    }
                }
                super::traits::AnalysisType::Quality => {
                    if line_lower.contains("quality") || line_lower.contains("best practice") || 
                       line_lower.contains("maintainability") || line_lower.contains("readability") {
                        findings.push(line.trim().to_string());
                    }
                }
                super::traits::AnalysisType::Bugs => {
                    if line_lower.contains("bug") || line_lower.contains("error") || 
                       line_lower.contains("issue") || line_lower.contains("problem") {
                        findings.push(line.trim().to_string());
                    }
                }
                _ => {
                    if line_lower.contains("finding") || line_lower.contains("issue") || 
                       line_lower.contains("problem") || line_lower.contains("suggestion") {
                        findings.push(line.trim().to_string());
                    }
                }
            }
        }
        
        findings
    }

    fn parse_analysis_suggestions(&self, content: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines {
            let line_lower = line.to_lowercase();
            
            if line_lower.contains("suggest") || line_lower.contains("recommend") || 
               line_lower.contains("should") || line_lower.contains("consider") ||
               line_lower.contains("improve") || line_lower.contains("fix") {
                suggestions.push(line.trim().to_string());
            }
        }
        
        suggestions
    }
}

#[async_trait]
impl AIProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn health_check(&self) -> Result<HealthCheck, ProviderError> {
        let start = Instant::now();
        
        // Simple health check by listing models
        match self.list_models().await {
            Ok(models) => {
                let response_time = start.elapsed().as_millis() as u64;
                Ok(HealthCheck {
                    is_available: true,
                    response_time_ms: response_time,
                    supported_models: models,
                    rate_limit_remaining: None,
                    error_message: None,
                })
            }
            Err(e) => {
                warn!("OpenRouter health check failed: {}", e);
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
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| ProviderError::AuthError("OpenRouter API key not configured".to_string()))?;

        let response = self.client
            .get(&format!("{}/models", self.config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!("Failed to list models: {}", response.status())));
        }

        let models_response: serde_json::Value = response.json().await
            .map_err(|e| ProviderError::ApiError(format!("Failed to parse models response: {}", e)))?;

        let models = models_response["data"].as_array()
            .ok_or_else(|| ProviderError::ApiError("Invalid models response format".to_string()))?
            .iter()
            .filter_map(|model| model["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(models)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        info!("OpenRouter completion request for model: {:?}", request.model);
        self.make_request(request).await
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError> {
        // TODO: Implement streaming
        Err(ProviderError::ApiError("Streaming not yet implemented for OpenRouter".to_string()))
    }

    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError> {
        let system_prompt = format!(
            "You are an expert code analyzer. Analyze the following {} code for {}. 
            Provide detailed findings with severity levels, suggestions for improvement, and actionable recommendations.",
            request.language,
            match request.analysis_type {
                super::traits::AnalysisType::Security => "security vulnerabilities and potential threats",
                super::traits::AnalysisType::Performance => "performance issues and optimization opportunities",
                super::traits::AnalysisType::Quality => "code quality issues and best practices violations",
                super::traits::AnalysisType::Bugs => "potential bugs and logical errors",
                super::traits::AnalysisType::Suggestions => "general improvements and suggestions",
                super::traits::AnalysisType::Documentation => "documentation completeness and clarity",
                super::traits::AnalysisType::Testing => "test coverage and testing strategies",
                super::traits::AnalysisType::Refactoring => "refactoring opportunities and code structure improvements",
            }
        );

        let completion_request = CompletionRequest::new(format!("Code to analyze:\n\n```{}\n{}\n```", request.language, request.code))
            .with_system_prompt(system_prompt)
            .with_temperature(0.3)
            .with_max_tokens(2000);

        let response = self.complete(completion_request).await?;
        
        // Parse the response into structured analysis
        // For now, return a simple analysis - this could be enhanced with structured parsing
        Ok(AnalysisResponse {
            analysis_type: request.analysis_type,
            findings: self.parse_analysis_findings(&content, &request.analysis_type),
            summary: response.choices.first()
                .map(|c| c.text.clone())
                .unwrap_or_else(|| "No analysis available".to_string()),
            confidence_score: 0.8,
            suggestions: self.parse_analysis_suggestions(&content),
        })
    }

    async fn generate_documentation(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate comprehensive documentation for the following {} code:\n\n```{}\n{}\n```",
            language, language, code
        ))
        .with_system_prompt("You are a technical documentation expert. Generate clear, comprehensive documentation including function descriptions, parameters, return values, examples, and usage notes.".to_string())
        .with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn generate_tests(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Generate comprehensive unit tests for the following {} code:\n\n```{}\n{}\n```",
            language, language, code
        ))
        .with_system_prompt("You are a testing expert. Generate thorough unit tests covering edge cases, error conditions, and normal operation. Use appropriate testing frameworks for the language.".to_string())
        .with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn explain_code(&self, code: &str, language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Explain what the following {} code does in detail:\n\n```{}\n{}\n```",
            language, language, code
        ))
        .with_system_prompt("You are a code explanation expert. Provide clear, detailed explanations of code functionality, logic flow, and purpose. Make it understandable for developers of all levels.".to_string())
        .with_temperature(0.4);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Refactor the following {} code according to these instructions: {}\n\nCode:\n```{}\n{}\n```",
            language, instructions, language, code
        ))
        .with_system_prompt("You are a refactoring expert. Improve code structure, readability, and maintainability while preserving functionality. Follow best practices and design patterns.".to_string())
        .with_temperature(0.3);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    async fn translate_code(&self, code: &str, from_language: &str, to_language: &str) -> Result<String, ProviderError> {
        let request = CompletionRequest::new(format!(
            "Translate the following {} code to {}:\n\n```{}\n{}\n```",
            from_language, to_language, from_language, code
        ))
        .with_system_prompt(format!(
            "You are a code translation expert. Convert code from {} to {} while maintaining functionality, following {} best practices and idioms.",
            from_language, to_language, to_language
        ))
        .with_temperature(0.2);

        let response = self.complete(request).await?;
        Ok(response.choices.first().map(|c| c.text.clone()).unwrap_or_default())
    }

    fn get_config(&self) -> &ProviderConfig {
        &self.config
    }

    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64> {
        // OpenRouter pricing varies by model - this is a rough estimate
        let tokens = request.prompt.len() / 4 + request.max_tokens.unwrap_or(1000) as usize;
        Some(tokens as f64 * 0.00001) // Rough estimate: $0.01 per 1K tokens
    }
}