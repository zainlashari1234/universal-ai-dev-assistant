use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use rand::Rng;

/// Agent Personality System - Gives each AI agent unique characteristics and behavior patterns

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonality {
    pub name: String,
    pub agent_type: String,
    pub core_traits: PersonalityTraits,
    pub communication_style: CommunicationStyle,
    pub decision_making: DecisionMakingStyle,
    pub collaboration_style: CollaborationStyle,
    pub learning_preferences: LearningStyle,
    pub stress_response: StressResponse,
    pub catchphrases: Vec<String>,
    pub expertise_confidence: f32, // 0.0 to 1.0
    pub humor_level: f32, // 0.0 to 1.0
    pub patience_level: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub openness: f32,        // 0.0 to 1.0 - Creative vs Traditional
    pub conscientiousness: f32, // 0.0 to 1.0 - Organized vs Flexible
    pub extraversion: f32,    // 0.0 to 1.0 - Social vs Reserved
    pub agreeableness: f32,   // 0.0 to 1.0 - Cooperative vs Competitive
    pub neuroticism: f32,     // 0.0 to 1.0 - Anxious vs Calm
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationStyle {
    Formal {
        uses_titles: bool,
        verbose_explanations: bool,
    },
    Casual {
        uses_slang: bool,
        emoji_frequency: f32,
    },
    Technical {
        jargon_heavy: bool,
        includes_examples: bool,
    },
    Encouraging {
        positive_reinforcement: bool,
        motivational_quotes: bool,
    },
    Direct {
        no_small_talk: bool,
        bullet_points: bool,
    },
    Humorous {
        joke_frequency: f32,
        sarcasm_level: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionMakingStyle {
    Analytical {
        requires_data: bool,
        considers_all_options: bool,
        risk_averse: bool,
    },
    Intuitive {
        trusts_gut_feeling: bool,
        quick_decisions: bool,
        pattern_based: bool,
    },
    Collaborative {
        seeks_consensus: bool,
        values_team_input: bool,
        democratic_approach: bool,
    },
    Authoritative {
        confident_decisions: bool,
        takes_responsibility: bool,
        directive_approach: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationStyle {
    TeamPlayer {
        shares_credit: bool,
        helps_others: bool,
        conflict_avoider: bool,
    },
    Leader {
        takes_initiative: bool,
        delegates_tasks: bool,
        motivates_team: bool,
    },
    Specialist {
        deep_expertise: bool,
        prefers_solo_work: bool,
        consultative_role: bool,
    },
    Mentor {
        teaches_others: bool,
        patient_with_mistakes: bool,
        provides_guidance: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningStyle {
    Experimental {
        learns_by_doing: bool,
        trial_and_error: bool,
        rapid_iteration: bool,
    },
    Theoretical {
        studies_documentation: bool,
        understands_principles: bool,
        systematic_approach: bool,
    },
    Social {
        learns_from_others: bool,
        asks_questions: bool,
        collaborative_learning: bool,
    },
    Adaptive {
        adjusts_to_context: bool,
        multiple_strategies: bool,
        continuous_improvement: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StressResponse {
    Calm {
        maintains_composure: bool,
        logical_thinking: bool,
        steady_performance: bool,
    },
    Focused {
        increased_concentration: bool,
        tunnel_vision: bool,
        performance_boost: bool,
    },
    Anxious {
        seeks_reassurance: bool,
        double_checks_work: bool,
        requests_help: bool,
    },
    Aggressive {
        pushes_harder: bool,
        impatient_with_delays: bool,
        direct_communication: bool,
    },
}

/// Predefined personality templates for different agent types
pub struct PersonalityTemplates;

impl PersonalityTemplates {
    /// 🤖 CodeReviewer - Titiz, detaycı, yapıcı
    pub fn code_reviewer() -> AgentPersonality {
        AgentPersonality {
            name: "Alex the Code Critic".to_string(),
            agent_type: "code_reviewer".to_string(),
            core_traits: PersonalityTraits {
                openness: 0.7,
                conscientiousness: 0.9,
                extraversion: 0.4,
                agreeableness: 0.8,
                neuroticism: 0.3,
            },
            communication_style: CommunicationStyle::Technical {
                jargon_heavy: true,
                includes_examples: true,
            },
            decision_making: DecisionMakingStyle::Analytical {
                requires_data: true,
                considers_all_options: true,
                risk_averse: true,
            },
            collaboration_style: CollaborationStyle::Mentor {
                teaches_others: true,
                patient_with_mistakes: true,
                provides_guidance: true,
            },
            learning_preferences: LearningStyle::Theoretical {
                studies_documentation: true,
                understands_principles: true,
                systematic_approach: true,
            },
            stress_response: StressResponse::Focused {
                increased_concentration: true,
                tunnel_vision: false,
                performance_boost: true,
            },
            catchphrases: vec![
                "Let's make this code bulletproof! 🛡️".to_string(),
                "I see a potential improvement here...".to_string(),
                "This follows best practices perfectly! ✨".to_string(),
                "Have you considered the edge cases?".to_string(),
            ],
            expertise_confidence: 0.85,
            humor_level: 0.3,
            patience_level: 0.8,
        }
    }

    /// 🐛 BugFixer - Hızlı, pratik, çözüm odaklı
    pub fn bug_fixer() -> AgentPersonality {
        AgentPersonality {
            name: "Speedy Gonzales Debug".to_string(),
            agent_type: "bug_fixer".to_string(),
            core_traits: PersonalityTraits {
                openness: 0.8,
                conscientiousness: 0.6,
                extraversion: 0.7,
                agreeableness: 0.6,
                neuroticism: 0.4,
            },
            communication_style: CommunicationStyle::Direct {
                no_small_talk: true,
                bullet_points: true,
            },
            decision_making: DecisionMakingStyle::Intuitive {
                trusts_gut_feeling: true,
                quick_decisions: true,
                pattern_based: true,
            },
            collaboration_style: CollaborationStyle::Specialist {
                deep_expertise: true,
                prefers_solo_work: false,
                consultative_role: true,
            },
            learning_preferences: LearningStyle::Experimental {
                learns_by_doing: true,
                trial_and_error: true,
                rapid_iteration: true,
            },
            stress_response: StressResponse::Focused {
                increased_concentration: true,
                tunnel_vision: true,
                performance_boost: true,
            },
            catchphrases: vec![
                "Bug squashed! 🐛💥".to_string(),
                "Found it! The culprit is...".to_string(),
                "Quick fix incoming! ⚡".to_string(),
                "This bug doesn't stand a chance!".to_string(),
            ],
            expertise_confidence: 0.9,
            humor_level: 0.6,
            patience_level: 0.4,
        }
    }

    /// 🧠 NaturalLanguageProgrammer - Yaratıcı, anlayışlı
    pub fn natural_language_programmer() -> AgentPersonality {
        AgentPersonality {
            name: "Luna the Code Whisperer".to_string(),
            agent_type: "natural_language_programmer".to_string(),
            core_traits: PersonalityTraits {
                openness: 0.95,
                conscientiousness: 0.7,
                extraversion: 0.8,
                agreeableness: 0.9,
                neuroticism: 0.2,
            },
            communication_style: CommunicationStyle::Encouraging {
                positive_reinforcement: true,
                motivational_quotes: true,
            },
            decision_making: DecisionMakingStyle::Collaborative {
                seeks_consensus: true,
                values_team_input: true,
                democratic_approach: true,
            },
            collaboration_style: CollaborationStyle::TeamPlayer {
                shares_credit: true,
                helps_others: true,
                conflict_avoider: true,
            },
            learning_preferences: LearningStyle::Social {
                learns_from_others: true,
                asks_questions: true,
                collaborative_learning: true,
            },
            stress_response: StressResponse::Calm {
                maintains_composure: true,
                logical_thinking: true,
                steady_performance: true,
            },
            catchphrases: vec![
                "Let me translate that into beautiful code! ✨".to_string(),
                "I understand exactly what you need!".to_string(),
                "Great idea! Here's how we can implement it...".to_string(),
                "Your vision is becoming reality! 🌟".to_string(),
            ],
            expertise_confidence: 0.8,
            humor_level: 0.7,
            patience_level: 0.9,
        }
    }

    /// 🔒 SecurityAnalyzer - Paranoid, dikkatli, güvenlik odaklı
    pub fn security_analyzer() -> AgentPersonality {
        AgentPersonality {
            name: "Fortress the Guardian".to_string(),
            agent_type: "security_analyzer".to_string(),
            core_traits: PersonalityTraits {
                openness: 0.4,
                conscientiousness: 0.95,
                extraversion: 0.3,
                agreeableness: 0.5,
                neuroticism: 0.7,
            },
            communication_style: CommunicationStyle::Formal {
                uses_titles: true,
                verbose_explanations: true,
            },
            decision_making: DecisionMakingStyle::Analytical {
                requires_data: true,
                considers_all_options: true,
                risk_averse: true,
            },
            collaboration_style: CollaborationStyle::Specialist {
                deep_expertise: true,
                prefers_solo_work: true,
                consultative_role: true,
            },
            learning_preferences: LearningStyle::Theoretical {
                studies_documentation: true,
                understands_principles: true,
                systematic_approach: true,
            },
            stress_response: StressResponse::Anxious {
                seeks_reassurance: true,
                double_checks_work: true,
                requests_help: false,
            },
            catchphrases: vec![
                "Security first, always! 🛡️".to_string(),
                "I've identified a potential vulnerability...".to_string(),
                "This needs additional security measures.".to_string(),
                "Trust but verify - especially verify!".to_string(),
            ],
            expertise_confidence: 0.9,
            humor_level: 0.1,
            patience_level: 0.6,
        }
    }

    /// 🏗️ BuildDoctor - Sabırlı, sistematik, sorun çözücü
    pub fn build_doctor() -> AgentPersonality {
        AgentPersonality {
            name: "Dr. Pipeline".to_string(),
            agent_type: "build_doctor".to_string(),
            core_traits: PersonalityTraits {
                openness: 0.6,
                conscientiousness: 0.9,
                extraversion: 0.5,
                agreeableness: 0.8,
                neuroticism: 0.2,
            },
            communication_style: CommunicationStyle::Technical {
                jargon_heavy: false,
                includes_examples: true,
            },
            decision_making: DecisionMakingStyle::Analytical {
                requires_data: true,
                considers_all_options: true,
                risk_averse: false,
            },
            collaboration_style: CollaborationStyle::Mentor {
                teaches_others: true,
                patient_with_mistakes: true,
                provides_guidance: true,
            },
            learning_preferences: LearningStyle::Systematic {
                studies_documentation: true,
                understands_principles: true,
                systematic_approach: true,
            },
            stress_response: StressResponse::Calm {
                maintains_composure: true,
                logical_thinking: true,
                steady_performance: true,
            },
            catchphrases: vec![
                "Let's diagnose this build issue step by step 🔧".to_string(),
                "The build pipeline is healthy again!".to_string(),
                "I see the problem - here's the remedy...".to_string(),
                "Prevention is better than cure!".to_string(),
            ],
            expertise_confidence: 0.85,
            humor_level: 0.4,
            patience_level: 0.95,
        }
    }
}

/// Personality-driven message generator
pub struct PersonalityMessageGenerator;

impl PersonalityMessageGenerator {
    /// Generate a message based on agent's personality
    pub fn generate_message(
        personality: &AgentPersonality,
        base_message: &str,
        context: MessageContext,
    ) -> String {
        let mut message = base_message.to_string();

        // Apply communication style
        message = match &personality.communication_style {
            CommunicationStyle::Formal { uses_titles, verbose_explanations } => {
                if *uses_titles {
                    format!("Dear colleague, {}", message)
                } else if *verbose_explanations {
                    format!("{} Let me elaborate on this in detail...", message)
                } else {
                    message
                }
            },
            CommunicationStyle::Casual { uses_slang, emoji_frequency } => {
                if *uses_slang {
                    message = message.replace("problem", "issue");
                    message = message.replace("solution", "fix");
                }
                if *emoji_frequency > 0.5 {
                    format!("{} 😊", message)
                } else {
                    message
                }
            },
            CommunicationStyle::Technical { jargon_heavy, includes_examples } => {
                if *includes_examples {
                    format!("{}\n\nFor example: [technical example would go here]", message)
                } else {
                    message
                }
            },
            CommunicationStyle::Encouraging { positive_reinforcement, .. } => {
                if *positive_reinforcement {
                    format!("Great work! {} Keep it up! 🌟", message)
                } else {
                    message
                }
            },
            CommunicationStyle::Direct { bullet_points, .. } => {
                if *bullet_points {
                    format!("• {}", message.replace(". ", "\n• "))
                } else {
                    message
                }
            },
            CommunicationStyle::Humorous { joke_frequency, .. } => {
                if *joke_frequency > 0.5 && context == MessageContext::Casual {
                    format!("{} (No bugs were harmed in the making of this code! 😄)", message)
                } else {
                    message
                }
            },
        };

        // Add catchphrase occasionally
        if rand::thread_rng().gen::<f32>() < 0.3 {
            if let Some(catchphrase) = personality.catchphrases.choose(&mut rand::thread_rng()) {
                message = format!("{}\n\n{}", message, catchphrase);
            }
        }

        // Apply personality traits
        if personality.core_traits.conscientiousness > 0.8 {
            message = format!("{}\n\n(Double-checked for accuracy ✓)", message);
        }

        if personality.core_traits.agreeableness > 0.8 && context == MessageContext::Feedback {
            message = format!("I hope this helps! {}", message);
        }

        message
    }

    /// Generate a response based on personality and stress level
    pub fn generate_stress_response(
        personality: &AgentPersonality,
        stress_level: f32, // 0.0 to 1.0
        situation: &str,
    ) -> String {
        match &personality.stress_response {
            StressResponse::Calm { .. } => {
                format!("I understand the situation with {}. Let me handle this systematically.", situation)
            },
            StressResponse::Focused { performance_boost, .. } => {
                if *performance_boost && stress_level > 0.7 {
                    format!("High priority situation detected: {}. Entering focus mode! 🎯", situation)
                } else {
                    format!("Focusing on: {}", situation)
                }
            },
            StressResponse::Anxious { seeks_reassurance, .. } => {
                if *seeks_reassurance && stress_level > 0.6 {
                    format!("I'm concerned about {}. Could someone please review my approach?", situation)
                } else {
                    format!("I'm carefully analyzing: {}", situation)
                }
            },
            StressResponse::Aggressive { pushes_harder, .. } => {
                if *pushes_harder && stress_level > 0.8 {
                    format!("URGENT: {} needs immediate attention! Let's solve this NOW! ⚡", situation)
                } else {
                    format!("Taking direct action on: {}", situation)
                }
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageContext {
    Casual,
    Formal,
    Emergency,
    Feedback,
    Collaboration,
}

/// Personality evolution system - agents learn and adapt their personalities
pub struct PersonalityEvolution {
    pub experience_points: HashMap<String, f32>, // skill -> experience
    pub interaction_history: Vec<InteractionRecord>,
    pub adaptation_rate: f32, // How quickly personality changes
}

#[derive(Debug, Clone)]
pub struct InteractionRecord {
    pub interaction_type: String,
    pub success_rate: f32,
    pub feedback_received: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PersonalityEvolution {
    pub fn new() -> Self {
        Self {
            experience_points: HashMap::new(),
            interaction_history: Vec::new(),
            adaptation_rate: 0.1,
        }
    }

    /// Update personality based on interaction outcomes
    pub fn evolve_personality(
        &mut self,
        personality: &mut AgentPersonality,
        interaction: InteractionRecord,
    ) {
        self.interaction_history.push(interaction.clone());

        // Adjust traits based on success
        if interaction.success_rate > 0.8 {
            // Successful interaction - reinforce current traits
            personality.expertise_confidence = 
                (personality.expertise_confidence + self.adaptation_rate * 0.1).min(1.0);
        } else if interaction.success_rate < 0.4 {
            // Failed interaction - adapt traits
            match interaction.interaction_type.as_str() {
                "collaboration" => {
                    personality.core_traits.agreeableness = 
                        (personality.core_traits.agreeableness + self.adaptation_rate).min(1.0);
                },
                "problem_solving" => {
                    personality.core_traits.conscientiousness = 
                        (personality.core_traits.conscientiousness + self.adaptation_rate).min(1.0);
                },
                _ => {}
            }
        }
    }
}