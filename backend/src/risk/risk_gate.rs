// P0 Day-4: Risk gate for patch blocking decisions
use anyhow::Result;
use serde::{Serialize, Deserialize};
use super::{RiskAssessment, RiskLevel, CoverageAnalyzer, PerformanceAnalyzer, RiskCalculator};
use crate::database::repositories::{RunsRepository, ArtifactsRepository};
use uuid::Uuid;
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskGateConfig {
    pub coverage_threshold: f32,
    pub max_coverage_drop: f32,
    pub max_performance_degradation: f32,
    pub max_execution_time_ms: u64,
    pub memory_threshold_mb: f64,
    pub risk_threshold: f32,
    pub auto_block_critical: bool,
}

impl Default for RiskGateConfig {
    fn default() -> Self {
        Self {
            coverage_threshold: 80.0,
            max_coverage_drop: 5.0,
            max_performance_degradation: 25.0,
            max_execution_time_ms: 300_000, // 5 minutes
            memory_threshold_mb: 512.0,
            risk_threshold: 0.6,
            auto_block_critical: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDecision {
    pub should_block: bool,
    pub risk_assessment: RiskAssessment,
    pub gate_config: RiskGateConfig,
    pub decision_reason: String,
    pub override_available: bool,
    pub manual_review_required: bool,
}

pub struct RiskGate {
    config: RiskGateConfig,
    coverage_analyzer: CoverageAnalyzer,
    performance_analyzer: PerformanceAnalyzer,
    risk_calculator: RiskCalculator,
}

impl RiskGate {
    pub fn new(config: RiskGateConfig) -> Self {
        let coverage_analyzer = CoverageAnalyzer::new(
            config.coverage_threshold,
            config.max_coverage_drop,
        );
        
        let performance_analyzer = PerformanceAnalyzer::new(
            config.max_performance_degradation,
            config.max_execution_time_ms,
            config.memory_threshold_mb,
        );
        
        let risk_calculator = RiskCalculator::new();
        
        Self {
            config,
            coverage_analyzer,
            performance_analyzer,
            risk_calculator,
        }
    }
    
    /// Evaluate if a patch should be blocked based on risk analysis
    pub async fn evaluate_patch(
        &self,
        patch_id: &str,
        current_run_id: Uuid,
        baseline_run_id: Option<Uuid>,
        runs_repo: &RunsRepository,
        artifacts_repo: &ArtifactsRepository,
    ) -> Result<RiskDecision> {
        info!(
            patch_id = patch_id,
            current_run_id = %current_run_id,
            baseline_run_id = ?baseline_run_id,
            "Starting risk gate evaluation"
        );
        
        // Get current run data
        let current_run = runs_repo.get_by_id(current_run_id).await?
            .ok_or_else(|| anyhow::anyhow!("Current run not found: {}", current_run_id))?;
        
        // Get baseline run data if available
        let baseline_run = if let Some(baseline_id) = baseline_run_id {
            runs_repo.get_by_id(baseline_id).await?
        } else {
            // Find most recent successful run for comparison
            self.find_baseline_run(runs_repo, &current_run).await?
        };
        
        // Analyze coverage delta
        let coverage_delta = if let Some(baseline) = &baseline_run {
            self.analyze_coverage_change(&current_run, baseline, artifacts_repo).await?
        } else {
            // No baseline available, analyze absolute coverage
            self.analyze_absolute_coverage(&current_run, artifacts_repo).await?
        };
        
        // Analyze performance delta
        let performance_delta = if let Some(baseline) = &baseline_run {
            self.analyze_performance_change(&current_run, baseline).await?
        } else {
            // No baseline available, analyze absolute performance
            self.analyze_absolute_performance(&current_run).await?
        };
        
        // Simulate security issues and breaking changes (in production, these would come from static analysis)
        let security_issues = self.simulate_security_analysis(patch_id).await;
        let breaking_changes = self.simulate_breaking_change_analysis(patch_id).await;
        
        // Calculate overall risk
        let risk_assessment = self.risk_calculator.calculate_risk(
            patch_id,
            &coverage_delta,
            &performance_delta,
            security_issues,
            breaking_changes,
            5, // files_changed - would come from git diff
            150, // lines_added - would come from git diff
            75,  // lines_removed - would come from git diff
        ).await?;
        
        // Make blocking decision
        let (should_block, decision_reason, override_available, manual_review_required) = 
            self.make_blocking_decision(&risk_assessment);
        
        let decision = RiskDecision {
            should_block,
            risk_assessment,
            gate_config: self.config.clone(),
            decision_reason,
            override_available,
            manual_review_required,
        };
        
        info!(
            patch_id = patch_id,
            should_block = should_block,
            risk_score = decision.risk_assessment.risk_score,
            decision_reason = %decision_reason,
            "Risk gate evaluation completed"
        );
        
        Ok(decision)
    }
    
    /// Find the most recent successful run for baseline comparison
    async fn find_baseline_run(
        &self,
        runs_repo: &RunsRepository,
        current_run: &crate::database::repositories::runs::RunRecord,
    ) -> Result<Option<crate::database::repositories::runs::RunRecord>> {
        // Get recent runs for the same project
        let recent_runs = runs_repo.get_by_project_id(current_run.project_id, 10, 0).await?;
        
        // Find the most recent successful run (excluding current run)
        for run in recent_runs {
            if run.id != current_run.id && run.status == "completed" {
                info!(baseline_run_id = %run.id, "Found baseline run for comparison");
                return Ok(Some(run));
            }
        }
        
        warn!("No baseline run found for comparison");
        Ok(None)
    }
    
    /// Analyze coverage change between runs
    async fn analyze_coverage_change(
        &self,
        current_run: &crate::database::repositories::runs::RunRecord,
        baseline_run: &crate::database::repositories::runs::RunRecord,
        _artifacts_repo: &ArtifactsRepository,
    ) -> Result<super::CoverageDelta> {
        // Extract coverage from test results (simplified)
        let current_coverage = self.extract_coverage_from_run(current_run).await;
        let baseline_coverage = self.extract_coverage_from_run(baseline_run).await;
        
        self.coverage_analyzer.analyze_delta(&baseline_coverage, &current_coverage).await
    }
    
    /// Analyze absolute coverage when no baseline available
    async fn analyze_absolute_coverage(
        &self,
        current_run: &crate::database::repositories::runs::RunRecord,
        _artifacts_repo: &ArtifactsRepository,
    ) -> Result<super::CoverageDelta> {
        let current_coverage = self.extract_coverage_from_run(current_run).await;
        
        // Create a dummy baseline for comparison
        let baseline_coverage = super::coverage_analyzer::CoverageReport {
            total_lines: current_coverage.total_lines,
            covered_lines: (current_coverage.total_lines as f32 * self.config.coverage_threshold / 100.0) as usize,
            percentage: self.config.coverage_threshold,
            file_coverage: std::collections::HashMap::new(),
            branch_coverage: None,
        };
        
        self.coverage_analyzer.analyze_delta(&baseline_coverage, &current_coverage).await
    }
    
    /// Extract coverage report from run record
    async fn extract_coverage_from_run(
        &self,
        run: &crate::database::repositories::runs::RunRecord,
    ) -> super::coverage_analyzer::CoverageReport {
        // Extract coverage from run's coverage_data JSON
        if let Some(coverage_data) = &run.coverage_data {
            if let Ok(coverage) = serde_json::from_value::<super::coverage_analyzer::CoverageReport>(coverage_data.clone()) {
                return coverage;
            }
        }
        
        // Fallback: create basic coverage report from test results
        let percentage = if let Some(test_results) = &run.test_results {
            // Extract coverage percentage from test results JSON
            test_results.get("coverage_percentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(75.0) as f32
        } else {
            75.0 // Default assumption
        };
        
        super::coverage_analyzer::CoverageReport {
            total_lines: 1000,
            covered_lines: (1000.0 * percentage / 100.0) as usize,
            percentage,
            file_coverage: std::collections::HashMap::new(),
            branch_coverage: None,
        }
    }
    
    /// Analyze performance change between runs
    async fn analyze_performance_change(
        &self,
        current_run: &crate::database::repositories::runs::RunRecord,
        baseline_run: &crate::database::repositories::runs::RunRecord,
    ) -> Result<super::PerformanceDelta> {
        let current_metrics = self.extract_performance_from_run(current_run).await;
        let baseline_metrics = self.extract_performance_from_run(baseline_run).await;
        
        self.performance_analyzer.analyze_delta(&baseline_metrics, &current_metrics).await
    }
    
    /// Analyze absolute performance when no baseline available
    async fn analyze_absolute_performance(
        &self,
        current_run: &crate::database::repositories::runs::RunRecord,
    ) -> Result<super::PerformanceDelta> {
        let current_metrics = self.extract_performance_from_run(current_run).await;
        
        // Create baseline assuming acceptable performance
        let baseline_metrics = super::performance_analyzer::PerformanceMetrics {
            execution_time_ms: (current_metrics.execution_time_ms as f32 * 0.9) as u64, // 10% faster baseline
            memory_usage_mb: current_metrics.memory_usage_mb.map(|m| m * 0.95), // 5% less memory
            cpu_usage_percent: current_metrics.cpu_usage_percent.map(|c| c * 0.9), // 10% less CPU
            test_metrics: std::collections::HashMap::new(),
            benchmark_results: Vec::new(),
        };
        
        self.performance_analyzer.analyze_delta(&baseline_metrics, &current_metrics).await
    }
    
    /// Extract performance metrics from run record
    async fn extract_performance_from_run(
        &self,
        run: &crate::database::repositories::runs::RunRecord,
    ) -> super::performance_analyzer::PerformanceMetrics {
        let execution_time_ms = run.duration_ms.unwrap_or(5000) as u64;
        
        // Extract from performance_metrics JSON if available
        let (memory_usage_mb, cpu_usage_percent) = if let Some(perf_data) = &run.performance_metrics {
            let memory = perf_data.get("memory_usage_mb").and_then(|v| v.as_f64());
            let cpu = perf_data.get("cpu_usage_percent").and_then(|v| v.as_f64());
            (memory, cpu)
        } else {
            (None, None)
        };
        
        super::performance_analyzer::PerformanceMetrics {
            execution_time_ms,
            memory_usage_mb,
            cpu_usage_percent,
            test_metrics: std::collections::HashMap::new(),
            benchmark_results: Vec::new(),
        }
    }
    
    /// Simulate security analysis (in production, would integrate with actual security scanners)
    async fn simulate_security_analysis(&self, _patch_id: &str) -> Vec<super::risk_calculator::SecurityIssue> {
        // Simulate finding security issues based on patch content
        vec![
            // Example security issue - in production this would come from static analysis
        ]
    }
    
    /// Simulate breaking change analysis (in production, would analyze API changes)
    async fn simulate_breaking_change_analysis(&self, _patch_id: &str) -> Vec<super::risk_calculator::BreakingChange> {
        // Simulate breaking change detection
        vec![
            // Example breaking change - in production this would come from API diff analysis
        ]
    }
    
    /// Make the final blocking decision based on risk assessment
    fn make_blocking_decision(&self, assessment: &RiskAssessment) -> (bool, String, bool, bool) {
        let should_block = assessment.should_block;
        
        let decision_reason = if should_block {
            match assessment.overall_risk {
                RiskLevel::Critical => "BLOCKED: Critical risk level detected".to_string(),
                RiskLevel::High => "BLOCKED: High risk level exceeds threshold".to_string(),
                RiskLevel::Medium => "BLOCKED: Risk score exceeds configured threshold".to_string(),
                RiskLevel::Low => "BLOCKED: Specific threshold violations detected".to_string(),
            }
        } else {
            match assessment.overall_risk {
                RiskLevel::Low => "APPROVED: Low risk, safe to merge".to_string(),
                RiskLevel::Medium => "APPROVED: Medium risk within acceptable limits".to_string(),
                _ => "APPROVED: Risk assessment passed".to_string(),
            }
        };
        
        // Override available for non-critical issues
        let override_available = !matches!(assessment.overall_risk, RiskLevel::Critical) && 
                                !assessment.security_issues.iter().any(|i| i.severity == "CRITICAL");
        
        // Manual review required for high-risk changes
        let manual_review_required = matches!(assessment.overall_risk, RiskLevel::High | RiskLevel::Critical) ||
                                   assessment.security_issues.len() > 2 ||
                                   !assessment.breaking_changes.is_empty();
        
        (should_block, decision_reason, override_available, manual_review_required)
    }
}