use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::env;

use crate::observability::get_metrics;
use crate::sandbox::{ExecutionRequest, SandboxConfig, SandboxRunner};
use crate::{AppState, CompletionRequest};

// API Request/Response Types
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanRequest {
    pub goal: String,
    pub context: PlanContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanContext {
    pub files: Vec<String>,
    pub constraints: PlanConstraints,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanConstraints {
    pub max_files: Option<usize>,
    pub max_loc: Option<usize>,
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanResponse {
    pub plan_id: String,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub estimated_duration: Duration,
    pub affected_files: Vec<String>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub step_type: StepType,
    pub dependencies: Vec<String>,
    pub estimated_duration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StepType {
    Analysis,
    CodeGeneration,
    Testing,
    Refactoring,
    Documentation,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchRequest {
    pub plan_id: String,
    pub target_files: Vec<String>,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub file: String,
    pub operation: ChangeOperation,
    pub content: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ChangeOperation {
    Create,
    Modify,
    Delete,
    Rename,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchResponse {
    pub patch_id: String,
    pub plan_id: String,
    pub changes_applied: Vec<AppliedChange>,
    pub conflicts: Vec<String>,
    pub success: bool,
    pub rollback_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppliedChange {
    pub file: String,
    pub operation: ChangeOperation,
    pub lines_changed: usize,
    pub backup_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunTestsRequest {
    pub patch_id: String,
    pub test_command: Option<String>,
    pub test_files: Vec<String>,
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunTestsResponse {
    pub run_id: String,
    pub patch_id: String,
    pub success: bool,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub execution_time: Duration,
    pub coverage: Option<TestCoverage>,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestCoverage {
    pub percentage: f32,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub files: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub file: String,
    pub line: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactsResponse {
    pub run_id: String,
    pub artifacts: Vec<ArtifactInfo>,
    pub total_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub name: String,
    pub artifact_type: String,
    pub size_bytes: u64,
    pub download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskReportResponse {
    pub patch_id: String,
    pub risk_level: RiskLevel,
    pub security_issues: Vec<SecurityIssue>,
    pub performance_impact: PerformanceImpact,
    pub breaking_changes: Vec<BreakingChange>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: String,
    pub description: String,
    pub file: String,
    pub line: Option<usize>,
    pub mitigation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceImpact {
    pub estimated_change: f32, // Percentage change
    pub affected_functions: Vec<String>,
    pub bottlenecks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BreakingChange {
    pub description: String,
    pub affected_apis: Vec<String>,
    pub migration_guide: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub patch_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollbackResponse {
    pub success: bool,
    pub restored_files: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
    pub details: Option<String>,
}

// API Handlers

// Remove duplicate apply_patch function - this was incorrectly returning PlanResponse

pub async fn apply_patch(
    State(state): State<AppState>,
    Json(request): Json<PatchRequest>,
) -> Result<Json<PatchResponse>, (StatusCode, Json<ApiError>)> {
    use crate::observability::tracing::{generate_request_id, create_request_span, record_patch_attributes};

    let request_id = generate_request_id();
    let span = create_request_span("apply_patch", &request_id);
    let _enter = span.enter();
    let start_time = Instant::now();
    
    // Record metrics
    let metrics = get_metrics();
    metrics.http_requests_total
        .with_label_values(&["/api/v1/patch", "POST", "200"])
        .inc();

    info!("Applying patch for plan: {}", request.plan_id);
    
    let patch_id = Uuid::new_v4().to_string();
    // Risk gate pre-check (optional automatic block)
    if env::var("ENABLE_RISK_GATE").unwrap_or_else(|_| "false".into()) == "true" {
        if let Err((status, err)) = preflight_risk_gate_check(&request.plan_id, &patch_id).await {
            return Err((status, Json(err)));
        }
    }
    let mut changes_applied = Vec::new();
    let mut conflicts = Vec::new();
    let mut rollback_commands = Vec::new();
    
    // Simulate patch application (in real implementation, this would actually modify files)
    for change in &request.changes {
        match change.operation {
            ChangeOperation::Create => {
                // Simulate file creation
                changes_applied.push(AppliedChange {
                    file: change.file.clone(),
                    operation: ChangeOperation::Create,
                    lines_changed: change.content.lines().count(),
                    backup_path: None,
                });
                rollback_commands.push(format!("rm {}", change.file));
            }
            ChangeOperation::Modify => {
                // Simulate file modification
                let backup_path = format!("{}.backup.{}", change.file, patch_id);
                changes_applied.push(AppliedChange {
                    file: change.file.clone(),
                    operation: ChangeOperation::Modify,
                    lines_changed: change.content.lines().count(),
                    backup_path: Some(backup_path.clone()),
                });
                rollback_commands.push(format!("mv {} {}", backup_path, change.file));
            }
            ChangeOperation::Delete => {
                // Simulate file deletion
                let backup_path = format!("{}.backup.{}", change.file, patch_id);
                changes_applied.push(AppliedChange {
                    file: change.file.clone(),
                    operation: ChangeOperation::Delete,
                    lines_changed: 0,
                    backup_path: Some(backup_path.clone()),
                });
                rollback_commands.push(format!("mv {} {}", backup_path, change.file));
            }
            ChangeOperation::Rename => {
                // Handle rename operation
                conflicts.push(format!("Rename operation not yet implemented for {}", change.file));
            }
        }
    }
    
    let success = conflicts.is_empty();

    // Optional strict risk enforcement after patch simulation (post-analysis gate)
    if env::var("RISK_BLOCK_ENFORCE").unwrap_or_else(|_| "false".into()) == "true" {
        if let Err((status, err)) = post_analysis_risk_gate(&state, &patch_id).await {
            return Err((status, Json(err)));
        }
    }
    
    let response = PatchResponse {
        patch_id,
        plan_id: request.plan_id,
        changes_applied,
        conflicts,
        success,
        rollback_commands,
    };
    
    // Record span attributes
    record_patch_attributes(&span, &patch_id, changes_applied.len());

    // Record latency
    let latency = start_time.elapsed();
    metrics.http_request_duration_ms
        .with_label_values(&["/api/v1/patch", "POST"])
        .observe(latency.as_millis() as f64);
    
    info!(
        request_id = %request_id,
        plan_id = %response.plan_id,
        patch_id = %response.patch_id,
        latency_ms = latency.as_millis(),
        success = success,
        "Patch applied"
    );
    
    Ok(Json(response))
}

pub async fn run_tests(
    State(state): State<AppState>,
    Json(request): Json<RunTestsRequest>,
) -> Result<Json<RunTestsResponse>, (StatusCode, Json<ApiError>)> {
    use crate::observability::tracing::{generate_request_id, create_request_span, record_run_attributes};
    use crate::database::repositories::{RunsRepository, CreateRunRequest, UpdateRunRequest};
    
    // Generate correlation ID and create request span  
    let request_id = generate_request_id();
    let span = create_request_span("run_tests", &request_id);
    let _enter = span.enter();
    
    let start_time = Instant::now();
    
    // Record metrics
    let metrics = get_metrics();
    metrics.http_requests_total
        .with_label_values(&["/api/v1/run-tests", "POST", "200"])
        .inc();

    info!(request_id = %request_id, patch_id = %request.patch_id, "Running tests");
    
    let run_id = Uuid::new_v4();
    
    // P0 Day-3: Create run record in database
    let runs_repo = RunsRepository::new(state.database.pool.clone());
    
    // Create initial run record
    let create_run_req = CreateRunRequest {
        project_id: Uuid::new_v4(), // TODO: Get from request/context
        patch_id: Uuid::parse_str(&request.patch_id).ok(),
        plan_id: None, // TODO: Link to plan if available
        user_id: Uuid::new_v4(), // TODO: Get from authentication
        run_type: "test".to_string(),
        command: request.test_command.clone(),
        environment: serde_json::json!({}),
        working_directory: None,
        timeout_seconds: request.timeout_s.map(|t| t as i32),
        metadata: serde_json::json!({"request_id": request_id}),
    };
    
    let run_record = match runs_repo.create(create_run_req).await {
        Ok(record) => record,
        Err(e) => {
            warn!(request_id = %request_id, error = %e, "Failed to create run record");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError {
                error: "Failed to create run record".to_string(),
                code: "DATABASE_ERROR".to_string(),
                details: Some(e.to_string()),
            })));
        }
    };
    
    // Determine language and sandbox runner (simplified)
    let language = if request.test_files.iter().any(|f| f.ends_with(".py")) {
        "python"
    } else if request.test_files.iter().any(|f| f.ends_with(".js") || f.ends_with(".ts")) {
        "javascript"
    } else {
        "unknown"
    };
    
    // Create sandbox execution request
    let sandbox_request = ExecutionRequest {
        code: "# Test execution placeholder".to_string(),
        language: language.to_string(),
        test_command: request.test_command.clone(),
        files: HashMap::new(), // Would be populated with actual test files
        environment: HashMap::new(),
        working_directory: None,
    };
    
    let sandbox_config = SandboxConfig {
        timeout: Duration::from_secs(request.timeout_s.unwrap_or(300)),
        ..SandboxConfig::default()
    };
    
    // For Sprint 1, simulate test execution
    let (tests_passed, tests_failed, coverage, failures) = if language == "python" {
        // Simulate Python test results
        (8, 2, Some(TestCoverage {
            percentage: 85.2,
            lines_covered: 156,
            lines_total: 183,
            files: {
                let mut files = HashMap::new();
                files.insert("main.py".to_string(), 90.5);
                files.insert("utils.py".to_string(), 78.3);
                files
            },
        }), vec![
            TestFailure {
                test_name: "test_edge_case".to_string(),
                error_message: "AssertionError: Expected 5, got 4".to_string(),
                file: "test_main.py".to_string(),
                line: Some(42),
            }
        ])
    } else if language == "javascript" {
        // Simulate Node.js test results
        (12, 1, Some(TestCoverage {
            percentage: 92.1,
            lines_covered: 241,
            lines_total: 262,
            files: {
                let mut files = HashMap::new();
                files.insert("index.js".to_string(), 95.2);
                files.insert("utils.js".to_string(), 88.7);
                files
            },
        }), vec![
            TestFailure {
                test_name: "should handle invalid input".to_string(),
                error_message: "TypeError: Cannot read property 'length' of undefined".to_string(),
                file: "utils.test.js".to_string(),
                line: Some(28),
            }
        ])
    } else {
        (0, 0, None, vec![])
    };
    
    let execution_time = Duration::from_millis(2500); // Simulated
    let success = tests_failed == 0;
    
    // P0 Day-3: Update run record with results
    let update_run_req = UpdateRunRequest {
        status: Some(if success { "completed".to_string() } else { "failed".to_string() }),
        started_at: Some(chrono::Utc::now() - chrono::Duration::milliseconds(execution_time.as_millis() as i64)),
        completed_at: Some(chrono::Utc::now()),
        duration_ms: Some(execution_time.as_millis() as i64),
        exit_code: Some(if success { 0 } else { 1 }),
        stdout_log: Some(format!("Tests passed: {}, Tests failed: {}", tests_passed, tests_failed)),
        stderr_log: if !failures.is_empty() { 
            Some(failures.iter().map(|f| format!("{}: {}", f.test_name, f.error_message)).collect::<Vec<_>>().join("\n"))
        } else { 
            None 
        },
        test_results: Some(serde_json::json!({
            "passed": tests_passed,
            "failed": tests_failed,
            "success": success,
            "failures": failures
        })),
        coverage_data: coverage.as_ref().map(|c| serde_json::to_value(c).unwrap_or_default()),
        performance_metrics: Some(serde_json::json!({
            "execution_time_ms": execution_time.as_millis(),
            "language": language
        })),
        error_message: if !success {
            Some(format!("{} test(s) failed", tests_failed))
        } else {
            None
        },
        metadata: Some(serde_json::json!({
            "request_id": request_id,
            "language": language,
            "test_files_count": request.test_files.len()
        })),
    };
    
    if let Err(e) = runs_repo.update(run_record.id, update_run_req).await {
        warn!(request_id = %request_id, run_id = %run_record.id, error = %e, "Failed to update run record");
    }
    
    // Record span attributes
    record_run_attributes(&span, &run_record.id.to_string(), language, request.test_files.len());
    
    let response = RunTestsResponse {
        run_id: run_record.id.to_string(),
        patch_id: request.patch_id,
        success,
        tests_passed,
        tests_failed,
        execution_time,
        coverage,
        failures,
    };
    
    // Record latency
    let latency = start_time.elapsed();
    metrics.http_request_duration_ms
        .with_label_values(&["/api/v1/run-tests", "POST"])
        .observe(latency.as_millis() as f64);
    
    info!(
        request_id = %request_id,
        run_id = %run_record.id,
        latency_ms = latency.as_millis(),
        tests_passed = tests_passed,
        tests_failed = tests_failed,
        success = success,
        "Tests completed and persisted"
    );
    
    Ok(Json(response))
}

pub async fn get_artifacts(
    Path(run_id): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<ArtifactsResponse>, (StatusCode, Json<ApiError>)> {
    let start_time = Instant::now();
    
    // Record metrics
    let metrics = get_metrics();
    metrics.http_requests_total
        .with_label_values(&["/api/v1/artifacts", "GET", "200"])
        .inc();

    info!("Getting artifacts for run: {}", run_id);
    
    // Simulate artifact collection
    let artifacts = vec![
        ArtifactInfo {
            name: "test_results.xml".to_string(),
            artifact_type: "test_report".to_string(),
            size_bytes: 2048,
            download_url: format!("/api/v1/artifacts/{}/download/test_results.xml", run_id),
        },
        ArtifactInfo {
            name: "coverage.json".to_string(),
            artifact_type: "coverage".to_string(),
            size_bytes: 4096,
            download_url: format!("/api/v1/artifacts/{}/download/coverage.json", run_id),
        },
        ArtifactInfo {
            name: "execution.log".to_string(),
            artifact_type: "log".to_string(),
            size_bytes: 1024,
            download_url: format!("/api/v1/artifacts/{}/download/execution.log", run_id),
        },
    ];
    
    let total_size = artifacts.iter().map(|a| a.size_bytes).sum();
    
    let response = ArtifactsResponse {
        run_id,
        artifacts,
        total_size,
    };
    
    // Record latency
    let latency = start_time.elapsed();
    metrics.http_request_duration_ms
        .with_label_values(&["/api/v1/artifacts", "GET"])
        .observe(latency.as_millis() as f64);
    
    info!("Artifacts retrieved in {:?}", latency);
    
    Ok(Json(response))
}

pub async fn get_risk_report(
    Path(patch_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RiskReportResponse>, (StatusCode, Json<ApiError>)> {
    use crate::observability::tracing::{generate_request_id, create_request_span};
    use crate::risk::{RiskGate, RiskGateConfig};
    use crate::database::repositories::{RunsRepository, ArtifactsRepository};
    
    // Generate correlation ID and create request span
    let request_id = generate_request_id();
    let span = create_request_span("get_risk_report", &request_id);
    let _enter = span.enter();
    
    let start_time = Instant::now();
    
    // Record metrics
    let metrics = get_metrics();
    metrics.http_requests_total
        .with_label_values(&["/api/v1/risk-report", "GET", "200"])
        .inc();

    info!(
        request_id = %request_id,
        patch_id = %patch_id,
        "Generating comprehensive risk report"
    );
    
    // P0 Day-4: Real risk analysis with coverage/performance Δ
    let risk_gate = RiskGate::new(RiskGateConfig::default());
    let runs_repo = RunsRepository::new(state.database.pool.clone());
    let artifacts_repo = ArtifactsRepository::new(state.database.pool.clone());
    
    // Find current run for this patch (simplified - would use proper patch tracking)
    let current_run_id = uuid::Uuid::parse_str(&patch_id)
        .unwrap_or_else(|_| uuid::Uuid::new_v4());
    
    let risk_decision = match risk_gate.evaluate_patch(
        &patch_id,
        current_run_id,
        None, // Let risk gate find baseline
        &runs_repo,
        &artifacts_repo,
    ).await {
        Ok(decision) => decision,
        Err(e) => {
            warn!(
                request_id = %request_id,
                patch_id = %patch_id,
                error = %e,
                "Risk analysis failed, using fallback"
            );
            // Fallback to simple risk analysis
            return Ok(Json(create_fallback_risk_report(patch_id)));
        }
    };
    
    // Convert risk assessment to API response format
    let response = RiskReportResponse {
        patch_id: patch_id.clone(),
        risk_level: match risk_decision.risk_assessment.overall_risk {
            crate::risk::RiskLevel::Low => RiskLevel::Low,
            crate::risk::RiskLevel::Medium => RiskLevel::Medium,
            crate::risk::RiskLevel::High => RiskLevel::High,
            crate::risk::RiskLevel::Critical => RiskLevel::Critical,
        },
        security_issues: risk_decision.risk_assessment.security_issues.into_iter().map(|issue| {
            SecurityIssue {
                severity: issue.severity,
                description: issue.description,
                file: issue.file,
                line: issue.line,
                mitigation: issue.mitigation,
            }
        }).collect(),
        performance_impact: PerformanceImpact {
            estimated_change: risk_decision.risk_assessment.metadata.performance_delta_ms as f32 / 1000.0,
            affected_functions: vec!["test_execution".to_string()],
            bottlenecks: if risk_decision.should_block { vec!["performance_regression".to_string()] } else { vec![] },
        },
        breaking_changes: risk_decision.risk_assessment.breaking_changes.into_iter().map(|change| {
            BreakingChange {
                description: change.description,
                affected_apis: change.affected_apis,
                migration_guide: change.migration_guide,
            }
        }).collect(),
        recommendations: risk_decision.risk_assessment.recommendations,
    };
    
    // Record latency
    let latency = start_time.elapsed();
    metrics.http_request_duration_ms
        .with_label_values(&["/api/v1/risk-report", "GET"])
        .observe(latency.as_millis() as f64);
    
    info!(
        request_id = %request_id,
        patch_id = %patch_id,
        risk_score = risk_decision.risk_assessment.risk_score,
        should_block = risk_decision.should_block,
        latency_ms = latency.as_millis(),
        "Risk report generated with comprehensive analysis"
    );
    
    Ok(Json(response))
}

/// Create fallback risk report when full analysis fails
fn create_fallback_risk_report(patch_id: String) -> RiskReportResponse {
    RiskReportResponse {
        patch_id,
        risk_level: RiskLevel::Medium,
        security_issues: vec![],
        performance_impact: PerformanceImpact {
            estimated_change: 0.0,
            affected_functions: vec![],
            bottlenecks: vec![],
        },
        breaking_changes: vec![],
        recommendations: vec![
            "Risk analysis incomplete - manual review recommended".to_string(),
            "Run tests and validate functionality before merging".to_string(),
        ],
    }
}

pub async fn rollback_patch(
    State(_state): State<AppState>,
    Json(request): Json<RollbackRequest>,
) -> Result<Json<RollbackResponse>, (StatusCode, Json<ApiError>)> {
    let start_time = Instant::now();
    
    // Record metrics
    let metrics = get_metrics();
    metrics.http_requests_total
        .with_label_values(&["/api/v1/rollback", "POST", "200"])
        .inc();

    info!("Rolling back patch: {} (reason: {})", request.patch_id, request.reason);
    
    // Simulate rollback execution
    let restored_files = vec![
        "src/main.py".to_string(),
        "src/utils.py".to_string(),
        "tests/test_main.py".to_string(),
    ];
    
    let response = RollbackResponse {
        success: true,
        restored_files,
        message: format!("Successfully rolled back patch {} due to: {}", request.patch_id, request.reason),
    };
    
    // Record latency
    let latency = start_time.elapsed();
    metrics.http_request_duration_ms
        .with_label_values(&["/api/v1/rollback", "POST"])
        .observe(latency.as_millis() as f64);
    
    info!("Rollback completed in {:?}", latency);
    
    Ok(Json(response))
}

// P0 Task #1: Implement POST /api/v1/plan endpoint - WITH TRACING
async fn create_plan(
    State(state): State<AppState>,
    Json(request): Json<PlanRequest>,
) -> Result<Json<PlanResponse>, (StatusCode, Json<ApiError>)> {
    use crate::observability::tracing::{generate_request_id, create_request_span, record_plan_attributes};
    
    // Generate correlation ID and create request span
    let request_id = generate_request_id();
    let span = create_request_span("create_plan", &request_id);
    let _enter = span.enter();
    
    info!(request_id = %request_id, goal = %request.goal, "Creating plan");
    let start_time = Instant::now();
    
    // Record metrics
    let metrics = get_metrics();
    metrics.http_requests_total
        .with_label_values(&["/api/v1/plan", "POST", "200"])
        .inc();

    let plan_id = Uuid::new_v4().to_string();
    
    // Create plan steps based on goal analysis
    let steps = vec![
        PlanStep {
            id: Uuid::new_v4().to_string(),
            description: format!("Analyze codebase for: {}", request.goal),
            step_type: StepType::Analysis,
            dependencies: vec![],
            estimated_duration: Duration::from_secs(30),
        },
        PlanStep {
            id: Uuid::new_v4().to_string(),
            description: "Generate code changes".to_string(),
            step_type: StepType::CodeGeneration,
            dependencies: vec![],
            estimated_duration: Duration::from_secs(120),
        },
        PlanStep {
            id: Uuid::new_v4().to_string(),
            description: "Run tests and validation".to_string(),
            step_type: StepType::Testing,
            dependencies: vec![],
            estimated_duration: Duration::from_secs(60),
        },
    ];

    let affected_files = request.context.files.clone();
    let estimated_duration = steps.iter()
        .map(|s| s.estimated_duration)
        .sum::<Duration>();

    let risk_level = if affected_files.len() > 10 {
        RiskLevel::High
    } else if affected_files.len() > 5 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };
    
    let response = PlanResponse {
        plan_id: plan_id.clone(),
        goal: request.goal.clone(),
        steps,
        estimated_duration,
        affected_files,
        risk_level,
    };
    
    // Record plan attributes in span
    record_plan_attributes(&span, &plan_id, &request.goal, response.steps.len());
    
    // Record latency
    let latency = start_time.elapsed();
    metrics.http_request_duration_ms
        .with_label_values(&["/api/v1/plan", "POST"])
        .observe(latency.as_millis() as f64);
    
    info!(
        request_id = %request_id,
        plan_id = %plan_id,
        latency_ms = latency.as_millis(),
        steps_count = response.steps.len(),
        "Plan created successfully"
    );
    
    Ok(Json(response))
}

async fn preflight_risk_gate_check(plan_id: &str, patch_id: &str) -> Result<(), (StatusCode, ApiError)> {
    // Placeholder: allow preflight for now; full blocking in post_analysis_risk_gate
    Ok(())
}

async fn post_analysis_risk_gate(state: &AppState, patch_id: &str) -> Result<(), (StatusCode, ApiError)> {
    use crate::risk::{RiskGate, RiskGateConfig};
    use crate::database::repositories::{RunsRepository, ArtifactsRepository};

    let gate = RiskGate::new(RiskGateConfig::default());
    let runs_repo = RunsRepository::new(state.database.pool.clone());
    let artifacts_repo = ArtifactsRepository::new(state.database.pool.clone());

    let current_run_id = uuid::Uuid::new_v4(); // TODO: Link to actual run id created earlier

    match gate.evaluate_patch(
        patch_id,
        current_run_id,
        None,
        &runs_repo,
        &artifacts_repo,
    ).await {
        Ok(decision) => {
            if decision.should_block {
                return Err((
                    StatusCode::CONFLICT,
                    ApiError {
                        error: "PATCH_BLOCKED_BY_RISK_GATE".to_string(),
                        code: "RISK_GATE_BLOCK".to_string(),
                        details: Some(format!("risk_score={} threshold_exceeded=true", decision.risk_assessment.risk_score)),
                    }
                ));
            }
            Ok(())
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError { error: "Risk analysis failed".into(), code: "RISK_ANALYSIS_ERROR".into(), details: Some(e.to_string()) }
        )),
    }
}
    use crate::risk::{RiskGate, RiskGateConfig};
    // Very lightweight pre-check (can be expanded): if configuration demands block high-risk patches before apply
    let gate = RiskGate::new(RiskGateConfig::default());
    // Here we would compute a preliminary risk score; for now, allow and leave blocking to post-analysis
    Ok(())
}

// Router setup
pub fn v1_routes() -> Router<AppState> {
    Router::new()
        .route("/plan", post(create_plan))
        .route("/patch", post(apply_patch))
        .route("/run-tests", post(run_tests))
        .route("/artifacts/:run_id", get(get_artifacts))
        .route("/risk-report/:patch_id", get(get_risk_report))
        .route("/rollback", post(rollback_patch))
}