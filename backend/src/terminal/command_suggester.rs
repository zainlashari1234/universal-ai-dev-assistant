use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error};
use regex::Regex;

use crate::providers::{ProviderRouter, CompletionRequest};
use super::{
    CommandSuggestion, TerminalContext, CommandEntry, SafetyLevel, 
    CommandCategory, TerminalRequest, QueryType
};

pub struct AICommandSuggester {
    provider_router: Arc<ProviderRouter>,
    safety_checker: SafetyChecker,
}

pub struct SafetyChecker {
    dangerous_patterns: Vec<Regex>,
    caution_patterns: Vec<Regex>,
}

impl AICommandSuggester {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self {
            provider_router,
            safety_checker: SafetyChecker::new(),
        }
    }

    pub async fn suggest_commands(
        &self,
        request: &TerminalRequest,
        context: &TerminalContext,
        history: &[CommandEntry],
    ) -> Result<Vec<CommandSuggestion>> {
        match request.query_type {
            QueryType::NaturalLanguage => {
                self.natural_language_to_commands(&request.query, context, history).await
            }
            QueryType::CommandExplanation => {
                self.explain_command(&request.query, context).await
            }
            QueryType::HistorySearch => {
                self.search_command_history(&request.query, history).await
            }
            QueryType::CommandExecution => {
                // Komut güvenlik kontrolü
                vec![self.analyze_command_safety(&request.query, context).await?]
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map(|v| v)
            }
        }
    }

    async fn natural_language_to_commands(
        &self,
        query: &str,
        context: &TerminalContext,
        history: &[CommandEntry],
    ) -> Result<Vec<CommandSuggestion>> {
        let prompt = self.build_nl_prompt(query, context, history);
        
        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(1000),
            temperature: Some(0.3),
            system_prompt: Some(self.get_system_prompt()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let suggestions = self.parse_ai_response(&response.text, context).await?;
        
        info!("Generated {} command suggestions for query: {}", suggestions.len(), query);
        Ok(suggestions)
    }

    fn build_nl_prompt(
        &self,
        query: &str,
        context: &TerminalContext,
        history: &[CommandEntry],
    ) -> String {
        let recent_commands = history
            .iter()
            .rev()
            .take(5)
            .map(|cmd| format!("$ {}", cmd.command))
            .collect::<Vec<_>>()
            .join("\n");

        let git_info = if let Some(git) = &context.git_status {
            format!("Git branch: {}, Changes: {}", git.branch, git.has_changes)
        } else {
            "Not a git repository".to_string()
        };

        let project_info = context.project_type
            .as_ref()
            .map(|pt| format!("Project type: {}", pt))
            .unwrap_or_else(|| "Unknown project type".to_string());

        format!(
            r#"Kullanıcı isteği: "{}"

Mevcut bağlam:
- Dizin: {}
- {}
- {}

Son komutlar:
{}

Bu isteği gerçekleştirmek için 3 farklı komut önerisi ver. Her öneri için:
1. Tam komut
2. Açıklama
3. Güven skoru (0.0-1.0)
4. Güvenlik seviyesi (safe/caution/dangerous)
5. Kategori (filesystem/git/development/system/network/database/docker/package)
6. Tahmini süre (saniye)

Format:
COMMAND: [komut]
EXPLANATION: [açıklama]
CONFIDENCE: [0.0-1.0]
SAFETY: [safe/caution/dangerous]
CATEGORY: [kategori]
TIME: [saniye]
---"#,
            query,
            context.current_directory,
            git_info,
            project_info,
            recent_commands
        )
    }

    fn get_system_prompt(&self) -> String {
        r#"Sen bir uzman sistem yöneticisi ve geliştiricisisin. Kullanıcıların doğal dil isteklerini güvenli ve etkili shell komutlarına dönüştürüyorsun.

Kurallar:
1. Her zaman güvenli komutlar öner
2. Tehlikeli komutlar için uyarı ver
3. Açıklamaları Türkçe yaz
4. Komutları mevcut bağlama uygun hale getir
5. Alternatif yaklaşımlar sun
6. Hataları önleyici öneriler ver

Tehlikeli komutlar: rm -rf, dd, mkfs, fdisk, > /dev/, sudo rm, chmod 777
Dikkatli komutlar: sudo, chmod, chown, mv, cp büyük dosyalar"#.to_string()
    }

    async fn parse_ai_response(
        &self,
        response: &str,
        context: &TerminalContext,
    ) -> Result<Vec<CommandSuggestion>> {
        let mut suggestions = Vec::new();
        let sections: Vec<&str> = response.split("---").collect();

        for section in sections {
            if let Some(suggestion) = self.parse_suggestion_section(section, context).await? {
                suggestions.push(suggestion);
            }
        }

        // Güvenlik kontrolü
        for suggestion in &mut suggestions {
            suggestion.safety_level = self.safety_checker.check_command(&suggestion.command);
        }

        Ok(suggestions)
    }

    async fn parse_suggestion_section(
        &self,
        section: &str,
        _context: &TerminalContext,
    ) -> Result<Option<CommandSuggestion>> {
        let lines: Vec<&str> = section.trim().lines().collect();
        let mut command = String::new();
        let mut explanation = String::new();
        let mut confidence = 0.8;
        let mut safety_level = SafetyLevel::Safe;
        let mut category = CommandCategory::System;
        let mut estimated_time = None;

        for line in lines {
            let line = line.trim();
            if line.starts_with("COMMAND:") {
                command = line.strip_prefix("COMMAND:").unwrap_or("").trim().to_string();
            } else if line.starts_with("EXPLANATION:") {
                explanation = line.strip_prefix("EXPLANATION:").unwrap_or("").trim().to_string();
            } else if line.starts_with("CONFIDENCE:") {
                if let Ok(conf) = line.strip_prefix("CONFIDENCE:").unwrap_or("0.8").trim().parse::<f32>() {
                    confidence = conf.clamp(0.0, 1.0);
                }
            } else if line.starts_with("SAFETY:") {
                let safety_str = line.strip_prefix("SAFETY:").unwrap_or("safe").trim();
                safety_level = match safety_str {
                    "safe" => SafetyLevel::Safe,
                    "caution" => SafetyLevel::Caution,
                    "dangerous" => SafetyLevel::Dangerous,
                    _ => SafetyLevel::Safe,
                };
            } else if line.starts_with("CATEGORY:") {
                let category_str = line.strip_prefix("CATEGORY:").unwrap_or("system").trim();
                category = match category_str {
                    "filesystem" => CommandCategory::FileSystem,
                    "git" => CommandCategory::Git,
                    "development" => CommandCategory::Development,
                    "system" => CommandCategory::System,
                    "network" => CommandCategory::Network,
                    "database" => CommandCategory::Database,
                    "docker" => CommandCategory::Docker,
                    "package" => CommandCategory::Package,
                    _ => CommandCategory::System,
                };
            } else if line.starts_with("TIME:") {
                if let Ok(time) = line.strip_prefix("TIME:").unwrap_or("0").trim().parse::<u32>() {
                    estimated_time = Some(time);
                }
            }
        }

        if command.is_empty() {
            return Ok(None);
        }

        Ok(Some(CommandSuggestion {
            command,
            explanation,
            confidence,
            safety_level,
            category,
            estimated_time,
        }))
    }

    async fn explain_command(
        &self,
        command: &str,
        context: &TerminalContext,
    ) -> Result<Vec<CommandSuggestion>> {
        let prompt = format!(
            r#"Bu komutu detaylı olarak açıkla: "{}"

Mevcut bağlam:
- Dizin: {}
- Proje tipi: {:?}

Açıklaman şunları içersin:
1. Komutun ne yaptığı
2. Parametrelerin anlamı
3. Olası riskler
4. Alternatif yaklaşımlar
5. Örnek çıktı

Açıklamayı Türkçe yaz."#,
            command,
            context.current_directory,
            context.project_type
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(800),
            temperature: Some(0.2),
            system_prompt: Some("Sen bir shell komut uzmanısın. Komutları detaylı ve anlaşılır şekilde açıklıyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let safety_level = self.safety_checker.check_command(command);

        Ok(vec![CommandSuggestion {
            command: command.to_string(),
            explanation: response.text,
            confidence: 0.9,
            safety_level,
            category: self.categorize_command(command),
            estimated_time: None,
        }])
    }

    async fn search_command_history(
        &self,
        query: &str,
        history: &[CommandEntry],
    ) -> Result<Vec<CommandSuggestion>> {
        let matching_commands: Vec<&CommandEntry> = history
            .iter()
            .filter(|entry| {
                entry.command.to_lowercase().contains(&query.to_lowercase()) ||
                entry.output.to_lowercase().contains(&query.to_lowercase())
            })
            .collect();

        let mut suggestions = Vec::new();
        for cmd in matching_commands.into_iter().take(5) {
            suggestions.push(CommandSuggestion {
                command: cmd.command.clone(),
                explanation: format!("Geçmişten: {} tarihinde çalıştırıldı", cmd.timestamp.format("%Y-%m-%d %H:%M")),
                confidence: 0.7,
                safety_level: cmd.safety_level.clone(),
                category: self.categorize_command(&cmd.command),
                estimated_time: Some((cmd.execution_time_ms / 1000) as u32),
            });
        }

        Ok(suggestions)
    }

    async fn analyze_command_safety(
        &self,
        command: &str,
        _context: &TerminalContext,
    ) -> Result<CommandSuggestion> {
        let safety_level = self.safety_checker.check_command(command);
        let explanation = match safety_level {
            SafetyLevel::Safe => "Bu komut güvenli görünüyor.".to_string(),
            SafetyLevel::Caution => "Bu komut dikkatli kullanılmalı. Etkilerini kontrol edin.".to_string(),
            SafetyLevel::Dangerous => "⚠️ TEHLİKELİ KOMUT! Bu komut sisteminize zarar verebilir.".to_string(),
            SafetyLevel::Blocked => "🚫 Bu komut güvenlik nedeniyle engellendi.".to_string(),
        };

        Ok(CommandSuggestion {
            command: command.to_string(),
            explanation,
            confidence: 0.95,
            safety_level,
            category: self.categorize_command(command),
            estimated_time: None,
        })
    }

    fn categorize_command(&self, command: &str) -> CommandCategory {
        let cmd_lower = command.to_lowercase();
        
        if cmd_lower.starts_with("git ") {
            CommandCategory::Git
        } else if cmd_lower.starts_with("docker ") || cmd_lower.starts_with("podman ") {
            CommandCategory::Docker
        } else if cmd_lower.contains("npm ") || cmd_lower.contains("yarn ") || cmd_lower.contains("pip ") || cmd_lower.contains("cargo ") {
            CommandCategory::Package
        } else if cmd_lower.contains("ls ") || cmd_lower.contains("find ") || cmd_lower.contains("grep ") || cmd_lower.contains("cat ") {
            CommandCategory::FileSystem
        } else if cmd_lower.contains("curl ") || cmd_lower.contains("wget ") || cmd_lower.contains("ping ") {
            CommandCategory::Network
        } else if cmd_lower.contains("mysql ") || cmd_lower.contains("psql ") || cmd_lower.contains("mongo ") {
            CommandCategory::Database
        } else if cmd_lower.contains("make ") || cmd_lower.contains("build ") || cmd_lower.contains("test ") {
            CommandCategory::Development
        } else {
            CommandCategory::System
        }
    }
}

impl SafetyChecker {
    pub fn new() -> Self {
        let dangerous_patterns = vec![
            Regex::new(r"rm\s+-rf\s+/").unwrap(),
            Regex::new(r"dd\s+.*of=/dev/").unwrap(),
            Regex::new(r"mkfs\.").unwrap(),
            Regex::new(r"fdisk\s+/dev/").unwrap(),
            Regex::new(r">\s*/dev/sd[a-z]").unwrap(),
            Regex::new(r"sudo\s+rm\s+-rf\s+/").unwrap(),
            Regex::new(r"chmod\s+777\s+/").unwrap(),
            Regex::new(r":\(\)\{\s*:\|:&\s*\};:").unwrap(), // fork bomb
        ];

        let caution_patterns = vec![
            Regex::new(r"sudo\s+").unwrap(),
            Regex::new(r"rm\s+-rf").unwrap(),
            Regex::new(r"chmod\s+").unwrap(),
            Regex::new(r"chown\s+").unwrap(),
            Regex::new(r"mv\s+.*\s+/").unwrap(),
            Regex::new(r"cp\s+-r\s+.*\s+/").unwrap(),
        ];

        Self {
            dangerous_patterns,
            caution_patterns,
        }
    }

    pub fn check_command(&self, command: &str) -> SafetyLevel {
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(command) {
                return SafetyLevel::Dangerous;
            }
        }

        for pattern in &self.caution_patterns {
            if pattern.is_match(command) {
                return SafetyLevel::Caution;
            }
        }

        SafetyLevel::Safe
    }
}