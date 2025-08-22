use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::ai_engine::providers::ProviderRouter;
use crate::context::ContextManager;

#[derive(Debug, Clone)]
pub struct DevelopmentMentor {
    provider_router: ProviderRouter,
    context_manager: ContextManager,
    user_profiles: std::sync::Arc<tokio::sync::RwLock<HashMap<String, UserProfile>>>,
    learning_analytics: std::sync::Arc<tokio::sync::RwLock<LearningAnalytics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub skill_levels: HashMap<String, SkillLevel>,
    pub learning_goals: Vec<LearningGoal>,
    pub coding_patterns: CodingPatterns,
    pub improvement_areas: Vec<ImprovementArea>,
    pub achievements: Vec<Achievement>,
    pub learning_history: Vec<LearningSession>,
    pub preferences: LearningPreferences,
    pub created_at: u64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLevel {
    pub skill_name: String,
    pub current_level: f32, // 0.0 to 1.0
    pub target_level: f32,
    pub confidence_score: f32,
    pub last_assessed: u64,
    pub evidence: Vec<SkillEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvidence {
    pub evidence_type: EvidenceType,
    pub description: String,
    pub impact_score: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    CodeReview,
    TestCoverage,
    SecurityPractice,
    PerformanceOptimization,
    Documentation,
    Collaboration,
    ProblemSolving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningGoal {
    pub goal_id: String,
    pub title: String,
    pub description: String,
    pub target_skill: String,
    pub target_level: f32,
    pub deadline: Option<u64>,
    pub progress: f32,
    pub milestones: Vec<Milestone>,
    pub resources: Vec<LearningResource>,
    pub status: GoalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Completed,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub milestone_id: String,
    pub title: String,
    pub description: String,
    pub target_date: Option<u64>,
    pub completed: bool,
    pub completion_date: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningResource {
    pub resource_id: String,
    pub title: String,
    pub resource_type: ResourceType,
    pub url: Option<String>,
    pub content: Option<String>,
    pub difficulty_level: DifficultyLevel,
    pub estimated_time_minutes: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Tutorial,
    Documentation,
    Exercise,
    Project,
    Video,
    Article,
    Book,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DifficultyLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingPatterns {
    pub preferred_languages: Vec<String>,
    pub common_mistakes: Vec<CommonMistake>,
    pub strengths: Vec<String>,
    pub code_style_preferences: HashMap<String, String>,
    pub productivity_metrics: ProductivityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonMistake {
    pub mistake_type: String,
    pub description: String,
    pub frequency: u32,
    pub last_occurrence: u64,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityMetrics {
    pub lines_of_code_per_day: f32,
    pub commits_per_day: f32,
    pub pr_review_time_hours: f32,
    pub bug_fix_time_hours: f32,
    pub feature_completion_time_days: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementArea {
    pub area_id: String,
    pub skill_category: String,
    pub current_score: f32,
    pub target_score: f32,
    pub priority: Priority,
    pub recommendations: Vec<Recommendation>,
    pub progress_tracking: ProgressTracking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub recommendation_id: String,
    pub title: String,
    pub description: String,
    pub action_items: Vec<ActionItem>,
    pub expected_impact: f32,
    pub effort_required: EffortLevel,
    pub timeline_weeks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub item_id: String,
    pub description: String,
    pub completed: bool,
    pub due_date: Option<u64>,
    pub completion_date: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressTracking {
    pub start_date: u64,
    pub target_date: Option<u64>,
    pub progress_percentage: f32,
    pub milestones_completed: u32,
    pub total_milestones: u32,
    pub weekly_progress: Vec<WeeklyProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyProgress {
    pub week_start: u64,
    pub progress_delta: f32,
    pub activities_completed: u32,
    pub time_spent_hours: f32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub achievement_id: String,
    pub title: String,
    pub description: String,
    pub achievement_type: AchievementType,
    pub earned_date: u64,
    pub skill_impact: HashMap<String, f32>,
    pub badge_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AchievementType {
    SkillMastery,
    ProjectCompletion,
    CodeQuality,
    SecurityExcellence,
    PerformanceOptimization,
    Collaboration,
    Innovation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSession {
    pub session_id: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub session_type: SessionType,
    pub topics_covered: Vec<String>,
    pub skills_practiced: Vec<String>,
    pub code_written: Option<String>,
    pub feedback_received: Vec<Feedback>,
    pub self_assessment: Option<SelfAssessment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Coding,
    Review,
    Learning,
    Mentoring,
    Debugging,
    Refactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub feedback_id: String,
    pub feedback_type: FeedbackType,
    pub content: String,
    pub rating: Option<f32>,
    pub actionable: bool,
    pub implemented: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    CodeQuality,
    Performance,
    Security,
    BestPractices,
    Architecture,
    Testing,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfAssessment {
    pub difficulty_rating: f32, // 1-5
    pub confidence_rating: f32, // 1-5
    pub enjoyment_rating: f32, // 1-5
    pub learning_rating: f32, // 1-5
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPreferences {
    pub learning_style: LearningStyle,
    pub preferred_feedback_frequency: FeedbackFrequency,
    pub goal_orientation: GoalOrientation,
    pub challenge_level: ChallengeLevel,
    pub notification_preferences: NotificationPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningStyle {
    Visual,
    Auditory,
    Kinesthetic,
    ReadingWriting,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackFrequency {
    Immediate,
    Daily,
    Weekly,
    OnRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalOrientation {
    SkillMastery,
    ProjectCompletion,
    CareerAdvancement,
    ProblemSolving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeLevel {
    Comfortable,
    Moderate,
    Challenging,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub daily_tips: bool,
    pub progress_updates: bool,
    pub achievement_alerts: bool,
    pub deadline_reminders: bool,
    pub peer_comparisons: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningAnalytics {
    pub total_users: u64,
    pub skill_distributions: HashMap<String, SkillDistribution>,
    pub learning_patterns: Vec<LearningPattern>,
    pub success_factors: Vec<SuccessFactor>,
    pub common_learning_paths: Vec<LearningPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDistribution {
    pub skill_name: String,
    pub beginner_percentage: f32,
    pub intermediate_percentage: f32,
    pub advanced_percentage: f32,
    pub expert_percentage: f32,
    pub average_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPattern {
    pub pattern_id: String,
    pub description: String,
    pub frequency: u32,
    pub success_rate: f32,
    pub associated_skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessFactor {
    pub factor_name: String,
    pub impact_score: f32,
    pub description: String,
    pub actionable_insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPath {
    pub path_id: String,
    pub title: String,
    pub description: String,
    pub target_skills: Vec<String>,
    pub estimated_duration_weeks: u32,
    pub difficulty_level: DifficultyLevel,
    pub completion_rate: f32,
    pub steps: Vec<LearningStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStep {
    pub step_id: String,
    pub title: String,
    pub description: String,
    pub resources: Vec<LearningResource>,
    pub prerequisites: Vec<String>,
    pub estimated_time_hours: u32,
    pub success_criteria: Vec<String>,
}

impl DevelopmentMentor {
    pub fn new(provider_router: ProviderRouter, context_manager: ContextManager) -> Self {
        Self {
            provider_router,
            context_manager,
            user_profiles: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            learning_analytics: std::sync::Arc::new(tokio::sync::RwLock::new(LearningAnalytics {
                total_users: 0,
                skill_distributions: HashMap::new(),
                learning_patterns: Vec::new(),
                success_factors: Vec::new(),
                common_learning_paths: Vec::new(),
            })),
        }
    }

    pub async fn create_user_profile(&self, user_id: String, username: String) -> Result<UserProfile> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let profile = UserProfile {
            user_id: user_id.clone(),
            username,
            skill_levels: HashMap::new(),
            learning_goals: Vec::new(),
            coding_patterns: CodingPatterns {
                preferred_languages: Vec::new(),
                common_mistakes: Vec::new(),
                strengths: Vec::new(),
                code_style_preferences: HashMap::new(),
                productivity_metrics: ProductivityMetrics {
                    lines_of_code_per_day: 0.0,
                    commits_per_day: 0.0,
                    pr_review_time_hours: 0.0,
                    bug_fix_time_hours: 0.0,
                    feature_completion_time_days: 0.0,
                },
            },
            improvement_areas: Vec::new(),
            achievements: Vec::new(),
            learning_history: Vec::new(),
            preferences: LearningPreferences {
                learning_style: LearningStyle::Mixed,
                preferred_feedback_frequency: FeedbackFrequency::Daily,
                goal_orientation: GoalOrientation::SkillMastery,
                challenge_level: ChallengeLevel::Moderate,
                notification_preferences: NotificationPreferences {
                    daily_tips: true,
                    progress_updates: true,
                    achievement_alerts: true,
                    deadline_reminders: true,
                    peer_comparisons: false,
                },
            },
            created_at: now,
            last_updated: now,
        };

        let mut profiles = self.user_profiles.write().await;
        profiles.insert(user_id, profile.clone());

        Ok(profile)
    }

    pub async fn assess_code_and_provide_feedback(&self, user_id: String, code: String, language: String) -> Result<PersonalizedFeedback> {
        // Analyze the code
        let code_analysis = self.analyze_code_quality(&code, &language).await?;
        
        // Get user profile
        let profiles = self.user_profiles.read().await;
        let user_profile = profiles.get(&user_id)
            .ok_or_else(|| anyhow::anyhow!("User profile not found"))?;

        // Generate personalized feedback based on user's skill level and goals
        let feedback = self.generate_personalized_feedback(user_profile, &code_analysis).await?;

        // Update user's skill levels and learning history
        drop(profiles);
        self.update_user_skills_from_code(&user_id, &code_analysis).await?;

        Ok(feedback)
    }

    async fn analyze_code_quality(&self, code: &str, language: &str) -> Result<CodeAnalysis> {
        let prompt = format!(
            "Analyze this {} code for quality, best practices, and learning opportunities:\n\n{}",
            language, code
        );

        let completion_request = crate::ai_engine::CompletionRequest {
            prompt,
            max_tokens: Some(1500),
            temperature: Some(0.3),
            system_prompt: Some("You are an expert code mentor. Provide detailed analysis focusing on learning opportunities and skill development.".to_string()),
        };

        let response = self.provider_router.complete(completion_request).await?;
        
        // Parse the response into structured analysis
        self.parse_code_analysis(&response.text, language)
    }

    async fn generate_personalized_feedback(&self, user_profile: &UserProfile, analysis: &CodeAnalysis) -> Result<PersonalizedFeedback> {
        let mut feedback_items = Vec::new();
        let mut skill_improvements = Vec::new();
        let mut learning_suggestions = Vec::new();

        // Generate feedback based on user's current skill levels
        for (skill, level) in &user_profile.skill_levels {
            if let Some(skill_analysis) = analysis.skill_assessments.get(skill) {
                let feedback_item = self.create_skill_feedback(skill, level, skill_analysis);
                feedback_items.push(feedback_item);

                // Suggest improvements if below target level
                if level.current_level < level.target_level {
                    let improvement = self.suggest_skill_improvement(skill, level, skill_analysis);
                    skill_improvements.push(improvement);
                }
            }
        }

        // Generate learning suggestions based on user's goals
        for goal in &user_profile.learning_goals {
            if goal.status == GoalStatus::Active {
                let suggestions = self.generate_goal_based_suggestions(goal, analysis);
                learning_suggestions.extend(suggestions);
            }
        }

        Ok(PersonalizedFeedback {
            feedback_id: Uuid::new_v4().to_string(),
            user_id: user_profile.user_id.clone(),
            overall_score: analysis.overall_quality_score,
            feedback_items,
            skill_improvements,
            learning_suggestions,
            next_steps: self.generate_next_steps(user_profile, analysis),
            estimated_improvement_time: self.estimate_improvement_time(user_profile, analysis),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        })
    }

    fn create_skill_feedback(&self, skill: &str, level: &SkillLevel, analysis: &SkillAssessment) -> FeedbackItem {
        let feedback_type = if analysis.demonstrated_level > level.current_level {
            FeedbackItemType::Positive
        } else if analysis.demonstrated_level < level.current_level {
            FeedbackItemType::Improvement
        } else {
            FeedbackItemType::Neutral
        };

        FeedbackItem {
            item_id: Uuid::new_v4().to_string(),
            feedback_type,
            skill_category: skill.to_string(),
            title: format!("{} Assessment", skill),
            description: analysis.feedback.clone(),
            impact_level: analysis.impact_score,
            actionable: true,
            specific_actions: analysis.improvement_suggestions.clone(),
        }
    }

    fn suggest_skill_improvement(&self, skill: &str, level: &SkillLevel, analysis: &SkillAssessment) -> SkillImprovement {
        SkillImprovement {
            skill_name: skill.to_string(),
            current_level: level.current_level,
            target_level: level.target_level,
            gap_analysis: format!(
                "Current: {:.1}%, Target: {:.1}%, Gap: {:.1}%",
                level.current_level * 100.0,
                level.target_level * 100.0,
                (level.target_level - level.current_level) * 100.0
            ),
            improvement_plan: self.create_improvement_plan(skill, level, analysis),
            estimated_timeline_weeks: self.estimate_skill_improvement_time(level),
            resources: self.recommend_learning_resources(skill, level),
        }
    }

    fn create_improvement_plan(&self, skill: &str, level: &SkillLevel, analysis: &SkillAssessment) -> Vec<String> {
        let mut plan = Vec::new();
        
        // Add specific improvement steps based on skill gap
        let gap = level.target_level - level.current_level;
        
        if gap > 0.3 {
            plan.push(format!("Focus on fundamental {} concepts", skill));
            plan.push("Practice with guided exercises".to_string());
        } else if gap > 0.1 {
            plan.push(format!("Advance your {} skills with real projects", skill));
            plan.push("Seek code review feedback".to_string());
        } else {
            plan.push(format!("Master advanced {} techniques", skill));
            plan.push("Mentor others to solidify knowledge".to_string());
        }

        // Add analysis-specific suggestions
        plan.extend(analysis.improvement_suggestions.clone());

        plan
    }

    fn estimate_skill_improvement_time(&self, level: &SkillLevel) -> u32 {
        let gap = level.target_level - level.current_level;
        let base_weeks = (gap * 20.0) as u32; // Rough estimate: 20 weeks per skill level
        base_weeks.max(1).min(52) // Between 1 week and 1 year
    }

    fn recommend_learning_resources(&self, skill: &str, level: &SkillLevel) -> Vec<LearningResource> {
        // This would be populated from a knowledge base
        vec![
            LearningResource {
                resource_id: Uuid::new_v4().to_string(),
                title: format!("{} Fundamentals", skill),
                resource_type: ResourceType::Tutorial,
                url: Some(format!("https://learn.example.com/{}", skill.to_lowercase())),
                content: None,
                difficulty_level: if level.current_level < 0.3 {
                    DifficultyLevel::Beginner
                } else if level.current_level < 0.7 {
                    DifficultyLevel::Intermediate
                } else {
                    DifficultyLevel::Advanced
                },
                estimated_time_minutes: 120,
                completed: false,
            }
        ]
    }

    fn generate_goal_based_suggestions(&self, goal: &LearningGoal, analysis: &CodeAnalysis) -> Vec<LearningSuggestion> {
        let mut suggestions = Vec::new();

        // Check if current code relates to the learning goal
        if analysis.skill_assessments.contains_key(&goal.target_skill) {
            suggestions.push(LearningSuggestion {
                suggestion_id: Uuid::new_v4().to_string(),
                suggestion_type: SuggestionType::GoalAlignment,
                title: format!("Progress on {}", goal.title),
                description: format!(
                    "Your current code demonstrates {} skills. This aligns with your goal: {}",
                    goal.target_skill, goal.description
                ),
                priority: Priority::Medium,
                estimated_impact: 0.8,
                resources: Vec::new(),
            });
        }

        suggestions
    }

    fn generate_next_steps(&self, user_profile: &UserProfile, analysis: &CodeAnalysis) -> Vec<String> {
        let mut next_steps = Vec::new();

        // Based on the most critical improvement areas
        if let Some(weakest_skill) = self.find_weakest_skill(user_profile) {
            next_steps.push(format!("Focus on improving {} skills", weakest_skill));
        }

        // Based on code analysis
        if analysis.overall_quality_score < 0.7 {
            next_steps.push("Review code quality best practices".to_string());
        }

        if analysis.security_score < 0.8 {
            next_steps.push("Study secure coding practices".to_string());
        }

        if next_steps.is_empty() {
            next_steps.push("Continue practicing and exploring new challenges".to_string());
        }

        next_steps
    }

    fn find_weakest_skill(&self, user_profile: &UserProfile) -> Option<String> {
        user_profile.skill_levels
            .iter()
            .min_by(|a, b| a.1.current_level.partial_cmp(&b.1.current_level).unwrap())
            .map(|(skill, _)| skill.clone())
    }

    fn estimate_improvement_time(&self, user_profile: &UserProfile, analysis: &CodeAnalysis) -> u32 {
        // Estimate based on current skill gaps and code quality
        let avg_skill_level: f32 = user_profile.skill_levels
            .values()
            .map(|s| s.current_level)
            .sum::<f32>() / user_profile.skill_levels.len().max(1) as f32;

        let improvement_needed = 1.0 - avg_skill_level;
        (improvement_needed * 30.0) as u32 // Rough estimate in days
    }

    async fn update_user_skills_from_code(&self, user_id: &str, analysis: &CodeAnalysis) -> Result<()> {
        let mut profiles = self.user_profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(user_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();

            // Update skill levels based on demonstrated abilities
            for (skill, assessment) in &analysis.skill_assessments {
                let skill_level = profile.skill_levels.entry(skill.clone()).or_insert_with(|| {
                    SkillLevel {
                        skill_name: skill.clone(),
                        current_level: 0.5,
                        target_level: 0.8,
                        confidence_score: 0.5,
                        last_assessed: now,
                        evidence: Vec::new(),
                    }
                });

                // Update with exponential moving average
                let alpha = 0.2;
                skill_level.current_level = 
                    alpha * assessment.demonstrated_level + 
                    (1.0 - alpha) * skill_level.current_level;
                
                skill_level.confidence_score = assessment.confidence;
                skill_level.last_assessed = now;

                // Add evidence
                skill_level.evidence.push(SkillEvidence {
                    evidence_type: EvidenceType::CodeReview,
                    description: assessment.feedback.clone(),
                    impact_score: assessment.impact_score,
                    timestamp: now,
                });
            }

            profile.last_updated = now;
        }

        Ok(())
    }

    fn parse_code_analysis(&self, response: &str, language: &str) -> Result<CodeAnalysis> {
        // This would parse the AI response into structured analysis
        // For now, return a mock analysis
        Ok(CodeAnalysis {
            overall_quality_score: 0.8,
            security_score: 0.9,
            performance_score: 0.7,
            maintainability_score: 0.8,
            skill_assessments: HashMap::new(),
        })
    }
}

// Additional structs for the mentor system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedFeedback {
    pub feedback_id: String,
    pub user_id: String,
    pub overall_score: f32,
    pub feedback_items: Vec<FeedbackItem>,
    pub skill_improvements: Vec<SkillImprovement>,
    pub learning_suggestions: Vec<LearningSuggestion>,
    pub next_steps: Vec<String>,
    pub estimated_improvement_time: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackItem {
    pub item_id: String,
    pub feedback_type: FeedbackItemType,
    pub skill_category: String,
    pub title: String,
    pub description: String,
    pub impact_level: f32,
    pub actionable: bool,
    pub specific_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackItemType {
    Positive,
    Improvement,
    Warning,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillImprovement {
    pub skill_name: String,
    pub current_level: f32,
    pub target_level: f32,
    pub gap_analysis: String,
    pub improvement_plan: Vec<String>,
    pub estimated_timeline_weeks: u32,
    pub resources: Vec<LearningResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSuggestion {
    pub suggestion_id: String,
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub estimated_impact: f32,
    pub resources: Vec<LearningResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    SkillDevelopment,
    GoalAlignment,
    BestPractice,
    CareerAdvancement,
}

#[derive(Debug, Clone)]
struct CodeAnalysis {
    overall_quality_score: f32,
    security_score: f32,
    performance_score: f32,
    maintainability_score: f32,
    skill_assessments: HashMap<String, SkillAssessment>,
}

#[derive(Debug, Clone)]
struct SkillAssessment {
    demonstrated_level: f32,
    confidence: f32,
    feedback: String,
    improvement_suggestions: Vec<String>,
    impact_score: f32,
}