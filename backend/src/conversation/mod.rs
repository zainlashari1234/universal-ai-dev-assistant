pub mod context_manager;
pub mod code_integration;
pub mod session_manager;
pub mod intent_analyzer;
pub mod workspace_analyzer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workspace_context: WorkspaceContext,
    pub conversation_history: Vec<ConversationTurn>,
    pub active_files: Vec<String>,
    pub code_context: CodeContext,
    pub session_metadata: SessionMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: Uuid,
    pub user_message: String,
    pub ai_response: String,
    pub intent: MessageIntent,
    pub code_changes: Option<Vec<CodeChange>>,
    pub files_referenced: Vec<String>,
    pub confidence_score: f32,
    pub execution_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub root_path: String,
    pub project_type: Option<String>,
    pub main_files: Vec<String>,
    pub git_info: Option<GitInfo>,
    pub dependencies: Vec<Dependency>,
    pub recent_changes: Vec<FileChange>,
    pub build_system: Option<BuildSystem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    pub current_file: Option<String>,
    pub selected_text: Option<TextSelection>,
    pub cursor_position: Option<Position>,
    pub open_files: Vec<OpenFile>,
    pub recent_functions: Vec<FunctionInfo>,
    pub imports: Vec<ImportInfo>,
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_type: SessionType,
    pub language: String,
    pub preferences: UserPreferences,
    pub active_tools: Vec<String>,
    pub collaboration_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageIntent {
    CodeGeneration,
    CodeExplanation,
    CodeReview,
    Debugging,
    Refactoring,
    Testing,
    Documentation,
    FileOperation,
    ProjectSetup,
    GeneralChat,
    TerminalCommand,
    WorkspaceNavigation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Development,
    CodeReview,
    Learning,
    Debugging,
    ProjectPlanning,
    Pair Programming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub old_content: Option<String>,
    pub new_content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
    Rename,
    Move,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSelection {
    pub start: Position,
    pub end: Position,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFile {
    pub path: String,
    pub language: String,
    pub content_preview: String,
    pub last_modified: DateTime<Utc>,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub file_path: String,
    pub line_number: usize,
    pub signature: String,
    pub doc_string: Option<String>,
    pub complexity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub module: String,
    pub alias: Option<String>,
    pub items: Vec<String>,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: String,
    pub line_number: usize,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Variable,
    Constant,
    Interface,
    Enum,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub commit_hash: String,
    pub has_uncommitted_changes: bool,
    pub remote_url: Option<String>,
    pub last_commit_message: String,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub dependency_type: DependencyType,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Runtime,
    Development,
    Build,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub file_path: String,
    pub change_type: String,
    pub timestamp: DateTime<Utc>,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,
    Npm,
    Maven,
    Gradle,
    Make,
    CMake,
    Poetry,
    Pip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub code_style: String,
    pub preferred_language: String,
    pub explanation_level: ExplanationLevel,
    pub auto_format: bool,
    pub show_suggestions: bool,
    pub enable_ai_completion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExplanationLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRequest {
    pub session_id: Option<Uuid>,
    pub message: String,
    pub workspace_path: Option<String>,
    pub current_file: Option<String>,
    pub selected_text: Option<TextSelection>,
    pub context_files: Vec<String>,
    pub intent_hint: Option<MessageIntent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub session_id: Uuid,
    pub ai_response: String,
    pub intent: MessageIntent,
    pub confidence_score: f32,
    pub code_changes: Option<Vec<CodeChange>>,
    pub suggested_actions: Vec<SuggestedAction>,
    pub file_references: Vec<String>,
    pub follow_up_questions: Vec<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: ActionType,
    pub description: String,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub priority: ActionPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    RunCommand,
    OpenFile,
    CreateFile,
    ModifyFile,
    RunTest,
    InstallDependency,
    GitOperation,
    FormatCode,
    RefactorCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl ConversationSession {
    pub fn new(user_id: Uuid, workspace_path: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            workspace_context: WorkspaceContext::new(workspace_path),
            conversation_history: Vec::new(),
            active_files: Vec::new(),
            code_context: CodeContext::default(),
            session_metadata: SessionMetadata::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_turn(&mut self, turn: ConversationTurn) {
        self.conversation_history.push(turn);
        self.updated_at = Utc::now();
        
        // Son 100 turn'ü tut
        if self.conversation_history.len() > 100 {
            self.conversation_history.remove(0);
        }
    }

    pub fn get_recent_context(&self, limit: usize) -> Vec<&ConversationTurn> {
        self.conversation_history
            .iter()
            .rev()
            .take(limit)
            .collect()
    }

    pub fn update_workspace_context(&mut self, context: WorkspaceContext) {
        self.workspace_context = context;
        self.updated_at = Utc::now();
    }

    pub fn add_active_file(&mut self, file_path: String) {
        if !self.active_files.contains(&file_path) {
            self.active_files.push(file_path);
        }
        
        // Son 20 dosyayı tut
        if self.active_files.len() > 20 {
            self.active_files.remove(0);
        }
    }
}

impl WorkspaceContext {
    pub fn new(root_path: Option<String>) -> Self {
        Self {
            root_path: root_path.unwrap_or_else(|| ".".to_string()),
            project_type: None,
            main_files: Vec::new(),
            git_info: None,
            dependencies: Vec::new(),
            recent_changes: Vec::new(),
            build_system: None,
        }
    }
}

impl Default for CodeContext {
    fn default() -> Self {
        Self {
            current_file: None,
            selected_text: None,
            cursor_position: None,
            open_files: Vec::new(),
            recent_functions: Vec::new(),
            imports: Vec::new(),
            symbols: Vec::new(),
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            session_type: SessionType::Development,
            language: "tr".to_string(),
            preferences: UserPreferences::default(),
            active_tools: Vec::new(),
            collaboration_mode: false,
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            code_style: "standard".to_string(),
            preferred_language: "rust".to_string(),
            explanation_level: ExplanationLevel::Intermediate,
            auto_format: true,
            show_suggestions: true,
            enable_ai_completion: true,
        }
    }
}