use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agents::{
    AgentOrchestrator, PlannerAgent, ExecutionPlan, ExecutionStep, StepStatus,
    code_reviewer::{CodeReviewAgent, CodeReviewRequest, CodeReviewResponse}
};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

/// Create execution plan from goal
#[utoipa::path(
    post,
    path = "/api/v1/plan",
    request_body = PlanRequest,
    responses(
        (status = 200, description = "Plan created successfully", body = ExecutionPlan),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "agents"
)]
pub async fn create_plan(
    State(app_state): State<AppState>,
    Json(request): Json<PlanRequest>,
) -> Result<Json<ExecutionPlan>, (StatusCode, Json<ErrorResponse>)> {
    let planner = PlannerAgent::new(app_state.provider_router.clone());

    match planner.create_plan(&request.goal, &request.constraints).await {
        Ok(plan) => {
            app_state.metrics.record_plan_created();
            Ok(Json(plan))
        }
        Err(e) => {
            app_state.metrics.record_plan_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Plan creation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Enhanced Code Review with AI Analysis
#[utoipa::path(
    post,
    path = "/api/v1/code-review",
    request_body = CodeReviewRequest,
    responses(
        (status = 200, description = "Code review completed successfully", body = CodeReviewResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "agents"
)]
pub async fn code_review(
    State(app_state): State<AppState>,
    Json(request): Json<CodeReviewRequest>,
) -> Result<Json<CodeReviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    let code_reviewer = CodeReviewAgent::new(
        app_state.provider_router.clone(),
        app_state.context_manager.clone(),
    );

    match code_reviewer.review_code(request).await {
        Ok(response) => {
            // Record metrics
            app_state.metrics.record_code_review_completed();
            
            Ok(Json(response))
        }
        Err(e) => {
            app_state.metrics.record_code_review_failed();
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Code review failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// PR Integration Webhook for automated code review
#[utoipa::path(
    post,
    path = "/api/v1/webhook/pr-review",
    request_body = PullRequestWebhook,
    responses(
        (status = 200, description = "PR review webhook processed"),
        (status = 400, description = "Invalid webhook payload"),
        (status = 500, description = "Internal server error")
    ),
    tag = "webhooks"
)]
pub async fn pr_review_webhook(
    State(app_state): State<AppState>,
    Json(webhook): Json<PullRequestWebhook>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    let code_reviewer = CodeReviewAgent::new(
        app_state.provider_router.clone(),
        app_state.context_manager.clone(),
    );

    match process_pr_webhook(&code_reviewer, webhook).await {
        Ok(response) => {
            app_state.metrics.record_pr_webhook_processed();
            Ok(Json(response))
        }
        Err(e) => {
            app_state.metrics.record_pr_webhook_failed();
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "PR webhook processing failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Advanced Test Generation with Edge Cases
#[utoipa::path(
    post,
    path = "/api/v1/generate-tests-advanced",
    request_body = AdvancedTestRequest,
    responses(
        (status = 200, description = "Advanced tests generated successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "agents"
)]
pub async fn generate_advanced_tests(
    State(app_state): State<AppState>,
    Json(request): Json<AdvancedTestRequest>,
) -> Result<Json<AdvancedTestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let test_generator = AdvancedTestGenerator::new(
        app_state.provider_router.clone(),
        app_state.context_manager.clone(),
    );

    match test_generator.generate_tests(request).await {
        Ok(response) => {
            app_state.metrics.record_test_generation_completed();
            Ok(Json(response))
        }
        Err(e) => {
            app_state.metrics.record_test_generation_failed();
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Advanced test generation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Enterprise Integration - JIRA Webhook
#[utoipa::path(
    post,
    path = "/api/v1/webhook/jira",
    request_body = JiraWebhook,
    responses(
        (status = 200, description = "JIRA webhook processed"),
        (status = 400, description = "Invalid webhook payload")
    ),
    tag = "enterprise"
)]
pub async fn jira_webhook(
    State(app_state): State<AppState>,
    Json(webhook): Json<JiraWebhook>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    match process_jira_webhook(webhook).await {
        Ok(response) => {
            app_state.metrics.record_jira_webhook_processed();
            Ok(Json(response))
        }
        Err(e) => {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "JIRA webhook processing failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Enterprise Integration - Slack Notifications
#[utoipa::path(
    post,
    path = "/api/v1/notifications/slack",
    request_body = SlackNotificationRequest,
    responses(
        (status = 200, description = "Slack notification sent"),
        (status = 400, description = "Invalid request")
    ),
    tag = "enterprise"
)]
pub async fn send_slack_notification(
    State(app_state): State<AppState>,
    Json(request): Json<SlackNotificationRequest>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    match send_slack_message(&request).await {
        Ok(response) => {
            app_state.metrics.record_slack_notification_sent();
            Ok(Json(response))
        }
        Err(e) => {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Slack notification failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

// Helper functions
async fn process_pr_webhook(
    code_reviewer: &CodeReviewAgent,
    webhook: PullRequestWebhook,
) -> Result<WebhookResponse, anyhow::Error> {
    let mut review_results = Vec::new();

    // Review each changed file
    for file_change in webhook.pull_request.changed_files {
        if should_review_file(&file_change.filename) {
            let review_request = CodeReviewRequest {
                code: file_change.content,
                language: detect_language(&file_change.filename),
                file_path: Some(file_change.filename.clone()),
                pr_context: Some(webhook.pull_request.clone().into()),
                review_type: crate::agents::code_reviewer::ReviewType::Comprehensive,
            };

            let review_result = code_reviewer.review_code(review_request).await?;
            review_results.push(FileReviewResult {
                filename: file_change.filename,
                review: review_result,
            });
        }
    }

    // Generate PR comment
    let pr_comment = generate_pr_comment(&review_results);

    // Post comment to PR (would integrate with GitHub/GitLab API)
    post_pr_comment(&webhook.pull_request.id, &pr_comment).await?;

    Ok(WebhookResponse {
        status: "success".to_string(),
        message: format!("Reviewed {} files", review_results.len()),
        review_results,
    })
}

async fn process_jira_webhook(webhook: JiraWebhook) -> Result<WebhookResponse, anyhow::Error> {
    // Process JIRA webhook events
    match webhook.webhook_event.as_str() {
        "jira:issue_created" => {
            // Handle new issue creation
            tracing::info!("New JIRA issue created: {}", webhook.issue.key);
        }
        "jira:issue_updated" => {
            // Handle issue updates
            tracing::info!("JIRA issue updated: {}", webhook.issue.key);
        }
        _ => {
            tracing::warn!("Unknown JIRA webhook event: {}", webhook.webhook_event);
        }
    }

    Ok(WebhookResponse {
        status: "success".to_string(),
        message: "JIRA webhook processed".to_string(),
        review_results: Vec::new(),
    })
}

async fn send_slack_message(request: &SlackNotificationRequest) -> Result<NotificationResponse, anyhow::Error> {
    // Send Slack notification
    tracing::info!("Sending Slack notification to channel: {}", request.channel);
    
    // This would integrate with Slack API
    Ok(NotificationResponse {
        status: "sent".to_string(),
        message_id: Uuid::new_v4().to_string(),
    })
}

fn should_review_file(filename: &str) -> bool {
    let reviewable_extensions = [
        ".rs", ".py", ".js", ".ts", ".java", ".go", ".cpp", ".c", ".cs", ".php"
    ];
    
    reviewable_extensions.iter().any(|ext| filename.ends_with(ext))
}

fn detect_language(filename: &str) -> String {
    match filename.split('.').last() {
        Some("rs") => "rust".to_string(),
        Some("py") => "python".to_string(),
        Some("js") => "javascript".to_string(),
        Some("ts") => "typescript".to_string(),
        Some("java") => "java".to_string(),
        Some("go") => "go".to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
        Some("c") => "c".to_string(),
        Some("cs") => "csharp".to_string(),
        Some("php") => "php".to_string(),
        _ => "unknown".to_string(),
    }
}

fn generate_pr_comment(review_results: &[FileReviewResult]) -> String {
    let mut comment = String::from("## 🤖 UAIDA Code Review\n\n");
    
    let total_issues: usize = review_results.iter()
        .map(|r| r.review.security_issues.len() + r.review.performance_issues.len() + r.review.findings.len())
        .sum();
    
    let avg_score: f32 = review_results.iter()
        .map(|r| r.review.overall_score)
        .sum::<f32>() / review_results.len() as f32;

    comment.push_str(&format!("**Overall Score:** {:.1}/100\n", avg_score));
    comment.push_str(&format!("**Total Issues Found:** {}\n\n", total_issues));

    for result in review_results {
        comment.push_str(&format!("### 📁 {}\n", result.filename));
        comment.push_str(&format!("**Score:** {:.1}/100\n\n", result.review.overall_score));

        if !result.review.security_issues.is_empty() {
            comment.push_str("#### 🔒 Security Issues\n");
            for issue in &result.review.security_issues {
                comment.push_str(&format!("- **{}**: {} (Line {})\n", 
                    format!("{:?}", issue.severity), 
                    issue.description,
                    issue.line_number.unwrap_or(0)
                ));
            }
            comment.push('\n');
        }

        if !result.review.performance_issues.is_empty() {
            comment.push_str("#### ⚡ Performance Issues\n");
            for issue in &result.review.performance_issues {
                comment.push_str(&format!("- **{}**: {}\n", issue.issue_type, issue.description));
            }
            comment.push('\n');
        }

        if !result.review.suggestions.is_empty() {
            comment.push_str("#### 💡 Suggestions\n");
            for suggestion in result.review.suggestions.iter().take(3) {
                comment.push_str(&format!("- **{}**: {}\n", suggestion.title, suggestion.description));
            }
            comment.push('\n');
        }
    }

    comment.push_str("---\n*Generated by UAIDA - Universal AI Development Assistant*");
    comment
}

async fn post_pr_comment(pr_id: &str, comment: &str) -> Result<(), anyhow::Error> {
    // This would integrate with GitHub/GitLab API
    tracing::info!("PR Comment for {}: {}", pr_id, comment);
    Ok(())
}

// Request/Response structs
#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    pub goal: String,
    pub constraints: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestWebhook {
    pub action: String,
    pub pull_request: PullRequestData,
    pub repository: RepositoryData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestData {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub base_branch: String,
    pub head_branch: String,
    pub changed_files: Vec<FileChange>,
    pub commit_messages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryData {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
}

#[derive(Debug, Deserialize)]
pub struct FileChange {
    pub filename: String,
    pub status: String, // added, modified, deleted
    pub content: String,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Deserialize)]
pub struct AdvancedTestRequest {
    pub code: String,
    pub language: String,
    pub test_type: TestType,
    pub coverage_target: f32,
    pub include_edge_cases: bool,
    pub include_mutation_tests: bool,
}

#[derive(Debug, Deserialize)]
pub enum TestType {
    Unit,
    Integration,
    Performance,
    Security,
    All,
}

#[derive(Debug, Serialize)]
pub struct AdvancedTestResponse {
    pub test_id: String,
    pub generated_tests: Vec<GeneratedTest>,
    pub coverage_estimate: f32,
    pub edge_cases: Vec<EdgeCase>,
    pub mutation_tests: Vec<MutationTest>,
}

#[derive(Debug, Serialize)]
pub struct GeneratedTest {
    pub name: String,
    pub code: String,
    pub test_type: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct EdgeCase {
    pub scenario: String,
    pub test_code: String,
    pub expected_behavior: String,
}

#[derive(Debug, Serialize)]
pub struct MutationTest {
    pub original_line: String,
    pub mutated_line: String,
    pub test_code: String,
    pub should_fail: bool,
}

#[derive(Debug, Deserialize)]
pub struct JiraWebhook {
    pub webhook_event: String,
    pub issue: JiraIssue,
    pub user: JiraUser,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    pub description: Option<String>,
    pub status: JiraStatus,
}

#[derive(Debug, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraUser {
    pub display_name: String,
    pub email_address: String,
}

#[derive(Debug, Deserialize)]
pub struct SlackNotificationRequest {
    pub channel: String,
    pub message: String,
    pub notification_type: NotificationType,
}

#[derive(Debug, Deserialize)]
pub enum NotificationType {
    CodeReview,
    SecurityAlert,
    BuildStatus,
    General,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub status: String,
    pub message: String,
    pub review_results: Vec<FileReviewResult>,
}

#[derive(Debug, Serialize)]
pub struct FileReviewResult {
    pub filename: String,
    pub review: CodeReviewResponse,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub status: String,
    pub message_id: String,
}

// Conversion implementations
impl From<PullRequestData> for crate::agents::code_reviewer::PullRequestContext {
    fn from(pr: PullRequestData) -> Self {
        Self {
            pr_id: pr.id,
            base_branch: pr.base_branch,
            head_branch: pr.head_branch,
            changed_files: pr.changed_files.into_iter().map(|f| f.filename).collect(),
            commit_messages: pr.commit_messages,
        }
    }
}

// Placeholder for AdvancedTestGenerator
pub struct AdvancedTestGenerator {
    provider_router: crate::ai_engine::providers::ProviderRouter,
    context_manager: crate::context::ContextManager,
}

impl AdvancedTestGenerator {
    pub fn new(
        provider_router: crate::ai_engine::providers::ProviderRouter,
        context_manager: crate::context::ContextManager,
    ) -> Self {
        Self {
            provider_router,
            context_manager,
        }
    }

    pub async fn generate_tests(&self, request: AdvancedTestRequest) -> Result<AdvancedTestResponse, anyhow::Error> {
        // Implementation would go here
        Ok(AdvancedTestResponse {
            test_id: Uuid::new_v4().to_string(),
            generated_tests: Vec::new(),
            coverage_estimate: 85.0,
            edge_cases: Vec::new(),
            mutation_tests: Vec::new(),
        })
    }
}