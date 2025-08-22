use super::streaming_traits::*;
use super::traits::CompletionRequest;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;
use tokio_stream::wrappers::LinesStream;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct OpenRouterStreaming {
    client: Client,
    base_url: String,
}

impl OpenRouterStreaming {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

#[async_trait]
impl StreamingProvider for OpenRouterStreaming {
    async fn create_stream(
        &self,
        request: &CompletionRequest,
        api_key: &str,
    ) -> Result<StreamingResult> {
        let model = request.model.as_deref().unwrap_or("gpt-4o-mini");
        
        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": request.system_prompt.as_deref().unwrap_or("You are a helpful AI assistant.")
                },
                {
                    "role": "user", 
                    "content": request.prompt
                }
            ],
            "max_tokens": request.max_tokens.unwrap_or(1000),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": true
        });

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("OpenRouter API error: {}", response.status()));
        }

        // Convert response stream to our StreamChunk format
        let stream = response
            .bytes_stream()
            .map(|chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        
                        // Parse SSE format
                        for line in chunk_str.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    return Ok(StreamChunk {
                                        content: "".to_string(),
                                        tokens_used: Some(1),
                                        finish_reason: Some("stop".to_string()),
                                        metadata: None,
                                    });
                                }
                                
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(choices) = parsed["choices"].as_array() {
                                        if let Some(choice) = choices.first() {
                                            if let Some(delta) = choice["delta"].as_object() {
                                                if let Some(content) = delta["content"].as_str() {
                                                    return Ok(StreamChunk {
                                                        content: content.to_string(),
                                                        tokens_used: Some(1),
                                                        finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                                                        metadata: Some(ChunkMetadata {
                                                            latency_ms: Some(50), // Simulated
                                                            cost_estimate: Some(StreamingUtils::calculate_cost_per_token("openrouter", model)),
                                                            quality_score: Some(StreamingUtils::estimate_quality_score(content, request.language.as_deref())),
                                                            provider_specific: Some(json!({"model": model})),
                                                        }),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Fallback for non-SSE content
                        Ok(StreamChunk {
                            content: chunk_str.to_string(),
                            tokens_used: Some(1),
                            finish_reason: None,
                            metadata: None,
                        })
                    }
                    Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            });

        Ok(Box::pin(stream))
    }

    async fn estimate_cost(&self, request: &CompletionRequest) -> Result<f64> {
        let model = request.model.as_deref().unwrap_or("gpt-4o-mini");
        let estimated_tokens = (request.prompt.len() / 4) as f64; // Rough estimation
        let cost_per_token = StreamingUtils::calculate_cost_per_token("openrouter", model);
        Ok(estimated_tokens * cost_per_token)
    }

    fn get_streaming_models(&self) -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "claude-3-sonnet".to_string(),
            "claude-3-haiku".to_string(),
            "llama-3.1-70b".to_string(),
            "mixtral-8x7b".to_string(),
        ]
    }
}