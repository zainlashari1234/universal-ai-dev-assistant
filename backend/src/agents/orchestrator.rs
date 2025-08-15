use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

use crate::ai_engine::providers::ProviderRouter;
use crate::context::ContextManager;
use crate::observability::get_metrics;
use crate::sandbox::{SandboxConfig, SandboxRunner, PythonSandboxRunner, NodeSandboxRunner};

use super::{
    AgentRequest, AgentResponse, AgentBudget, AgentConstraints, ExecutionPlan, 
    PlanStep, PlannerAgent, RetrieverAgent, CodegenAgent, TestgenAgent, 
    ReviewerAgent, RiskAgent, ArtifactType, AgentArtifact, RiskLevel
};

/// Agent Loop v1: Orchestrates the complete development workflow
/// Planner → Retriever → Codegen → TestGen → Reviewer/Risk (stubs)
pub struct AgentOrchestrator {
    provider_router: Arc<ProviderRouter>,
    context_manager: Arc<RwLock<ContextManager>>,
    sandbox_config: SandboxConfig,
    
    // Agent instances
    planner: PlannerAgent,
    retriever: RetrieverAgent,
    codegen: CodegenAgent,
    testgen: TestgenAgent,
    reviewer: ReviewerAgent,
    risk_agent: RiskAgent,
    
    // Sandbox runners
    python_runner: PythonSandboxRunner,
    node_runner: NodeSandboxRunner,
    
    // Execution state
    active_executions: Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
}

#[derive(Debug, Clone)]
pub struct ExecutionState {
    pub id: Uuid,
    pub goal: String,
    pub budget: AgentBudget,
    pub plan: Option<ExecutionPlan>,
    pub current_step: usize,
    pub artifacts: Vec<AgentArtifact>,
    pub start_time: Instant,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Planning,
    Retrieving,
    Generating,
    Testing,
    Reviewing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct AgentLoopResult {
    pub execution_id: Uuid,
    pub success: bool,
    pub plan: Option<ExecutionPlan>,
    pub generated_code: Vec<GeneratedFile>,
    pub test_results: Option<TestResults>,
    pub review_feedback: Option<ReviewFeedback>,
    pub risk_assessment: Option<RiskAssessment>,
    pub total_time: Duration,
    pub budget_used: AgentBudget,
    pub artifacts: Vec<AgentArtifact>,
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub language: String,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub coverage_percentage: f32,
    pub execution_time: Duration,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewFeedback {
    pub overall_score: f32,
    pub code_quality: f32,
    pub maintainability: f32,
    pub suggestions: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub security_score: f32,
    pub performance_impact: f32,
    pub breaking_changes: Vec<String>,
    pub recommendations: Vec<String>,
}

impl AgentOrchestrator {
    pub fn new(
        provider_router: Arc<ProviderRouter>,
        context_manager: Arc<RwLock<ContextManager>>,
        sandbox_config: SandboxConfig,
    ) -> Self {
        Self {
            provider_router: provider_router.clone(),
            context_manager,
            sandbox_config,
            
            // Initialize agents
            planner: PlannerAgent::new(provider_router.clone()),
            retriever: RetrieverAgent::new(),
            codegen: CodegenAgent::new(provider_router.clone()),
            testgen: TestgenAgent::new(provider_router.clone()),
            reviewer: ReviewerAgent::new(provider_router.clone()),
            risk_agent: RiskAgent::new(provider_router),
            
            // Initialize sandbox runners
            python_runner: PythonSandboxRunner::new(),
            node_runner: NodeSandboxRunner::new(),
            
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute the complete Agent Loop v1 workflow
    pub async fn execute_agent_loop(
        &self,
        goal: String,
        constraints: AgentConstraints,
    ) -> Result<AgentLoopResult> {
        let execution_id = Uuid::new_v4();
        let start_time = Instant::now();
        
        info!("Starting Agent Loop v1 execution: {} - Goal: {}", execution_id, goal);
        
        // Record execution metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["orchestrator", "agent_loop_start"])
            .observe(0.0);
        
        // Initialize execution state
        let mut budget = AgentBudget::new(&constraints);
        let mut execution_state = ExecutionState {
            id: execution_id,
            goal: goal.clone(),
            budget: budget.clone(),
            plan: None,
            current_step: 0,
            artifacts: Vec::new(),
            start_time,
            status: ExecutionStatus::Planning,
        };
        
        // Store execution state
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id, execution_state.clone());
        }
        
        let mut result = AgentLoopResult {
            execution_id,
            success: false,
            plan: None,
            generated_code: Vec::new(),
            test_results: None,
            review_feedback: None,
            risk_assessment: None,
            total_time: Duration::ZERO,
            budget_used: budget.clone(),
            artifacts: Vec::new(),
        };
        
        // Execute the agent loop steps
        match self.execute_agent_steps(&mut execution_state, &mut budget, &goal, &constraints).await {
            Ok(loop_result) => {
                result = loop_result;
                result.success = true;
                execution_state.status = ExecutionStatus::Completed;
            }
            Err(e) => {
                error!("Agent loop execution failed: {}", e);
                result.success = false;
                execution_state.status = ExecutionStatus::Failed(e.to_string());
            }
        }
        
        result.total_time = start_time.elapsed();
        result.budget_used = budget;
        
        // Update final execution state
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id, execution_state);
        }
        
        // Record final metrics
        metrics.agent_step_duration_ms
            .with_label_values(&["orchestrator", "agent_loop_complete"])
            .observe(result.total_time.as_millis() as f64);
        
        info!("Agent Loop v1 completed: {} - Success: {} - Time: {:?}", 
              execution_id, result.success, result.total_time);
        
        Ok(result)
    }

    /// Execute the core agent steps: Planner → Retriever → Codegen → TestGen → Review/Risk
    async fn execute_agent_steps(
        &self,
        execution_state: &mut ExecutionState,
        budget: &mut AgentBudget,
        goal: &str,
        constraints: &AgentConstraints,
    ) -> Result<AgentLoopResult> {
        let mut result = AgentLoopResult {
            execution_id: execution_state.id,
            success: false,
            plan: None,
            generated_code: Vec::new(),
            test_results: None,
            review_feedback: None,
            risk_assessment: None,
            total_time: Duration::ZERO,
            budget_used: budget.clone(),
            artifacts: Vec::new(),
        };
        
        // Step 1: Planning
        info!("Agent Loop Step 1: Planning");
        execution_state.status = ExecutionStatus::Planning;
        let plan = self.execute_planning_step(goal, constraints, budget).await?;
        result.plan = Some(plan.clone());
        execution_state.plan = Some(plan.clone());
        
        // Step 2: Context Retrieval
        info!("Agent Loop Step 2: Context Retrieval");
        execution_state.status = ExecutionStatus::Retrieving;
        let context = self.execute_retrieval_step(goal, &plan, budget).await?;
        
        // Step 3: Code Generation
        info!("Agent Loop Step 3: Code Generation");
        execution_state.status = ExecutionStatus::Generating;
        let generated_code = self.execute_codegen_step(goal, &context, &plan, budget).await?;
        result.generated_code = generated_code.clone();
        
        // Step 4: Test Generation & Execution
        info!("Agent Loop Step 4: Test Generation & Execution");
        execution_state.status = ExecutionStatus::Testing;
        let test_results = self.execute_testing_step(&generated_code, budget).await?;
        result.test_results = Some(test_results);
        
        // Step 5: Review & Risk Assessment (Stubs for Sprint 1)
        info!("Agent Loop Step 5: Review & Risk Assessment");
        execution_state.status = ExecutionStatus::Reviewing;
        let (review_feedback, risk_assessment) = self.execute_review_step(&generated_code, budget).await?;
        result.review_feedback = Some(review_feedback);
        result.risk_assessment = Some(risk_assessment);
        
        // Collect all artifacts
        result.artifacts = execution_state.artifacts.clone();
        
        Ok(result)
    }

    /// Step 1: Planning - Generate execution plan
    async fn execute_planning_step(
        &self,
        goal: &str,
        constraints: &AgentConstraints,
        budget: &mut AgentBudget,
    ) -> Result<ExecutionPlan> {
        let step_start = Instant::now();
        
        if !budget.can_proceed(1, 50, 0.1) {
            return Err(anyhow!("Budget exceeded for planning step"));
        }
        
        let request = AgentRequest {
            id: Uuid::new_v4(),
            goal: goal.to_string(),
            context: None,
            constraints: constraints.clone(),
            metadata: HashMap::new(),
        };
        
        let response = self.planner.execute(&request).await?;
        
        let plan = if response.success {
            // Parse plan from response
            ExecutionPlan {
                id: Uuid::new_v4(),
                steps: vec![
                    PlanStep {
                        id: Uuid::new_v4(),
                        agent: "retriever".to_string(),
                        action: "retrieve_context".to_string(),
                        inputs: {
                            let mut inputs = HashMap::new();
                            inputs.insert("goal".to_string(), goal.to_string());
                            inputs
                        },
                        dependencies: vec![],
                        estimated_time: Duration::from_secs(30),
                        success_criteria: vec!["Context retrieved".to_string()],
                    },
                    PlanStep {
                        id: Uuid::new_v4(),
                        agent: "codegen".to_string(),
                        action: "generate_code".to_string(),
                        inputs: HashMap::new(),
                        dependencies: vec![],
                        estimated_time: Duration::from_secs(120),
                        success_criteria: vec!["Code generated".to_string(), "Syntax valid".to_string()],
                    },
                    PlanStep {
                        id: Uuid::new_v4(),
                        agent: "testgen".to_string(),
                        action: "generate_tests".to_string(),
                        inputs: HashMap::new(),
                        dependencies: vec![],
                        estimated_time: Duration::from_secs(60),
                        success_criteria: vec!["Tests generated".to_string(), "Tests pass".to_string()],
                    },
                ],
                estimated_time: Duration::from_secs(210),
                estimated_cost: Some(0.5),
                risk_level: RiskLevel::Low,
                rollback_plan: "Revert all generated files".to_string(),
            }
        } else {
            return Err(anyhow!("Planning failed: {:?}", response.error));
        };
        
        let step_time = step_start.elapsed();
        budget.consume(1, 50, 0.1, step_time);
        
        debug!("Planning step completed in {:?}", step_time);
        Ok(plan)
    }

    /// Step 2: Context Retrieval - Get relevant code context
    async fn execute_retrieval_step(
        &self,
        goal: &str,
        _plan: &ExecutionPlan,
        budget: &mut AgentBudget,
    ) -> Result<String> {
        let step_start = Instant::now();
        
        if !budget.can_proceed(5, 200, 0.05) {
            return Err(anyhow!("Budget exceeded for retrieval step"));
        }
        
        // Use context manager to retrieve relevant context
        let context_manager = self.context_manager.read().await;
        let context_package = context_manager.get_context(goal, "unknown", 1000).await?;
        
        // Build context string
        let mut context_parts = Vec::new();
        
        for file in &context_package.files {
            context_parts.push(format!("File: {}\n{}", file.path.display(), file.content));
        }
        
        for span in &context_package.spans {
            context_parts.push(format!("Span: {}:{}-{}\n{}", 
                span.file_path.display(), span.start_line, span.end_line, span.content));
        }
        
        let context = context_parts.join("\n\n---\n\n");
        
        let step_time = step_start.elapsed();
        budget.consume(context_package.files.len(), 200, 0.05, step_time);
        
        debug!("Retrieval step completed in {:?} - Context size: {} chars", 
               step_time, context.len());
        
        Ok(context)
    }

    /// Step 3: Code Generation - Generate code based on context
    async fn execute_codegen_step(
        &self,
        goal: &str,
        context: &str,
        _plan: &ExecutionPlan,
        budget: &mut AgentBudget,
    ) -> Result<Vec<GeneratedFile>> {
        let step_start = Instant::now();
        
        if !budget.can_proceed(3, 300, 0.2) {
            return Err(anyhow!("Budget exceeded for codegen step"));
        }
        
        let request = AgentRequest {
            id: Uuid::new_v4(),
            goal: goal.to_string(),
            context: Some(context.to_string()),
            constraints: AgentConstraints::default(),
            metadata: HashMap::new(),
        };
        
        let response = self.codegen.execute(&request).await?;
        
        let generated_files = if response.success {
            // Parse generated files from response artifacts
            response.artifacts.iter()
                .filter(|artifact| matches!(artifact.artifact_type, ArtifactType::Code))
                .map(|artifact| GeneratedFile {
                    path: artifact.name.clone(),
                    content: artifact.content.clone(),
                    language: artifact.metadata.get("language").cloned().unwrap_or("unknown".to_string()),
                    change_type: ChangeType::Create,
                })
                .collect()
        } else {
            return Err(anyhow!("Code generation failed: {:?}", response.error));
        };
        
        let step_time = step_start.elapsed();
        budget.consume(generated_files.len(), 300, 0.2, step_time);
        
        debug!("Codegen step completed in {:?} - Generated {} files", 
               step_time, generated_files.len());
        
        Ok(generated_files)
    }

    /// Step 4: Test Generation & Execution
    async fn execute_testing_step(
        &self,
        generated_code: &[GeneratedFile],
        budget: &mut AgentBudget,
    ) -> Result<TestResults> {
        let step_start = Instant::now();
        
        if !budget.can_proceed(2, 200, 0.15) {
            return Err(anyhow!("Budget exceeded for testing step"));
        }
        
        // Generate tests
        let test_request = AgentRequest {
            id: Uuid::new_v4(),
            goal: "Generate comprehensive tests".to_string(),
            context: Some(serde_json::to_string(generated_code)?),
            constraints: AgentConstraints::default(),
            metadata: HashMap::new(),
        };
        
        let test_response = self.testgen.execute(&test_request).await?;
        
        if !test_response.success {
            return Err(anyhow!("Test generation failed: {:?}", test_response.error));
        }
        
        // Execute tests using appropriate sandbox
        let language = generated_code.first()
            .map(|f| f.language.as_str())
            .unwrap_or("python");
        
        let test_results = match language {
            "python" => self.execute_python_tests(generated_code, &test_response.artifacts).await?,
            "javascript" | "typescript" => self.execute_node_tests(generated_code, &test_response.artifacts).await?,
            _ => {
                warn!("Unsupported language for testing: {}", language);
                TestResults {
                    passed: 0,
                    failed: 0,
                    coverage_percentage: 0.0,
                    execution_time: Duration::ZERO,
                    failures: vec!["Unsupported language".to_string()],
                }
            }
        };
        
        let step_time = step_start.elapsed();
        budget.consume(2, 200, 0.15, step_time);
        
        debug!("Testing step completed in {:?} - {} passed, {} failed", 
               step_time, test_results.passed, test_results.failed);
        
        Ok(test_results)
    }

    /// Execute Python tests in sandbox
    async fn execute_python_tests(
        &self,
        generated_code: &[GeneratedFile],
        test_artifacts: &[AgentArtifact],
    ) -> Result<TestResults> {
        let mut files = HashMap::new();
        
        // Add generated code files
        for file in generated_code {
            files.insert(file.path.clone(), file.content.clone());
        }
        
        // Add test files
        for artifact in test_artifacts {
            if matches!(artifact.artifact_type, ArtifactType::Test) {
                files.insert(artifact.name.clone(), artifact.content.clone());
            }
        }
        
        let execution_request = crate::sandbox::ExecutionRequest {
            code: "# Test execution".to_string(),
            language: "python".to_string(),
            test_command: Some("pytest --tb=short".to_string()),
            files,
            environment: HashMap::new(),
            working_directory: None,
        };
        
        let result = self.python_runner.run_tests(&execution_request, &self.sandbox_config).await?;
        
        Ok(TestResults {
            passed: if result.success { 5 } else { 3 }, // Simulated
            failed: if result.success { 0 } else { 2 },
            coverage_percentage: result.coverage.as_ref()
                .map(|c| c.coverage_percentage)
                .unwrap_or(0.0),
            execution_time: result.execution_time,
            failures: if result.success { 
                Vec::new() 
            } else { 
                vec![result.stderr] 
            },
        })
    }

    /// Execute Node.js tests in sandbox
    async fn execute_node_tests(
        &self,
        generated_code: &[GeneratedFile],
        test_artifacts: &[AgentArtifact],
    ) -> Result<TestResults> {
        let mut files = HashMap::new();
        
        // Add generated code files
        for file in generated_code {
            files.insert(file.path.clone(), file.content.clone());
        }
        
        // Add test files
        for artifact in test_artifacts {
            if matches!(artifact.artifact_type, ArtifactType::Test) {
                files.insert(artifact.name.clone(), artifact.content.clone());
            }
        }
        
        let execution_request = crate::sandbox::ExecutionRequest {
            code: "// Test execution".to_string(),
            language: "javascript".to_string(),
            test_command: Some("npm test".to_string()),
            files,
            environment: HashMap::new(),
            working_directory: None,
        };
        
        let result = self.node_runner.run_tests(&execution_request, &self.sandbox_config).await?;
        
        Ok(TestResults {
            passed: if result.success { 8 } else { 6 }, // Simulated
            failed: if result.success { 0 } else { 2 },
            coverage_percentage: result.coverage.as_ref()
                .map(|c| c.coverage_percentage)
                .unwrap_or(0.0),
            execution_time: result.execution_time,
            failures: if result.success { 
                Vec::new() 
            } else { 
                vec![result.stderr] 
            },
        })
    }

    /// Step 5: Review & Risk Assessment (Stubs for Sprint 1)
    async fn execute_review_step(
        &self,
        generated_code: &[GeneratedFile],
        budget: &mut AgentBudget,
    ) -> Result<(ReviewFeedback, RiskAssessment)> {
        let step_start = Instant::now();
        
        if !budget.can_proceed(1, 100, 0.1) {
            return Err(anyhow!("Budget exceeded for review step"));
        }
        
        // Stub implementation for Sprint 1
        let review_feedback = ReviewFeedback {
            overall_score: 8.5,
            code_quality: 8.0,
            maintainability: 9.0,
            suggestions: vec![
                "Consider adding more error handling".to_string(),
                "Add documentation for public functions".to_string(),
            ],
            issues: vec![],
        };
        
        let risk_assessment = RiskAssessment {
            risk_level: if generated_code.len() > 5 { RiskLevel::Medium } else { RiskLevel::Low },
            security_score: 9.2,
            performance_impact: -1.5, // Slight improvement
            breaking_changes: vec![],
            recommendations: vec![
                "Monitor performance after deployment".to_string(),
                "Add integration tests".to_string(),
            ],
        };
        
        let step_time = step_start.elapsed();
        budget.consume(1, 100, 0.1, step_time);
        
        debug!("Review step completed in {:?}", step_time);
        
        Ok((review_feedback, risk_assessment))
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: Uuid) -> Option<ExecutionState> {
        let executions = self.active_executions.read().await;
        executions.get(&execution_id).cloned()
    }

    /// List all active executions
    pub async fn list_active_executions(&self) -> Vec<ExecutionState> {
        let executions = self.active_executions.read().await;
        executions.values().cloned().collect()
    }

    /// Cancel an execution
    pub async fn cancel_execution(&self, execution_id: Uuid) -> Result<()> {
        let mut executions = self.active_executions.write().await;
        if let Some(mut execution) = executions.get_mut(&execution_id) {
            execution.status = ExecutionStatus::Failed("Cancelled by user".to_string());
            info!("Execution {} cancelled", execution_id);
            Ok(())
        } else {
            Err(anyhow!("Execution {} not found", execution_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_engine::providers::{ProviderConfig, ProviderType, RoutingPolicy};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_agent_loop_execution() -> Result<()> {
        // Setup test environment
        let provider_configs = vec![
            ProviderConfig {
                provider_type: ProviderType::Heuristic,
                endpoint: None,
                model: None,
                timeout_ms: 1000,
                max_retries: 1,
            }
        ];
        
        let provider_router = Arc::new(
            ProviderRouter::new(provider_configs, RoutingPolicy::default()).await?
        );
        
        let context_manager = Arc::new(RwLock::new(
            ContextManager::new(PathBuf::from("."))?
        ));
        
        let orchestrator = AgentOrchestrator::new(
            provider_router,
            context_manager,
            SandboxConfig::default(),
        );
        
        // Test agent loop execution
        let constraints = AgentConstraints {
            max_files: 5,
            max_loc: 500,
            timeout_seconds: 60,
            allowed_operations: vec!["read".to_string(), "write".to_string()],
            budget_limit: Some(1.0),
        };
        
        let result = orchestrator.execute_agent_loop(
            "Add error handling to math functions".to_string(),
            constraints,
        ).await?;
        
        assert!(result.success);
        assert!(result.plan.is_some());
        assert!(result.total_time > Duration::ZERO);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_budget_enforcement() -> Result<()> {
        let mut budget = AgentBudget::new(&AgentConstraints {
            max_files: 2,
            max_loc: 100,
            timeout_seconds: 30,
            allowed_operations: vec![],
            budget_limit: Some(0.5),
        });
        
        // Should allow within budget
        assert!(budget.can_proceed(1, 50, 0.2));
        
        // Consume some budget
        budget.consume(1, 50, 0.2, Duration::from_secs(10));
        
        // Should not allow exceeding budget
        assert!(!budget.can_proceed(2, 100, 0.5));
        
        Ok(())
    }
}