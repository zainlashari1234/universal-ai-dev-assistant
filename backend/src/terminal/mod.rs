pub mod ai_terminal;
pub mod command_suggester;
pub mod history_manager;
pub mod shell_integration;
pub mod safety_checker;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workspace_path: Option<String>,
    pub command_history: Vec<CommandEntry>,
    pub context: TerminalContext,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub id: Uuid,
    pub command: String,
    pub output: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub ai_suggested: bool,
    pub safety_level: SafetyLevel,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalContext {
    pub current_directory: String,
    pub environment_vars: HashMap<String, String>,
    pub git_status: Option<GitStatus>,
    pub project_type: Option<String>,
    pub recent_files: Vec<String>,
    pub active_processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub has_changes: bool,
    pub ahead_commits: u32,
    pub behind_commits: u32,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyLevel {
    Safe,
    Caution,
    Dangerous,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSuggestion {
    pub command: String,
    pub explanation: String,
    pub confidence: f32,
    pub safety_level: SafetyLevel,
    pub category: CommandCategory,
    pub estimated_time: Option<u32>, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandCategory {
    FileSystem,
    Git,
    Development,
    System,
    Network,
    Database,
    Docker,
    Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalRequest {
    pub session_id: Option<Uuid>,
    pub query: String,
    pub query_type: QueryType,
    pub context: Option<TerminalContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    NaturalLanguage,
    CommandExecution,
    CommandExplanation,
    HistorySearch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalResponse {
    pub session_id: Uuid,
    pub suggestions: Vec<CommandSuggestion>,
    pub execution_result: Option<CommandExecutionResult>,
    pub explanation: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub command: String,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: i32,
    pub execution_time_ms: u64,
}

impl TerminalSession {
    pub fn new(user_id: Uuid, workspace_path: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            workspace_path: workspace_path.clone(),
            command_history: Vec::new(),
            context: TerminalContext::new(workspace_path),
            created_at: Utc::now(),
            last_activity: Utc::now(),
        }
    }

    pub fn add_command(&mut self, entry: CommandEntry) {
        self.command_history.push(entry);
        self.last_activity = Utc::now();
        
        // Keep only last 1000 commands
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }
    }

    pub fn get_recent_commands(&self, limit: usize) -> Vec<&CommandEntry> {
        self.command_history
            .iter()
            .rev()
            .take(limit)
            .collect()
    }

    pub fn search_history(&self, query: &str) -> Vec<&CommandEntry> {
        self.command_history
            .iter()
            .filter(|entry| {
                entry.command.to_lowercase().contains(&query.to_lowercase()) ||
                entry.output.to_lowercase().contains(&query.to_lowercase())
            })
            .collect()
    }
}

impl TerminalContext {
    pub fn new(workspace_path: Option<String>) -> Self {
        let current_directory = workspace_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string());

        Self {
            current_directory,
            environment_vars: std::env::vars().collect(),
            git_status: None,
            project_type: None,
            recent_files: Vec::new(),
            active_processes: Vec::new(),
        }
    }

    pub async fn update_git_status(&mut self) -> anyhow::Result<()> {
        // Git durumunu güncelle
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["status", "--porcelain", "-b"])
            .current_dir(&self.current_directory)
            .output()
            .await
        {
            if output.status.success() {
                let status_output = String::from_utf8_lossy(&output.stdout);
                self.git_status = Some(self.parse_git_status(&status_output));
            }
        }
        Ok(())
    }

    fn parse_git_status(&self, output: &str) -> GitStatus {
        let lines: Vec<&str> = output.lines().collect();
        let mut branch = "main".to_string();
        let mut modified_files = Vec::new();
        let mut has_changes = false;

        for line in lines {
            if line.starts_with("##") {
                // Branch bilgisi
                if let Some(branch_info) = line.strip_prefix("## ") {
                    branch = branch_info.split("...").next()
                        .unwrap_or("main")
                        .to_string();
                }
            } else if !line.trim().is_empty() {
                // Değiştirilmiş dosyalar
                has_changes = true;
                if let Some(file) = line.get(3..) {
                    modified_files.push(file.to_string());
                }
            }
        }

        GitStatus {
            branch,
            has_changes,
            ahead_commits: 0, // Bu bilgiyi ayrı bir komutla alacağız
            behind_commits: 0,
            modified_files,
        }
    }

    pub fn detect_project_type(&mut self) {
        let current_path = std::path::Path::new(&self.current_directory);
        
        if current_path.join("Cargo.toml").exists() {
            self.project_type = Some("rust".to_string());
        } else if current_path.join("package.json").exists() {
            self.project_type = Some("node".to_string());
        } else if current_path.join("requirements.txt").exists() || current_path.join("pyproject.toml").exists() {
            self.project_type = Some("python".to_string());
        } else if current_path.join("pom.xml").exists() {
            self.project_type = Some("java".to_string());
        } else if current_path.join("go.mod").exists() {
            self.project_type = Some("go".to_string());
        }
    }
}

impl Default for SafetyLevel {
    fn default() -> Self {
        SafetyLevel::Safe
    }
}