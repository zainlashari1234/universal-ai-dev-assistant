// P0 Day-3: Artifacts repository for test artifact persistence  
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: Uuid,
    pub run_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub file_path: Option<String>,
    pub storage_type: String,
    pub artifact_type: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub content_preview: Option<String>,
    pub download_url: Option<String>,
    pub storage_metadata: serde_json::Value,
    pub retention_until: Option<DateTime<Utc>>,
    pub is_public: bool,
    pub download_count: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateArtifactRequest {
    pub run_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub file_path: Option<String>,
    pub storage_type: String,
    pub artifact_type: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub content_preview: Option<String>,
    pub download_url: Option<String>,
    pub storage_metadata: serde_json::Value,
    pub retention_until: Option<DateTime<Utc>>,
    pub is_public: bool,
    pub metadata: serde_json::Value,
}

pub struct ArtifactsRepository {
    pool: PgPool,
}

impl ArtifactsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Create a new artifact record
    pub async fn create(&self, request: CreateArtifactRequest) -> Result<ArtifactRecord> {
        let record = sqlx::query_as::<_, ArtifactRecord>(
            r#"
            INSERT INTO artifacts (
                run_id, project_id, user_id, name, file_path,
                storage_type, artifact_type, mime_type, size_bytes,
                checksum_sha256, content_preview, download_url,
                storage_metadata, retention_until, is_public, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#
        )
        .bind(request.run_id)
        .bind(request.project_id)
        .bind(request.user_id)
        .bind(request.name)
        .bind(request.file_path)
        .bind(request.storage_type)
        .bind(request.artifact_type)
        .bind(request.mime_type)
        .bind(request.size_bytes)
        .bind(request.checksum_sha256)
        .bind(request.content_preview)
        .bind(request.download_url)
        .bind(request.storage_metadata)
        .bind(request.retention_until)
        .bind(request.is_public)
        .bind(request.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Get artifact by ID
    pub async fn get_by_id(&self, artifact_id: Uuid) -> Result<Option<ArtifactRecord>> {
        let record = sqlx::query_as::<_, ArtifactRecord>(
            "SELECT * FROM artifacts WHERE id = $1"
        )
        .bind(artifact_id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    /// Get artifacts by run ID
    pub async fn get_by_run_id(&self, run_id: Uuid) -> Result<Vec<ArtifactRecord>> {
        let records = sqlx::query_as::<_, ArtifactRecord>(
            "SELECT * FROM artifacts WHERE run_id = $1 ORDER BY created_at DESC"
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(records)
    }
    
    /// Get artifacts by project ID with pagination
    pub async fn get_by_project_id(&self, project_id: Uuid, limit: i64, offset: i64) -> Result<Vec<ArtifactRecord>> {
        let records = sqlx::query_as::<_, ArtifactRecord>(
            r#"
            SELECT * FROM artifacts 
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
    
    /// Increment download count
    pub async fn increment_download_count(&self, artifact_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE artifacts SET download_count = download_count + 1, updated_at = NOW() WHERE id = $1"
        )
        .bind(artifact_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Delete artifact by ID
    pub async fn delete(&self, artifact_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM artifacts WHERE id = $1")
            .bind(artifact_id)
            .execute(&self.pool)
            .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    /// Get artifact statistics
    pub async fn get_stats(&self, project_id: Option<Uuid>) -> Result<ArtifactStats> {
        let (total_count, total_size) = if let Some(pid) = project_id {
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM artifacts WHERE project_id = $1"
            )
            .bind(pid)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM artifacts"
            )
            .fetch_one(&self.pool)
            .await?
        };
        
        Ok(ArtifactStats {
            total_count,
            total_size_bytes: total_size,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactStats {
    pub total_count: i64,
    pub total_size_bytes: i64,
}