// P0 Day-4: Risk calculator for overall risk assessment
use anyhow::Result;
use serde::{Serialize, Deserialize};
use super::{CoverageDelta, CoverageRiskLevel, PerformanceDelta, PerformanceRiskLevel};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_score: f32,  // 0.0 (no risk) to 1.0 (maximum risk)
    pub should_block: bool,
    pub coverage_risk: CoverageRiskLevel,
    pub performance_risk: PerformanceRiskLevel,
    pub security_issues: Vec<SecurityIssue>,
    pub breaking_changes: Vec<BreakingChange>,
    pub rollback_commands: Vec<String>,
    pub recommendations: Vec<String>,
    pub metadata: RiskMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // 0.0 - 0.3
    Medium,   // 0.3 - 0.6  
    High,     // 0.6 - 0.8
    Critical, // 0.8 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: String,
    pub description: String,
    pub file: String,
    pub line: Option<usize>,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    pub description: String,
    pub affected_apis: Vec<String>,
    pub migration_guide: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetadata {
    pub patch_id: String,
    pub files_changed: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub test_coverage_before: f32,
    pub test_coverage_after: f32,
    pub performance_delta_ms: i64,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct RiskCalculator {
    coverage_weight: f32,
    performance_weight: f32,
    security_weight: f32,
    breaking_changes_weight: f32,
    risk_threshold: f32,
}

impl RiskCalculator {
    pub fn new() -> Self {
        Self {
            coverage_weight: 0.3,
            performance_weight: 0.3,
            security_weight: 0.25,
            breaking_changes_weight: 0.15,
            risk_threshold: 0.6, // Block patches with risk score > 0.6
        }
    }
    
    pub fn with_weights(
        coverage_weight: f32,
        performance_weight: f32,
        security_weight: f32,
        breaking_changes_weight: f32,
    ) -> Self {
        Self {
            coverage_weight,
            performance_weight,
            security_weight,
            breaking_changes_weight,
            risk_threshold: 0.6,
        }
    }
    
    /// Calculate overall risk assessment
    pub async fn calculate_risk(
        &self,
        patch_id: &str,
        coverage_delta: &CoverageDelta,
        performance_delta: &PerformanceDelta,
        security_issues: Vec<SecurityIssue>,
        breaking_changes: Vec<BreakingChange>,
        files_changed: usize,
        lines_added: usize,
        lines_removed: usize,
    ) -> Result<RiskAssessment> {
        info!(
            patch_id = patch_id,
            files_changed = files_changed,
            lines_added = lines_added,
            "Calculating risk assessment"
        );
        
        // Calculate individual risk scores
        let coverage_score = self.calculate_coverage_risk_score(&coverage_delta.risk_level);
        let performance_score = self.calculate_performance_risk_score(&performance_delta.risk_level);
        let security_score = self.calculate_security_risk_score(&security_issues);
        let breaking_changes_score = self.calculate_breaking_changes_score(&breaking_changes);
        
        // Calculate weighted overall risk score
        let risk_score = (coverage_score * self.coverage_weight) +
                        (performance_score * self.performance_weight) +
                        (security_score * self.security_weight) +
                        (breaking_changes_score * self.breaking_changes_weight);
        
        // Determine overall risk level
        let overall_risk = match risk_score {
            s if s <= 0.3 => RiskLevel::Low,
            s if s <= 0.6 => RiskLevel::Medium,
            s if s <= 0.8 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };
        
        // Determine if patch should be blocked
        let should_block = risk_score > self.risk_threshold ||
                          matches!(coverage_delta.risk_level, CoverageRiskLevel::Critical) ||
                          matches!(performance_delta.risk_level, PerformanceRiskLevel::Critical) ||
                          security_issues.iter().any(|issue| issue.severity == "CRITICAL");
        
        // Generate rollback commands
        let rollback_commands = self.generate_rollback_commands(patch_id, files_changed);
        
        // Combine recommendations
        let mut recommendations = Vec::new();
        recommendations.extend(coverage_delta.recommendations.clone());
        recommendations.extend(performance_delta.recommendations.clone());
        recommendations.extend(self.generate_security_recommendations(&security_issues));
        recommendations.extend(self.generate_breaking_change_recommendations(&breaking_changes));
        
        let metadata = RiskMetadata {
            patch_id: patch_id.to_string(),
            files_changed,
            lines_added,
            lines_removed,
            test_coverage_before: coverage_delta.baseline_percentage,
            test_coverage_after: coverage_delta.current_percentage,
            performance_delta_ms: performance_delta.execution_delta_ms,
            analysis_timestamp: chrono::Utc::now(),
        };
        
        let assessment = RiskAssessment {
            overall_risk,
            risk_score,
            should_block,
            coverage_risk: coverage_delta.risk_level.clone(),
            performance_risk: performance_delta.risk_level.clone(),
            security_issues,
            breaking_changes,
            rollback_commands,
            recommendations,
            metadata,
        };
        
        info!(
            patch_id = patch_id,
            risk_score = risk_score,
            overall_risk = ?assessment.overall_risk,
            should_block = should_block,
            "Risk assessment completed"
        );
        
        Ok(assessment)
    }
    
    /// Calculate coverage risk score (0.0 to 1.0)
    fn calculate_coverage_risk_score(&self, risk_level: &CoverageRiskLevel) -> f32 {
        match risk_level {
            CoverageRiskLevel::Low => 0.1,
            CoverageRiskLevel::Medium => 0.4,
            CoverageRiskLevel::High => 0.7,
            CoverageRiskLevel::Critical => 1.0,
        }
    }
    
    /// Calculate performance risk score (0.0 to 1.0)
    fn calculate_performance_risk_score(&self, risk_level: &PerformanceRiskLevel) -> f32 {
        match risk_level {
            PerformanceRiskLevel::Improved => 0.0,
            PerformanceRiskLevel::Low => 0.2,
            PerformanceRiskLevel::Medium => 0.5,
            PerformanceRiskLevel::High => 0.8,
            PerformanceRiskLevel::Critical => 1.0,
        }
    }
    
    /// Calculate security risk score based on issues
    fn calculate_security_risk_score(&self, issues: &[SecurityIssue]) -> f32 {
        if issues.is_empty() {
            return 0.0;
        }
        
        let mut max_score = 0.0;
        for issue in issues {
            let score = match issue.severity.as_str() {
                "LOW" => 0.2,
                "MEDIUM" => 0.4,
                "HIGH" => 0.7,
                "CRITICAL" => 1.0,
                _ => 0.3,
            };
            max_score = max_score.max(score);
        }
        
        // Factor in number of issues
        let count_factor = (issues.len() as f32).sqrt() / 10.0;
        (max_score + count_factor).min(1.0)
    }
    
    /// Calculate breaking changes risk score
    fn calculate_breaking_changes_score(&self, breaking_changes: &[BreakingChange]) -> f32 {
        if breaking_changes.is_empty() {
            0.0
        } else {
            // Each breaking change adds significant risk
            (breaking_changes.len() as f32 * 0.3).min(1.0)
        }
    }
    
    /// Generate rollback commands for the patch
    fn generate_rollback_commands(&self, patch_id: &str, files_changed: usize) -> Vec<String> {
        vec![
            format!("# Rollback commands for patch {}", patch_id),
            "git log --oneline -n 5".to_string(),
            "git diff HEAD~1 --name-only".to_string(),
            "git reset --hard HEAD~1".to_string(),
            format!("# Alternative: git revert {}", patch_id),
            "# Verify rollback: run tests and check functionality".to_string(),
            format!("# {} files will be restored", files_changed),
        ]
    }
    
    /// Generate security-specific recommendations
    fn generate_security_recommendations(&self, issues: &[SecurityIssue]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if !issues.is_empty() {
            recommendations.push(format!("Address {} security issue(s) before merging", issues.len()));
            
            for issue in issues {
                if issue.severity == "CRITICAL" || issue.severity == "HIGH" {
                    recommendations.push(format!(
                        "{} security issue in {}: {}",
                        issue.severity, issue.file, issue.description
                    ));
                }
            }
        }
        
        recommendations
    }
    
    /// Generate breaking change recommendations
    fn generate_breaking_change_recommendations(&self, breaking_changes: &[BreakingChange]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        for change in breaking_changes {
            recommendations.push(format!(
                "Breaking change detected: {}. Update affected APIs: {}",
                change.description,
                change.affected_apis.join(", ")
            ));
        }
        
        recommendations
    }
    
    /// Set custom risk threshold
    pub fn set_risk_threshold(&mut self, threshold: f32) {
        self.risk_threshold = threshold.clamp(0.0, 1.0);
    }
}