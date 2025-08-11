use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: TeamRole,
    pub status: MemberStatus,
    pub current_file: Option<String>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub ai_preferences: AIPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamRole {
    Developer,
    Senior,
    Lead,
    Architect,
    QA,
    DevOps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberStatus {
    Online,
    Away,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPreferences {
    pub completion_style: String,
    pub security_level: SecurityLevel,
    pub auto_review: bool,
    pub suggestion_frequency: SuggestionFrequency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Strict,
    Balanced,
    Permissive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionFrequency {
    Aggressive,
    Normal,
    Conservative,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSyncEvent {
    pub id: Uuid,
    pub event_type: SyncEventType,
    pub member_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEventType {
    FileOpened,
    FileModified,
    FileSaved,
    AIAnalysisRequested,
    SecurityIssueFound,
    PerformanceIssueFound,
    CodeReviewRequested,
    MemberJoined,
    MemberLeft,
    StatusChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInsights {
    pub productivity_metrics: ProductivityMetrics,
    pub code_quality_trends: CodeQualityTrends,
    pub collaboration_patterns: CollaborationPatterns,
    pub ai_usage_stats: AIUsageStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityMetrics {
    pub lines_of_code_per_day: HashMap<Uuid, u32>,
    pub commits_per_day: HashMap<Uuid, u32>,
    pub files_modified: HashMap<Uuid, u32>,
    pub ai_suggestions_accepted: HashMap<Uuid, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeQualityTrends {
    pub security_issues_over_time: Vec<(chrono::DateTime<chrono::Utc>, u32)>,
    pub performance_issues_over_time: Vec<(chrono::DateTime<chrono::Utc>, u32)>,
    pub complexity_trends: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
    pub test_coverage_trends: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPatterns {
    pub pair_programming_sessions: HashMap<(Uuid, Uuid), u32>,
    pub code_review_frequency: HashMap<Uuid, u32>,
    pub knowledge_sharing_events: Vec<KnowledgeSharingEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSharingEvent {
    pub teacher: Uuid,
    pub learner: Uuid,
    pub topic: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub effectiveness_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIUsageStats {
    pub completions_requested: HashMap<Uuid, u32>,
    pub analyses_performed: HashMap<Uuid, u32>,
    pub suggestions_accepted: HashMap<Uuid, u32>,
    pub time_saved_minutes: HashMap<Uuid, u32>,
}

pub struct TeamSyncManager {
    members: Arc<RwLock<HashMap<Uuid, TeamMember>>>,
    event_sender: broadcast::Sender<TeamSyncEvent>,
    insights: Arc<RwLock<TeamInsights>>,
}

impl TeamSyncManager {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            members: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            insights: Arc::new(RwLock::new(TeamInsights {
                productivity_metrics: ProductivityMetrics {
                    lines_of_code_per_day: HashMap::new(),
                    commits_per_day: HashMap::new(),
                    files_modified: HashMap::new(),
                    ai_suggestions_accepted: HashMap::new(),
                },
                code_quality_trends: CodeQualityTrends {
                    security_issues_over_time: Vec::new(),
                    performance_issues_over_time: Vec::new(),
                    complexity_trends: Vec::new(),
                    test_coverage_trends: Vec::new(),
                },
                collaboration_patterns: CollaborationPatterns {
                    pair_programming_sessions: HashMap::new(),
                    code_review_frequency: HashMap::new(),
                    knowledge_sharing_events: Vec::new(),
                },
                ai_usage_stats: AIUsageStats {
                    completions_requested: HashMap::new(),
                    analyses_performed: HashMap::new(),
                    suggestions_accepted: HashMap::new(),
                    time_saved_minutes: HashMap::new(),
                },
            })),
        }
    }

    pub async fn add_member(&self, member: TeamMember) -> Result<()> {
        let mut members = self.members.write().await;
        let member_id = member.id;
        members.insert(member_id, member);
        
        let event = TeamSyncEvent {
            id: Uuid::new_v4(),
            event_type: SyncEventType::MemberJoined,
            member_id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({"message": "New team member joined"}),
        };
        
        self.broadcast_event(event).await?;
        info!("Team member {} added", member_id);
        
        Ok(())
    }

    pub async fn remove_member(&self, member_id: Uuid) -> Result<()> {
        let mut members = self.members.write().await;
        
        if members.remove(&member_id).is_some() {
            let event = TeamSyncEvent {
                id: Uuid::new_v4(),
                event_type: SyncEventType::MemberLeft,
                member_id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"message": "Team member left"}),
            };
            
            self.broadcast_event(event).await?;
            info!("Team member {} removed", member_id);
        }
        
        Ok(())
    }

    pub async fn update_member_status(&self, member_id: Uuid, status: MemberStatus) -> Result<()> {
        let mut members = self.members.write().await;
        
        if let Some(member) = members.get_mut(&member_id) {
            member.status = status.clone();
            member.last_activity = chrono::Utc::now();
            
            let event = TeamSyncEvent {
                id: Uuid::new_v4(),
                event_type: SyncEventType::StatusChanged,
                member_id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"new_status": status}),
            };
            
            self.broadcast_event(event).await?;
        }
        
        Ok(())
    }

    pub async fn track_file_activity(&self, member_id: Uuid, file_path: String, activity: SyncEventType) -> Result<()> {
        let mut members = self.members.write().await;
        
        if let Some(member) = members.get_mut(&member_id) {
            member.current_file = Some(file_path.clone());
            member.last_activity = chrono::Utc::now();
            
            let event = TeamSyncEvent {
                id: Uuid::new_v4(),
                event_type: activity,
                member_id,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({"file_path": file_path}),
            };
            
            self.broadcast_event(event).await?;
            self.update_productivity_metrics(member_id, &activity).await?;
        }
        
        Ok(())
    }

    pub async fn track_ai_usage(&self, member_id: Uuid, usage_type: AIUsageType, time_saved: u32) -> Result<()> {
        let mut insights = self.insights.write().await;
        
        match usage_type {
            AIUsageType::Completion => {
                *insights.ai_usage_stats.completions_requested.entry(member_id).or_insert(0) += 1;
            }
            AIUsageType::Analysis => {
                *insights.ai_usage_stats.analyses_performed.entry(member_id).or_insert(0) += 1;
            }
            AIUsageType::SuggestionAccepted => {
                *insights.ai_usage_stats.suggestions_accepted.entry(member_id).or_insert(0) += 1;
            }
        }
        
        *insights.ai_usage_stats.time_saved_minutes.entry(member_id).or_insert(0) += time_saved;
        
        let event = TeamSyncEvent {
            id: Uuid::new_v4(),
            event_type: SyncEventType::AIAnalysisRequested,
            member_id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "usage_type": usage_type,
                "time_saved": time_saved
            }),
        };
        
        self.broadcast_event(event).await?;
        
        Ok(())
    }

    pub async fn get_team_insights(&self) -> Result<TeamInsights> {
        let insights = self.insights.read().await;
        Ok(insights.clone())
    }

    pub async fn get_active_members(&self) -> Result<Vec<TeamMember>> {
        let members = self.members.read().await;
        let active_members: Vec<TeamMember> = members
            .values()
            .filter(|member| matches!(member.status, MemberStatus::Online | MemberStatus::Away))
            .cloned()
            .collect();
        
        Ok(active_members)
    }

    pub async fn suggest_pair_programming(&self, member_id: Uuid) -> Result<Option<Uuid>> {
        let members = self.members.read().await;
        let insights = self.insights.read().await;
        
        if let Some(current_member) = members.get(&member_id) {
            // Find best pair programming partner based on:
            // 1. Complementary skills
            // 2. Availability
            // 3. Past collaboration success
            
            let mut best_partner = None;
            let mut best_score = 0.0;
            
            for (other_id, other_member) in members.iter() {
                if *other_id == member_id || !matches!(other_member.status, MemberStatus::Online) {
                    continue;
                }
                
                let mut score = 0.0;
                
                // Role compatibility
                score += self.calculate_role_compatibility(&current_member.role, &other_member.role);
                
                // Past collaboration success
                if let Some(past_sessions) = insights.collaboration_patterns.pair_programming_sessions.get(&(member_id, *other_id)) {
                    score += (*past_sessions as f64) * 0.1;
                }
                
                // Current file overlap (working on related files)
                if let (Some(current_file), Some(other_file)) = (&current_member.current_file, &other_member.current_file) {
                    if self.files_are_related(current_file, other_file) {
                        score += 2.0;
                    }
                }
                
                if score > best_score {
                    best_score = score;
                    best_partner = Some(*other_id);
                }
            }
            
            return Ok(best_partner);
        }
        
        Ok(None)
    }

    pub async fn create_knowledge_sharing_session(&self, teacher_id: Uuid, learner_id: Uuid, topic: String) -> Result<()> {
        let mut insights = self.insights.write().await;
        
        let event = KnowledgeSharingEvent {
            teacher: teacher_id,
            learner: learner_id,
            topic: topic.clone(),
            timestamp: chrono::Utc::now(),
            effectiveness_score: 0.0, // Will be updated later based on feedback
        };
        
        insights.collaboration_patterns.knowledge_sharing_events.push(event);
        
        let sync_event = TeamSyncEvent {
            id: Uuid::new_v4(),
            event_type: SyncEventType::CodeReviewRequested,
            member_id: teacher_id,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "type": "knowledge_sharing",
                "learner": learner_id,
                "topic": topic
            }),
        };
        
        self.broadcast_event(sync_event).await?;
        
        Ok(())
    }

    pub fn subscribe_to_events(&self) -> broadcast::Receiver<TeamSyncEvent> {
        self.event_sender.subscribe()
    }

    async fn broadcast_event(&self, event: TeamSyncEvent) -> Result<()> {
        match self.event_sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Failed to broadcast event: {}", e);
                Ok(()) // Don't fail the operation if broadcast fails
            }
        }
    }

    async fn update_productivity_metrics(&self, member_id: Uuid, activity: &SyncEventType) -> Result<()> {
        let mut insights = self.insights.write().await;
        
        match activity {
            SyncEventType::FileModified => {
                *insights.productivity_metrics.files_modified.entry(member_id).or_insert(0) += 1;
            }
            SyncEventType::FileSaved => {
                // Estimate lines of code (simplified)
                *insights.productivity_metrics.lines_of_code_per_day.entry(member_id).or_insert(0) += 10;
            }
            _ => {}
        }
        
        Ok(())
    }

    fn calculate_role_compatibility(&self, role1: &TeamRole, role2: &TeamRole) -> f64 {
        match (role1, role2) {
            (TeamRole::Developer, TeamRole::Senior) => 3.0,
            (TeamRole::Senior, TeamRole::Developer) => 3.0,
            (TeamRole::Developer, TeamRole::Lead) => 2.0,
            (TeamRole::Lead, TeamRole::Developer) => 2.0,
            (TeamRole::QA, TeamRole::Developer) => 2.5,
            (TeamRole::Developer, TeamRole::QA) => 2.5,
            (TeamRole::DevOps, TeamRole::Developer) => 2.0,
            (TeamRole::Developer, TeamRole::DevOps) => 2.0,
            _ => 1.0,
        }
    }

    fn files_are_related(&self, file1: &str, file2: &str) -> bool {
        // Simple heuristic: files in same directory or with similar names
        let path1 = std::path::Path::new(file1);
        let path2 = std::path::Path::new(file2);
        
        // Same directory
        if path1.parent() == path2.parent() {
            return true;
        }
        
        // Similar names (same stem)
        if let (Some(stem1), Some(stem2)) = (path1.file_stem(), path2.file_stem()) {
            if stem1 == stem2 {
                return true;
            }
        }
        
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIUsageType {
    Completion,
    Analysis,
    SuggestionAccepted,
}

impl Default for TeamSyncManager {
    fn default() -> Self {
        Self::new()
    }
}