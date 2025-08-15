// Sprint 2: Agent Loop v1 - 6-agent workflow optimization
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::{info, warn, error, instrument};
use uuid::Uuid;

use super::{
    PlannerAgent, RetrieverAgent, CodegenAgent, TestgenAgent, 
    ReviewerAgent, RiskAgent, RiskGate
};
use crate::observability::tracing::{create_agent_span, record_plan_attributes};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopRequest {
    pub goal: String,
    pub context: ExecutionContext,
    pub config: AgentLoopConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub workspace_root: String,
    pub files: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub preferences: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopConfig {
    pub max_iterations: usize,
    pub timeout_seconds: u64,
    pub parallel_agents: usize,
    pub quality_threshold: f64,
    pub risk_threshold: f64,
    pub enable_auto_approval: bool,
    pub enable_rollback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopResult {
    pub execution_id: String,
    pub success: bool,
    pub plan: Option<ExecutionPlan>,
    pub patch: Option<PatchResult>,
    pub test_results: Option<TestResults>,
    pub review_results: Option<ReviewResults>,
    pub risk_assessment: Option<RiskAssessment>,
    pub artifacts: Vec<Artifact>,
    pub execution_time: Duration,
    pub iterations: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: String,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub affected_files: Vec<String>,
    pub estimated_duration: Duration,
    pub complexity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub action: String,
    pub dependencies: Vec<String>,
    pub estimated_duration: Duration,
    pub status: String,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    pub id: String,
    pub files: Vec<FileChange>,
    pub summary: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub content: String,
    pub change_type: String, // "create", "modify", "delete"
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub passed: bool,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub coverage: f64,
    pub execution_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResults {
    pub approved: bool,
    pub quality_score: f64,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: String,
    pub category: String,
    pub file: String,
    pub line: Option<usize>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_level: String,
    pub risk_score: f64,
    pub blocked: bool,
    pub factors: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub artifact_type: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

/// Optimized Agent Loop v1 - Orchestrates 6-agent workflow
pub struct AgentLoop {
    planner: Arc<PlannerAgent>,
    retriever: Arc<RetrieverAgent>,
    codegen: Arc<CodegenAgent>,
    testgen: Arc<TestgenAgent>,
    reviewer: Arc<ReviewerAgent>,
    risk_gate: Arc<RiskGate>,
    
    // Concurrency control
    semaphore: Arc<Semaphore>,
    
    // State management
    executions: Arc<RwLock<HashMap<String, AgentLoopResult>>>,
    
    // Metrics
    metrics: Arc<RwLock<AgentLoopMetrics>>,
}

#[derive(Debug, Default)]
struct AgentLoopMetrics {
    total_executions: usize,
    successful_executions: usize,
    failed_executions: usize,
    average_execution_time: Duration,
    agent_performance: HashMap<String, AgentPerformance>,
}

#[derive(Debug, Default)]
struct AgentPerformance {
    total_calls: usize,
    successful_calls: usize,
    average_duration: Duration,
    error_rate: f64,
}

impl AgentLoop {
    pub fn new(
        planner: PlannerAgent,
        retriever: RetrieverAgent,
        codegen: CodegenAgent,
        testgen: TestgenAgent,
        reviewer: ReviewerAgent,
        risk_gate: RiskGate,
        max_concurrent: usize,
    ) -> Self {
        Self {
            planner: Arc::new(planner),
            retriever: Arc::new(retriever),
            codegen: Arc::new(codegen),
            testgen: Arc::new(testgen),
            reviewer: Arc::new(reviewer),
            risk_gate: Arc::new(risk_gate),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            executions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(AgentLoopMetrics::default())),
        }
    }
    
    #[instrument(skip(self, request), fields(goal = %request.goal))]
    pub async fn execute(&self, request: AgentLoopRequest) -> Result<AgentLoopResult> {
        let execution_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();
        
        info!("Starting agent loop execution: {}", execution_id);
        
        // Acquire semaphore for concurrency control
        let _permit = self.semaphore.acquire().await?;
        
        let span = create_agent_span("agent_loop", "execute", Some(&execution_id));
        let _enter = span.enter();
        
        let mut result = AgentLoopResult {
            execution_id: execution_id.clone(),
            success: false,
            plan: None,
            patch: None,
            test_results: None,
            review_results: None,
            risk_assessment: None,
            artifacts: Vec::new(),
            execution_time: Duration::default(),
            iterations: 0,
            error: None,
        };
        
        // Execute the 6-agent workflow
        match self.execute_workflow(&request, &execution_id).await {
            Ok(workflow_result) => {
                result = workflow_result;
                result.success = true;
            }
            Err(e) => {
                error!("Agent loop execution failed: {}", e);
                result.error = Some(e.to_string());
            }
        }
        
        result.execution_time = start_time.elapsed();
        
        // Store result
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.clone(), result.clone());
        }
        
        // Update metrics
        self.update_metrics(&result).await;
        
        info!(
            "Agent loop execution completed: {} (success: {}, time: {:?})",
            execution_id, result.success, result.execution_time
        );
        
        Ok(result)
    }
    
    async fn execute_workflow(
        &self,
        request: &AgentLoopRequest,
        execution_id: &str,
    ) -> Result<AgentLoopResult> {
        let mut result = AgentLoopResult {
            execution_id: execution_id.to_string(),
            success: false,
            plan: None,
            patch: None,
            test_results: None,
            review_results: None,
            risk_assessment: None,
            artifacts: Vec::new(),
            execution_time: Duration::default(),
            iterations: 0,
            error: None,
        };
        
        // Phase 1: Planning & Context Retrieval (Parallel)
        let (plan, context) = self.phase1_plan_and_context(request).await?;
        result.plan = Some(plan.clone());
        
        // Phase 2: Code Generation
        let patch = self.phase2_code_generation(&plan, &context, request).await?;
        result.patch = Some(patch.clone());
        
        // Phase 3: Test Generation & Execution (Parallel)
        let test_results = self.phase3_test_generation(&patch, &context, request).await?;
        result.test_results = Some(test_results);
        
        // Phase 4: Review & Risk Assessment (Parallel)
        let (review_results, risk_assessment) = self.phase4_review_and_risk(&patch, &plan).await?;
        result.review_results = Some(review_results);
        result.risk_assessment = Some(risk_assessment.clone());
        
        // Phase 5: Decision & Application
        if self.should_apply_patch(&risk_assessment, &result.review_results.as_ref().unwrap(), request) {
            self.phase5_apply_patch(&patch, request).await?;
            result.success = true;
        } else {
            result.error = Some("Patch rejected by risk gate or review".to_string());
        }
        
        result.iterations = 1;
        Ok(result)
    }
    
    async fn phase1_plan_and_context(
        &self,
        request: &AgentLoopRequest,
    ) -> Result<(ExecutionPlan, Vec<String>)> {
        info!("Phase 1: Planning & Context Retrieval");
        
        // Execute planner and retriever in parallel
        let (plan_result, context_result) = tokio::try_join!(
            self.execute_planner(request),
            self.execute_retriever(request)
        )?;
        
        Ok((plan_result, context_result))
    }
    
    async fn execute_planner(&self, request: &AgentLoopRequest) -> Result<ExecutionPlan> {
        let span = create_agent_span("planner", "create_plan", None);
        let _enter = span.enter();
        
        let start_time = Instant::now();
        
        // Simulate planner execution
        let plan = ExecutionPlan {
            id: Uuid::new_v4().to_string(),
            goal: request.goal.clone(),
            steps: vec![
                PlanStep {
                    id: Uuid::new_v4().to_string(),
                    description: "Analyze requirements".to_string(),
                    action: "analyze".to_string(),
                    dependencies: vec![],
                    estimated_duration: Duration::from_secs(30),
                    status: "pending".to_string(),
                    agent: "planner".to_string(),
                },
                PlanStep {
                    id: Uuid::new_v4().to_string(),
                    description: "Generate code changes".to_string(),
                    action: "codegen".to_string(),
                    dependencies: vec![],
                    estimated_duration: Duration::from_secs(120),
                    status: "pending".to_string(),
                    agent: "codegen".to_string(),
                },
                PlanStep {
                    id: Uuid::new_v4().to_string(),
                    description: "Generate tests".to_string(),
                    action: "testgen".to_string(),
                    dependencies: vec![],
                    estimated_duration: Duration::from_secs(60),
                    status: "pending".to_string(),
                    agent: "testgen".to_string(),
                },
            ],
            affected_files: request.context.files.clone(),
            estimated_duration: Duration::from_secs(210),
            complexity_score: 0.6,
        };
        
        record_plan_attributes(&span, &plan.id, &plan.goal, plan.steps.len());
        
        self.record_agent_performance("planner", start_time.elapsed(), true).await;
        
        Ok(plan)
    }
    
    async fn execute_retriever(&self, request: &AgentLoopRequest) -> Result<Vec<String>> {
        let span = create_agent_span("retriever", "get_context", None);
        let _enter = span.enter();
        
        let start_time = Instant::now();
        
        // Simulate context retrieval
        let context = request.context.files.clone();
        
        self.record_agent_performance("retriever", start_time.elapsed(), true).await;
        
        Ok(context)
    }
    
    async fn phase2_code_generation(
        &self,
        plan: &ExecutionPlan,
        context: &[String],
        request: &AgentLoopRequest,
    ) -> Result<PatchResult> {
        info!("Phase 2: Code Generation");
        
        let span = create_agent_span("codegen", "generate_patch", Some(&plan.id));
        let _enter = span.enter();
        
        let start_time = Instant::now();
        
        // Simulate code generation
        let patch = PatchResult {
            id: Uuid::new_v4().to_string(),
            files: vec![
                FileChange {
                    path: "src/main.rs".to_string(),
                    content: "// Generated code\nfn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
                    change_type: "modify".to_string(),
                    diff: Some("+ println!(\"Hello, world!\");".to_string()),
                }
            ],
            summary: format!("Implemented: {}", request.goal),
            confidence: 0.85,
        };
        
        self.record_agent_performance("codegen", start_time.elapsed(), true).await;
        
        Ok(patch)
    }
    
    async fn phase3_test_generation(
        &self,
        patch: &PatchResult,
        context: &[String],
        request: &AgentLoopRequest,
    ) -> Result<TestResults> {
        info!("Phase 3: Test Generation & Execution");
        
        let span = create_agent_span("testgen", "generate_and_run_tests", Some(&patch.id));
        let _enter = span.enter();
        
        let start_time = Instant::now();
        
        // Simulate test generation and execution
        let test_results = TestResults {
            passed: true,
            total_tests: 5,
            passed_tests: 5,
            failed_tests: 0,
            coverage: 0.85,
            execution_time: Duration::from_secs(10),
        };
        
        self.record_agent_performance("testgen", start_time.elapsed(), true).await;
        
        Ok(test_results)
    }
    
    async fn phase4_review_and_risk(
        &self,
        patch: &PatchResult,
        plan: &ExecutionPlan,
    ) -> Result<(ReviewResults, RiskAssessment)> {
        info!("Phase 4: Review & Risk Assessment");
        
        // Execute reviewer and risk assessment in parallel
        let (review_result, risk_result) = tokio::try_join!(
            self.execute_reviewer(patch),
            self.execute_risk_assessment(patch, plan)
        )?;
        
        Ok((review_result, risk_result))
    }
    
    async fn execute_reviewer(&self, patch: &PatchResult) -> Result<ReviewResults> {
        let span = create_agent_span("reviewer", "review_patch", Some(&patch.id));
        let _enter = span.enter();
        
        let start_time = Instant::now();
        
        // Simulate code review
        let review_results = ReviewResults {
            approved: true,
            quality_score: 8.5,
            issues: vec![],
            suggestions: vec!["Consider adding more comments".to_string()],
        };
        
        self.record_agent_performance("reviewer", start_time.elapsed(), true).await;
        
        Ok(review_results)
    }
    
    async fn execute_risk_assessment(&self, patch: &PatchResult, plan: &ExecutionPlan) -> Result<RiskAssessment> {
        let span = create_agent_span("risk", "assess_risk", Some(&patch.id));
        let _enter = span.enter();
        
        let start_time = Instant::now();
        
        // Simulate risk assessment
        let risk_assessment = RiskAssessment {
            risk_level: "low".to_string(),
            risk_score: 0.2,
            blocked: false,
            factors: vec!["Small change".to_string(), "Good test coverage".to_string()],
            recommendations: vec!["Proceed with deployment".to_string()],
        };
        
        self.record_agent_performance("risk", start_time.elapsed(), true).await;
        
        Ok(risk_assessment)
    }
    
    fn should_apply_patch(
        &self,
        risk_assessment: &RiskAssessment,
        review_results: &ReviewResults,
        request: &AgentLoopRequest,
    ) -> bool {
        if risk_assessment.blocked {
            return false;
        }
        
        if risk_assessment.risk_score > request.config.risk_threshold {
            return false;
        }
        
        if !review_results.approved {
            return false;
        }
        
        if review_results.quality_score < request.config.quality_threshold {
            return false;
        }
        
        true
    }
    
    async fn phase5_apply_patch(&self, patch: &PatchResult, request: &AgentLoopRequest) -> Result<()> {
        info!("Phase 5: Applying patch");
        
        // Simulate patch application
        for file_change in &patch.files {
            info!("Applying change to: {}", file_change.path);
            // In real implementation, would write to filesystem
        }
        
        Ok(())
    }
    
    async fn record_agent_performance(&self, agent: &str, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write().await;
        let performance = metrics.agent_performance.entry(agent.to_string()).or_default();
        
        performance.total_calls += 1;
        if success {
            performance.successful_calls += 1;
        }
        
        // Update average duration
        let total_duration = performance.average_duration * (performance.total_calls - 1) as u32 + duration;
        performance.average_duration = total_duration / performance.total_calls as u32;
        
        // Update error rate
        performance.error_rate = 1.0 - (performance.successful_calls as f64 / performance.total_calls as f64);
    }
    
    async fn update_metrics(&self, result: &AgentLoopResult) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_executions += 1;
        if result.success {
            metrics.successful_executions += 1;
        } else {
            metrics.failed_executions += 1;
        }
        
        // Update average execution time
        let total_time = metrics.average_execution_time * (metrics.total_executions - 1) as u32 + result.execution_time;
        metrics.average_execution_time = total_time / metrics.total_executions as u32;
    }
    
    pub async fn get_execution(&self, execution_id: &str) -> Option<AgentLoopResult> {
        let executions = self.executions.read().await;
        executions.get(execution_id).cloned()
    }
    
    pub async fn get_metrics(&self) -> AgentLoopMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        // Implementation for canceling ongoing execution
        warn!("Canceling execution: {}", execution_id);
        Ok(())
    }
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            timeout_seconds: 300,
            parallel_agents: 3,
            quality_threshold: 7.0,
            risk_threshold: 0.7,
            enable_auto_approval: true,
            enable_rollback: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::*;
    
    #[tokio::test]
    async fn test_agent_loop_execution() {
        // Create mock agents
        let planner = PlannerAgent::new().await.unwrap();
        let retriever = RetrieverAgent::new().await.unwrap();
        let codegen = CodegenAgent::new().await.unwrap();
        let testgen = TestgenAgent::new().await.unwrap();
        let reviewer = ReviewerAgent::new().await.unwrap();
        let risk_gate = RiskGate::new(reviewer.clone(), RiskAgent::new().await.unwrap());
        
        let agent_loop = AgentLoop::new(
            planner, retriever, codegen, testgen, reviewer, risk_gate, 3
        );
        
        let request = AgentLoopRequest {
            goal: "Add a hello world function".to_string(),
            context: ExecutionContext {
                workspace_root: "/tmp/test".to_string(),
                files: vec!["src/main.rs".to_string()],
                metadata: HashMap::new(),
                constraints: vec![],
                preferences: HashMap::new(),
            },
            config: AgentLoopConfig::default(),
        };
        
        let result = agent_loop.execute(request).await.unwrap();
        assert!(result.success);
        assert!(result.plan.is_some());
        assert!(result.patch.is_some());
    }
}