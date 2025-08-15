// P0 Day-4: Performance analyzer for calculating performance delta
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub execution_time_ms: u64,
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
    pub test_metrics: HashMap<String, TestMetric>,
    pub benchmark_results: Vec<BenchmarkResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetric {
    pub test_name: String,
    pub duration_ms: u64,
    pub status: String,
    pub memory_delta_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub variance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDelta {
    pub baseline_execution_ms: u64,
    pub current_execution_ms: u64,
    pub execution_delta_ms: i64,
    pub execution_delta_percent: f32,
    pub memory_delta_mb: Option<f64>,
    pub cpu_delta_percent: Option<f64>,
    pub risk_level: PerformanceRiskLevel,
    pub degraded_tests: Vec<String>,
    pub improved_tests: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceRiskLevel {
    Improved,   // Performance got better
    Low,        // <10% degradation
    Medium,     // 10-25% degradation
    High,       // 25-50% degradation
    Critical,   // >50% degradation
}

pub struct PerformanceAnalyzer {
    max_degradation_percent: f32,
    max_execution_time_ms: u64,
    memory_threshold_mb: f64,
}

impl PerformanceAnalyzer {
    pub fn new(max_degradation_percent: f32, max_execution_time_ms: u64, memory_threshold_mb: f64) -> Self {
        Self {
            max_degradation_percent,
            max_execution_time_ms,
            memory_threshold_mb,
        }
    }
    
    /// Analyze performance delta between baseline and current run
    pub async fn analyze_delta(
        &self,
        baseline: &PerformanceMetrics,
        current: &PerformanceMetrics,
    ) -> Result<PerformanceDelta> {
        info!(
            baseline_time = baseline.execution_time_ms,
            current_time = current.execution_time_ms,
            "Analyzing performance delta"
        );
        
        let execution_delta_ms = (current.execution_time_ms as i64) - (baseline.execution_time_ms as i64);
        let execution_delta_percent = if baseline.execution_time_ms > 0 {
            (execution_delta_ms as f32 / baseline.execution_time_ms as f32) * 100.0
        } else {
            0.0
        };
        
        // Determine risk level based on performance change
        let risk_level = match execution_delta_percent {
            d if d < -5.0 => PerformanceRiskLevel::Improved,
            d if d <= 10.0 => PerformanceRiskLevel::Low,
            d if d <= 25.0 => PerformanceRiskLevel::Medium,
            d if d <= 50.0 => PerformanceRiskLevel::High,
            _ => PerformanceRiskLevel::Critical,
        };
        
        // Analyze test-level performance changes
        let (degraded_tests, improved_tests) = self.analyze_test_performance(baseline, current);
        
        // Calculate memory delta
        let memory_delta_mb = match (baseline.memory_usage_mb, current.memory_usage_mb) {
            (Some(baseline_mem), Some(current_mem)) => Some(current_mem - baseline_mem),
            _ => None,
        };
        
        // Calculate CPU delta
        let cpu_delta_percent = match (baseline.cpu_usage_percent, current.cpu_usage_percent) {
            (Some(baseline_cpu), Some(current_cpu)) => Some(current_cpu - baseline_cpu),
            _ => None,
        };
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &risk_level,
            execution_delta_percent,
            &degraded_tests,
            memory_delta_mb,
        );
        
        let delta = PerformanceDelta {
            baseline_execution_ms: baseline.execution_time_ms,
            current_execution_ms: current.execution_time_ms,
            execution_delta_ms,
            execution_delta_percent,
            memory_delta_mb,
            cpu_delta_percent,
            risk_level,
            degraded_tests,
            improved_tests,
            recommendations,
        };
        
        info!(
            execution_delta_percent = execution_delta_percent,
            risk_level = ?delta.risk_level,
            degraded_tests_count = delta.degraded_tests.len(),
            "Performance delta analysis completed"
        );
        
        Ok(delta)
    }
    
    /// Check if performance meets thresholds
    pub fn meets_thresholds(&self, metrics: &PerformanceMetrics) -> bool {
        metrics.execution_time_ms <= self.max_execution_time_ms &&
        metrics.memory_usage_mb.unwrap_or(0.0) <= self.memory_threshold_mb
    }
    
    /// Check if performance degradation is acceptable
    pub fn is_degradation_acceptable(&self, delta: &PerformanceDelta) -> bool {
        delta.execution_delta_percent <= self.max_degradation_percent
    }
    
    /// Parse performance metrics from test output
    pub async fn parse_metrics_from_output(&self, output: &str, stderr: &str) -> Result<PerformanceMetrics> {
        let execution_time_ms = self.extract_execution_time(output, stderr);
        let memory_usage_mb = self.extract_memory_usage(output, stderr);
        let cpu_usage_percent = self.extract_cpu_usage(output, stderr);
        let test_metrics = self.extract_test_metrics(output);
        let benchmark_results = self.extract_benchmark_results(output);
        
        Ok(PerformanceMetrics {
            execution_time_ms,
            memory_usage_mb,
            cpu_usage_percent,
            test_metrics,
            benchmark_results,
        })
    }
    
    /// Analyze individual test performance changes
    fn analyze_test_performance(
        &self,
        baseline: &PerformanceMetrics,
        current: &PerformanceMetrics,
    ) -> (Vec<String>, Vec<String>) {
        let mut degraded = Vec::new();
        let mut improved = Vec::new();
        
        for (test_name, current_metric) in &current.test_metrics {
            if let Some(baseline_metric) = baseline.test_metrics.get(test_name) {
                let delta_percent = if baseline_metric.duration_ms > 0 {
                    ((current_metric.duration_ms as f32 - baseline_metric.duration_ms as f32) 
                     / baseline_metric.duration_ms as f32) * 100.0
                } else {
                    0.0
                };
                
                if delta_percent > 20.0 {  // 20% slower
                    degraded.push(test_name.clone());
                } else if delta_percent < -10.0 {  // 10% faster
                    improved.push(test_name.clone());
                }
            }
        }
        
        (degraded, improved)
    }
    
    /// Generate performance recommendations
    fn generate_recommendations(
        &self,
        risk_level: &PerformanceRiskLevel,
        delta_percent: f32,
        degraded_tests: &[String],
        memory_delta: Option<f64>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        match risk_level {
            PerformanceRiskLevel::Critical => {
                recommendations.push("CRITICAL: Severe performance regression detected. Block this patch.".to_string());
                recommendations.push("Profile the code to identify bottlenecks.".to_string());
                recommendations.push("Consider reverting changes and optimizing.".to_string());
            }
            PerformanceRiskLevel::High => {
                recommendations.push("HIGH RISK: Significant performance degradation.".to_string());
                recommendations.push("Review algorithms and data structures.".to_string());
                recommendations.push("Consider performance optimizations.".to_string());
            }
            PerformanceRiskLevel::Medium => {
                recommendations.push("MEDIUM: Noticeable performance impact.".to_string());
                recommendations.push("Monitor performance in production.".to_string());
            }
            PerformanceRiskLevel::Low => {
                recommendations.push("Minor performance impact is acceptable.".to_string());
            }
            PerformanceRiskLevel::Improved => {
                recommendations.push("Excellent: Performance improved!".to_string());
            }
        }
        
        if !degraded_tests.is_empty() {
            recommendations.push(format!(
                "Focus optimization on these {} slow tests: {}",
                degraded_tests.len(),
                degraded_tests.join(", ")
            ));
        }
        
        if let Some(mem_delta) = memory_delta {
            if mem_delta > 50.0 {
                recommendations.push(format!("Memory usage increased by {:.1} MB. Check for memory leaks.", mem_delta));
            }
        }
        
        recommendations
    }
    
    /// Extract execution time from output
    fn extract_execution_time(&self, output: &str, stderr: &str) -> u64 {
        // Look for patterns like "completed in 2.5s", "took 1500ms", etc.
        let combined = format!("{}\n{}", output, stderr);
        
        for line in combined.lines() {
            // Pattern: "completed in X.Xs" or "took Xms"
            if let Some(time_ms) = self.parse_time_from_line(line) {
                return time_ms;
            }
        }
        
        0  // Default if no time found
    }
    
    /// Parse time from a single line
    fn parse_time_from_line(&self, line: &str) -> Option<u64> {
        let line_lower = line.to_lowercase();
        
        // Look for milliseconds
        if line_lower.contains("ms") {
            for word in line.split_whitespace() {
                if word.ends_with("ms") {
                    if let Ok(ms) = word.trim_end_matches("ms").parse::<u64>() {
                        return Some(ms);
                    }
                }
            }
        }
        
        // Look for seconds
        if line_lower.contains("s") && (line_lower.contains("took") || line_lower.contains("completed")) {
            for word in line.split_whitespace() {
                if word.ends_with("s") && !word.ends_with("ms") {
                    if let Ok(s) = word.trim_end_matches("s").parse::<f64>() {
                        return Some((s * 1000.0) as u64);
                    }
                }
            }
        }
        
        None
    }
    
    /// Extract memory usage from output
    fn extract_memory_usage(&self, output: &str, _stderr: &str) -> Option<f64> {
        // Look for patterns like "Memory: 125.5 MB"
        for line in output.lines() {
            if line.to_lowercase().contains("memory") {
                for word in line.split_whitespace() {
                    if word.to_lowercase().contains("mb") {
                        if let Ok(mb) = word.trim_end_matches("MB").trim_end_matches("mb").parse::<f64>() {
                            return Some(mb);
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Extract CPU usage from output
    fn extract_cpu_usage(&self, output: &str, _stderr: &str) -> Option<f64> {
        // Look for patterns like "CPU: 85.2%"
        for line in output.lines() {
            if line.to_lowercase().contains("cpu") {
                for word in line.split_whitespace() {
                    if word.ends_with("%") {
                        if let Ok(pct) = word.trim_end_matches("%").parse::<f64>() {
                            return Some(pct);
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Extract individual test metrics
    fn extract_test_metrics(&self, output: &str) -> HashMap<String, TestMetric> {
        let mut metrics = HashMap::new();
        
        for line in output.lines() {
            // Look for test result lines like "test_function ... ok (150ms)"
            if line.contains("...") && (line.contains("ok") || line.contains("PASSED")) {
                if let Some(metric) = self.parse_test_metric_line(line) {
                    metrics.insert(metric.test_name.clone(), metric);
                }
            }
        }
        
        metrics
    }
    
    /// Parse test metric from a single line
    fn parse_test_metric_line(&self, line: &str) -> Option<TestMetric> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }
        
        let test_name = parts[0].to_string();
        let status = if line.contains("ok") || line.contains("PASSED") {
            "passed"
        } else {
            "failed"
        }.to_string();
        
        // Extract duration from parentheses
        let duration_ms = line
            .chars()
            .collect::<String>()
            .split('(')
            .nth(1)?
            .split(')')
            .next()?
            .trim_end_matches("ms")
            .parse::<u64>()
            .unwrap_or(0);
        
        Some(TestMetric {
            test_name,
            duration_ms,
            status,
            memory_delta_mb: None,
        })
    }
    
    /// Extract benchmark results
    fn extract_benchmark_results(&self, output: &str) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        
        for line in output.lines() {
            if line.contains("benchmark") || line.contains("bench:") {
                if let Some(result) = self.parse_benchmark_line(line) {
                    results.push(result);
                }
            }
        }
        
        results
    }
    
    /// Parse benchmark result from a single line
    fn parse_benchmark_line(&self, line: &str) -> Option<BenchmarkResult> {
        // Simple benchmark parser - would be more sophisticated in production
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }
        
        Some(BenchmarkResult {
            name: parts[0].to_string(),
            value: parts[1].parse().unwrap_or(0.0),
            unit: parts.get(2).unwrap_or(&"ops/sec").to_string(),
            variance: None,
        })
    }
}