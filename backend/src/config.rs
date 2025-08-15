use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: ProvidersConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub features: FeaturesConfig,
    pub rate_limiting: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub openrouter: ProviderConfig,
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub google: ProviderConfig,
    pub groq: ProviderConfig,
    pub together: ProviderConfig,
    pub cohere: ProviderConfig,
    pub ollama: ProviderConfig,
    pub preferred_models: Vec<String>,
    pub fallback_models: Vec<String>,
    pub provider_priorities: HashMap<String, u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub priority: u8,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub enable_migrations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub enable_auth: bool,
    pub api_key_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub enable_analytics: bool,
    pub enable_caching: bool,
    pub enable_streaming: bool,
    pub enable_function_calling: bool,
    pub enable_code_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub enable_per_user_limits: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok(); // Load .env file if it exists

        let server = ServerConfig {
            host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        };

        let providers = ProvidersConfig {
            openrouter: ProviderConfig {
                enabled: std::env::var("OPENROUTER_API_KEY").is_ok(),
                api_key: std::env::var("OPENROUTER_API_KEY").ok(),
                base_url: std::env::var("OPENROUTER_BASE_URL")
                    .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
                priority: std::env::var("OPENROUTER_PRIORITY")
                    .unwrap_or_else(|_| "9".to_string())
                    .parse()
                    .unwrap_or(9),
                models: vec![
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "openai/gpt-4o".to_string(),
                    "google/gemini-pro-1.5".to_string(),
                    "meta-llama/llama-3.1-70b-instruct".to_string(),
                ],
            },
            openai: ProviderConfig {
                enabled: std::env::var("OPENAI_API_KEY").is_ok(),
                api_key: std::env::var("OPENAI_API_KEY").ok(),
                base_url: std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
                priority: 8,
                models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ],
            },
            anthropic: ProviderConfig {
                enabled: std::env::var("ANTHROPIC_API_KEY").is_ok(),
                api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
                base_url: std::env::var("ANTHROPIC_BASE_URL")
                    .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
                priority: 8,
                models: vec![
                    "claude-3-5-sonnet-20241022".to_string(),
                    "claude-3-haiku-20240307".to_string(),
                ],
            },
            google: ProviderConfig {
                enabled: std::env::var("GOOGLE_API_KEY").is_ok(),
                api_key: std::env::var("GOOGLE_API_KEY").ok(),
                base_url: std::env::var("GOOGLE_BASE_URL")
                    .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
                priority: 7,
                models: vec![
                    "gemini-pro".to_string(),
                    "gemini-pro-vision".to_string(),
                ],
            },
            groq: ProviderConfig {
                enabled: std::env::var("GROQ_API_KEY").is_ok(),
                api_key: std::env::var("GROQ_API_KEY").ok(),
                base_url: std::env::var("GROQ_BASE_URL")
                    .unwrap_or_else(|_| "https://api.groq.com/openai/v1".to_string()),
                timeout_seconds: 15,
                max_retries: 2,
                priority: 6,
                models: vec![
                    "llama-3.1-70b-versatile".to_string(),
                    "mixtral-8x7b-32768".to_string(),
                ],
            },
            together: ProviderConfig {
                enabled: std::env::var("TOGETHER_API_KEY").is_ok(),
                api_key: std::env::var("TOGETHER_API_KEY").ok(),
                base_url: std::env::var("TOGETHER_BASE_URL")
                    .unwrap_or_else(|_| "https://api.together.xyz/v1".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
                priority: 5,
                models: vec![
                    "meta-llama/Llama-3-70b-chat-hf".to_string(),
                    "mistralai/Mixtral-8x7B-Instruct-v0.1".to_string(),
                ],
            },
            cohere: ProviderConfig {
                enabled: std::env::var("COHERE_API_KEY").is_ok(),
                api_key: std::env::var("COHERE_API_KEY").ok(),
                base_url: std::env::var("COHERE_BASE_URL")
                    .unwrap_or_else(|_| "https://api.cohere.ai/v1".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
                priority: 4,
                models: vec![
                    "command-r-plus".to_string(),
                    "command-r".to_string(),
                ],
            },
            ollama: ProviderConfig {
                enabled: true, // Always enabled as fallback
                api_key: None,
                base_url: std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
                timeout_seconds: 60,
                max_retries: 2,
                priority: 3,
                models: vec![
                    "qwen2.5-coder:7b".to_string(),
                    "codellama:7b".to_string(),
                    "deepseek-coder:6.7b".to_string(),
                ],
            },
            preferred_models: std::env::var("PREFERRED_MODELS")
                .unwrap_or_else(|_| "gpt-4o,claude-3-5-sonnet-20241022,gemini-pro".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            fallback_models: std::env::var("FALLBACK_MODELS")
                .unwrap_or_else(|_| "gpt-3.5-turbo,claude-3-haiku-20240307,qwen2.5-coder:7b".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            provider_priorities: HashMap::new(), // Will be populated from individual priorities
        };

        let database = DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./data/uaida.db".to_string()),
            max_connections: 10,
            enable_migrations: true,
        };

        let security = SecurityConfig {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
            enable_auth: std::env::var("ENABLE_AUTH")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            api_key_required: std::env::var("API_KEY_REQUIRED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        };

        let features = FeaturesConfig {
            enable_analytics: std::env::var("ENABLE_ANALYTICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_caching: std::env::var("ENABLE_CACHING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_streaming: std::env::var("ENABLE_STREAMING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_function_calling: std::env::var("ENABLE_FUNCTION_CALLING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_code_execution: std::env::var("ENABLE_CODE_EXECUTION")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        };

        let rate_limiting = RateLimitConfig {
            requests_per_minute: std::env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            burst_size: std::env::var("RATE_LIMIT_BURST")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            enable_per_user_limits: true,
        };

        Ok(Config {
            server,
            providers,
            database,
            security,
            features,
            rate_limiting,
        })
    }
}