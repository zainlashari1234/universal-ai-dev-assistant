// P0 Task #4: Reviewer/Risk gate from stub → real
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use uuid::Uuid;

use super::{RiskLevel, ReviewerAgent, RiskAgent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchData {
    pub patch_id: String,
    pub affected_files: Vec<FileChange>,
    pub description: String,
    pub author: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub risk_score: f64,
    pub risk_factors: Vec<String>,
    pub recommendations: Vec<String>,
    pub rollback_commands: Vec<String>,
    pub assessment_time: Duration,
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub review_id: String,
    pub overall_score: f64,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<String>,
    pub coverage_delta: Option<f64>,
    pub performance_delta: Option<f64>,
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: String,
    pub category: String,
    pub file: String,
    pub line: Option<usize>,
    pub description: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub patch_id: String,
    pub approved: bool,
    pub risk_assessment: RiskAssessment,
    pub review_result: ReviewResult,
    pub gate_decision: String,
    pub required_actions: Vec<String>,
    pub rollback_plan: String,
}

/// Risk Gate: Comprehensive patch review and risk assessment system
pub struct RiskGate {
    reviewer: ReviewerAgent,
    risk_agent: RiskAgent,
    risk_threshold: f64,
    auto_approve_threshold: f64,
}

impl RiskGate {
    pub fn new(reviewer: ReviewerAgent, risk_agent: RiskAgent) -> Self {
        Self {
            reviewer,
            risk_agent,
            risk_threshold: 0.7, // Block patches with risk score >= 0.7
            auto_approve_threshold: 0.2, // Auto-approve patches with risk score <= 0.2
        }
    }

    /// Main gate function: assess patch and make approval decision
    pub async fn evaluate_patch(&self, patch: &PatchData) -> Result<GateResult> {
        info!("Risk gate evaluating patch: {}", patch.patch_id);
        let start_time = Instant::now();

        // Step 1: Risk Assessment
        let risk_assessment = self.assess_patch_risk(patch).await?;
        
        // Step 2: Code Review
        let review_result = self.review_patch_quality(patch).await?;
        
        // Step 3: Gate Decision Logic
        let gate_decision = self.make_gate_decision(&risk_assessment, &review_result);
        let approved = gate_decision.approved;
        
        // Step 4: Generate rollback plan
        let rollback_plan = self.generate_rollback_plan(patch);
        
        let result = GateResult {
            patch_id: patch.patch_id.clone(),
            approved,
            risk_assessment,
            review_result,
            gate_decision: gate_decision.reason,
            required_actions: gate_decision.required_actions,
            rollback_plan,
        };

        let evaluation_time = start_time.elapsed();
        info!(
            "Risk gate evaluation completed for {}: approved={}, time={:?}",
            patch.patch_id, approved, evaluation_time
        );

        Ok(result)
    }

    /// Assess patch risk using RiskAgent
    async fn assess_patch_risk(&self, patch: &PatchData) -> Result<RiskAssessment> {
        let start_time = Instant::now();
        
        // Calculate risk score based on multiple factors
        let mut risk_score = 0.0;
        let mut risk_factors = Vec::new();
        let mut recommendations = Vec::new();
        
        // File count risk
        let file_count = patch.affected_files.len();
        if file_count > 10 {
            risk_score += 0.3;
            risk_factors.push("High number of affected files".to_string());
            recommendations.push("Consider breaking this into smaller patches".to_string());
        } else if file_count > 5 {
            risk_score += 0.1;
        }
        
        // LOC change risk
        let total_loc = patch.affected_files.iter()
            .map(|f| f.additions + f.deletions)
            .sum::<usize>();
        
        if total_loc > 500 {
            risk_score += 0.4;
            risk_factors.push("Large code changes (>500 LOC)".to_string());
            recommendations.push("Split into smaller, reviewable chunks".to_string());
        } else if total_loc > 100 {
            risk_score += 0.2;
            recommendations.push("Ensure adequate testing for medium-sized changes".to_string());
        }
        
        // Critical file risk
        for file in &patch.affected_files {
            if self.is_critical_file(&file.path) {
                risk_score += 0.2;
                risk_factors.push(format!("Critical file modified: {}", file.path));
                recommendations.push(format!("Extra review required for critical file: {}", file.path));
            }
        }
        
        // Test coverage risk
        let has_tests = patch.affected_files.iter()
            .any(|f| f.path.contains("test") || f.path.contains("spec"));
        
        if !has_tests && total_loc > 50 {
            risk_score += 0.3;
            risk_factors.push("No test files included in significant changes".to_string());
            recommendations.push("Add comprehensive tests for the changes".to_string());
        }
        
        // Security risk analysis
        for file in &patch.affected_files {
            if let Some(content) = &file.content {
                if self.contains_security_risks(content) {
                    risk_score += 0.4;
                    risk_factors.push(format!("Potential security issues in {}", file.path));
                    recommendations.push("Security review required - check for vulnerabilities".to_string());
                }
            }
        }
        
        // Performance risk
        for file in &patch.affected_files {
            if let Some(content) = &file.content {
                if self.contains_performance_risks(content) {
                    risk_score += 0.2;
                    risk_factors.push(format!("Potential performance issues in {}", file.path));
                    recommendations.push("Performance testing recommended".to_string());
                }
            }
        }
        
        // Database migration risk
        if patch.affected_files.iter().any(|f| f.path.contains("migration") || f.path.contains("schema")) {
            risk_score += 0.3;
            risk_factors.push("Database schema changes detected".to_string());
            recommendations.push("Ensure backward compatibility and test migrations".to_string());
        }
        
        // Configuration file risk
        if patch.affected_files.iter().any(|f| f.path.contains("config") || f.path.ends_with(".env")) {
            risk_score += 0.2;
            risk_factors.push("Configuration changes detected".to_string());
            recommendations.push("Verify configuration changes in all environments".to_string());
        }
        
        // Determine risk level and blocking threshold
        let risk_level = if risk_score >= 0.8 {
            RiskLevel::Critical
        } else if risk_score >= 0.6 {
            RiskLevel::High
        } else if risk_score >= 0.3 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        
        // Risk gate: Block if risk is too high
        let blocked = risk_score >= self.risk_threshold;
        if blocked {
            risk_factors.push("RISK GATE TRIGGERED: Patch blocked due to high risk".to_string());
            recommendations.push("Manual review and approval required before proceeding".to_string());
        }
        
        // Generate rollback commands
        let rollback_commands = self.generate_rollback_commands(patch);
        
        let assessment = RiskAssessment {
            risk_level: risk_level.clone(),
            risk_score,
            risk_factors,
            recommendations,
            rollback_commands,
            assessment_time: start_time.elapsed(),
            blocked,
        };
        
        // Log assessment with structured data
        info!(
            risk_level = ?risk_level,
            risk_score = risk_score,
            factors_count = risk_factors.len(),
            blocked = blocked,
            "Risk assessment completed"
        );
        
        Ok(assessment)
    }

    /// Review patch quality using ReviewerAgent
    async fn review_patch_quality(&self, patch: &PatchData) -> Result<ReviewResult> {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut total_score = 0.0;
        let mut file_count = 0;

        for file in &patch.affected_files {
            if let Some(content) = &file.content {
                file_count += 1;
                let file_score = self.review_file_quality(&file.path, content, &mut issues, &mut suggestions);
                total_score += file_score;
            }
        }

        let overall_score = if file_count > 0 { total_score / file_count as f64 } else { 0.0 };
        
        // Calculate coverage and performance deltas (simplified)
        let coverage_delta = self.estimate_coverage_delta(patch);
        let performance_delta = self.estimate_performance_delta(patch);
        
        // Approval logic based on score and issues
        let high_severity_issues = issues.iter().any(|i| i.severity == "high" || i.severity == "critical");
        let approved = overall_score >= 7.0 && !high_severity_issues;

        Ok(ReviewResult {
            review_id: Uuid::new_v4().to_string(),
            overall_score,
            issues,
            suggestions,
            coverage_delta,
            performance_delta,
            approved,
        })
    }

    /// Review individual file quality
    fn review_file_quality(&self, filename: &str, content: &str, issues: &mut Vec<ReviewIssue>, suggestions: &mut Vec<String>) -> f64 {
        let mut score = 10.0;
        
        // Check for code quality issues
        
        // Long functions
        if self.has_long_functions(content) {
            score -= 1.0;
            issues.push(ReviewIssue {
                severity: "medium".to_string(),
                category: "maintainability".to_string(),
                file: filename.to_string(),
                line: None,
                description: "Function is too long (>50 lines)".to_string(),
                suggestion: Some("Consider breaking into smaller functions".to_string()),
            });
        }
        
        // Missing documentation
        if !self.has_adequate_documentation(content) {
            score -= 0.5;
            issues.push(ReviewIssue {
                severity: "low".to_string(),
                category: "documentation".to_string(),
                file: filename.to_string(),
                line: None,
                description: "Missing or inadequate documentation".to_string(),
                suggestion: Some("Add docstrings and comments".to_string()),
            });
        }
        
        // TODO/FIXME comments
        if content.contains("TODO") || content.contains("FIXME") {
            score -= 0.5;
            issues.push(ReviewIssue {
                severity: "low".to_string(),
                category: "completeness".to_string(),
                file: filename.to_string(),
                line: None,
                description: "Contains TODO/FIXME comments".to_string(),
                suggestion: Some("Resolve TODO items before merging".to_string()),
            });
        }
        
        // Complex conditions
        if self.has_complex_conditions(content) {
            score -= 1.0;
            issues.push(ReviewIssue {
                severity: "medium".to_string(),
                category: "complexity".to_string(),
                file: filename.to_string(),
                line: None,
                description: "Complex conditional logic detected".to_string(),
                suggestion: Some("Simplify conditions or extract to functions".to_string()),
            });
        }
        
        // Security issues
        if self.contains_security_risks(content) {
            score -= 2.0;
            issues.push(ReviewIssue {
                severity: "high".to_string(),
                category: "security".to_string(),
                file: filename.to_string(),
                line: None,
                description: "Potential security vulnerability".to_string(),
                suggestion: Some("Review and fix security issues".to_string()),
            });
        }
        
        score.max(0.0)
    }

    /// Make final gate decision
    fn make_gate_decision(&self, risk: &RiskAssessment, review: &ReviewResult) -> GateDecision {
        let mut required_actions = Vec::new();
        
        // Check risk blocking
        if risk.blocked {
            return GateDecision {
                approved: false,
                reason: "Blocked by risk gate due to high risk score".to_string(),
                required_actions: vec![
                    "Address high-risk factors".to_string(),
                    "Obtain manual approval from senior reviewer".to_string(),
                ],
            };
        }
        
        // Check review blocking
        if !review.approved {
            required_actions.push("Fix code quality issues".to_string());
            if review.overall_score < 7.0 {
                required_actions.push("Improve code quality score".to_string());
            }
        }
        
        // Check for critical issues
        let has_critical_issues = review.issues.iter().any(|i| i.severity == "critical");
        if has_critical_issues {
            return GateDecision {
                approved: false,
                reason: "Critical issues found in code review".to_string(),
                required_actions: vec!["Fix all critical issues".to_string()],
            };
        }
        
        // Auto-approve low risk patches
        if risk.risk_score <= self.auto_approve_threshold && review.approved {
            return GateDecision {
                approved: true,
                reason: "Auto-approved: low risk and high quality".to_string(),
                required_actions: vec![],
            };
        }
        
        // Manual review required for medium risk
        if risk.risk_score > self.auto_approve_threshold && risk.risk_score < self.risk_threshold {
            required_actions.push("Manual review recommended".to_string());
        }
        
        let approved = review.approved && !risk.blocked;
        let reason = if approved {
            "Approved after review and risk assessment".to_string()
        } else {
            "Requires fixes before approval".to_string()
        };
        
        GateDecision {
            approved,
            reason,
            required_actions,
        }
    }

    /// Generate comprehensive rollback plan
    fn generate_rollback_plan(&self, patch: &PatchData) -> String {
        let mut rollback_commands = Vec::new();
        
        rollback_commands.push("# Rollback Plan".to_string());
        rollback_commands.push(format!("# Generated for patch: {}", patch.patch_id));
        rollback_commands.push("".to_string());
        
        rollback_commands.push("# 1. Stash current changes".to_string());
        rollback_commands.push("git stash push -m \"Pre-rollback stash\"".to_string());
        rollback_commands.push("".to_string());
        
        rollback_commands.push("# 2. Revert the patch".to_string());
        rollback_commands.push(format!("git revert {} --no-edit", patch.patch_id));
        rollback_commands.push("".to_string());
        
        rollback_commands.push("# 3. Verify rollback".to_string());
        rollback_commands.push("make test".to_string());
        rollback_commands.push("make lint".to_string());
        rollback_commands.push("".to_string());
        
        rollback_commands.push("# 4. If tests fail, manual cleanup may be required".to_string());
        for file in &patch.affected_files {
            rollback_commands.push(format!("# Check file: {}", file.path));
        }
        
        rollback_commands.join("\n")
    }

    // Helper methods
    fn is_critical_file(&self, path: &str) -> bool {
        let critical_patterns = [
            "main.rs", "lib.rs", "mod.rs", "config", "security", 
            "auth", "database", "migration", "schema"
        ];
        
        critical_patterns.iter().any(|pattern| path.contains(pattern))
    }

    fn contains_security_risks(&self, content: &str) -> bool {
        let security_patterns = [
            "password", "secret", "private_key", "api_key",
            "execute(", "eval(", "os.system", "subprocess.call",
            "pickle.loads", "yaml.load", "sql", "query"
        ];
        
        let content_lower = content.to_lowercase();
        security_patterns.iter().any(|pattern| content_lower.contains(pattern))
    }

    fn contains_performance_risks(&self, content: &str) -> bool {
        let performance_patterns = [
            "for.*for.*for", "while.*while", "read()", 
            "*.join(", "sleep(", "time.sleep"
        ];
        
        performance_patterns.iter().any(|pattern| content.contains(pattern))
    }

    fn generate_rollback_commands(&self, patch: &PatchData) -> Vec<String> {
        vec![
            format!("git revert {}", patch.patch_id),
            "git push origin main".to_string(),
            "make test".to_string(),
        ]
    }

    fn has_long_functions(&self, content: &str) -> bool {
        // Simple heuristic: check for functions with >50 lines
        let lines: Vec<&str> = content.lines().collect();
        let mut in_function = false;
        let mut function_lines = 0;
        
        for line in lines {
            if line.trim().starts_with("def ") || line.trim().starts_with("fn ") {
                in_function = true;
                function_lines = 1;
            } else if in_function {
                function_lines += 1;
                if function_lines > 50 {
                    return true;
                }
                if line.trim().is_empty() && !line.starts_with(' ') {
                    in_function = false;
                }
            }
        }
        false
    }

    fn has_adequate_documentation(&self, content: &str) -> bool {
        let doc_indicators = ["\"\"\"", "///", "/**", "#"];
        doc_indicators.iter().any(|indicator| content.contains(indicator))
    }

    fn has_complex_conditions(&self, content: &str) -> bool {
        // Check for complex boolean expressions
        content.lines().any(|line| {
            let and_count = line.matches(" and ").count() + line.matches(" && ").count();
            let or_count = line.matches(" or ").count() + line.matches(" || ").count();
            and_count + or_count > 3
        })
    }

    fn estimate_coverage_delta(&self, _patch: &PatchData) -> Option<f64> {
        // Simplified: return None for now, would integrate with coverage tools
        None
    }

    fn estimate_performance_delta(&self, _patch: &PatchData) -> Option<f64> {
        // Simplified: return None for now, would integrate with benchmarking
        None
    }
}

#[derive(Debug)]
struct GateDecision {
    approved: bool,
    reason: String,
    required_actions: Vec<String>,
}