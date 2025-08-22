use anyhow::Result;
use std::sync::Arc;
use regex::Regex;
use tracing::{info, debug};

use crate::providers::{ProviderRouter, CompletionRequest};
use super::{MessageIntent, ConversationTurn, WorkspaceContext, CodeContext};

pub struct IntentAnalyzer {
    provider_router: Arc<ProviderRouter>,
    intent_patterns: IntentPatterns,
}

struct IntentPatterns {
    code_generation: Vec<Regex>,
    code_explanation: Vec<Regex>,
    code_review: Vec<Regex>,
    debugging: Vec<Regex>,
    refactoring: Vec<Regex>,
    testing: Vec<Regex>,
    documentation: Vec<Regex>,
    file_operation: Vec<Regex>,
    project_setup: Vec<Regex>,
    terminal_command: Vec<Regex>,
    workspace_navigation: Vec<Regex>,
}

impl IntentAnalyzer {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self {
            provider_router,
            intent_patterns: IntentPatterns::new(),
        }
    }

    pub async fn analyze_intent(
        &self,
        message: &str,
        workspace_context: &WorkspaceContext,
        code_context: &CodeContext,
        conversation_history: &[ConversationTurn],
    ) -> Result<(MessageIntent, f32)> {
        // Önce pattern-based hızlı analiz
        if let Some((intent, confidence)) = self.pattern_based_analysis(message) {
            if confidence > 0.8 {
                debug!("High confidence pattern match: {:?} ({})", intent, confidence);
                return Ok((intent, confidence));
            }
        }

        // AI-based derin analiz
        let ai_result = self.ai_based_analysis(
            message,
            workspace_context,
            code_context,
            conversation_history,
        ).await?;

        Ok(ai_result)
    }

    fn pattern_based_analysis(&self, message: &str) -> Option<(MessageIntent, f32)> {
        let message_lower = message.to_lowercase();

        // Code Generation patterns
        for pattern in &self.intent_patterns.code_generation {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::CodeGeneration, 0.85));
            }
        }

        // Code Explanation patterns
        for pattern in &self.intent_patterns.code_explanation {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::CodeExplanation, 0.9));
            }
        }

        // Debugging patterns
        for pattern in &self.intent_patterns.debugging {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::Debugging, 0.85));
            }
        }

        // Terminal Command patterns
        for pattern in &self.intent_patterns.terminal_command {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::TerminalCommand, 0.9));
            }
        }

        // File Operation patterns
        for pattern in &self.intent_patterns.file_operation {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::FileOperation, 0.85));
            }
        }

        // Code Review patterns
        for pattern in &self.intent_patterns.code_review {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::CodeReview, 0.8));
            }
        }

        // Refactoring patterns
        for pattern in &self.intent_patterns.refactoring {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::Refactoring, 0.8));
            }
        }

        // Testing patterns
        for pattern in &self.intent_patterns.testing {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::Testing, 0.85));
            }
        }

        // Documentation patterns
        for pattern in &self.intent_patterns.documentation {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::Documentation, 0.8));
            }
        }

        // Project Setup patterns
        for pattern in &self.intent_patterns.project_setup {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::ProjectSetup, 0.85));
            }
        }

        // Workspace Navigation patterns
        for pattern in &self.intent_patterns.workspace_navigation {
            if pattern.is_match(&message_lower) {
                return Some((MessageIntent::WorkspaceNavigation, 0.8));
            }
        }

        None
    }

    async fn ai_based_analysis(
        &self,
        message: &str,
        workspace_context: &WorkspaceContext,
        code_context: &CodeContext,
        conversation_history: &[ConversationTurn],
    ) -> Result<(MessageIntent, f32)> {
        let context_info = self.build_context_info(workspace_context, code_context, conversation_history);
        
        let prompt = format!(
            r#"Kullanıcının mesajının intent'ini analiz et ve en uygun kategoriyi belirle.

Kullanıcı mesajı: "{}"

Mevcut bağlam:
{}

Mümkün intent kategorileri:
1. CodeGeneration - Kod yazma, oluşturma istekleri
2. CodeExplanation - Kod açıklama, anlama istekleri  
3. CodeReview - Kod inceleme, gözden geçirme
4. Debugging - Hata bulma, düzeltme
5. Refactoring - Kod yeniden düzenleme
6. Testing - Test yazma, test çalıştırma
7. Documentation - Dokümantasyon yazma
8. FileOperation - Dosya işlemleri (oluştur, sil, taşı)
9. ProjectSetup - Proje kurulumu, yapılandırma
10. TerminalCommand - Terminal komut çalıştırma
11. WorkspaceNavigation - Dosya/klasör gezinme
12. GeneralChat - Genel sohbet

Sadece kategori adını ve güven skorunu (0.0-1.0) döndür.
Format: KATEGORI:SKOR

Örnek: CodeGeneration:0.85"#,
            message,
            context_info
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(50),
            temperature: Some(0.1),
            system_prompt: Some("Sen bir intent analiz uzmanısın. Kullanıcı mesajlarının amacını doğru şekilde kategorize ediyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let result = self.parse_ai_response(&response.text)?;

        info!("AI intent analysis: {:?} (confidence: {})", result.0, result.1);
        Ok(result)
    }

    fn build_context_info(
        &self,
        workspace_context: &WorkspaceContext,
        code_context: &CodeContext,
        conversation_history: &[ConversationTurn],
    ) -> String {
        let mut context_parts = Vec::new();

        // Workspace bilgisi
        context_parts.push(format!("Proje tipi: {:?}", workspace_context.project_type));
        context_parts.push(format!("Ana dosyalar: {:?}", workspace_context.main_files));

        // Kod bağlamı
        if let Some(current_file) = &code_context.current_file {
            context_parts.push(format!("Mevcut dosya: {}", current_file));
        }

        if let Some(selected_text) = &code_context.selected_text {
            context_parts.push(format!("Seçili metin var: {} karakter", selected_text.text.len()));
        }

        // Son konuşma bağlamı
        if let Some(last_turn) = conversation_history.last() {
            context_parts.push(format!("Son intent: {:?}", last_turn.intent));
        }

        context_parts.join("\n")
    }

    fn parse_ai_response(&self, response: &str) -> Result<(MessageIntent, f32)> {
        let response = response.trim();
        
        if let Some(colon_pos) = response.find(':') {
            let intent_str = &response[..colon_pos];
            let score_str = &response[colon_pos + 1..];
            
            let intent = match intent_str {
                "CodeGeneration" => MessageIntent::CodeGeneration,
                "CodeExplanation" => MessageIntent::CodeExplanation,
                "CodeReview" => MessageIntent::CodeReview,
                "Debugging" => MessageIntent::Debugging,
                "Refactoring" => MessageIntent::Refactoring,
                "Testing" => MessageIntent::Testing,
                "Documentation" => MessageIntent::Documentation,
                "FileOperation" => MessageIntent::FileOperation,
                "ProjectSetup" => MessageIntent::ProjectSetup,
                "TerminalCommand" => MessageIntent::TerminalCommand,
                "WorkspaceNavigation" => MessageIntent::WorkspaceNavigation,
                _ => MessageIntent::GeneralChat,
            };
            
            let confidence = score_str.parse::<f32>().unwrap_or(0.5);
            Ok((intent, confidence.clamp(0.0, 1.0)))
        } else {
            // Fallback: sadece intent adı verilmişse
            let intent = match response {
                "CodeGeneration" => MessageIntent::CodeGeneration,
                "CodeExplanation" => MessageIntent::CodeExplanation,
                "CodeReview" => MessageIntent::CodeReview,
                "Debugging" => MessageIntent::Debugging,
                "Refactoring" => MessageIntent::Refactoring,
                "Testing" => MessageIntent::Testing,
                "Documentation" => MessageIntent::Documentation,
                "FileOperation" => MessageIntent::FileOperation,
                "ProjectSetup" => MessageIntent::ProjectSetup,
                "TerminalCommand" => MessageIntent::TerminalCommand,
                "WorkspaceNavigation" => MessageIntent::WorkspaceNavigation,
                _ => MessageIntent::GeneralChat,
            };
            Ok((intent, 0.7))
        }
    }

    pub fn get_intent_suggestions(&self, intent: &MessageIntent) -> Vec<String> {
        match intent {
            MessageIntent::CodeGeneration => vec![
                "Hangi dilde kod yazayım?".to_string(),
                "Kod örnekleri görmek ister misiniz?".to_string(),
                "Test kodları da ekleyeyim mi?".to_string(),
            ],
            MessageIntent::CodeExplanation => vec![
                "Hangi kısmını açıklayayım?".to_string(),
                "Adım adım açıklama ister misiniz?".to_string(),
                "Örneklerle açıklayayım mı?".to_string(),
            ],
            MessageIntent::Debugging => vec![
                "Hata mesajını paylaşır mısınız?".to_string(),
                "Hangi durumda hata oluyor?".to_string(),
                "Log dosyalarını kontrol edelim mi?".to_string(),
            ],
            MessageIntent::Testing => vec![
                "Unit test mi integration test mi?".to_string(),
                "Hangi test framework'ü kullanayım?".to_string(),
                "Mock'lar gerekli mi?".to_string(),
            ],
            MessageIntent::Refactoring => vec![
                "Hangi kısmı refactor edelim?".to_string(),
                "Performans odaklı mı yoksa okunabilirlik mi?".to_string(),
                "Design pattern kullanayım mı?".to_string(),
            ],
            _ => vec![
                "Nasıl yardımcı olabilirim?".to_string(),
                "Daha detay verebilir misiniz?".to_string(),
            ],
        }
    }
}

impl IntentPatterns {
    fn new() -> Self {
        Self {
            code_generation: vec![
                Regex::new(r"\b(yaz|oluştur|kod|fonksiyon|sınıf|method)\b").unwrap(),
                Regex::new(r"\b(create|write|generate|implement|build)\b").unwrap(),
                Regex::new(r"\b(nasıl.*yaz|nasıl.*oluştur)\b").unwrap(),
                Regex::new(r"\b(bir.*yaz|bir.*oluştur)\b").unwrap(),
            ],
            
            code_explanation: vec![
                Regex::new(r"\b(açıkla|anlat|nedir|ne yapar|nasıl çalışır)\b").unwrap(),
                Regex::new(r"\b(explain|what does|how does|what is)\b").unwrap(),
                Regex::new(r"\b(bu kod|bu fonksiyon|bu sınıf).*\b(ne|nasıl)\b").unwrap(),
                Regex::new(r"\?(.*)(açıkla|anlat)").unwrap(),
            ],
            
            debugging: vec![
                Regex::new(r"\b(hata|bug|sorun|çalışmıyor|error)\b").unwrap(),
                Regex::new(r"\b(debug|fix|solve|problem|issue)\b").unwrap(),
                Regex::new(r"\b(neden.*çalışmıyor|niye.*hata)\b").unwrap(),
                Regex::new(r"\b(exception|crash|fail)\b").unwrap(),
            ],
            
            terminal_command: vec![
                Regex::new(r"\b(komut|terminal|shell|bash|cmd)\b").unwrap(),
                Regex::new(r"\b(çalıştır|run|execute)\b").unwrap(),
                Regex::new(r"\b(git|npm|cargo|pip|docker)\s").unwrap(),
                Regex::new(r"^\s*(ls|cd|mkdir|rm|cp|mv)\b").unwrap(),
            ],
            
            file_operation: vec![
                Regex::new(r"\b(dosya|file|klasör|folder|directory)\b").unwrap(),
                Regex::new(r"\b(oluştur|sil|taşı|kopyala|rename)\b").unwrap(),
                Regex::new(r"\b(create|delete|move|copy|rename)\b").unwrap(),
                Regex::new(r"\b(yeni.*dosya|yeni.*klasör)\b").unwrap(),
            ],
            
            code_review: vec![
                Regex::new(r"\b(incele|gözden geçir|review|kontrol et)\b").unwrap(),
                Regex::new(r"\b(kod.*incele|kod.*review)\b").unwrap(),
                Regex::new(r"\b(kalite|quality|best practice)\b").unwrap(),
                Regex::new(r"\b(optimize|iyileştir|improve)\b").unwrap(),
            ],
            
            refactoring: vec![
                Regex::new(r"\b(refactor|yeniden.*düzenle|restructure)\b").unwrap(),
                Regex::new(r"\b(temizle|clean|organize)\b").unwrap(),
                Regex::new(r"\b(daha.*iyi|better|optimize)\b").unwrap(),
                Regex::new(r"\b(pattern|design.*pattern)\b").unwrap(),
            ],
            
            testing: vec![
                Regex::new(r"\b(test|unit.*test|integration.*test)\b").unwrap(),
                Regex::new(r"\b(test.*yaz|test.*oluştur)\b").unwrap(),
                Regex::new(r"\b(mock|stub|spy)\b").unwrap(),
                Regex::new(r"\b(assert|expect|should)\b").unwrap(),
            ],
            
            documentation: vec![
                Regex::new(r"\b(dokümantasyon|documentation|readme)\b").unwrap(),
                Regex::new(r"\b(comment|yorum|açıklama)\b").unwrap(),
                Regex::new(r"\b(doc|docs|document)\b").unwrap(),
                Regex::new(r"\b(api.*doc|javadoc|rustdoc)\b").unwrap(),
            ],
            
            project_setup: vec![
                Regex::new(r"\b(proje.*kur|project.*setup|initialize)\b").unwrap(),
                Regex::new(r"\b(config|configuration|ayar)\b").unwrap(),
                Regex::new(r"\b(install|yükle|setup)\b").unwrap(),
                Regex::new(r"\b(dependency|bağımlılık|package)\b").unwrap(),
            ],
            
            workspace_navigation: vec![
                Regex::new(r"\b(dosya.*bul|file.*find|search)\b").unwrap(),
                Regex::new(r"\b(nerede|where|locate)\b").unwrap(),
                Regex::new(r"\b(listele|list|show)\b").unwrap(),
                Regex::new(r"\b(navigate|gezin|browse)\b").unwrap(),
            ],
        }
    }
}