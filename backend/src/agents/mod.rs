pub mod planner;
pub mod retriever;
pub mod codegen;
pub mod testgen;
pub mod reviewer;
pub mod risk;
pub mod risk_gate;
pub mod build_doctor;
pub mod orchestrator;
pub mod test_first;
pub mod security_analyzer;
pub mod advanced_risk;
pub mod agent_communication;
pub mod agent_personality;
pub mod dynamic_agent_factory;
pub mod knowledge_mesh;

pub use planner::*;
pub use retriever::*;
pub use codegen::*;
pub use testgen::*;
pub use reviewer::*;
pub use risk::*;
pub use risk_gate::*;
pub use build_doctor::*;
pub use orchestrator::*;
pub use test_first::*;
pub use security_analyzer::*;
pub use advanced_risk::*;
pub use agent_communication::*;
pub use agent_personality::*;
pub use dynamic_agent_factory::*;
pub use knowledge_mesh::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub id: Uuid,
    pub goal: String,
    pub context: Option<String>,
    pub constraints: AgentConstraints,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConstraints {
    pub max_files: usize,
    pub max_loc: usize,
    pub timeout_seconds: u64,
    pub allowed_operations: Vec<String>,
    pub budget_limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: Uuid,
    pub agent_name: String,
    pub success: bool,
    pub result: serde_json::Value,
    pub artifacts: Vec<AgentArtifact>,
    pub execution_time: Duration,
    pub cost: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArtifact {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    Plan,
    Code,
    Test,
    Documentation,
    Report,
    Patch,
    Analysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: Uuid,
    pub steps: Vec<PlanStep>,
    pub estimated_time: Duration,
    pub estimated_cost: Option<f64>,
    pub risk_level: RiskLevel,
    pub rollback_plan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: Uuid,
    pub agent: String,
    pub action: String,
    pub inputs: HashMap<String, String>,
    pub dependencies: Vec<Uuid>,
    pub estimated_time: Duration,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBudget {
    pub time_limit: Duration,
    pub file_limit: usize,
    pub loc_limit: usize,
    pub cost_limit: Option<f64>,
    pub used_time: Duration,
    pub used_files: usize,
    pub used_loc: usize,
    pub used_cost: f64,
}

impl AgentBudget {
    pub fn new(constraints: &AgentConstraints) -> Self {
        Self {
            time_limit: Duration::from_secs(constraints.timeout_seconds),
            file_limit: constraints.max_files,
            loc_limit: constraints.max_loc,
            cost_limit: constraints.budget_limit,
            used_time: Duration::ZERO,
            used_files: 0,
            used_loc: 0,
            used_cost: 0.0,
        }
    }

    pub fn can_proceed(&self, additional_files: usize, additional_loc: usize, additional_cost: f64) -> bool {
        if self.used_files + additional_files > self.file_limit {
            return false;
        }
        if self.used_loc + additional_loc > self.loc_limit {
            return false;
        }
        if let Some(limit) = self.cost_limit {
            if self.used_cost + additional_cost > limit {
                return false;
            }
        }
        true
    }

    pub fn consume(&mut self, files: usize, loc: usize, cost: f64, time: Duration) {
        self.used_files += files;
        self.used_loc += loc;
        self.used_cost += cost;
        self.used_time += time;
    }

    pub fn remaining_budget(&self) -> String {
        format!(
            "Files: {}/{}, LOC: {}/{}, Cost: ${:.2}/{}, Time: {:?}/{}",
            self.used_files,
            self.file_limit,
            self.used_loc,
            self.loc_limit,
            self.used_cost,
            self.cost_limit.map(|c| format!("{:.2}", c)).unwrap_or_else(|| "∞".to_string()),
            self.used_time,
            self.time_limit.as_secs()
        )
    }
}

impl Default for AgentConstraints {
    fn default() -> Self {
        Self {
            max_files: 10,
            max_loc: 1000,
            timeout_seconds: 300,
            allowed_operations: vec![
                "read".to_string(),
                "write".to_string(),
                "analyze".to_string(),
                "test".to_string(),
            ],
            budget_limit: Some(1.0),
        }
    }
}