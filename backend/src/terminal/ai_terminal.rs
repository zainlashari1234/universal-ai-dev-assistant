use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, error};

use crate::providers::ProviderRouter;
use super::{
    TerminalSession, TerminalContext, CommandEntry, TerminalRequest, TerminalResponse,
    QueryType, SafetyLevel, command_suggester::AICommandSuggester,
    history_manager::HistoryManager, shell_integration::ShellExecutor,
};

pub struct AITerminalService {
    command_suggester: AICommandSuggester,
    history_manager: HistoryManager,
    shell_executor: ShellExecutor,
    provider_router: Arc<ProviderRouter>,
}

impl AITerminalService {
    pub fn new(
        provider_router: Arc<ProviderRouter>,
        history_manager: HistoryManager,
    ) -> Self {
        Self {
            command_suggester: AICommandSuggester::new(provider_router.clone()),
            history_manager,
            shell_executor: ShellExecutor::new(),
            provider_router,
        }
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        workspace_path: Option<String>,
    ) -> Result<TerminalSession> {
        let mut session = TerminalSession::new(user_id, workspace_path);
        
        // Context'i güncelle
        session.context.detect_project_type();
        session.context.update_git_status().await?;
        
        // Veritabanına kaydet
        self.history_manager.create_session(&session).await?;
        
        info!("New terminal session created: {} for user: {}", session.id, user_id);
        Ok(session)
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<TerminalSession>> {
        self.history_manager.get_session(session_id).await
    }

    pub async fn process_request(
        &self,
        request: TerminalRequest,
    ) -> Result<TerminalResponse> {
        let session_id = request.session_id.unwrap_or_else(Uuid::new_v4);
        
        // Session'ı al veya oluştur
        let mut session = if let Some(existing_session) = self.get_session(session_id).await? {
            existing_session
        } else {
            return Err(anyhow::anyhow!("Session not found: {}", session_id));
        };

        let context = request.context.as_ref().unwrap_or(&session.context);
        
        match request.query_type {
            QueryType::NaturalLanguage => {
                self.handle_natural_language_query(&request, &session, context).await
            }
            QueryType::CommandExecution => {
                self.handle_command_execution(&mut session, &request.query, context).await
            }
            QueryType::CommandExplanation => {
                self.handle_command_explanation(&request, &session, context).await
            }
            QueryType::HistorySearch => {
                self.handle_history_search(&request, &session).await
            }
        }
    }

    async fn handle_natural_language_query(
        &self,
        request: &TerminalRequest,
        session: &TerminalSession,
        context: &TerminalContext,
    ) -> Result<TerminalResponse> {
        info!("Processing natural language query: {}", request.query);

        let suggestions = self.command_suggester.suggest_commands(
            request,
            context,
            &session.command_history,
        ).await?;

        let mut warnings = Vec::new();
        
        // Tehlikeli komutlar için uyarı ekle
        for suggestion in &suggestions {
            if matches!(suggestion.safety_level, SafetyLevel::Dangerous) {
                warnings.push(format!("⚠️ Tehlikeli komut: {}", suggestion.command));
            } else if matches!(suggestion.safety_level, SafetyLevel::Caution) {
                warnings.push(format!("⚠️ Dikkatli kullanın: {}", suggestion.command));
            }
        }

        Ok(TerminalResponse {
            session_id: session.id,
            suggestions,
            execution_result: None,
            explanation: Some(format!("'{}' için {} komut önerisi bulundu", request.query, suggestions.len())),
            warnings,
        })
    }

    async fn handle_command_execution(
        &self,
        session: &mut TerminalSession,
        command: &str,
        context: &TerminalContext,
    ) -> Result<TerminalResponse> {
        info!("Executing command: {}", command);

        // Komut güvenlik kontrolü
        let safety_level = self.command_suggester.safety_checker.check_command(command);
        
        if matches!(safety_level, SafetyLevel::Blocked) {
            return Ok(TerminalResponse {
                session_id: session.id,
                suggestions: vec![],
                execution_result: None,
                explanation: Some("Komut güvenlik nedeniyle engellendi".to_string()),
                warnings: vec!["🚫 Bu komut çalıştırılamaz".to_string()],
            });
        }

        // Komutu çalıştır
        let execution_result = self.shell_executor.execute_command(
            command,
            context,
            safety_level.clone(),
        ).await?;

        // Komut geçmişine ekle
        let command_entry = CommandEntry {
            id: Uuid::new_v4(),
            command: command.to_string(),
            output: execution_result.output.clone(),
            exit_code: execution_result.exit_code,
            execution_time_ms: execution_result.execution_time_ms,
            ai_suggested: false,
            safety_level: safety_level.clone(),
            timestamp: Utc::now(),
        };

        session.add_command(command_entry.clone());
        self.history_manager.add_command(session.id, &command_entry).await?;

        let mut warnings = Vec::new();
        if execution_result.exit_code != 0 {
            warnings.push("Komut hata ile sonlandı".to_string());
        }

        if matches!(safety_level, SafetyLevel::Dangerous) {
            warnings.push("⚠️ Tehlikeli komut çalıştırıldı".to_string());
        }

        Ok(TerminalResponse {
            session_id: session.id,
            suggestions: vec![],
            execution_result: Some(execution_result),
            explanation: None,
            warnings,
        })
    }

    async fn handle_command_explanation(
        &self,
        request: &TerminalRequest,
        session: &TerminalSession,
        context: &TerminalContext,
    ) -> Result<TerminalResponse> {
        info!("Explaining command: {}", request.query);

        let suggestions = self.command_suggester.suggest_commands(
            request,
            context,
            &session.command_history,
        ).await?;

        Ok(TerminalResponse {
            session_id: session.id,
            suggestions,
            execution_result: None,
            explanation: Some("Komut açıklaması".to_string()),
            warnings: vec![],
        })
    }

    async fn handle_history_search(
        &self,
        request: &TerminalRequest,
        session: &TerminalSession,
    ) -> Result<TerminalResponse> {
        info!("Searching command history: {}", request.query);

        let suggestions = self.command_suggester.suggest_commands(
            request,
            &session.context,
            &session.command_history,
        ).await?;

        Ok(TerminalResponse {
            session_id: session.id,
            suggestions,
            execution_result: None,
            explanation: Some(format!("Geçmişte '{}' ile ilgili {} komut bulundu", request.query, suggestions.len())),
            warnings: vec![],
        })
    }

    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TerminalSession>> {
        self.history_manager.get_user_sessions(user_id, limit).await
    }

    pub async fn search_user_commands(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<CommandEntry>> {
        self.history_manager.search_user_commands(user_id, query, limit).await
    }

    pub async fn get_command_statistics(
        &self,
        user_id: Uuid,
    ) -> Result<super::history_manager::CommandStatistics> {
        self.history_manager.get_command_statistics(user_id).await
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        self.history_manager.delete_session(session_id).await
    }

    pub async fn cleanup_old_sessions(&self, days_old: i32) -> Result<u64> {
        self.history_manager.cleanup_old_sessions(days_old).await
    }

    pub async fn validate_command(&self, command: &str) -> super::shell_integration::CommandValidation {
        self.shell_executor.validate_command(command)
    }

    pub async fn get_command_completion(
        &self,
        partial_command: &str,
        context: &TerminalContext,
    ) -> Result<Vec<String>> {
        self.shell_executor.get_command_completion(partial_command, context).await
    }

    pub async fn start_interactive_session(
        &self,
        command: &str,
        context: &TerminalContext,
    ) -> Result<super::shell_integration::InteractiveSession> {
        self.shell_executor.execute_interactive_command(command, context).await
    }
}

// Terminal servisi için yardımcı fonksiyonlar
impl AITerminalService {
    pub async fn suggest_next_commands(
        &self,
        session: &TerminalSession,
        last_command: &str,
    ) -> Result<Vec<super::CommandSuggestion>> {
        let context_prompt = format!(
            "Son çalıştırılan komut: {}
            Proje tipi: {:?}
            Mevcut dizin: {}
            
            Bu komuttan sonra kullanıcının çalıştırmak isteyebileceği 3 komut öner.",
            last_command,
            session.context.project_type,
            session.context.current_directory
        );

        let request = TerminalRequest {
            session_id: Some(session.id),
            query: context_prompt,
            query_type: QueryType::NaturalLanguage,
            context: Some(session.context.clone()),
        };

        self.command_suggester.suggest_commands(
            &request,
            &session.context,
            &session.command_history,
        ).await
    }

    pub async fn analyze_command_pattern(
        &self,
        user_id: Uuid,
    ) -> Result<CommandPatternAnalysis> {
        let stats = self.get_command_statistics(user_id).await?;
        let recent_commands = self.search_user_commands(user_id, "", 50).await?;

        // Komut kategorilerini analiz et
        let mut category_counts = std::collections::HashMap::new();
        for cmd in &recent_commands {
            let category = self.command_suggester.categorize_command(&cmd.command);
            *category_counts.entry(format!("{:?}", category)).or_insert(0) += 1;
        }

        // En sık kullanılan saatleri bul
        let mut hour_counts = std::collections::HashMap::new();
        for cmd in &recent_commands {
            let hour = cmd.timestamp.hour();
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        let most_active_hour = hour_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| hour)
            .unwrap_or(9); // Default 9 AM

        Ok(CommandPatternAnalysis {
            total_commands: stats.total_commands,
            success_rate: stats.success_rate(),
            ai_usage_rate: stats.ai_usage_rate(),
            most_used_categories: category_counts,
            most_active_hour,
            average_commands_per_session: if stats.total_sessions > 0 {
                stats.total_commands as f64 / stats.total_sessions as f64
            } else {
                0.0
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommandPatternAnalysis {
    pub total_commands: i64,
    pub success_rate: f64,
    pub ai_usage_rate: f64,
    pub most_used_categories: std::collections::HashMap<String, i32>,
    pub most_active_hour: u32,
    pub average_commands_per_session: f64,
}