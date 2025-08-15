// P0 Day-3: Runs repository for test execution persistence
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: Uuid,
    pub project_id: Uuid,
    pub patch_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    pub user_id: Uuid,
    pub run_type: String, // Will be enum in real usage
    pub status: String,   // Will be enum in real usage
    pub command: Option<String>,
    pub environment: serde_json::Value,
    pub working_directory: Option<String>,
    pub timeout_seconds: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub stdout_log: Option<String>,
    pub stderr_log: Option<String>,
    pub test_results: Option<serde_json::Value>,
    pub coverage_data: Option<serde_json::Value>,
    pub performance_metrics: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRunRequest {
    pub project_id: Uuid,
    pub patch_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    pub user_id: Uuid,
    pub run_type: String,
    pub command: Option<String>,
    pub environment: serde_json::Value,
    pub working_directory: Option<String>,
    pub timeout_seconds: Option<i32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRunRequest {
    pub status: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub stdout_log: Option<String>,
    pub stderr_log: Option<String>,
    pub test_results: Option<serde_json::Value>,
    pub coverage_data: Option<serde_json::Value>,
    pub performance_metrics: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub struct RunsRepository {
    pool: PgPool,
}

impl RunsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Create a new test run record
    pub async fn create(&self, request: CreateRunRequest) -> Result<RunRecord> {
        let record = sqlx::query_as::<_, RunRecord>(
            r#"
            INSERT INTO runs (
                project_id, patch_id, plan_id, user_id, run_type,
                command, environment, working_directory, timeout_seconds, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(request.project_id)
        .bind(request.patch_id)
        .bind(request.plan_id)
        .bind(request.user_id)
        .bind(request.run_type)
        .bind(request.command)
        .bind(request.environment)
        .bind(request.working_directory)
        .bind(request.timeout_seconds)
        .bind(request.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Update an existing run record
    pub async fn update(&self, run_id: Uuid, request: UpdateRunRequest) -> Result<RunRecord> {
        let record = sqlx::query_as::<_, RunRecord>(
            r#"
            UPDATE runs SET
                status = COALESCE($2, status),
                started_at = COALESCE($3, started_at),
                completed_at = COALESCE($4, completed_at),
                duration_ms = COALESCE($5, duration_ms),
                exit_code = COALESCE($6, exit_code),
                stdout_log = COALESCE($7, stdout_log),
                stderr_log = COALESCE($8, stderr_log),
                test_results = COALESCE($9, test_results),
                coverage_data = COALESCE($10, coverage_data),
                performance_metrics = COALESCE($11, performance_metrics),
                error_message = COALESCE($12, error_message),
                metadata = COALESCE($13, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(run_id)
        .bind(request.status)
        .bind(request.started_at)
        .bind(request.completed_at)
        .bind(request.duration_ms)
        .bind(request.exit_code)
        .bind(request.stdout_log)
        .bind(request.stderr_log)
        .bind(request.test_results)
        .bind(request.coverage_data)
        .bind(request.performance_metrics)
        .bind(request.error_message)
        .bind(request.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Get run by ID
    pub async fn get_by_id(&self, run_id: Uuid) -> Result<Option<RunRecord>> {
        let record = sqlx::query_as::<_, RunRecord>(
            "SELECT * FROM runs WHERE id = $1"
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Get runs by project ID
    pub async fn get_by_project_id(&self, project_id: Uuid, limit: i64, offset: i64) -> Result<Vec<RunRecord>> {
        let records = sqlx::query_as::<_, RunRecord>(
            r#"
            SELECT * FROM runs 
            WHERE project_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(records)
    }
    
    /// Get runs by patch ID
    pub async fn get_by_patch_id(&self, patch_id: Uuid) -> Result<Vec<RunRecord>> {
        let records = sqlx::query_as::<_, RunRecord>(
            "SELECT * FROM runs WHERE patch_id = $1 ORDER BY created_at DESC"
        )
        .bind(patch_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(records)
    }
    
    /// Delete run by ID
    pub async fn delete(&self, run_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM runs WHERE id = $1")
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        
        Ok(result.rows_affected() > 0)
    }
}