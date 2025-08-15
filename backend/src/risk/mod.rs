// P0 Day-4: Risk gate implementation with coverage/performance Δ
pub mod coverage_analyzer;
pub mod performance_analyzer;
pub mod risk_calculator;
pub mod risk_gate;

pub use coverage_analyzer::{CoverageAnalyzer, CoverageDelta, CoverageRiskLevel};
pub use performance_analyzer::{PerformanceAnalyzer, PerformanceDelta, PerformanceRiskLevel};
pub use risk_calculator::{RiskCalculator, RiskAssessment, RiskLevel, SecurityIssue, BreakingChange};
pub use risk_gate::{RiskGate, RiskGateConfig, RiskDecision};

/// Public structures for risk assessment
pub struct RiskModule {
    pub coverage_analyzer: CoverageAnalyzer,
    pub performance_analyzer: PerformanceAnalyzer,
    pub risk_calculator: RiskCalculator,
    pub risk_gate: RiskGate,
}