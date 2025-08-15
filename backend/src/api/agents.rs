use crate::agents::*;
use crate::ai_engine::providers::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct PlanRequest {
    pub goal: String,
    pub repo: Option<String>,
    pub constraints: Option<AgentConstraints>,
}

#[derive(Serialize, Deserialize)]
pub struct PlanResponse {
    pub plan_id: Uuid,
    pub steps: Vec<PlanStep>,
    pub budget: String,
    pub estimated_cost: Option<f64>,
    pub estimated_time_seconds: u64,
    pub risk_level: String,
}

#[derive(Serialize, Deserialize)]
pub struct PatchRequest {
    pub plan_id: Uuid,
    pub step_id: Option<Uuid>,
    pub apply: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PatchResponse {
    pub patch_id: Uuid,
    pub diff: String,
    pub files: Vec<String>,
    pub metrics: PatchMetrics,
}

#[derive(Serialize, Deserialize)]
pub struct PatchMetrics {
    pub lines_added: usize,
    pub lines_removed: usize,
    pub files_modified: usize,
    pub complexity_change: f32,
}

#[derive(Serialize, Deserialize)]
pub struct RunTestsRequest {
    pub patch_id: Uuid,
    pub env: Option<TestEnvironment>,
    pub coverage: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct TestEnvironment {
    pub language: String,
    pub framework: String,
}

#[derive(Serialize, Deserialize)]
pub struct RunTestsResponse {
    pub run_id: Uuid,
    pub pass: usize,
    pub fail: usize,
    pub coverage: Option<f32>,
    pub logs_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct RiskReportResponse {
    pub risk_score: f32,
    pub risk_level: String,
    pub regressions: Vec<String>,
    pub perf_delta: Option<f32>,
    pub security_flags: Vec<String>,
    pub rollback_cmd: String,
}

#[derive(Serialize, Deserialize)]
pub struct ArtifactResponse {
    pub artifact_type: String,
    pub urls: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RollbackRequest {
    pub patch_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct RollbackResponse {
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct TestFirstPatchRequest {
    pub goal: String,
    pub language: String,
    pub existing_code: Option<String>,
    pub context: Option<String>,
    pub test_framework: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TestFirstPatchResponse {
    pub patch_id: uuid::Uuid,
    pub failing_tests: String,
    pub implementation_code: String,
    pub test_results: TestExecutionResults,
    pub coverage_delta: CoverageDelta,
    pub validation_status: String,
}

pub async fn create_plan(
    State(state): State<crate::AppState>,
    Json(request): Json<PlanRequest>,
) -> Result<Json<PlanResponse>, StatusCode> {
    info!("Creating plan for goal: {}", request.goal);

    let planner = PlannerAgent::new(Box::new(state.provider_router.as_ref().clone()));
    
    let agent_request = AgentRequest {
        id: Uuid::new_v4(),
        goal: request.goal,
        context: request.repo,
        constraints: request.constraints.unwrap_or_default(),
        metadata: std::collections::HashMap::new(),
    };

    match planner.create_plan(&agent_request).await {
        Ok(response) => {
            if response.success {
                if let Ok(plan) = serde_json::from_value::<ExecutionPlan>(response.result) {
                    let budget = AgentBudget::new(&agent_request.constraints);
                    
                    let plan_response = PlanResponse {
                        plan_id: plan.id,
                        steps: plan.steps,
                        budget: budget.remaining_budget(),
                        estimated_cost: plan.estimated_cost,
                        estimated_time_seconds: plan.estimated_time.as_secs(),
                        risk_level: format!("{:?}", plan.risk_level),
                    };
                    
                    Ok(Json(plan_response))
                } else {
                    warn!("Failed to parse plan from agent response");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            } else {
                warn!("Plan creation failed: {:?}", response.error);
                Err(StatusCode::BAD_REQUEST)
            }
        }
        Err(e) => {
            warn!("Plan creation error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn create_patch(
    State(state): State<crate::AppState>,
    Json(request): Json<PatchRequest>,
) -> Result<Json<PatchResponse>, StatusCode> {
    info!("Creating patch for plan: {}", request.plan_id);

    // For MVP, generate a simple patch
    let patch_id = Uuid::new_v4();
    
    // Use codegen agent to generate code
    let completion_request = CompletionRequest {
        prompt: "Generate a simple Python function".to_string(),
        language: "python".to_string(),
        max_tokens: Some(200),
        temperature: Some(0.1),
        context: None,
    };

    match state.provider_router.complete(&completion_request).await {
        Ok(response) => {
            let patch_response = PatchResponse {
                patch_id,
                diff: format!("+{}", response.text),
                files: vec!["main.py".to_string()],
                metrics: PatchMetrics {
                    lines_added: response.text.lines().count(),
                    lines_removed: 0,
                    files_modified: 1,
                    complexity_change: 0.1,
                },
            };
            
            Ok(Json(patch_response))
        }
        Err(e) => {
            warn!("Patch creation error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn run_tests(
    State(state): State<crate::AppState>,
    Json(request): Json<RunTestsRequest>,
) -> Result<Json<RunTestsResponse>, StatusCode> {
    info!("Running tests for patch: {}", request.patch_id);

    // For MVP, simulate test execution
    let run_id = Uuid::new_v4();
    
    let response = RunTestsResponse {
        run_id,
        pass: 5,
        fail: 0,
        coverage: if request.coverage.unwrap_or(false) { Some(85.0) } else { None },
        logs_url: format!("/api/v1/artifacts/{}/logs", run_id),
    };
    
    Ok(Json(response))
}

pub async fn get_risk_report(
    State(state): State<crate::AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RiskReportResponse>, StatusCode> {
    info!("Getting risk report for: {}", id);

    let response = RiskReportResponse {
        risk_score: 0.3,
        risk_level: "low".to_string(),
        regressions: vec![],
        perf_delta: Some(0.05),
        security_flags: vec![],
        rollback_cmd: "git reset --hard HEAD~1".to_string(),
    };
    
    Ok(Json(response))
}

pub async fn get_artifacts(
    State(state): State<crate::AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    info!("Getting artifacts for: {}", id);

    let response = ArtifactResponse {
        artifact_type: "logs".to_string(),
        urls: vec![
            format!("/api/v1/artifacts/{}/execution.log", id),
            format!("/api/v1/artifacts/{}/coverage.json", id),
        ],
    };
    
    Ok(Json(response))
}

pub async fn rollback_patch(
    State(state): State<crate::AppState>,
    Json(request): Json<RollbackRequest>,
) -> Result<Json<RollbackResponse>, StatusCode> {
    info!("Rolling back patch: {}", request.patch_id);

    let response = RollbackResponse {
        status: "success".to_string(),
    };
    
    Ok(Json(response))
}

pub async fn create_test_first_patch(
    State(state): State<crate::AppState>,
    Json(request): Json<TestFirstPatchRequest>,
) -> Result<Json<TestFirstPatchResponse>, StatusCode> {
    info!("Creating test-first patch for goal: {}", request.goal);

    let test_first_agent = TestFirstAgent::new(Box::new(state.provider_router.as_ref().clone()));
    
    let test_first_request = TestFirstRequest {
        goal: request.goal,
        language: request.language,
        existing_code: request.existing_code,
        context: request.context,
        test_framework: request.test_framework,
    };

    match test_first_agent.generate_test_first_patch(&test_first_request).await {
        Ok(response) => {
            let patch_id = uuid::Uuid::new_v4();
            
            let api_response = TestFirstPatchResponse {
                patch_id,
                failing_tests: response.failing_tests,
                implementation_code: response.implementation_code,
                test_results: response.test_results,
                coverage_delta: response.coverage_delta,
                validation_status: format!("{:?}", response.validation_status),
            };
            
            Ok(Json(api_response))
        }
        Err(e) => {
            warn!("Test-first patch creation error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn analyze_security(
    State(state): State<crate::AppState>,
    Json(request): Json<SecurityAnalysisRequest>,
) -> Result<Json<SecurityAnalysisResponse>, StatusCode> {
    info!("Running security analysis for {} code", request.language);

    let security_analyzer = SecurityAnalyzer::new();
    
    match security_analyzer.analyze_security(&request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Security analysis error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn analyze_build(
    State(state): State<crate::AppState>,
    Json(request): Json<BuildAnalysisRequest>,
) -> Result<Json<BuildAnalysisResponse>, StatusCode> {
    info!("Running build analysis for {} project", request.language);

    let build_doctor = BuildDoctorAgent::new();
    
    match build_doctor.analyze_build(&request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Build analysis error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn agent_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/plan", post(create_plan))
        .route("/patch", post(create_patch))
        .route("/test-first-patch", post(create_test_first_patch))
        .route("/security-analysis", post(analyze_security))
        .route("/build-analysis", post(analyze_build))
        .route("/run-tests", post(run_tests))
        .route("/risk-report/:id", get(get_risk_report))
        .route("/artifacts/:id", get(get_artifacts))
        .route("/rollback", post(rollback_patch))
}