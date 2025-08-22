use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, debug, error};

use crate::providers::ProviderRouter;
use super::{
    ConversationSession, ConversationTurn, ConversationRequest, ConversationResponse,
    MessageIntent, SuggestedAction, ActionType, ActionPriority,
    session_manager::SessionManager,
    context_manager::{ContextManager, RelevantContext},
    intent_analyzer::IntentAnalyzer,
    code_integration::{CodeIntegrationService, CodeGenerationRequest},
    workspace_analyzer::WorkspaceAnalyzer,
};

pub struct ConversationService {
    provider_router: Arc<ProviderRouter>,
    session_manager: SessionManager,
    context_manager: ContextManager,
    intent_analyzer: IntentAnalyzer,
    code_integration: CodeIntegrationService,
    workspace_analyzer: WorkspaceAnalyzer,
}

impl ConversationService {
    pub fn new(
        provider_router: Arc<ProviderRouter>,
        session_manager: SessionManager,
    ) -> Self {
        Self {
            provider_router: provider_router.clone(),
            session_manager,
            context_manager: ContextManager::new(),
            intent_analyzer: IntentAnalyzer::new(provider_router.clone()),
            code_integration: CodeIntegrationService::new(provider_router.clone()),
            workspace_analyzer: WorkspaceAnalyzer::new(),
        }
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        workspace_path: Option<String>,
    ) -> Result<ConversationSession> {
        info!("Creating new conversation session for user: {}", user_id);

        let mut session = ConversationSession::new(user_id, workspace_path.clone());

        // Workspace'i analiz et
        if let Some(path) = &workspace_path {
            match self.workspace_analyzer.analyze_workspace(path).await {
                Ok(workspace_context) => {
                    session.workspace_context = workspace_context;
                    info!("Workspace analyzed successfully: {}", path);
                }
                Err(e) => {
                    error!("Failed to analyze workspace {}: {}", path, e);
                }
            }
        }

        // Veritabanına kaydet
        self.session_manager.create_session(&session).await?;

        Ok(session)
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Option<ConversationSession>> {
        self.session_manager.get_session(session_id).await
    }

    pub async fn process_message(&self, request: ConversationRequest) -> Result<ConversationResponse> {
        let start_time = Instant::now();
        
        info!("Processing message: {}", request.message);

        // Session'ı al veya oluştur
        let mut session = if let Some(session_id) = request.session_id {
            self.session_manager.get_session(session_id).await?
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?
        } else {
            return Err(anyhow::anyhow!("Session ID is required"));
        };

        // Context'i güncelle
        self.context_manager.update_code_context(
            &mut session,
            request.current_file.as_deref(),
            request.selected_text,
            request.context_files,
        ).await?;

        if let Some(workspace_path) = &request.workspace_path {
            self.context_manager.update_workspace_context(
                &mut session,
                Some(workspace_path),
            ).await?;
        }

        // Intent analizi
        let (intent, confidence) = if let Some(hint) = request.intent_hint {
            (hint, 0.9) // Hint verilmişse yüksek güven
        } else {
            self.intent_analyzer.analyze_intent(
                &request.message,
                &session.workspace_context,
                &session.code_context,
                &session.conversation_history,
            ).await?
        };

        debug!("Detected intent: {:?} (confidence: {})", intent, confidence);

        // İlgili bağlamı topla
        let relevant_context = self.context_manager.get_relevant_context(
            &session,
            &request.message,
            &intent,
        ).await?;

        // Intent'e göre yanıt oluştur
        let response = self.generate_response(
            &request.message,
            &intent,
            &session,
            &relevant_context,
        ).await?;

        // Conversation turn'ü oluştur
        let turn = ConversationTurn {
            id: Uuid::new_v4(),
            user_message: request.message.clone(),
            ai_response: response.ai_response.clone(),
            intent: intent.clone(),
            code_changes: response.code_changes.clone(),
            files_referenced: response.file_references.clone(),
            confidence_score: confidence,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: Utc::now(),
        };

        // Session'ı güncelle
        session.add_turn(turn.clone());
        self.session_manager.add_conversation_turn(session.id, &turn).await?;
        self.session_manager.update_session(&session).await?;

        // Önerilen aksiyonları ekle
        let suggested_actions = self.generate_suggested_actions(&intent, &response, &session).await?;

        // Follow-up sorularını oluştur
        let follow_up_questions = self.intent_analyzer.get_intent_suggestions(&intent);

        Ok(ConversationResponse {
            session_id: session.id,
            ai_response: response.ai_response,
            intent,
            confidence_score: confidence,
            code_changes: response.code_changes,
            suggested_actions,
            file_references: response.file_references,
            follow_up_questions,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    async fn generate_response(
        &self,
        message: &str,
        intent: &MessageIntent,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        match intent {
            MessageIntent::CodeGeneration => {
                self.handle_code_generation(message, session, context).await
            }
            MessageIntent::CodeExplanation => {
                self.handle_code_explanation(message, session, context).await
            }
            MessageIntent::CodeReview => {
                self.handle_code_review(message, session, context).await
            }
            MessageIntent::Debugging => {
                self.handle_debugging(message, session, context).await
            }
            MessageIntent::Refactoring => {
                self.handle_refactoring(message, session, context).await
            }
            MessageIntent::Testing => {
                self.handle_testing(message, session, context).await
            }
            MessageIntent::Documentation => {
                self.handle_documentation(message, session, context).await
            }
            MessageIntent::FileOperation => {
                self.handle_file_operation(message, session, context).await
            }
            MessageIntent::ProjectSetup => {
                self.handle_project_setup(message, session, context).await
            }
            MessageIntent::TerminalCommand => {
                self.handle_terminal_command(message, session, context).await
            }
            MessageIntent::WorkspaceNavigation => {
                self.handle_workspace_navigation(message, session, context).await
            }
            MessageIntent::GeneralChat => {
                self.handle_general_chat(message, session, context).await
            }
        }
    }

    async fn handle_code_generation(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling code generation request");

        let code_request = CodeGenerationRequest {
            description: message.to_string(),
            target_file: session.code_context.current_file.clone(),
            language: self.detect_target_language(session, context),
            style_preferences: None,
        };

        let result = self.code_integration.generate_code(
            &code_request,
            &session.workspace_context,
            &session.code_context,
        ).await?;

        Ok(InternalResponse {
            ai_response: format!(
                "Kod oluşturdum:\n\n```\n{}\n```\n\n{}",
                result.code,
                result.explanation
            ),
            code_changes: Some(result.changes),
            file_references: vec![],
        })
    }

    async fn handle_code_explanation(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling code explanation request");

        let code_to_explain = if let Some(selected) = &context.selected_code {
            selected.clone()
        } else if let Some(current_file_content) = &context.current_file_content {
            current_file_content.clone()
        } else {
            return Ok(InternalResponse {
                ai_response: "Açıklanacak kod bulunamadı. Lütfen bir kod seçin veya dosya açın.".to_string(),
                code_changes: None,
                file_references: vec![],
            });
        };

        let explanation = self.code_integration.explain_code(
            &code_to_explain,
            session.code_context.current_file.as_deref(),
            &session.workspace_context,
        ).await?;

        Ok(InternalResponse {
            ai_response: explanation.explanation,
            code_changes: None,
            file_references: session.code_context.current_file.clone().into_iter().collect(),
        })
    }

    async fn handle_code_review(
        &self,
        _message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling code review request");

        let code_to_review = if let Some(selected) = &context.selected_code {
            selected.clone()
        } else if let Some(current_file_content) = &context.current_file_content {
            current_file_content.clone()
        } else {
            return Ok(InternalResponse {
                ai_response: "İncelenecek kod bulunamadı. Lütfen bir kod seçin veya dosya açın.".to_string(),
                code_changes: None,
                file_references: vec![],
            });
        };

        let review = self.code_integration.review_code(
            &code_to_review,
            session.code_context.current_file.as_deref(),
            &session.workspace_context,
        ).await?;

        let response = format!(
            "Kod İncelemesi (Genel Puan: {:.1}/10)\n\n{}",
            review.overall_score,
            review.suggestions.join("\n")
        );

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: session.code_context.current_file.clone().into_iter().collect(),
        })
    }

    async fn handle_debugging(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling debugging request");

        let code_to_debug = if let Some(selected) = &context.selected_code {
            selected.clone()
        } else if let Some(current_file_content) = &context.current_file_content {
            current_file_content.clone()
        } else {
            return self.handle_general_debugging_advice(message, session).await;
        };

        let fix = self.code_integration.fix_code(
            &code_to_debug,
            message, // Hata mesajı olarak kullan
            session.code_context.current_file.as_deref(),
            &session.workspace_context,
        ).await?;

        let response = format!(
            "Hata Analizi ve Çözüm:\n\n{}\n\nDüzeltilmiş Kod:\n```\n{}\n```",
            fix.explanation,
            fix.fixed_code
        );

        Ok(InternalResponse {
            ai_response: response,
            code_changes: Some(vec![super::CodeChange {
                file_path: session.code_context.current_file.clone().unwrap_or_else(|| "fixed_code.txt".to_string()),
                change_type: super::ChangeType::Modify,
                old_content: Some(fix.original_code),
                new_content: fix.fixed_code,
                line_start: 0,
                line_end: 0,
                description: "Bug fix".to_string(),
            }]),
            file_references: session.code_context.current_file.clone().into_iter().collect(),
        })
    }

    async fn handle_refactoring(
        &self,
        _message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling refactoring request");

        let code_to_refactor = if let Some(selected) = &context.selected_code {
            selected.clone()
        } else if let Some(current_file_content) = &context.current_file_content {
            current_file_content.clone()
        } else {
            return Ok(InternalResponse {
                ai_response: "Refactor edilecek kod bulunamadı. Lütfen bir kod seçin veya dosya açın.".to_string(),
                code_changes: None,
                file_references: vec![],
            });
        };

        let suggestion = self.code_integration.suggest_refactoring(
            &code_to_refactor,
            session.code_context.current_file.as_deref(),
            &session.workspace_context,
        ).await?;

        let response = format!(
            "Refactoring Önerisi:\n\n{}\n\nRefactor Edilmiş Kod:\n```\n{}\n```\n\nFaydalar:\n{}",
            suggestion.explanation,
            suggestion.refactored_code,
            suggestion.benefits.join("\n")
        );

        Ok(InternalResponse {
            ai_response: response,
            code_changes: Some(vec![super::CodeChange {
                file_path: session.code_context.current_file.clone().unwrap_or_else(|| "refactored_code.txt".to_string()),
                change_type: super::ChangeType::Modify,
                old_content: Some(suggestion.original_code),
                new_content: suggestion.refactored_code,
                line_start: 0,
                line_end: 0,
                description: "Refactoring".to_string(),
            }]),
            file_references: session.code_context.current_file.clone().into_iter().collect(),
        })
    }

    async fn handle_testing(
        &self,
        _message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling testing request");

        let code_to_test = if let Some(selected) = &context.selected_code {
            selected.clone()
        } else if let Some(current_file_content) = &context.current_file_content {
            current_file_content.clone()
        } else {
            return Ok(InternalResponse {
                ai_response: "Test edilecek kod bulunamadı. Lütfen bir kod seçin veya dosya açın.".to_string(),
                code_changes: None,
                file_references: vec![],
            });
        };

        let test_generation = self.code_integration.generate_tests(
            &code_to_test,
            session.code_context.current_file.as_deref(),
            &session.workspace_context,
        ).await?;

        let response = format!(
            "Test Kodları ({} framework):\n\n```\n{}\n```\n\nTahmini Coverage: {:.1}%",
            test_generation.framework,
            test_generation.test_code,
            test_generation.coverage_estimate
        );

        let test_file_name = if let Some(current_file) = &session.code_context.current_file {
            format!("{}_test.rs", current_file.trim_end_matches(".rs"))
        } else {
            "tests.rs".to_string()
        };

        Ok(InternalResponse {
            ai_response: response,
            code_changes: Some(vec![super::CodeChange {
                file_path: test_file_name,
                change_type: super::ChangeType::Create,
                old_content: None,
                new_content: test_generation.test_code,
                line_start: 0,
                line_end: 0,
                description: "Generated tests".to_string(),
            }]),
            file_references: session.code_context.current_file.clone().into_iter().collect(),
        })
    }

    async fn handle_documentation(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling documentation request");

        let prompt = self.build_documentation_prompt(message, session, context);
        let response = self.generate_ai_response(&prompt, "documentation").await?;

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: session.code_context.current_file.clone().into_iter().collect(),
        })
    }

    async fn handle_file_operation(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling file operation request");

        let prompt = self.build_file_operation_prompt(message, session, context);
        let response = self.generate_ai_response(&prompt, "file_operation").await?;

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: context.relevant_files.clone(),
        })
    }

    async fn handle_project_setup(
        &self,
        message: &str,
        session: &ConversationSession,
        _context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling project setup request");

        let prompt = format!(
            "Proje kurulum isteği: {}\n\nMevcut proje tipi: {:?}\nBuild sistem: {:?}\n\nDetaylı kurulum adımları ver.",
            message,
            session.workspace_context.project_type,
            session.workspace_context.build_system
        );

        let response = self.generate_ai_response(&prompt, "project_setup").await?;

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: vec![],
        })
    }

    async fn handle_terminal_command(
        &self,
        message: &str,
        session: &ConversationSession,
        _context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling terminal command request");

        let response = format!(
            "Terminal komutu için: {}\n\nTerminal modunu kullanın: `uaida terminal --interactive`\n\nVeya direkt komut çalıştırın.",
            message
        );

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: vec![],
        })
    }

    async fn handle_workspace_navigation(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling workspace navigation request");

        let response = format!(
            "Workspace navigasyon: {}\n\nMevcut dizin yapısı:\n{}\n\nAna dosyalar: {:?}",
            message,
            context.directory_structure.join("\n"),
            session.workspace_context.main_files
        );

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: context.relevant_files.clone(),
        })
    }

    async fn handle_general_chat(
        &self,
        message: &str,
        session: &ConversationSession,
        context: &RelevantContext,
    ) -> Result<InternalResponse> {
        info!("Handling general chat request");

        let prompt = self.build_general_chat_prompt(message, session, context);
        let response = self.generate_ai_response(&prompt, "general").await?;

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: vec![],
        })
    }

    async fn handle_general_debugging_advice(
        &self,
        message: &str,
        session: &ConversationSession,
    ) -> Result<InternalResponse> {
        let prompt = format!(
            "Debugging yardımı: {}\n\nProje tipi: {:?}\n\nGenel debugging tavsiyeleri ver.",
            message,
            session.workspace_context.project_type
        );

        let response = self.generate_ai_response(&prompt, "debugging").await?;

        Ok(InternalResponse {
            ai_response: response,
            code_changes: None,
            file_references: vec![],
        })
    }

    // Helper methods
    async fn generate_ai_response(&self, prompt: &str, context_type: &str) -> Result<String> {
        let system_prompt = match context_type {
            "documentation" => "Sen bir teknik yazım uzmanısın. Açık, anlaşılır ve kapsamlı dokümantasyon yazıyorsun.",
            "file_operation" => "Sen bir dosya sistemi uzmanısın. Dosya işlemlerinde güvenli ve etkili yöntemler öneriyorsun.",
            "project_setup" => "Sen bir proje kurulum uzmanısın. Adım adım, net kurulum talimatları veriyorsun.",
            "debugging" => "Sen bir debugging uzmanısın. Sistematik hata bulma ve çözme yöntemleri öneriyorsun.",
            _ => "Sen yardımcı bir AI asistanısın. Kullanıcıların sorularını net ve faydalı şekilde yanıtlıyorsun.",
        };

        let completion_request = crate::providers::CompletionRequest {
            prompt: prompt.to_string(),
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(2000),
            temperature: Some(0.3),
            system_prompt: Some(system_prompt.to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        Ok(response.text)
    }

    fn detect_target_language(&self, session: &ConversationSession, context: &RelevantContext) -> Option<String> {
        // Mevcut dosyadan dil tespit et
        if let Some(current_file) = &session.code_context.current_file {
            if let Some(extension) = std::path::Path::new(current_file).extension() {
                return match extension.to_str() {
                    Some("rs") => Some("rust".to_string()),
                    Some("js") => Some("javascript".to_string()),
                    Some("ts") => Some("typescript".to_string()),
                    Some("py") => Some("python".to_string()),
                    Some("java") => Some("java".to_string()),
                    _ => None,
                };
            }
        }

        // Proje tipinden tespit et
        session.workspace_context.project_type.clone()
    }

    fn build_documentation_prompt(&self, message: &str, session: &ConversationSession, context: &RelevantContext) -> String {
        format!(
            "Dokümantasyon isteği: {}\n\nMevcut kod:\n{}\n\nProje bağlamı: {:?}\n\nKapsamlı dokümantasyon yaz.",
            message,
            context.current_file_content.as_deref().unwrap_or("Kod bulunamadı"),
            session.workspace_context.project_type
        )
    }

    fn build_file_operation_prompt(&self, message: &str, session: &ConversationSession, context: &RelevantContext) -> String {
        format!(
            "Dosya işlemi: {}\n\nMevcut dizin: {}\n\nDizin yapısı:\n{}\n\nGüvenli dosya işlemi öner.",
            message,
            session.workspace_context.root_path,
            context.directory_structure.join("\n")
        )
    }

    fn build_general_chat_prompt(&self, message: &str, session: &ConversationSession, context: &RelevantContext) -> String {
        let mut prompt_parts = vec![format!("Kullanıcı sorusu: {}", message)];

        if let Some(project_type) = &session.workspace_context.project_type {
            prompt_parts.push(format!("Proje tipi: {}", project_type));
        }

        if let Some(last_conversation) = &context.last_conversation {
            prompt_parts.push(format!("Son konuşma bağlamı: {}", last_conversation.user_message));
        }

        prompt_parts.push("Yardımcı ve bilgilendirici bir yanıt ver.".to_string());

        prompt_parts.join("\n\n")
    }

    async fn generate_suggested_actions(
        &self,
        intent: &MessageIntent,
        response: &InternalResponse,
        session: &ConversationSession,
    ) -> Result<Vec<SuggestedAction>> {
        let mut actions = Vec::new();

        match intent {
            MessageIntent::CodeGeneration => {
                if response.code_changes.is_some() {
                    actions.push(SuggestedAction {
                        action_type: ActionType::CreateFile,
                        description: "Oluşturulan kodu dosyaya kaydet".to_string(),
                        command: None,
                        file_path: session.code_context.current_file.clone(),
                        priority: ActionPriority::High,
                    });
                }
                actions.push(SuggestedAction {
                    action_type: ActionType::RunTest,
                    description: "Kod için test yaz".to_string(),
                    command: None,
                    file_path: None,
                    priority: ActionPriority::Medium,
                });
            }
            MessageIntent::Testing => {
                actions.push(SuggestedAction {
                    action_type: ActionType::RunCommand,
                    description: "Testleri çalıştır".to_string(),
                    command: Some("cargo test".to_string()),
                    file_path: None,
                    priority: ActionPriority::High,
                });
            }
            MessageIntent::Debugging => {
                actions.push(SuggestedAction {
                    action_type: ActionType::RunCommand,
                    description: "Kodu derle ve çalıştır".to_string(),
                    command: Some("cargo run".to_string()),
                    file_path: None,
                    priority: ActionPriority::High,
                });
            }
            _ => {}
        }

        Ok(actions)
    }

    // Public API methods
    pub async fn get_user_sessions(&self, user_id: Uuid, limit: i64) -> Result<Vec<ConversationSession>> {
        self.session_manager.get_user_sessions(user_id, limit).await
    }

    pub async fn search_conversations(&self, user_id: Uuid, query: &str, limit: i64) -> Result<Vec<ConversationTurn>> {
        self.session_manager.search_conversations(user_id, query, limit).await
    }

    pub async fn get_conversation_statistics(&self, user_id: Uuid) -> Result<super::session_manager::ConversationStatistics> {
        self.session_manager.get_conversation_statistics(user_id).await
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        self.session_manager.delete_session(session_id).await
    }
}

#[derive(Debug, Clone)]
struct InternalResponse {
    ai_response: String,
    code_changes: Option<Vec<super::CodeChange>>,
    file_references: Vec<String>,
}