use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::ai_engine::providers::ProviderRouter;
use crate::observability::get_metrics;

use super::{AgentRequest, AgentResponse, AgentArtifact, ArtifactType};

/// PlannerAgent: Analyzes goals and creates execution plans
/// Responsible for breaking down high-level goals into actionable steps
pub struct PlannerAgent {
    provider_router: Arc<ProviderRouter>,
}

impl PlannerAgent {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self { provider_router }
    }

    /// Execute planning for a given goal
    pub async fn execute(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let start_time = Instant::now();
        
        info!("PlannerAgent executing for goal: {}", request.goal);
        
        // Record metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["planner", "execute"])
            .observe(0.0); // Will update at the end
        
        let mut artifacts = Vec::new();
        let mut cost = 0.0;
        
        // Analyze the goal and create a plan
        let plan_result = self.analyze_goal_and_create_plan(&request.goal, &request.context).await;
        
        let (success, result, error) = match plan_result {
            Ok(plan) => {
                cost += 0.1; // Estimated cost for planning
                
                // Create plan artifact
                artifacts.push(AgentArtifact {
                    name: "execution_plan.json".to_string(),
                    artifact_type: ArtifactType::Plan,
                    content: serde_json::to_string_pretty(&plan)?,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("goal".to_string(), request.goal.clone());
                        meta.insert("steps_count".to_string(), plan["steps"].as_array().map(|s| s.len()).unwrap_or(0).to_string());
                        meta
                    },
                });
                
                (true, plan, None)
            }
            Err(e) => {
                warn!("Planning failed: {}", e);
                (false, json!({"error": "Planning failed"}), Some(e.to_string()))
            }
        };
        
        let execution_time = start_time.elapsed();
        
        // Record execution time
        metrics.agent_step_duration_ms
            .with_label_values(&["planner", "execute"])
            .observe(execution_time.as_millis() as f64);
        
        let response = AgentResponse {
            id: request.id,
            agent_name: "planner".to_string(),
            success,
            result,
            artifacts,
            execution_time,
            cost: Some(cost),
            error,
        };
        
        debug!("PlannerAgent completed in {:?} (success: {})", execution_time, success);
        
        Ok(response)
    }

    /// Analyze goal and create detailed execution plan
    async fn analyze_goal_and_create_plan(&self, goal: &str, context: &Option<String>) -> Result<Value> {
        // Use AI provider to analyze the goal
        let analysis_prompt = self.build_analysis_prompt(goal, context);
        
        let analysis_result = self.provider_router
            .complete(&analysis_prompt, context.as_deref())
            .await?;
        
        if analysis_result.is_empty() {
            return Err(anyhow!("No analysis result from provider"));
        }
        
        // Parse the analysis and create structured plan
        let plan = self.create_structured_plan(goal, &analysis_result[0]).await?;
        
        Ok(plan)
    }

    /// Build prompt for goal analysis
    fn build_analysis_prompt(&self, goal: &str, context: &Option<String>) -> String {
        let context_section = if let Some(ctx) = context {
            format!("\n\nContext:\n{}", ctx)
        } else {
            String::new()
        };
        
        format!(
            r#"Analyze the following development goal and create a detailed execution plan.

Goal: {}{}

Please analyze:
1. What needs to be implemented or changed
2. Which files are likely to be affected
3. What are the main steps required
4. What are the potential risks and challenges
5. What tests should be created
6. Estimated complexity and time

Provide a structured analysis that can be used to create an execution plan."#,
            goal, context_section
        )
    }

    /// Create structured execution plan from analysis
    async fn create_structured_plan(&self, goal: &str, analysis: &str) -> Result<Value> {
        // Extract key information from analysis
        let complexity = self.estimate_complexity(goal, analysis);
        let risk_level = self.assess_risk_level(goal, analysis);
        let affected_files = self.identify_affected_files(analysis);
        let steps = self.generate_plan_steps(goal, analysis).await?;
        
        let plan = json!({
            "id": Uuid::new_v4().to_string(),
            "goal": goal,
            "analysis": analysis,
            "complexity": complexity,
            "risk_level": risk_level,
            "affected_files": affected_files,
            "steps": steps,
            "estimated_duration_minutes": self.estimate_duration(&steps),
            "success_criteria": self.define_success_criteria(goal),
            "rollback_strategy": self.create_rollback_strategy(&affected_files),
            "created_at": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(plan)
    }

    /// Estimate complexity of the goal
    fn estimate_complexity(&self, goal: &str, analysis: &str) -> String {
        let goal_lower = goal.to_lowercase();
        let analysis_lower = analysis.to_lowercase();
        
        // Simple heuristic-based complexity estimation
        let complexity_indicators = [
            ("simple", 1, vec!["fix", "update", "change", "modify"]),
            ("medium", 2, vec!["add", "implement", "create", "refactor"]),
            ("complex", 3, vec!["redesign", "architecture", "system", "integration"]),
            ("very_complex", 4, vec!["migrate", "rewrite", "overhaul", "complete"]),
        ];
        
        let mut max_score = 0;
        let mut complexity = "simple";
        
        for (level, score, keywords) in &complexity_indicators {
            for keyword in keywords {
                if goal_lower.contains(keyword) || analysis_lower.contains(keyword) {
                    if *score > max_score {
                        max_score = *score;
                        complexity = level;
                    }
                }
            }
        }
        
        // Additional factors
        if goal.len() > 100 {
            max_score += 1;
        }
        if analysis.contains("multiple") || analysis.contains("several") {
            max_score += 1;
        }
        
        match max_score {
            0..=1 => "simple",
            2..=3 => "medium", 
            4..=5 => "complex",
            _ => "very_complex",
        }.to_string()
    }

    /// Assess risk level
    fn assess_risk_level(&self, goal: &str, analysis: &str) -> String {
        let risk_keywords = [
            ("critical", vec!["security", "authentication", "database", "migration"]),
            ("high", vec!["breaking", "api", "interface", "public"]),
            ("medium", vec!["refactor", "change", "modify", "update"]),
            ("low", vec!["add", "fix", "improve", "optimize"]),
        ];
        
        let goal_lower = goal.to_lowercase();
        let analysis_lower = analysis.to_lowercase();
        
        for (level, keywords) in &risk_keywords {
            for keyword in keywords {
                if goal_lower.contains(keyword) || analysis_lower.contains(keyword) {
                    return level.to_string();
                }
            }
        }
        
        "low".to_string()
    }

    /// Identify potentially affected files
    fn identify_affected_files(&self, _analysis: &str) -> Vec<String> {
        // Simplified implementation for Sprint 2
        vec![
            "src/main.py".to_string(),
            "src/utils.py".to_string(),
            "tests/test_main.py".to_string(),
        ]
    }

    /// Generate detailed plan steps
    async fn generate_plan_steps(&self, goal: &str, _analysis: &str) -> Result<Vec<Value>> {
        let mut steps = Vec::new();
        
        // Standard development workflow steps
        steps.push(json!({
            "id": Uuid::new_v4().to_string(),
            "name": "Context Analysis",
            "description": "Analyze existing codebase and understand current implementation",
            "agent": "retriever",
            "estimated_minutes": 5,
            "inputs": {
                "goal": goal
            },
            "outputs": ["context_package"],
            "success_criteria": ["Relevant files identified", "Dependencies mapped"]
        }));
        
        steps.push(json!({
            "id": Uuid::new_v4().to_string(),
            "name": "Code Generation",
            "description": "Generate or modify code to achieve the goal",
            "agent": "codegen",
            "estimated_minutes": 15,
            "inputs": {
                "goal": goal,
                "context": "from_previous_step"
            },
            "outputs": ["generated_files"],
            "success_criteria": ["Code compiles", "Follows best practices", "Implements requirements"]
        }));
        
        steps.push(json!({
            "id": Uuid::new_v4().to_string(),
            "name": "Test Generation",
            "description": "Create comprehensive tests for the changes",
            "agent": "testgen",
            "estimated_minutes": 10,
            "inputs": {
                "generated_code": "from_previous_step"
            },
            "outputs": ["test_files"],
            "success_criteria": ["Tests cover main functionality", "Edge cases included"]
        }));
        
        Ok(steps)
    }

    /// Estimate total duration from steps
    fn estimate_duration(&self, steps: &[Value]) -> u64 {
        steps.iter()
            .filter_map(|step| step["estimated_minutes"].as_u64())
            .sum()
    }

    /// Define success criteria for the goal
    fn define_success_criteria(&self, goal: &str) -> Vec<String> {
        let mut criteria = vec![
            "Implementation matches the goal requirements".to_string(),
            "All tests pass successfully".to_string(),
            "Code quality meets standards".to_string(),
        ];
        
        // Add goal-specific criteria
        let goal_lower = goal.to_lowercase();
        
        if goal_lower.contains("performance") || goal_lower.contains("optimize") {
            criteria.push("Performance improvement measurable".to_string());
        }
        
        if goal_lower.contains("security") || goal_lower.contains("auth") {
            criteria.push("Security audit passed".to_string());
        }
        
        criteria
    }

    /// Create rollback strategy
    fn create_rollback_strategy(&self, affected_files: &[String]) -> String {
        if affected_files.is_empty() {
            return "No rollback needed - no files modified".to_string();
        }
        
        let mut strategy = String::from("Rollback strategy:\n");
        strategy.push_str("1. Create backup of all modified files before changes\n");
        strategy.push_str("2. If issues detected, restore files from backup\n");
        strategy.push_str("3. Run regression tests\n");
        strategy.push_str("4. Verify system stability");
        
        strategy
    }
}