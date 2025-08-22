use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, error};

use super::{ConversationSession, ConversationTurn, WorkspaceContext, CodeContext, SessionMetadata};

pub struct SessionManager {
    pool: Arc<PgPool>,
}

impl SessionManager {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn create_session(&self, session: &ConversationSession) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO conversation_sessions (id, user_id, workspace_context, session_metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            session.id,
            session.user_id,
            serde_json::to_value(&session.workspace_context)?,
            serde_json::to_value(&session.session_metadata)?,
            session.created_at,
            session.updated_at
        )
        .execute(&*self.pool)
        .await?;

        info!("Conversation session created: {}", session.id);
        Ok(())
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<ConversationSession>> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, workspace_context, session_metadata, created_at, updated_at
            FROM conversation_sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let workspace_context: WorkspaceContext = serde_json::from_value(row.workspace_context)?;
            let session_metadata: SessionMetadata = serde_json::from_value(row.session_metadata)?;
            let conversation_history = self.get_session_turns(session_id).await?;

            Ok(Some(ConversationSession {
                id: row.id,
                user_id: row.user_id,
                workspace_context,
                conversation_history,
                active_files: Vec::new(), // Bu bilgiyi ayrı tutacağız
                code_context: CodeContext::default(),
                session_metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_session(&self, session: &ConversationSession) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE conversation_sessions 
            SET workspace_context = $1, session_metadata = $2, updated_at = $3
            WHERE id = $4
            "#,
            serde_json::to_value(&session.workspace_context)?,
            serde_json::to_value(&session.session_metadata)?,
            Utc::now(),
            session.id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_conversation_turn(&self, session_id: Uuid, turn: &ConversationTurn) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO conversation_turns (id, session_id, user_message, ai_response, intent, code_changes, files_referenced, confidence_score, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            turn.id,
            session_id,
            turn.user_message,
            turn.ai_response,
            serde_json::to_string(&turn.intent)?,
            serde_json::to_value(&turn.code_changes)?,
            &turn.files_referenced,
            turn.confidence_score,
            turn.timestamp
        )
        .execute(&*self.pool)
        .await?;

        // Session'ın updated_at'ini güncelle
        sqlx::query!(
            r#"
            UPDATE conversation_sessions 
            SET updated_at = $1
            WHERE id = $2
            "#,
            Utc::now(),
            session_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_session_turns(&self, session_id: Uuid) -> Result<Vec<ConversationTurn>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_message, ai_response, intent, code_changes, files_referenced, confidence_score, created_at
            FROM conversation_turns
            WHERE session_id = $1
            ORDER BY created_at ASC
            "#,
            session_id
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut turns = Vec::new();
        for row in rows {
            let intent: super::MessageIntent = serde_json::from_str(&row.intent)?;
            let code_changes: Option<Vec<super::CodeChange>> = serde_json::from_value(row.code_changes)?;

            turns.push(ConversationTurn {
                id: row.id,
                user_message: row.user_message,
                ai_response: row.ai_response,
                intent,
                code_changes,
                files_referenced: row.files_referenced,
                confidence_score: row.confidence_score,
                execution_time_ms: 0, // Bu bilgiyi ayrı tutacağız
                timestamp: row.created_at,
            });
        }

        Ok(turns)
    }

    pub async fn get_user_sessions(&self, user_id: Uuid, limit: i64) -> Result<Vec<ConversationSession>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_id, workspace_context, session_metadata, created_at, updated_at
            FROM conversation_sessions
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let workspace_context: WorkspaceContext = serde_json::from_value(row.workspace_context)?;
            let session_metadata: SessionMetadata = serde_json::from_value(row.session_metadata)?;
            
            // Son birkaç turn'ü al (performans için)
            let conversation_history = self.get_recent_turns(row.id, 10).await?;

            sessions.push(ConversationSession {
                id: row.id,
                user_id: row.user_id,
                workspace_context,
                conversation_history,
                active_files: Vec::new(),
                code_context: CodeContext::default(),
                session_metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(sessions)
    }

    pub async fn get_recent_turns(&self, session_id: Uuid, limit: i64) -> Result<Vec<ConversationTurn>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_message, ai_response, intent, code_changes, files_referenced, confidence_score, created_at
            FROM conversation_turns
            WHERE session_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            session_id,
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut turns = Vec::new();
        for row in rows {
            let intent: super::MessageIntent = serde_json::from_str(&row.intent)?;
            let code_changes: Option<Vec<super::CodeChange>> = serde_json::from_value(row.code_changes)?;

            turns.push(ConversationTurn {
                id: row.id,
                user_message: row.user_message,
                ai_response: row.ai_response,
                intent,
                code_changes,
                files_referenced: row.files_referenced,
                confidence_score: row.confidence_score,
                execution_time_ms: 0,
                timestamp: row.created_at,
            });
        }

        // Chronological order'a çevir
        turns.reverse();
        Ok(turns)
    }

    pub async fn search_conversations(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<ConversationTurn>> {
        let rows = sqlx::query!(
            r#"
            SELECT ct.id, ct.user_message, ct.ai_response, ct.intent, ct.code_changes, ct.files_referenced, ct.confidence_score, ct.created_at
            FROM conversation_turns ct
            JOIN conversation_sessions cs ON ct.session_id = cs.id
            WHERE cs.user_id = $1 
            AND (ct.user_message ILIKE $2 OR ct.ai_response ILIKE $2)
            ORDER BY ct.created_at DESC
            LIMIT $3
            "#,
            user_id,
            format!("%{}%", query),
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut turns = Vec::new();
        for row in rows {
            let intent: super::MessageIntent = serde_json::from_str(&row.intent)?;
            let code_changes: Option<Vec<super::CodeChange>> = serde_json::from_value(row.code_changes)?;

            turns.push(ConversationTurn {
                id: row.id,
                user_message: row.user_message,
                ai_response: row.ai_response,
                intent,
                code_changes,
                files_referenced: row.files_referenced,
                confidence_score: row.confidence_score,
                execution_time_ms: 0,
                timestamp: row.created_at,
            });
        }

        Ok(turns)
    }

    pub async fn get_conversation_statistics(&self, user_id: Uuid) -> Result<ConversationStatistics> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(DISTINCT cs.id) as total_sessions,
                COUNT(ct.id) as total_turns,
                AVG(ct.confidence_score) as avg_confidence,
                COUNT(ct.id) FILTER (WHERE ct.code_changes IS NOT NULL) as turns_with_code_changes
            FROM conversation_sessions cs
            LEFT JOIN conversation_turns ct ON cs.id = ct.session_id
            WHERE cs.user_id = $1
            "#,
            user_id
        )
        .fetch_one(&*self.pool)
        .await?;

        let intent_distribution = sqlx::query!(
            r#"
            SELECT ct.intent, COUNT(*) as count
            FROM conversation_turns ct
            JOIN conversation_sessions cs ON ct.session_id = cs.id
            WHERE cs.user_id = $1
            GROUP BY ct.intent
            ORDER BY count DESC
            "#,
            user_id
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut intent_counts = std::collections::HashMap::new();
        for row in intent_distribution {
            intent_counts.insert(row.intent, row.count.unwrap_or(0));
        }

        Ok(ConversationStatistics {
            total_sessions: stats.total_sessions.unwrap_or(0),
            total_turns: stats.total_turns.unwrap_or(0),
            average_confidence: stats.avg_confidence.unwrap_or(0.0) as f32,
            turns_with_code_changes: stats.turns_with_code_changes.unwrap_or(0),
            intent_distribution: intent_counts,
        })
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        // Önce conversation turns'leri sil
        sqlx::query!(
            "DELETE FROM conversation_turns WHERE session_id = $1",
            session_id
        )
        .execute(&*self.pool)
        .await?;

        // Sonra session'ı sil
        sqlx::query!(
            "DELETE FROM conversation_sessions WHERE id = $1",
            session_id
        )
        .execute(&*self.pool)
        .await?;

        info!("Conversation session deleted: {}", session_id);
        Ok(())
    }

    pub async fn cleanup_old_sessions(&self, days_old: i32) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM conversation_sessions 
            WHERE updated_at < NOW() - INTERVAL '%d days'
            "#,
            days_old
        )
        .execute(&*self.pool)
        .await?;

        info!("Cleaned up {} old conversation sessions", result.rows_affected());
        Ok(result.rows_affected())
    }

    pub async fn get_session_summary(&self, session_id: Uuid) -> Result<Option<SessionSummary>> {
        let session_info = sqlx::query!(
            r#"
            SELECT cs.id, cs.created_at, cs.updated_at, cs.workspace_context,
                   COUNT(ct.id) as turn_count,
                   COUNT(ct.id) FILTER (WHERE ct.code_changes IS NOT NULL) as code_changes_count
            FROM conversation_sessions cs
            LEFT JOIN conversation_turns ct ON cs.id = ct.session_id
            WHERE cs.id = $1
            GROUP BY cs.id, cs.created_at, cs.updated_at, cs.workspace_context
            "#,
            session_id
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = session_info {
            let workspace_context: WorkspaceContext = serde_json::from_value(row.workspace_context)?;
            
            Ok(Some(SessionSummary {
                session_id: row.id,
                created_at: row.created_at,
                updated_at: row.updated_at,
                turn_count: row.turn_count.unwrap_or(0),
                code_changes_count: row.code_changes_count.unwrap_or(0),
                project_type: workspace_context.project_type,
                workspace_path: workspace_context.root_path,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationStatistics {
    pub total_sessions: i64,
    pub total_turns: i64,
    pub average_confidence: f32,
    pub turns_with_code_changes: i64,
    pub intent_distribution: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub turn_count: i64,
    pub code_changes_count: i64,
    pub project_type: Option<String>,
    pub workspace_path: String,
}

impl ConversationStatistics {
    pub fn most_common_intent(&self) -> Option<String> {
        self.intent_distribution
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(intent, _)| intent.clone())
    }

    pub fn code_generation_rate(&self) -> f32 {
        if self.total_turns == 0 {
            0.0
        } else {
            self.turns_with_code_changes as f32 / self.total_turns as f32
        }
    }

    pub fn average_turns_per_session(&self) -> f32 {
        if self.total_sessions == 0 {
            0.0
        } else {
            self.total_turns as f32 / self.total_sessions as f32
        }
    }
}