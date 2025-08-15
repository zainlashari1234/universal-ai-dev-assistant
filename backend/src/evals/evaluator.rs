// P0 Day-5: Evaluation runner and orchestrator
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};
use super::suites::{HumanEvalSuite, SWEBenchSuite, CodeCompletionSuite, SuiteResult};
use super::publisher::{EvalPublisher, PublishConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    pub ai_provider: String,
    pub model_name: String,
    pub suites_to_run: Vec<String>,
    pub output_config: PublishConfig,
    pub parallel_execution: bool,
    pub max_retries: usize,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            ai_provider: "ollama".to_string(),
            model_name: "qwen2.5-coder:7b-instruct".to_string(),
            suites_to_run: vec![
                "HumanEval+".to_string(),
                "SWE-bench Lite".to_string(),
                "Code Completion".to_string(),
            ],
            output_config: PublishConfig::default(),
            parallel_execution: false,
            max_retries: 2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvalResult {
    pub config: EvalConfig,
    pub suite_results: Vec<SuiteResult>,
    pub overall_metrics: EvalMetrics,
    pub execution_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvalMetrics {
    pub total_tests: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub overall_success_rate: f32,
    pub average_score: f32,
    pub suite_count: usize,
    pub performance_metrics: HashMap<String, f32>,
}

pub struct EvalRunner {
    config: EvalConfig,
    publisher: EvalPublisher,
}

impl EvalRunner {
    pub fn new(config: EvalConfig) -> Self {
        let publisher = EvalPublisher::new(config.output_config.clone());
        Self { config, publisher }
    }
    
    /// Run all configured evaluation suites
    pub async fn run_evaluations(&self) -> Result<EvalResult> {
        let start_time = std::time::Instant::now();
        
        info!(
            ai_provider = %self.config.ai_provider,
            model_name = %self.config.model_name,
            suites = ?self.config.suites_to_run,
            "Starting evaluation run"
        );
        
        let mut suite_results = Vec::new();
        
        for suite_name in &self.config.suites_to_run {
            match self.run_single_suite(suite_name).await {
                Ok(result) => {
                    info!(
                        suite_name = %suite_name,
                        success_rate = result.success_rate,
                        "Suite evaluation completed"
                    );
                    suite_results.push(result);
                }
                Err(e) => {
                    error!(
                        suite_name = %suite_name,
                        error = %e,
                        "Suite evaluation failed"
                    );
                    
                    // Continue with other suites unless configured otherwise
                    if self.config.max_retries > 0 {
                        warn!("Retrying suite evaluation: {}", suite_name);
                        // Could implement retry logic here
                    }
                }
            }
        }
        
        // Calculate overall metrics
        let overall_metrics = self.calculate_overall_metrics(&suite_results);
        
        let eval_result = EvalResult {
            config: self.config.clone(),
            suite_results: suite_results.clone(),
            overall_metrics,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: Utc::now(),
        };
        
        // Publish results
        self.publisher.publish_results(&suite_results).await?;
        
        info!(
            total_suites = suite_results.len(),
            overall_success_rate = eval_result.overall_metrics.overall_success_rate,
            execution_time_ms = eval_result.execution_time_ms,
            "Evaluation run completed and published"
        );
        
        Ok(eval_result)
    }
    
    /// Run a single evaluation suite
    async fn run_single_suite(&self, suite_name: &str) -> Result<SuiteResult> {
        info!(suite_name = %suite_name, "Running evaluation suite");
        
        match suite_name {
            "HumanEval+" => {
                let suite = HumanEvalSuite::new();
                suite.run_evaluation(&self.config.ai_provider).await
            }
            "SWE-bench Lite" => {
                let suite = SWEBenchSuite::new();
                suite.run_evaluation(&self.config.ai_provider).await
            }
            "Code Completion" => {
                let suite = CodeCompletionSuite::new();
                suite.run_evaluation(&self.config.ai_provider).await
            }
            _ => {
                error!(suite_name = %suite_name, "Unknown evaluation suite");
                Err(anyhow::anyhow!("Unknown evaluation suite: {}", suite_name))
            }
        }
    }
    
    /// Calculate overall metrics across all suites
    fn calculate_overall_metrics(&self, suite_results: &[SuiteResult]) -> EvalMetrics {
        let total_tests: usize = suite_results.iter().map(|r| r.total_tests).sum();
        let total_passed: usize = suite_results.iter().map(|r| r.passed_tests).sum();
        let total_failed: usize = suite_results.iter().map(|r| r.failed_tests).sum();
        
        let overall_success_rate = if total_tests > 0 {
            (total_passed as f32 / total_tests as f32) * 100.0
        } else {
            0.0
        };
        
        let average_score = if !suite_results.is_empty() {
            suite_results.iter().map(|r| r.average_score).sum::<f32>() / suite_results.len() as f32
        } else {
            0.0
        };
        
        // Calculate performance metrics
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("total_execution_time_ms".to_string(), 
                                  suite_results.iter().map(|r| r.execution_time_ms as f32).sum());
        performance_metrics.insert("average_suite_time_ms".to_string(), 
                                  if !suite_results.is_empty() {
                                      suite_results.iter().map(|r| r.execution_time_ms as f32).sum::<f32>() / suite_results.len() as f32
                                  } else {
                                      0.0
                                  });
        
        EvalMetrics {
            total_tests,
            total_passed,
            total_failed,
            overall_success_rate,
            average_score,
            suite_count: suite_results.len(),
            performance_metrics,
        }
    }
    
    /// Create evaluation summary for CLI output
    pub fn create_summary(&self, result: &EvalResult) -> String {
        let mut summary = String::new();
        
        summary.push_str("🎯 EVALUATION SUMMARY\n");
        summary.push_str("========================\n\n");
        
        summary.push_str(&format!("📊 Overall Results:\n"));
        summary.push_str(&format!("   Success Rate: {:.1}%\n", result.overall_metrics.overall_success_rate));
        summary.push_str(&format!("   Tests Passed: {}/{}\n", result.overall_metrics.total_passed, result.overall_metrics.total_tests));
        summary.push_str(&format!("   Average Score: {:.2}\n", result.overall_metrics.average_score));
        summary.push_str(&format!("   Execution Time: {:.2}s\n\n", result.execution_time_ms as f64 / 1000.0));
        
        summary.push_str("📋 Suite Breakdown:\n");
        for suite_result in &result.suite_results {
            summary.push_str(&format!(
                "   {} {}: {:.1}% ({}/{})\n",
                if suite_result.success_rate >= 70.0 { "✅" } else { "❌" },
                suite_result.suite_name,
                suite_result.success_rate,
                suite_result.passed_tests,
                suite_result.total_tests
            ));
        }
        
        summary.push_str("\n📁 Published Results:\n");
        summary.push_str(&format!("   Output Directory: {}\n", result.config.output_config.output_dir.display()));
        summary.push_str("   Format: JSON + HTML reports\n");
        summary.push_str("   README: Updated with badges\n");
        
        summary
    }
}