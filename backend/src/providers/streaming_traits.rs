use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures_util::Stream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    pub stream_id: String,
    pub estimated_tokens: Option<u32>,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
    pub metadata: Option<ChunkMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub latency_ms: Option<u64>,
    pub cost_estimate: Option<f64>,
    pub quality_score: Option<f32>,
    pub provider_specific: Option<serde_json::Value>,
}

pub type StreamingResult = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

#[async_trait]
pub trait StreamingProvider: Send + Sync {
    async fn create_stream(
        &self,
        request: &super::traits::CompletionRequest,
        api_key: &str,
    ) -> Result<StreamingResult>;

    async fn estimate_cost(
        &self,
        request: &super::traits::CompletionRequest,
    ) -> Result<f64>;

    fn get_streaming_models(&self) -> Vec<String>;
    
    fn supports_streaming(&self) -> bool {
        true
    }
}

// Streaming utilities
pub struct StreamingUtils;

impl StreamingUtils {
    pub fn calculate_cost_per_token(provider: &str, model: &str) -> f64 {
        match (provider, model) {
            ("openai", "gpt-4o") => 0.00003,
            ("openai", "gpt-4o-mini") => 0.00000015,
            ("openai", "gpt-3.5-turbo") => 0.000001,
            ("anthropic", "claude-3-sonnet") => 0.000015,
            ("anthropic", "claude-3-haiku") => 0.00000025,
            ("google", "gemini-pro") => 0.000001,
            ("openrouter", _) => 0.000002, // Average OpenRouter pricing
            ("groq", _) => 0.0000001, // Groq is very cheap
            ("together", _) => 0.0000008,
            ("cohere", _) => 0.000001,
            _ => 0.000002, // Default fallback
        }
    }

    pub fn estimate_quality_score(content: &str, language: Option<&str>) -> f32 {
        let mut score = 0.5; // Base score

        // Length factor
        let length = content.len();
        if length > 100 && length < 2000 {
            score += 0.1;
        }

        // Code quality factors
        if let Some(lang) = language {
            match lang {
                "rust" | "python" | "javascript" | "typescript" => {
                    if content.contains("fn ") || content.contains("def ") || content.contains("function ") {
                        score += 0.2;
                    }
                    if content.contains("//") || content.contains("#") || content.contains("/*") {
                        score += 0.1; // Has comments
                    }
                }
                _ => {}
            }
        }

        // General quality indicators
        if content.contains("```") {
            score += 0.1; // Proper code formatting
        }
        
        if content.lines().count() > 5 {
            score += 0.1; // Multi-line response
        }

        // Coherence check (simple)
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() > 10 {
            score += 0.1;
        }

        score.min(1.0)
    }

    pub fn detect_security_issues(content: &str, language: Option<&str>) -> f32 {
        let mut security_score = 1.0;

        // Common security anti-patterns
        let security_issues = [
            "eval(",
            "exec(",
            "system(",
            "shell_exec(",
            "password",
            "secret",
            "api_key",
            "private_key",
            "SELECT * FROM",
            "DROP TABLE",
            "DELETE FROM",
            "innerHTML",
            "document.write",
        ];

        for issue in &security_issues {
            if content.to_lowercase().contains(&issue.to_lowercase()) {
                security_score -= 0.1;
            }
        }

        // Language-specific checks
        if let Some(lang) = language {
            match lang {
                "javascript" | "typescript" => {
                    if content.contains("dangerouslySetInnerHTML") {
                        security_score -= 0.2;
                    }
                }
                "python" => {
                    if content.contains("pickle.loads") || content.contains("yaml.load") {
                        security_score -= 0.2;
                    }
                }
                "sql" => {
                    if !content.contains("$1") && !content.contains("?") && content.contains("SELECT") {
                        security_score -= 0.3; // Potential SQL injection
                    }
                }
                _ => {}
            }
        }

        security_score.max(0.0)
    }
}