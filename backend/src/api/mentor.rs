use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agents::development_mentor::{
    DevelopmentMentor, UserProfile, PersonalizedFeedback, LearningGoal, 
    SkillLevel, LearningPreferences, Achievement
};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

/// Create or update user profile for personalized mentoring
#[utoipa::path(
    post,
    path = "/api/v1/mentor/profile",
    request_body = CreateProfileRequest,
    responses(
        (status = 200, description = "Profile created successfully", body = UserProfile),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn create_user_profile(
    State(app_state): State<AppState>,
    Json(request): Json<CreateProfileRequest>,
) -> Result<Json<UserProfile>, (StatusCode, Json<ErrorResponse>)> {
    let mentor = DevelopmentMentor::new(
        app_state.provider_router.clone(),
        app_state.context_manager.clone(),
    );

    match mentor.create_user_profile(request.user_id, request.username).await {
        Ok(profile) => {
            app_state.metrics.record_mentor_profile_created();
            Ok(Json(profile))
        }
        Err(e) => {
            app_state.metrics.record_mentor_operation_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Profile creation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Get personalized code feedback and learning suggestions
#[utoipa::path(
    post,
    path = "/api/v1/mentor/feedback",
    request_body = CodeFeedbackRequest,
    responses(
        (status = 200, description = "Feedback generated successfully", body = PersonalizedFeedback),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn get_code_feedback(
    State(app_state): State<AppState>,
    Json(request): Json<CodeFeedbackRequest>,
) -> Result<Json<PersonalizedFeedback>, (StatusCode, Json<ErrorResponse>)> {
    let mentor = DevelopmentMentor::new(
        app_state.provider_router.clone(),
        app_state.context_manager.clone(),
    );

    match mentor.assess_code_and_provide_feedback(
        request.user_id,
        request.code,
        request.language,
    ).await {
        Ok(feedback) => {
            app_state.metrics.record_mentor_feedback_generated();
            Ok(Json(feedback))
        }
        Err(e) => {
            app_state.metrics.record_mentor_operation_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Feedback generation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Set learning goals for personalized development path
#[utoipa::path(
    post,
    path = "/api/v1/mentor/goals",
    request_body = SetGoalsRequest,
    responses(
        (status = 200, description = "Goals set successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn set_learning_goals(
    State(app_state): State<AppState>,
    Json(request): Json<SetGoalsRequest>,
) -> Result<Json<GoalsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Implementation would update user's learning goals
    let response = GoalsResponse {
        success: true,
        goals_set: request.goals.len() as u32,
        estimated_completion_weeks: 12, // Would be calculated
        personalized_plan: vec![
            "Start with fundamentals".to_string(),
            "Practice with real projects".to_string(),
            "Seek feedback and iterate".to_string(),
        ],
    };

    app_state.metrics.record_mentor_goals_set();
    Ok(Json(response))
}

/// Get skill assessment and progress tracking
#[utoipa::path(
    get,
    path = "/api/v1/mentor/skills/{user_id}",
    responses(
        (status = 200, description = "Skills retrieved successfully", body = SkillsResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn get_skill_assessment(
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<SkillsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Implementation would retrieve user's current skill levels
    let response = SkillsResponse {
        user_id: user_id.clone(),
        skill_levels: HashMap::new(), // Would be populated from user profile
        skill_trends: Vec::new(),
        recommendations: vec![
            SkillRecommendation {
                skill_name: "Rust Programming".to_string(),
                current_level: 0.6,
                target_level: 0.8,
                priority: "High".to_string(),
                estimated_weeks: 8,
                resources: vec![
                    "The Rust Programming Language Book".to_string(),
                    "Rustlings Exercises".to_string(),
                ],
            }
        ],
        next_milestone: Some("Complete async programming module".to_string()),
    };

    app_state.metrics.record_mentor_skills_assessed();
    Ok(Json(response))
}

/// Get personalized learning path recommendations
#[utoipa::path(
    get,
    path = "/api/v1/mentor/learning-path/{user_id}",
    responses(
        (status = 200, description = "Learning path generated", body = LearningPathResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn get_learning_path(
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
    Query(params): Query<LearningPathQuery>,
) -> Result<Json<LearningPathResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = LearningPathResponse {
        user_id: user_id.clone(),
        path_id: uuid::Uuid::new_v4().to_string(),
        title: format!("Personalized Learning Path for {}", params.target_skill.unwrap_or_else(|| "General Development".to_string())),
        estimated_duration_weeks: 16,
        difficulty_level: "Intermediate".to_string(),
        milestones: vec![
            LearningMilestone {
                milestone_id: uuid::Uuid::new_v4().to_string(),
                title: "Foundation Building".to_string(),
                description: "Master the fundamentals".to_string(),
                estimated_weeks: 4,
                skills_covered: vec!["Syntax".to_string(), "Basic Concepts".to_string()],
                resources: vec![
                    LearningResourceInfo {
                        title: "Interactive Tutorial".to_string(),
                        resource_type: "Tutorial".to_string(),
                        estimated_hours: 20,
                        difficulty: "Beginner".to_string(),
                    }
                ],
            },
            LearningMilestone {
                milestone_id: uuid::Uuid::new_v4().to_string(),
                title: "Practical Application".to_string(),
                description: "Build real-world projects".to_string(),
                estimated_weeks: 8,
                skills_covered: vec!["Project Structure".to_string(), "Best Practices".to_string()],
                resources: vec![
                    LearningResourceInfo {
                        title: "Hands-on Projects".to_string(),
                        resource_type: "Project".to_string(),
                        estimated_hours: 40,
                        difficulty: "Intermediate".to_string(),
                    }
                ],
            },
        ],
        success_metrics: vec![
            "Complete 3 projects".to_string(),
            "Pass skill assessments".to_string(),
            "Receive positive code reviews".to_string(),
        ],
    };

    app_state.metrics.record_mentor_learning_path_generated();
    Ok(Json(response))
}

/// Track progress and update achievements
#[utoipa::path(
    post,
    path = "/api/v1/mentor/progress",
    request_body = ProgressUpdateRequest,
    responses(
        (status = 200, description = "Progress updated successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn update_progress(
    State(app_state): State<AppState>,
    Json(request): Json<ProgressUpdateRequest>,
) -> Result<Json<ProgressResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = ProgressResponse {
        success: true,
        new_achievements: vec![
            AchievementInfo {
                title: "Code Quality Improver".to_string(),
                description: "Consistently improved code quality over 5 sessions".to_string(),
                earned_date: chrono::Utc::now().timestamp() as u64,
                badge_url: Some("https://badges.example.com/quality-improver.png".to_string()),
            }
        ],
        skill_updates: HashMap::new(),
        next_recommendations: vec![
            "Focus on error handling patterns".to_string(),
            "Practice test-driven development".to_string(),
        ],
        overall_progress_percentage: 67.5,
    };

    app_state.metrics.record_mentor_progress_updated();
    Ok(Json(response))
}

/// Get daily personalized tips and suggestions
#[utoipa::path(
    get,
    path = "/api/v1/mentor/daily-tips/{user_id}",
    responses(
        (status = 200, description = "Daily tips generated", body = DailyTipsResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "mentor"
)]
pub async fn get_daily_tips(
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<DailyTipsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = DailyTipsResponse {
        user_id: user_id.clone(),
        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        tips: vec![
            DailyTip {
                tip_id: uuid::Uuid::new_v4().to_string(),
                category: "Code Quality".to_string(),
                title: "Use meaningful variable names".to_string(),
                description: "Choose descriptive names that explain the purpose of your variables. This makes your code self-documenting.".to_string(),
                example_code: Some("// Good\nlet user_count = users.len();\n\n// Avoid\nlet n = users.len();".to_string()),
                difficulty_level: "Beginner".to_string(),
                estimated_impact: "High".to_string(),
            },
            DailyTip {
                tip_id: uuid::Uuid::new_v4().to_string(),
                category: "Performance".to_string(),
                title: "Avoid unnecessary allocations".to_string(),
                description: "Reuse existing data structures when possible to reduce memory allocations and improve performance.".to_string(),
                example_code: Some("// Efficient\nlet mut buffer = Vec::with_capacity(1000);\nfor item in items {\n    buffer.clear();\n    // process item\n}".to_string()),
                difficulty_level: "Intermediate".to_string(),
                estimated_impact: "Medium".to_string(),
            },
        ],
        personalized_focus: "Based on your recent code, focus on error handling patterns this week.".to_string(),
        challenge_of_the_day: Some(CodingChallenge {
            challenge_id: uuid::Uuid::new_v4().to_string(),
            title: "Implement a simple cache".to_string(),
            description: "Create a basic LRU cache with get and put operations".to_string(),
            difficulty: "Intermediate".to_string(),
            estimated_time_minutes: 45,
            skills_practiced: vec!["Data Structures".to_string(), "Algorithms".to_string()],
        }),
    };

    app_state.metrics.record_mentor_daily_tips_generated();
    Ok(Json(response))
}

// Request/Response structs
#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub user_id: String,
    pub username: String,
    pub initial_skills: Option<HashMap<String, f32>>,
    pub learning_preferences: Option<LearningPreferences>,
}

#[derive(Debug, Deserialize)]
pub struct CodeFeedbackRequest {
    pub user_id: String,
    pub code: String,
    pub language: String,
    pub context: Option<String>,
    pub specific_focus: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SetGoalsRequest {
    pub user_id: String,
    pub goals: Vec<LearningGoalRequest>,
}

#[derive(Debug, Deserialize)]
pub struct LearningGoalRequest {
    pub title: String,
    pub description: String,
    pub target_skill: String,
    pub target_level: f32,
    pub deadline_weeks: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct LearningPathQuery {
    pub target_skill: Option<String>,
    pub current_level: Option<f32>,
    pub time_commitment_hours_per_week: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ProgressUpdateRequest {
    pub user_id: String,
    pub session_type: String,
    pub skills_practiced: Vec<String>,
    pub time_spent_minutes: u32,
    pub self_assessment: Option<SelfAssessmentRequest>,
    pub completed_milestones: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SelfAssessmentRequest {
    pub difficulty_rating: f32,
    pub confidence_rating: f32,
    pub enjoyment_rating: f32,
    pub learning_rating: f32,
    pub notes: Option<String>,
}

// Response structs
#[derive(Debug, Serialize)]
pub struct GoalsResponse {
    pub success: bool,
    pub goals_set: u32,
    pub estimated_completion_weeks: u32,
    pub personalized_plan: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillsResponse {
    pub user_id: String,
    pub skill_levels: HashMap<String, SkillLevel>,
    pub skill_trends: Vec<SkillTrend>,
    pub recommendations: Vec<SkillRecommendation>,
    pub next_milestone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillTrend {
    pub skill_name: String,
    pub trend_direction: String, // "improving", "stable", "declining"
    pub change_percentage: f32,
    pub time_period_days: u32,
}

#[derive(Debug, Serialize)]
pub struct SkillRecommendation {
    pub skill_name: String,
    pub current_level: f32,
    pub target_level: f32,
    pub priority: String,
    pub estimated_weeks: u32,
    pub resources: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningPathResponse {
    pub user_id: String,
    pub path_id: String,
    pub title: String,
    pub estimated_duration_weeks: u32,
    pub difficulty_level: String,
    pub milestones: Vec<LearningMilestone>,
    pub success_metrics: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningMilestone {
    pub milestone_id: String,
    pub title: String,
    pub description: String,
    pub estimated_weeks: u32,
    pub skills_covered: Vec<String>,
    pub resources: Vec<LearningResourceInfo>,
}

#[derive(Debug, Serialize)]
pub struct LearningResourceInfo {
    pub title: String,
    pub resource_type: String,
    pub estimated_hours: u32,
    pub difficulty: String,
}

#[derive(Debug, Serialize)]
pub struct ProgressResponse {
    pub success: bool,
    pub new_achievements: Vec<AchievementInfo>,
    pub skill_updates: HashMap<String, f32>,
    pub next_recommendations: Vec<String>,
    pub overall_progress_percentage: f32,
}

#[derive(Debug, Serialize)]
pub struct AchievementInfo {
    pub title: String,
    pub description: String,
    pub earned_date: u64,
    pub badge_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DailyTipsResponse {
    pub user_id: String,
    pub date: String,
    pub tips: Vec<DailyTip>,
    pub personalized_focus: String,
    pub challenge_of_the_day: Option<CodingChallenge>,
}

#[derive(Debug, Serialize)]
pub struct DailyTip {
    pub tip_id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub example_code: Option<String>,
    pub difficulty_level: String,
    pub estimated_impact: String,
}

#[derive(Debug, Serialize)]
pub struct CodingChallenge {
    pub challenge_id: String,
    pub title: String,
    pub description: String,
    pub difficulty: String,
    pub estimated_time_minutes: u32,
    pub skills_practiced: Vec<String>,
}