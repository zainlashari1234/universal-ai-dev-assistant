use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPairProgrammingSession {
    pub id: Uuid,
    pub developer_id: String,
    pub ai_personality: AIPersonality,
    pub session_type: SessionType,
    pub conversation_history: Vec<ConversationEntry>,
    pub code_context: CodeContext,
    pub collaboration_metrics: CollaborationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIPersonality {
    Mentor,      // Guides and teaches
    Challenger,  // Questions and challenges decisions
    Supporter,   // Encourages and validates
    Expert,      // Provides deep technical knowledge
    Creative,    // Suggests innovative solutions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    CodeReview,
    ProblemSolving,
    Debugging,
    Learning,
    Brainstorming,
    Refactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub speaker: Speaker,
    pub message: String,
    pub message_type: MessageType,
    pub code_snippet: Option<String>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Speaker {
    Developer,
    AI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Question,
    Answer,
    Suggestion,
    Explanation,
    Challenge,
    Encouragement,
    CodeReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub suggestion_type: SuggestionType,
    pub content: String,
    pub confidence: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    CodeImprovement,
    AlternativeApproach,
    BestPractice,
    PerformanceOptimization,
    SecurityEnhancement,
    TestingStrategy,
}

pub struct AIPairProgrammingEngine {
    active_sessions: HashMap<Uuid, AIPairProgrammingSession>,
    ai_personalities: HashMap<AIPersonality, PersonalityConfig>,
}

#[derive(Debug, Clone)]
struct PersonalityConfig {
    response_style: String,
    question_frequency: f64,
    challenge_level: f64,
    encouragement_level: f64,
}

impl AIPairProgrammingEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            active_sessions: HashMap::new(),
            ai_personalities: HashMap::new(),
        };
        engine.initialize_personalities();
        engine
    }

    pub async fn start_session(
        &mut self,
        developer_id: String,
        ai_personality: AIPersonality,
        session_type: SessionType,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();
        
        let session = AIPairProgrammingSession {
            id: session_id,
            developer_id,
            ai_personality: ai_personality.clone(),
            session_type,
            conversation_history: Vec::new(),
            code_context: CodeContext::default(),
            collaboration_metrics: CollaborationMetrics::default(),
        };

        self.active_sessions.insert(session_id, session);
        
        // Send welcome message based on personality
        self.send_welcome_message(session_id, &ai_personality).await?;
        
        Ok(session_id)
    }

    async fn send_welcome_message(&mut self, session_id: Uuid, personality: &AIPersonality) -> Result<()> {
        let welcome_message = match personality {
            AIPersonality::Mentor => "Hello! I'm here to guide you through this coding session. What would you like to work on today?",
            AIPersonality::Challenger => "Ready to push your coding skills? I'll be asking tough questions to help you think deeper!",
            AIPersonality::Supporter => "Great to see you coding! I'm here to support and encourage you. Let's build something amazing!",
            AIPersonality::Expert => "I'm your technical expert for this session. Feel free to ask me about complex technical details.",
            AIPersonality::Creative => "Let's think outside the box! I'm here to help you explore creative and innovative solutions.",
        };

        self.add_conversation_entry(
            session_id,
            Speaker::AI,
            welcome_message.to_string(),
            MessageType::Encouragement,
            None,
        ).await?;

        Ok(())
    }

    async fn add_conversation_entry(
        &mut self,
        session_id: Uuid,
        speaker: Speaker,
        message: String,
        message_type: MessageType,
        code_snippet: Option<String>,
    ) -> Result<()> {
        if let Some(session) = self.active_sessions.get_mut(&session_id) {
            let entry = ConversationEntry {
                timestamp: chrono::Utc::now(),
                speaker,
                message,
                message_type,
                code_snippet,
                suggestions: Vec::new(),
            };
            session.conversation_history.push(entry);
        }
        Ok(())
    }

    fn initialize_personalities(&mut self) {
        self.ai_personalities.insert(
            AIPersonality::Mentor,
            PersonalityConfig {
                response_style: "Guiding and educational".to_string(),
                question_frequency: 0.7,
                challenge_level: 0.3,
                encouragement_level: 0.8,
            },
        );

        self.ai_personalities.insert(
            AIPersonality::Challenger,
            PersonalityConfig {
                response_style: "Questioning and probing".to_string(),
                question_frequency: 0.9,
                challenge_level: 0.9,
                encouragement_level: 0.4,
            },
        );

        // Add other personalities...
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeContext {
    pub current_file: Option<String>,
    pub current_function: Option<String>,
    pub recent_changes: Vec<String>,
    pub project_structure: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollaborationMetrics {
    pub questions_asked: u32,
    pub suggestions_given: u32,
    pub code_improvements: u32,
    pub session_duration: chrono::Duration,
}

impl Default for AIPairProgrammingEngine {
    fn default() -> Self {
        Self::new()
    }
}