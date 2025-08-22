use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};

#[derive(Debug, Clone)]
pub struct RealTimeCollaborationHub {
    sessions: Arc<RwLock<HashMap<String, CollaborationSession>>>,
    users: Arc<RwLock<HashMap<String, ConnectedUser>>>,
    code_sharing: Arc<RwLock<HashMap<String, SharedCodeSession>>>,
    conflict_resolver: ConflictResolver,
    activity_tracker: ActivityTracker,
    event_broadcaster: broadcast::Sender<CollaborationEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub session_id: String,
    pub project_id: String,
    pub participants: Vec<String>,
    pub shared_files: HashMap<String, SharedFile>,
    pub active_cursors: HashMap<String, CursorPosition>,
    pub chat_messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub last_activity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub current_session: Option<String>,
    pub status: UserStatus,
    pub permissions: UserPermissions,
    pub connected_at: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserStatus {
    Online,
    Away,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    pub can_edit: bool,
    pub can_comment: bool,
    pub can_share: bool,
    pub can_invite: bool,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFile {
    pub file_id: String,
    pub file_path: String,
    pub content: String,
    pub language: String,
    pub last_modified_by: String,
    pub last_modified_at: u64,
    pub version: u32,
    pub edit_history: Vec<EditOperation>,
    pub active_editors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    pub operation_id: String,
    pub user_id: String,
    pub operation_type: OperationType,
    pub position: TextPosition,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert,
    Delete,
    Replace,
    Move,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPosition {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub user_id: String,
    pub file_id: String,
    pub position: TextPosition,
    pub selection_start: Option<TextPosition>,
    pub selection_end: Option<TextPosition>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: String,
    pub user_id: String,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub thread_id: Option<String>,
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Code,
    File,
    System,
    AIAssistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedCodeSession {
    pub session_id: String,
    pub code: String,
    pub language: String,
    pub participants: Vec<String>,
    pub execution_results: Vec<ExecutionResult>,
    pub ai_suggestions: Vec<AISuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub code: String,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub executed_by: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISuggestion {
    pub suggestion_id: String,
    pub suggestion_type: SuggestionType,
    pub content: String,
    pub confidence: f32,
    pub applied: bool,
    pub applied_by: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    CodeCompletion,
    BugFix,
    Optimization,
    Refactoring,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationEvent {
    pub event_id: String,
    pub event_type: EventType,
    pub session_id: String,
    pub user_id: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    UserJoined,
    UserLeft,
    FileOpened,
    FileEdited,
    CursorMoved,
    ChatMessage,
    ConflictDetected,
    ConflictResolved,
    AIAssistance,
    CodeExecution,
}

#[derive(Debug, Clone)]
pub struct ConflictResolver {
    pending_conflicts: Arc<RwLock<HashMap<String, Conflict>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub conflict_id: String,
    pub file_id: String,
    pub conflicting_operations: Vec<EditOperation>,
    pub resolution_strategy: ResolutionStrategy,
    pub resolved: bool,
    pub resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    LastWriteWins,
    MergeChanges,
    UserChoice,
    AIMediated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub resolution_id: String,
    pub resolved_content: String,
    pub resolved_by: String,
    pub resolution_method: ResolutionStrategy,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ActivityTracker {
    activities: Arc<RwLock<Vec<ActivityEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub activity_id: String,
    pub user_id: String,
    pub activity_type: ActivityType,
    pub description: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    CodeEdit,
    FileShare,
    Comment,
    Review,
    Execution,
    AIInteraction,
}

impl RealTimeCollaborationHub {
    pub fn new() -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000);
        
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            code_sharing: Arc::new(RwLock::new(HashMap::new())),
            conflict_resolver: ConflictResolver::new(),
            activity_tracker: ActivityTracker::new(),
            event_broadcaster,
        }
    }

    pub async fn create_session(&self, project_id: String, creator_id: String) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let session = CollaborationSession {
            session_id: session_id.clone(),
            project_id,
            participants: vec![creator_id.clone()],
            shared_files: HashMap::new(),
            active_cursors: HashMap::new(),
            chat_messages: Vec::new(),
            created_at: now,
            last_activity: now,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        // Broadcast session creation event
        let event = CollaborationEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: EventType::UserJoined,
            session_id: session_id.clone(),
            user_id: creator_id,
            data: serde_json::json!({"action": "session_created"}),
            timestamp: now,
        };

        let _ = self.event_broadcaster.send(event);

        Ok(session_id)
    }

    pub async fn join_session(&self, session_id: String, user_id: String) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            if !session.participants.contains(&user_id) {
                session.participants.push(user_id.clone());
                session.last_activity = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();

                // Broadcast user joined event
                let event = CollaborationEvent {
                    event_id: Uuid::new_v4().to_string(),
                    event_type: EventType::UserJoined,
                    session_id: session_id.clone(),
                    user_id: user_id.clone(),
                    data: serde_json::json!({"action": "user_joined"}),
                    timestamp: session.last_activity,
                };

                let _ = self.event_broadcaster.send(event);
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    pub async fn share_file(&self, session_id: String, file_path: String, content: String, language: String, user_id: String) -> Result<String> {
        let file_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let shared_file = SharedFile {
            file_id: file_id.clone(),
            file_path: file_path.clone(),
            content,
            language,
            last_modified_by: user_id.clone(),
            last_modified_at: now,
            version: 1,
            edit_history: Vec::new(),
            active_editors: vec![user_id.clone()],
        };

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.shared_files.insert(file_id.clone(), shared_file);
            session.last_activity = now;

            // Broadcast file shared event
            let event = CollaborationEvent {
                event_id: Uuid::new_v4().to_string(),
                event_type: EventType::FileOpened,
                session_id: session_id.clone(),
                user_id,
                data: serde_json::json!({
                    "file_id": file_id,
                    "file_path": file_path,
                    "action": "file_shared"
                }),
                timestamp: now,
            };

            let _ = self.event_broadcaster.send(event);

            Ok(file_id)
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    pub async fn apply_edit_operation(&self, session_id: String, file_id: String, operation: EditOperation) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            if let Some(file) = session.shared_files.get_mut(&file_id) {
                // Check for conflicts
                let conflict = self.conflict_resolver.detect_conflict(&file.edit_history, &operation).await?;
                
                if let Some(conflict) = conflict {
                    // Handle conflict
                    self.handle_conflict(session_id.clone(), conflict).await?;
                } else {
                    // Apply operation
                    file.content = self.apply_operation_to_content(&file.content, &operation)?;
                    file.edit_history.push(operation.clone());
                    file.version += 1;
                    file.last_modified_by = operation.user_id.clone();
                    file.last_modified_at = operation.timestamp;

                    // Broadcast edit event
                    let event = CollaborationEvent {
                        event_id: Uuid::new_v4().to_string(),
                        event_type: EventType::FileEdited,
                        session_id: session_id.clone(),
                        user_id: operation.user_id,
                        data: serde_json::json!({
                            "file_id": file_id,
                            "operation": operation,
                            "new_version": file.version
                        }),
                        timestamp: operation.timestamp,
                    };

                    let _ = self.event_broadcaster.send(event);
                }
            }
        }

        Ok(())
    }

    async fn handle_conflict(&self, session_id: String, conflict: Conflict) -> Result<()> {
        // Broadcast conflict detected event
        let event = CollaborationEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: EventType::ConflictDetected,
            session_id: session_id.clone(),
            user_id: "system".to_string(),
            data: serde_json::json!({
                "conflict_id": conflict.conflict_id,
                "file_id": conflict.file_id,
                "strategy": conflict.resolution_strategy
            }),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        let _ = self.event_broadcaster.send(event);

        // Attempt automatic resolution based on strategy
        match conflict.resolution_strategy {
            ResolutionStrategy::AIMediated => {
                self.resolve_conflict_with_ai(&conflict).await?;
            }
            ResolutionStrategy::MergeChanges => {
                self.merge_conflicting_changes(&conflict).await?;
            }
            _ => {
                // Store conflict for manual resolution
                self.conflict_resolver.store_conflict(conflict).await?;
            }
        }

        Ok(())
    }

    async fn resolve_conflict_with_ai(&self, conflict: &Conflict) -> Result<()> {
        // This would integrate with AI to resolve conflicts
        // For now, just log the conflict
        tracing::info!("AI-mediated conflict resolution for conflict: {}", conflict.conflict_id);
        Ok(())
    }

    async fn merge_conflicting_changes(&self, conflict: &Conflict) -> Result<()> {
        // Implement three-way merge algorithm
        tracing::info!("Merging conflicting changes for conflict: {}", conflict.conflict_id);
        Ok(())
    }

    fn apply_operation_to_content(&self, content: &str, operation: &EditOperation) -> Result<String> {
        let mut result = content.to_string();
        
        match operation.operation_type {
            OperationType::Insert => {
                let offset = operation.position.offset as usize;
                if offset <= result.len() {
                    result.insert_str(offset, &operation.content);
                }
            }
            OperationType::Delete => {
                let start = operation.position.offset as usize;
                let end = start + operation.content.len();
                if start < result.len() && end <= result.len() {
                    result.drain(start..end);
                }
            }
            OperationType::Replace => {
                let start = operation.position.offset as usize;
                let end = start + operation.content.len();
                if start < result.len() {
                    result.replace_range(start..end.min(result.len()), &operation.content);
                }
            }
            OperationType::Move => {
                // Implement move operation
                tracing::warn!("Move operation not yet implemented");
            }
        }

        Ok(result)
    }

    pub async fn update_cursor_position(&self, session_id: String, cursor: CursorPosition) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.active_cursors.insert(cursor.user_id.clone(), cursor.clone());
            session.last_activity = cursor.timestamp;

            // Broadcast cursor movement
            let event = CollaborationEvent {
                event_id: Uuid::new_v4().to_string(),
                event_type: EventType::CursorMoved,
                session_id: session_id.clone(),
                user_id: cursor.user_id,
                data: serde_json::json!(cursor),
                timestamp: cursor.timestamp,
            };

            let _ = self.event_broadcaster.send(event);
        }

        Ok(())
    }

    pub async fn send_chat_message(&self, session_id: String, message: ChatMessage) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.chat_messages.push(message.clone());
            session.last_activity = message.timestamp;

            // Broadcast chat message
            let event = CollaborationEvent {
                event_id: Uuid::new_v4().to_string(),
                event_type: EventType::ChatMessage,
                session_id: session_id.clone(),
                user_id: message.user_id,
                data: serde_json::json!(message),
                timestamp: message.timestamp,
            };

            let _ = self.event_broadcaster.send(event);
        }

        Ok(())
    }

    pub async fn get_session_info(&self, session_id: String) -> Result<CollaborationSession> {
        let sessions = self.sessions.read().await;
        
        sessions.get(&session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
    }

    pub async fn get_active_sessions(&self, user_id: String) -> Result<Vec<CollaborationSession>> {
        let sessions = self.sessions.read().await;
        
        let user_sessions: Vec<CollaborationSession> = sessions
            .values()
            .filter(|session| session.participants.contains(&user_id))
            .cloned()
            .collect();

        Ok(user_sessions)
    }

    pub fn subscribe_to_events(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.event_broadcaster.subscribe()
    }
}

impl ConflictResolver {
    fn new() -> Self {
        Self {
            pending_conflicts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn detect_conflict(&self, edit_history: &[EditOperation], new_operation: &EditOperation) -> Result<Option<Conflict>> {
        // Simple conflict detection based on overlapping positions
        for existing_op in edit_history.iter().rev().take(10) { // Check last 10 operations
            if self.operations_conflict(existing_op, new_operation) {
                let conflict = Conflict {
                    conflict_id: Uuid::new_v4().to_string(),
                    file_id: "file_id".to_string(), // Would be passed as parameter
                    conflicting_operations: vec![existing_op.clone(), new_operation.clone()],
                    resolution_strategy: ResolutionStrategy::AIMediated,
                    resolved: false,
                    resolution: None,
                };
                
                return Ok(Some(conflict));
            }
        }

        Ok(None)
    }

    fn operations_conflict(&self, op1: &EditOperation, op2: &EditOperation) -> bool {
        // Check if operations overlap in position and are from different users
        if op1.user_id == op2.user_id {
            return false;
        }

        let op1_end = op1.position.offset + op1.content.len() as u32;
        let op2_end = op2.position.offset + op2.content.len() as u32;

        // Check for overlap
        !(op1_end <= op2.position.offset || op2_end <= op1.position.offset)
    }

    async fn store_conflict(&self, conflict: Conflict) -> Result<()> {
        let mut conflicts = self.pending_conflicts.write().await;
        conflicts.insert(conflict.conflict_id.clone(), conflict);
        Ok(())
    }
}

impl ActivityTracker {
    fn new() -> Self {
        Self {
            activities: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn track_activity(&self, activity: ActivityEvent) -> Result<()> {
        let mut activities = self.activities.write().await;
        activities.push(activity);
        
        // Keep only last 1000 activities
        if activities.len() > 1000 {
            activities.drain(0..activities.len() - 1000);
        }

        Ok(())
    }

    pub async fn get_recent_activities(&self, limit: usize) -> Result<Vec<ActivityEvent>> {
        let activities = self.activities.read().await;
        let recent: Vec<ActivityEvent> = activities
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();

        Ok(recent)
    }
}