use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub preferences: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompletionLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub language: String,
    pub prompt_length: i32,
    pub completion_length: i32,
    pub processing_time_ms: i32,
    pub model_used: String,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub language: String,
    pub repository_url: Option<String>,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    pub file_path: String,
    pub content: String,
    pub language: String,
    pub imports: Vec<String>,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub docstring: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub methods: Vec<FunctionInfo>,
    pub properties: Vec<String>,
    pub inheritance: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: SecuritySeverity,
    pub category: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub average_response_time: f64,
    pub p95_response_time: f64,
    pub p99_response_time: f64,
    pub error_rate: f64,
    pub requests_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub size_mb: u64,
    pub parameters: u64,
    pub supported_languages: Vec<String>,
    pub capabilities: Vec<String>,
    pub is_loaded: bool,
    pub load_time_ms: u64,
}