use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("API request failed: {0}")]
    ApiError(String),
    #[error("Authentication failed: {0}")]
    AuthError(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub language: Option<String>,
    pub context: Option<String>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<Tool>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub model: String,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub text: String,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub code: String,
    pub language: String,
    pub analysis_type: AnalysisType,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    Security,
    Performance,
    Quality,
    Bugs,
    Suggestions,
    Documentation,
    Testing,
    Refactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub analysis_type: AnalysisType,
    pub findings: Vec<Finding>,
    pub summary: String,
    pub confidence_score: f32,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub code_snippet: Option<String>,
    pub fix_suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub title: String,
    pub description: String,
    pub code_example: Option<String>,
    pub impact: String,
    pub effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub is_available: bool,
    pub response_time_ms: u64,
    pub supported_models: Vec<String>,
    pub rate_limit_remaining: Option<u32>,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Check if the provider is available and healthy
    async fn health_check(&self) -> Result<HealthCheck, ProviderError>;
    
    /// Get list of available models
    async fn list_models(&self) -> Result<Vec<String>, ProviderError>;
    
    /// Generate code completion
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError>;
    
    /// Stream code completion (if supported)
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<String, ProviderError>>, ProviderError>;
    
    /// Analyze code for issues, suggestions, etc.
    async fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResponse, ProviderError>;
    
    /// Generate code documentation
    async fn generate_documentation(&self, code: &str, language: &str) -> Result<String, ProviderError>;
    
    /// Generate unit tests
    async fn generate_tests(&self, code: &str, language: &str) -> Result<String, ProviderError>;
    
    /// Explain code functionality
    async fn explain_code(&self, code: &str, language: &str) -> Result<String, ProviderError>;
    
    /// Refactor code with suggestions
    async fn refactor_code(&self, code: &str, language: &str, instructions: &str) -> Result<String, ProviderError>;
    
    /// Convert code between languages
    async fn translate_code(&self, code: &str, from_language: &str, to_language: &str) -> Result<String, ProviderError>;
    
    /// Get provider-specific configuration
    fn get_config(&self) -> &crate::config::ProviderConfig;
    
    /// Calculate estimated cost for a request
    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64>;
}

impl CompletionRequest {
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            model: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            top_p: Some(0.9),
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            stream: Some(false),
            language: None,
            context: None,
            system_prompt: None,
            tools: None,
            metadata: None,
        }
    }
    
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }
    
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
    
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
    
    pub fn with_system_prompt(mut self, system_prompt: String) -> Self {
        self.system_prompt = Some(system_prompt);
        self
    }
    
    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }
    
    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
    
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }
}