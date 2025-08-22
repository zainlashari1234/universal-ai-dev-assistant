use anyhow::Result;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, error};

use super::{TerminalSession, CommandEntry, TerminalContext, SafetyLevel};

pub struct HistoryManager {
    pool: Arc<PgPool>,
}

impl HistoryManager {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn create_session(&self, session: &TerminalSession) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO terminal_sessions (id, user_id, workspace_path, session_data, created_at, last_activity)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            session.id,
            session.user_id,
            session.workspace_path,
            serde_json::to_value(&session.context)?,
            session.created_at,
            session.last_activity
        )
        .execute(&*self.pool)
        .await?;

        info!("Terminal session created: {}", session.id);
        Ok(())
    }

    pub async fn update_session(&self, session: &TerminalSession) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE terminal_sessions 
            SET session_data = $1, last_activity = $2
            WHERE id = $3
            "#,
            serde_json::to_value(&session.context)?,
            session.last_activity,
            session.id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<TerminalSession>> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, workspace_path, session_data, created_at, last_activity
            FROM terminal_sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let context: TerminalContext = serde_json::from_value(row.session_data)?;
            let command_history = self.get_session_commands(session_id).await?;

            Ok(Some(TerminalSession {
                id: row.id,
                user_id: row.user_id,
                workspace_path: row.workspace_path,
                command_history,
                context,
                created_at: row.created_at,
                last_activity: row.last_activity,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_sessions(&self, user_id: Uuid, limit: i64) -> Result<Vec<TerminalSession>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_id, workspace_path, session_data, created_at, last_activity
            FROM terminal_sessions
            WHERE user_id = $1
            ORDER BY last_activity DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let context: TerminalContext = serde_json::from_value(row.session_data)?;
            let command_history = self.get_session_commands(row.id).await?;

            sessions.push(TerminalSession {
                id: row.id,
                user_id: row.user_id,
                workspace_path: row.workspace_path,
                command_history,
                context,
                created_at: row.created_at,
                last_activity: row.last_activity,
            });
        }

        Ok(sessions)
    }

    pub async fn add_command(&self, session_id: Uuid, command: &CommandEntry) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO command_history (id, session_id, command, output, exit_code, ai_suggested, safety_level, executed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            command.id,
            session_id,
            command.command,
            command.output,
            command.exit_code,
            command.ai_suggested,
            serde_json::to_string(&command.safety_level)?,
            command.timestamp
        )
        .execute(&*self.pool)
        .await?;

        // Session'ın son aktivite zamanını güncelle
        sqlx::query!(
            r#"
            UPDATE terminal_sessions 
            SET last_activity = $1
            WHERE id = $2
            "#,
            Utc::now(),
            session_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_session_commands(&self, session_id: Uuid) -> Result<Vec<CommandEntry>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, command, output, exit_code, ai_suggested, safety_level, executed_at
            FROM command_history
            WHERE session_id = $1
            ORDER BY executed_at ASC
            "#,
            session_id
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut commands = Vec::new();
        for row in rows {
            let safety_level: SafetyLevel = serde_json::from_str(&row.safety_level)?;
            
            commands.push(CommandEntry {
                id: row.id,
                command: row.command,
                output: row.output,
                exit_code: row.exit_code,
                execution_time_ms: 0, // Bu bilgiyi ayrı tutacağız
                ai_suggested: row.ai_suggested,
                safety_level,
                timestamp: row.executed_at,
            });
        }

        Ok(commands)
    }

    pub async fn search_user_commands(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<CommandEntry>> {
        let rows = sqlx::query!(
            r#"
            SELECT ch.id, ch.command, ch.output, ch.exit_code, ch.ai_suggested, ch.safety_level, ch.executed_at
            FROM command_history ch
            JOIN terminal_sessions ts ON ch.session_id = ts.id
            WHERE ts.user_id = $1 
            AND (ch.command ILIKE $2 OR ch.output ILIKE $2)
            ORDER BY ch.executed_at DESC
            LIMIT $3
            "#,
            user_id,
            format!("%{}%", query),
            limit
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut commands = Vec::new();
        for row in rows {
            let safety_level: SafetyLevel = serde_json::from_str(&row.safety_level)?;
            
            commands.push(CommandEntry {
                id: row.id,
                command: row.command,
                output: row.output,
                exit_code: row.exit_code,
                execution_time_ms: 0,
                ai_suggested: row.ai_suggested,
                safety_level,
                timestamp: row.executed_at,
            });
        }

        Ok(commands)
    }

    pub async fn get_command_statistics(&self, user_id: Uuid) -> Result<CommandStatistics> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_commands,
                COUNT(*) FILTER (WHERE ai_suggested = true) as ai_suggested_count,
                COUNT(*) FILTER (WHERE exit_code = 0) as successful_commands,
                COUNT(DISTINCT session_id) as total_sessions
            FROM command_history ch
            JOIN terminal_sessions ts ON ch.session_id = ts.id
            WHERE ts.user_id = $1
            "#,
            user_id
        )
        .fetch_one(&*self.pool)
        .await?;

        let most_used = sqlx::query!(
            r#"
            SELECT command, COUNT(*) as usage_count
            FROM command_history ch
            JOIN terminal_sessions ts ON ch.session_id = ts.id
            WHERE ts.user_id = $1
            GROUP BY command
            ORDER BY usage_count DESC
            LIMIT 10
            "#,
            user_id
        )
        .fetch_all(&*self.pool)
        .await?;

        let most_used_commands: Vec<(String, i64)> = most_used
            .into_iter()
            .map(|row| (row.command, row.usage_count.unwrap_or(0)))
            .collect();

        Ok(CommandStatistics {
            total_commands: stats.total_commands.unwrap_or(0),
            ai_suggested_count: stats.ai_suggested_count.unwrap_or(0),
            successful_commands: stats.successful_commands.unwrap_or(0),
            total_sessions: stats.total_sessions.unwrap_or(0),
            most_used_commands,
        })
    }

    pub async fn cleanup_old_sessions(&self, days_old: i32) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM terminal_sessions 
            WHERE last_activity < NOW() - INTERVAL '%d days'
            "#,
            days_old
        )
        .execute(&*self.pool)
        .await?;

        info!("Cleaned up {} old terminal sessions", result.rows_affected());
        Ok(result.rows_affected())
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        // Önce komut geçmişini sil
        sqlx::query!(
            "DELETE FROM command_history WHERE session_id = $1",
            session_id
        )
        .execute(&*self.pool)
        .await?;

        // Sonra session'ı sil
        sqlx::query!(
            "DELETE FROM terminal_sessions WHERE id = $1",
            session_id
        )
        .execute(&*self.pool)
        .await?;

        info!("Terminal session deleted: {}", session_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CommandStatistics {
    pub total_commands: i64,
    pub ai_suggested_count: i64,
    pub successful_commands: i64,
    pub total_sessions: i64,
    pub most_used_commands: Vec<(String, i64)>,
}

impl CommandStatistics {
    pub fn success_rate(&self) -> f64 {
        if self.total_commands == 0 {
            0.0
        } else {
            self.successful_commands as f64 / self.total_commands as f64
        }
    }

    pub fn ai_usage_rate(&self) -> f64 {
        if self.total_commands == 0 {
            0.0
        } else {
            self.ai_suggested_count as f64 / self.total_commands as f64
        }
    }
}