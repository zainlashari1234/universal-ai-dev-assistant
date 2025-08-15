pub mod openrouter;
pub mod openai;
pub mod anthropic;
pub mod google;
pub mod groq;
pub mod together;
pub mod cohere;
pub mod ollama;
pub mod router;
pub mod traits;

pub use router::ProviderRouter;
pub use traits::{AIProvider, CompletionRequest, CompletionResponse, ProviderError};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    OpenRouter,
    OpenAI,
    Anthropic,
    Google,
    Groq,
    Together,
    Cohere,
    Ollama,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenRouter => write!(f, "openrouter"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Groq => write!(f, "groq"),
            ProviderType::Together => write!(f, "together"),
            ProviderType::Cohere => write!(f, "cohere"),
            ProviderType::Ollama => write!(f, "ollama"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_type: ProviderType,
    pub is_available: bool,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub models_available: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub provider_type: ProviderType,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub tokens_processed: u64,
    pub cost_usd: f64,
}