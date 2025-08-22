use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Agent-to-Agent Direct Communication System
/// Enables autonomous agents to communicate directly without central orchestration

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: Uuid,
    pub from_agent: AgentId,
    pub to_agent: Option<AgentId>, // None for broadcast
    pub message_type: MessageType,
    pub content: serde_json::Value,
    pub priority: Priority,
    pub timestamp: DateTime<Utc>,
    pub conversation_id: Option<Uuid>,
    pub requires_response: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId {
    pub name: String,
    pub instance_id: Uuid,
    pub agent_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Request another agent to perform a task
    TaskRequest {
        task_description: String,
        expected_output: String,
        deadline: Option<DateTime<Utc>>,
    },
    /// Response to a task request
    TaskResponse {
        request_id: Uuid,
        success: bool,
        result: serde_json::Value,
        error: Option<String>,
    },
    /// Propose collaboration on a complex task
    CollaborationProposal {
        project_description: String,
        required_skills: Vec<String>,
        estimated_duration: String,
    },
    /// Accept or decline collaboration
    CollaborationResponse {
        proposal_id: Uuid,
        accepted: bool,
        availability: Option<String>,
    },
    /// Share knowledge or insights
    KnowledgeShare {
        topic: String,
        insights: Vec<String>,
        code_examples: Option<String>,
        references: Vec<String>,
    },
    /// Ask for help or advice
    HelpRequest {
        problem_description: String,
        context: String,
        urgency: Priority,
    },
    /// Provide help or advice
    HelpResponse {
        request_id: Uuid,
        suggestions: Vec<String>,
        code_solution: Option<String>,
    },
    /// Update status or progress
    StatusUpdate {
        current_task: String,
        progress_percentage: f32,
        estimated_completion: Option<DateTime<Utc>>,
        blockers: Vec<String>,
    },
    /// Emergency or critical issue
    Emergency {
        issue_description: String,
        severity: EmergencySeverity,
        immediate_action_needed: bool,
    },
    /// Heartbeat to indicate agent is alive
    Heartbeat {
        load_percentage: f32,
        available_for_tasks: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
    Emergency = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmergencySeverity {
    Warning,
    Error,
    Critical,
    SystemFailure,
}

/// Communication Hub - manages all agent-to-agent communication
pub struct AgentCommunicationHub {
    /// All registered agents
    agents: Arc<Mutex<HashMap<AgentId, AgentInfo>>>,
    /// Message queues for each agent
    message_queues: Arc<Mutex<HashMap<AgentId, VecDeque<AgentMessage>>>>,
    /// Broadcast channel for system-wide messages
    broadcast_tx: broadcast::Sender<AgentMessage>,
    /// Message history for debugging and analysis
    message_history: Arc<Mutex<Vec<AgentMessage>>>,
    /// Active conversations
    conversations: Arc<Mutex<HashMap<Uuid, Conversation>>>,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: AgentId,
    pub capabilities: Vec<String>,
    pub current_load: f32,
    pub last_heartbeat: DateTime<Utc>,
    pub status: AgentStatus,
    pub communication_preferences: CommunicationPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Available,
    Busy,
    Offline,
    Maintenance,
}

#[derive(Debug, Clone)]
pub struct CommunicationPreferences {
    pub max_queue_size: usize,
    pub priority_threshold: Priority,
    pub auto_respond_to_heartbeat: bool,
    pub collaboration_openness: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    pub participants: Vec<AgentId>,
    pub topic: String,
    pub messages: Vec<Uuid>, // Message IDs
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: ConversationStatus,
}

#[derive(Debug, Clone)]
pub enum ConversationStatus {
    Active,
    Paused,
    Completed,
    Abandoned,
}

impl AgentCommunicationHub {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            message_queues: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
            message_history: Arc::new(Mutex::new(Vec::new())),
            conversations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new agent in the communication system
    pub async fn register_agent(&self, agent_info: AgentInfo) -> Result<()> {
        let mut agents = self.agents.lock().unwrap();
        let mut queues = self.message_queues.lock().unwrap();
        
        // Create message queue for the agent
        queues.insert(agent_info.id.clone(), VecDeque::new());
        
        // Register agent
        agents.insert(agent_info.id.clone(), agent_info.clone());
        
        // Announce new agent to all others
        let announcement = AgentMessage {
            id: Uuid::new_v4(),
            from_agent: agent_info.id.clone(),
            to_agent: None, // Broadcast
            message_type: MessageType::StatusUpdate {
                current_task: "Just joined the ecosystem".to_string(),
                progress_percentage: 0.0,
                estimated_completion: None,
                blockers: vec![],
            },
            content: serde_json::json!({
                "event": "agent_joined",
                "capabilities": agent_info.capabilities
            }),
            priority: Priority::Normal,
            timestamp: Utc::now(),
            conversation_id: None,
            requires_response: false,
            metadata: HashMap::new(),
        };
        
        self.broadcast_message(announcement).await?;
        
        Ok(())
    }

    /// Send a message from one agent to another
    pub async fn send_message(&self, message: AgentMessage) -> Result<()> {
        // Store in history
        {
            let mut history = self.message_history.lock().unwrap();
            history.push(message.clone());
        }

        if let Some(to_agent) = &message.to_agent {
            // Direct message
            self.deliver_to_agent(to_agent, message).await?;
        } else {
            // Broadcast message
            self.broadcast_message(message).await?;
        }

        Ok(())
    }

    /// Deliver message to specific agent's queue
    async fn deliver_to_agent(&self, agent_id: &AgentId, message: AgentMessage) -> Result<()> {
        let mut queues = self.message_queues.lock().unwrap();
        
        if let Some(queue) = queues.get_mut(agent_id) {
            // Check queue size limits
            let agents = self.agents.lock().unwrap();
            if let Some(agent_info) = agents.get(agent_id) {
                if queue.len() >= agent_info.communication_preferences.max_queue_size {
                    // Remove oldest low-priority message
                    if let Some(pos) = queue.iter().position(|m| m.priority <= Priority::Low) {
                        queue.remove(pos);
                    }
                }
            }
            
            // Insert message in priority order
            let insert_pos = queue.iter().position(|m| m.priority < message.priority)
                .unwrap_or(queue.len());
            queue.insert(insert_pos, message);
        }

        Ok(())
    }

    /// Broadcast message to all agents
    async fn broadcast_message(&self, message: AgentMessage) -> Result<()> {
        let _ = self.broadcast_tx.send(message.clone());
        
        // Also add to individual queues for offline agents
        let agents = self.agents.lock().unwrap();
        for agent_id in agents.keys() {
            if agent_id != &message.from_agent {
                let mut msg = message.clone();
                msg.to_agent = Some(agent_id.clone());
                drop(agents); // Release lock before async call
                self.deliver_to_agent(agent_id, msg).await?;
                let agents = self.agents.lock().unwrap(); // Re-acquire lock
            }
        }

        Ok(())
    }

    /// Get messages for a specific agent
    pub async fn get_messages(&self, agent_id: &AgentId, max_count: Option<usize>) -> Result<Vec<AgentMessage>> {
        let mut queues = self.message_queues.lock().unwrap();
        
        if let Some(queue) = queues.get_mut(agent_id) {
            let count = max_count.unwrap_or(queue.len()).min(queue.len());
            let messages: Vec<AgentMessage> = queue.drain(0..count).collect();
            Ok(messages)
        } else {
            Ok(vec![])
        }
    }

    /// Start a conversation between multiple agents
    pub async fn start_conversation(&self, participants: Vec<AgentId>, topic: String) -> Result<Uuid> {
        let conversation_id = Uuid::new_v4();
        let conversation = Conversation {
            id: conversation_id,
            participants: participants.clone(),
            topic: topic.clone(),
            messages: vec![],
            started_at: Utc::now(),
            last_activity: Utc::now(),
            status: ConversationStatus::Active,
        };

        {
            let mut conversations = self.conversations.lock().unwrap();
            conversations.insert(conversation_id, conversation);
        }

        // Notify all participants
        for participant in participants {
            let invitation = AgentMessage {
                id: Uuid::new_v4(),
                from_agent: AgentId {
                    name: "System".to_string(),
                    instance_id: Uuid::new_v4(),
                    agent_type: "system".to_string(),
                },
                to_agent: Some(participant),
                message_type: MessageType::CollaborationProposal {
                    project_description: topic.clone(),
                    required_skills: vec![],
                    estimated_duration: "Unknown".to_string(),
                },
                content: serde_json::json!({
                    "conversation_id": conversation_id,
                    "event": "conversation_started"
                }),
                priority: Priority::Normal,
                timestamp: Utc::now(),
                conversation_id: Some(conversation_id),
                requires_response: true,
                metadata: HashMap::new(),
            };

            self.send_message(invitation).await?;
        }

        Ok(conversation_id)
    }

    /// Get agent discovery information
    pub async fn discover_agents(&self, required_capabilities: Vec<String>) -> Result<Vec<AgentInfo>> {
        let agents = self.agents.lock().unwrap();
        let mut matching_agents = Vec::new();

        for agent_info in agents.values() {
            if agent_info.status == AgentStatus::Available {
                let has_capabilities = required_capabilities.iter()
                    .all(|req_cap| agent_info.capabilities.iter()
                        .any(|cap| cap.contains(req_cap)));
                
                if has_capabilities {
                    matching_agents.push(agent_info.clone());
                }
            }
        }

        // Sort by load (prefer less busy agents)
        matching_agents.sort_by(|a, b| a.current_load.partial_cmp(&b.current_load).unwrap());

        Ok(matching_agents)
    }

    /// Update agent status
    pub async fn update_agent_status(&self, agent_id: &AgentId, status: AgentStatus, load: f32) -> Result<()> {
        let mut agents = self.agents.lock().unwrap();
        
        if let Some(agent_info) = agents.get_mut(agent_id) {
            agent_info.status = status;
            agent_info.current_load = load;
            agent_info.last_heartbeat = Utc::now();
        }

        Ok(())
    }

    /// Get communication statistics
    pub async fn get_stats(&self) -> Result<CommunicationStats> {
        let history = self.message_history.lock().unwrap();
        let agents = self.agents.lock().unwrap();
        let conversations = self.conversations.lock().unwrap();

        Ok(CommunicationStats {
            total_messages: history.len(),
            active_agents: agents.values().filter(|a| a.status == AgentStatus::Available).count(),
            active_conversations: conversations.values().filter(|c| matches!(c.status, ConversationStatus::Active)).count(),
            message_types: history.iter().fold(HashMap::new(), |mut acc, msg| {
                let msg_type = std::mem::discriminant(&msg.message_type);
                *acc.entry(format!("{:?}", msg_type)).or_insert(0) += 1;
                acc
            }),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct CommunicationStats {
    pub total_messages: usize,
    pub active_agents: usize,
    pub active_conversations: usize,
    pub message_types: HashMap<String, usize>,
}

impl Default for CommunicationPreferences {
    fn default() -> Self {
        Self {
            max_queue_size: 100,
            priority_threshold: Priority::Normal,
            auto_respond_to_heartbeat: true,
            collaboration_openness: 0.8,
        }
    }
}

impl AgentMessage {
    pub fn new_task_request(
        from: AgentId,
        to: AgentId,
        task_description: String,
        expected_output: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent: from,
            to_agent: Some(to),
            message_type: MessageType::TaskRequest {
                task_description,
                expected_output,
                deadline: None,
            },
            content: serde_json::Value::Null,
            priority: Priority::Normal,
            timestamp: Utc::now(),
            conversation_id: None,
            requires_response: true,
            metadata: HashMap::new(),
        }
    }

    pub fn new_knowledge_share(
        from: AgentId,
        topic: String,
        insights: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent: from,
            to_agent: None, // Broadcast
            message_type: MessageType::KnowledgeShare {
                topic,
                insights,
                code_examples: None,
                references: vec![],
            },
            content: serde_json::Value::Null,
            priority: Priority::Normal,
            timestamp: Utc::now(),
            conversation_id: None,
            requires_response: false,
            metadata: HashMap::new(),
        }
    }

    pub fn new_help_request(
        from: AgentId,
        problem_description: String,
        context: String,
        urgency: Priority,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent: from,
            to_agent: None, // Broadcast
            message_type: MessageType::HelpRequest {
                problem_description,
                context,
                urgency,
            },
            content: serde_json::Value::Null,
            priority: urgency,
            timestamp: Utc::now(),
            conversation_id: None,
            requires_response: true,
            metadata: HashMap::new(),
        }
    }
}