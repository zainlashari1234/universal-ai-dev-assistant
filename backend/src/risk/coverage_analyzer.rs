// P0 Day-4: Coverage analyzer for calculating coverage delta
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub total_lines: usize,
    pub covered_lines: usize,
    pub percentage: f32,
    pub file_coverage: HashMap<String, FileCoverage>,
    pub branch_coverage: Option<BranchCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub lines_total: usize,
    pub lines_covered: usize,
    pub percentage: f32,
    pub missed_lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCoverage {
    pub branches_total: usize,
    pub branches_covered: usize,
    pub percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageDelta {
    pub baseline_percentage: f32,
    pub current_percentage: f32,
    pub delta_percentage: f32,
    pub delta_lines: i32,
    pub risk_level: CoverageRiskLevel,
    pub affected_files: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageRiskLevel {
    Low,      // +0% to +10% or small decrease
    Medium,   // -1% to -5% decrease
    High,     // -5% to -15% decrease
    Critical, // >-15% decrease
}

pub struct CoverageAnalyzer {
    min_coverage_threshold: f32,
    max_coverage_drop: f32,
}

impl CoverageAnalyzer {
    pub fn new(min_coverage_threshold: f32, max_coverage_drop: f32) -> Self {
        Self {
            min_coverage_threshold,
            max_coverage_drop,
        }
    }
    
    /// Analyze coverage delta between baseline and current run
    pub async fn analyze_delta(
        &self,
        baseline: &CoverageReport,
        current: &CoverageReport,
    ) -> Result<CoverageDelta> {
        info!(
            baseline_coverage = baseline.percentage,
            current_coverage = current.percentage,
            "Analyzing coverage delta"
        );
        
        let delta_percentage = current.percentage - baseline.percentage;
        let delta_lines = (current.covered_lines as i32) - (baseline.covered_lines as i32);
        
        // Determine risk level based on coverage drop
        let risk_level = match delta_percentage {
            d if d >= -1.0 => CoverageRiskLevel::Low,
            d if d >= -5.0 => CoverageRiskLevel::Medium,
            d if d >= -15.0 => CoverageRiskLevel::High,
            _ => CoverageRiskLevel::Critical,
        };
        
        // Find affected files with significant coverage changes
        let affected_files = self.find_affected_files(baseline, current);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&risk_level, delta_percentage, &affected_files);
        
        let delta = CoverageDelta {
            baseline_percentage: baseline.percentage,
            current_percentage: current.percentage,
            delta_percentage,
            delta_lines,
            risk_level,
            affected_files,
            recommendations,
        };
        
        info!(
            delta_percentage = delta_percentage,
            risk_level = ?delta.risk_level,
            affected_files_count = delta.affected_files.len(),
            "Coverage delta analysis completed"
        );
        
        Ok(delta)
    }
    
    /// Check if coverage meets minimum thresholds
    pub fn meets_thresholds(&self, coverage: &CoverageReport) -> bool {
        coverage.percentage >= self.min_coverage_threshold
    }
    
    /// Check if coverage drop is acceptable
    pub fn is_coverage_drop_acceptable(&self, delta: &CoverageDelta) -> bool {
        delta.delta_percentage >= -self.max_coverage_drop
    }
    
    /// Parse coverage report from test output
    pub async fn parse_coverage_from_output(&self, output: &str, format: &str) -> Result<CoverageReport> {
        match format {
            "lcov" => self.parse_lcov_format(output).await,
            "cobertura" => self.parse_cobertura_format(output).await,
            "jacoco" => self.parse_jacoco_format(output).await,
            "pytest-cov" => self.parse_pytest_cov_format(output).await,
            _ => {
                warn!(format = format, "Unknown coverage format, using simple parser");
                self.parse_simple_format(output).await
            }
        }
    }
    
    /// Find files with significant coverage changes
    fn find_affected_files(&self, baseline: &CoverageReport, current: &CoverageReport) -> Vec<String> {
        let mut affected = Vec::new();
        
        for (file, current_cov) in &current.file_coverage {
            if let Some(baseline_cov) = baseline.file_coverage.get(file) {
                let delta = current_cov.percentage - baseline_cov.percentage;
                if delta.abs() > 5.0 {  // 5% threshold for file-level changes
                    affected.push(file.clone());
                }
            } else {
                // New file, always include
                affected.push(file.clone());
            }
        }
        
        affected
    }
    
    /// Generate recommendations based on coverage analysis
    fn generate_recommendations(&self, risk_level: &CoverageRiskLevel, delta: f32, affected_files: &[String]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        match risk_level {
            CoverageRiskLevel::Critical => {
                recommendations.push("CRITICAL: Coverage dropped significantly. Block this patch.".to_string());
                recommendations.push("Add comprehensive tests before merging.".to_string());
                recommendations.push("Review untested code paths in affected files.".to_string());
            }
            CoverageRiskLevel::High => {
                recommendations.push("HIGH RISK: Consider blocking this patch.".to_string());
                recommendations.push("Add tests for critical paths.".to_string());
            }
            CoverageRiskLevel::Medium => {
                recommendations.push("Add tests for newly added code.".to_string());
                recommendations.push("Review test coverage in affected files.".to_string());
            }
            CoverageRiskLevel::Low => {
                if delta > 0.0 {
                    recommendations.push("Good: Coverage improved.".to_string());
                } else {
                    recommendations.push("Minor coverage decrease is acceptable.".to_string());
                }
            }
        }
        
        if !affected_files.is_empty() {
            recommendations.push(format!(
                "Focus testing efforts on these {} affected files: {}",
                affected_files.len(),
                affected_files.join(", ")
            ));
        }
        
        recommendations
    }
    
    /// Parse pytest-cov format coverage output
    async fn parse_pytest_cov_format(&self, output: &str) -> Result<CoverageReport> {
        // Simple pytest-cov parser - in production would be more robust
        let lines: Vec<&str> = output.lines().collect();
        let mut file_coverage = HashMap::new();
        let mut total_lines = 0;
        let mut covered_lines = 0;
        
        for line in lines {
            if line.contains(".py") && line.contains("%") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let filename = parts[0].to_string();
                    let statements: usize = parts[1].parse().unwrap_or(0);
                    let missed: usize = parts[2].parse().unwrap_or(0);
                    let coverage_pct: f32 = parts[3].trim_end_matches('%').parse().unwrap_or(0.0);
                    
                    let file_covered = statements - missed;
                    total_lines += statements;
                    covered_lines += file_covered;
                    
                    file_coverage.insert(filename, FileCoverage {
                        lines_total: statements,
                        lines_covered: file_covered,
                        percentage: coverage_pct,
                        missed_lines: Vec::new(), // Would need line-by-line analysis
                    });
                }
            }
        }
        
        let percentage = if total_lines > 0 {
            (covered_lines as f32 / total_lines as f32) * 100.0
        } else {
            0.0
        };
        
        Ok(CoverageReport {
            total_lines,
            covered_lines,
            percentage,
            file_coverage,
            branch_coverage: None, // Would need detailed analysis
        })
    }
    
    /// Parse simple coverage format (basic percentage)
    async fn parse_simple_format(&self, output: &str) -> Result<CoverageReport> {
        // Extract percentage from output like "Coverage: 85.2%"
        let percentage = output
            .lines()
            .find_map(|line| {
                if line.to_lowercase().contains("coverage") {
                    line.chars()
                        .collect::<String>()
                        .split_whitespace()
                        .find_map(|word| {
                            if word.ends_with('%') {
                                word.trim_end_matches('%').parse::<f32>().ok()
                            } else {
                                None
                            }
                        })
                } else {
                    None
                }
            })
            .unwrap_or(0.0);
        
        Ok(CoverageReport {
            total_lines: 100,
            covered_lines: (percentage as usize),
            percentage,
            file_coverage: HashMap::new(),
            branch_coverage: None,
        })
    }
    
    /// Parse LCOV format (placeholder - would need full implementation)
    async fn parse_lcov_format(&self, _output: &str) -> Result<CoverageReport> {
        // Placeholder for LCOV parser
        warn!("LCOV format parser not fully implemented");
        self.parse_simple_format(_output).await
    }
    
    /// Parse Cobertura format (placeholder)
    async fn parse_cobertura_format(&self, _output: &str) -> Result<CoverageReport> {
        warn!("Cobertura format parser not fully implemented");
        self.parse_simple_format(_output).await
    }
    
    /// Parse JaCoCo format (placeholder)
    async fn parse_jacoco_format(&self, _output: &str) -> Result<CoverageReport> {
        warn!("JaCoCo format parser not fully implemented");
        self.parse_simple_format(_output).await
    }
}