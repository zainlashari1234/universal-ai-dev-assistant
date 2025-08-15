// P0 Day-3: Completion logs repository for AI completion tracking
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CompletionLogRecord {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub provider: String,
    pub model_name: Option<String>,
    pub prompt_text: String,
    pub prompt_tokens: Option<i32>,
    pub completion_text: Option<String>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub status: String,
    pub confidence_score: Option<f32>,
    pub language: Option<String>,
    pub context_size: Option<i32>,
    pub processing_time_ms: Option<i64>,
    pub cost_cents: Option<i32>,
    pub error_message: Option<String>,
    pub request_metadata: serde_json::Value,
    pub response_metadata: serde_json::Value,
    pub feedback_score: Option<i32>,
    pub feedback_comment: Option<String>,
    pub is_accepted: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCompletionLogRequest {
    pub user_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub provider: String,
    pub model_name: Option<String>,
    pub prompt_text: String,
    pub prompt_tokens: Option<i32>,
    pub language: Option<String>,
    pub context_size: Option<i32>,
    pub request_metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCompletionLogRequest {
    pub completion_text: Option<String>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub status: Option<String>,
    pub confidence_score: Option<f32>,
    pub processing_time_ms: Option<i64>,
    pub cost_cents: Option<i32>,
    pub error_message: Option<String>,
    pub response_metadata: Option<serde_json::Value>,
    pub feedback_score: Option<i32>,
    pub feedback_comment: Option<String>,
    pub is_accepted: Option<bool>,
}

pub struct CompletionLogsRepository {
    pool: PgPool,
}

impl CompletionLogsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Create a new completion log record
    pub async fn create(&self, request: CreateCompletionLogRequest) -> Result<CompletionLogRecord> {
        let record = sqlx::query_as::<_, CompletionLogRecord>(
            r#"
            INSERT INTO completion_logs (
                user_id, project_id, session_id, provider, model_name,
                prompt_text, prompt_tokens, language, context_size, request_metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(request.user_id)
        .bind(request.project_id)
        .bind(request.session_id)
        .bind(request.provider)
        .bind(request.model_name)
        .bind(request.prompt_text)
        .bind(request.prompt_tokens)
        .bind(request.language)
        .bind(request.context_size)
        .bind(request.request_metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Update completion log with results
    pub async fn update(&self, log_id: Uuid, request: UpdateCompletionLogRequest) -> Result<CompletionLogRecord> {
        let record = sqlx::query_as::<_, CompletionLogRecord>(
            r#"
            UPDATE completion_logs SET
                completion_text = COALESCE($2, completion_text),
                completion_tokens = COALESCE($3, completion_tokens),
                total_tokens = COALESCE($4, total_tokens),
                status = COALESCE($5, status),
                confidence_score = COALESCE($6, confidence_score),
                processing_time_ms = COALESCE($7, processing_time_ms),
                cost_cents = COALESCE($8, cost_cents),
                error_message = COALESCE($9, error_message),
                response_metadata = COALESCE($10, response_metadata),
                feedback_score = COALESCE($11, feedback_score),
                feedback_comment = COALESCE($12, feedback_comment),
                is_accepted = COALESCE($13, is_accepted),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(log_id)
        .bind(request.completion_text)
        .bind(request.completion_tokens)
        .bind(request.total_tokens)
        .bind(request.status)
        .bind(request.confidence_score)
        .bind(request.processing_time_ms)
        .bind(request.cost_cents)
        .bind(request.error_message)
        .bind(request.response_metadata)
        .bind(request.feedback_score)
        .bind(request.feedback_comment)
        .bind(request.is_accepted)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Get completion logs by user ID
    pub async fn get_by_user_id(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<CompletionLogRecord>> {
        let records = sqlx::query_as::<_, CompletionLogRecord>(
            r#"
            SELECT * FROM completion_logs 
            WHERE user_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(records)
    }
    
    /// Get completion analytics
    pub async fn get_analytics(&self, user_id: Option<Uuid>, days: i32) -> Result<CompletionAnalytics> {
        let base_query = if user_id.is_some() {
            "WHERE user_id = $1 AND created_at >= NOW() - INTERVAL '%d days'"
        } else {
            "WHERE created_at >= NOW() - INTERVAL '%d days'"
        };
        
        let query = format!(
            "SELECT COUNT(*) as total, AVG(confidence_score) as avg_confidence, COUNT(*) FILTER (WHERE is_accepted = true) as accepted_count FROM completion_logs {}",
            base_query.replace("%d", &days.to_string())
        );
        
        let (total, avg_confidence, accepted_count): (i64, Option<f64>, i64) = if let Some(uid) = user_id {
            sqlx::query_as(&query)
                .bind(uid)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as(&query)
                .fetch_one(&self.pool)
                .await?
        };
        
        Ok(CompletionAnalytics {
            total_completions: total,
            average_confidence: avg_confidence.unwrap_or(0.0) as f32,
            acceptance_rate: if total > 0 { (accepted_count as f32 / total as f32) * 100.0 } else { 0.0 },
            accepted_completions: accepted_count,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionAnalytics {
    pub total_completions: i64,
    pub average_confidence: f32,
    pub acceptance_rate: f32,
    pub accepted_completions: i64,
}