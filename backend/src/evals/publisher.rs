// P0 Day-5: Evaluation results publisher
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};
use super::suites::SuiteResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishConfig {
    pub output_dir: PathBuf,
    pub generate_html: bool,
    pub update_readme: bool,
    pub create_badges: bool,
}

impl Default for PublishConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("docs/evals"),
            generate_html: true,
            update_readme: true,
            create_badges: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishedResults {
    pub suite_name: String,
    pub date: String,
    pub file_path: PathBuf,
    pub html_path: Option<PathBuf>,
    pub badge_url: Option<String>,
}

pub struct EvalPublisher {
    config: PublishConfig,
}

impl EvalPublisher {
    pub fn new(config: PublishConfig) -> Self {
        Self { config }
    }
    
    /// Publish evaluation results to docs/evals/{suite}/{date}/results.json
    pub async fn publish_results(&self, results: &[SuiteResult]) -> Result<Vec<PublishedResults>> {
        info!(
            results_count = results.len(),
            output_dir = ?self.config.output_dir,
            "Publishing evaluation results"
        );
        
        let mut published = Vec::new();
        
        for result in results {
            let published_result = self.publish_single_result(result).await?;
            published.push(published_result);
        }
        
        // Update README if configured
        if self.config.update_readme {
            self.update_readme_badges(&published).await?;
        }
        
        info!(
            published_count = published.len(),
            "Evaluation results published successfully"
        );
        
        Ok(published)
    }
    
    /// Publish a single evaluation result
    async fn publish_single_result(&self, result: &SuiteResult) -> Result<PublishedResults> {
        let date = result.timestamp.format("%Y%m%d").to_string();
        let suite_name_clean = result.suite_name.replace(" ", "_").replace("+", "Plus").to_lowercase();
        
        // Create directory structure: docs/evals/{suite}/{date}/
        let suite_dir = self.config.output_dir.join(&suite_name_clean).join(&date);
        fs::create_dir_all(&suite_dir)?;
        
        // Write results.json
        let results_path = suite_dir.join("results.json");
        let json_content = serde_json::to_string_pretty(result)?;
        fs::write(&results_path, json_content)?;
        
        info!(
            suite_name = %result.suite_name,
            file_path = ?results_path,
            "Published evaluation results JSON"
        );
        
        // Generate HTML report if configured
        let html_path = if self.config.generate_html {
            let html_path = suite_dir.join("report.html");
            let html_content = self.generate_html_report(result)?;
            fs::write(&html_path, html_content)?;
            
            info!(
                suite_name = %result.suite_name,
                html_path = ?html_path,
                "Generated HTML report"
            );
            
            Some(html_path)
        } else {
            None
        };
        
        // Create badge URL if configured
        let badge_url = if self.config.create_badges {
            Some(self.generate_badge_url(result))
        } else {
            None
        };
        
        Ok(PublishedResults {
            suite_name: result.suite_name.clone(),
            date,
            file_path: results_path,
            html_path,
            badge_url,
        })
    }
    
    /// Generate HTML report for evaluation results
    fn generate_html_report(&self, result: &SuiteResult) -> Result<String> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} Evaluation Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f5f5f5; padding: 20px; border-radius: 8px; }}
        .metrics {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin: 20px 0; }}
        .metric {{ background: white; padding: 15px; border: 1px solid #ddd; border-radius: 8px; }}
        .metric h3 {{ margin: 0 0 10px 0; color: #333; }}
        .metric .value {{ font-size: 2em; font-weight: bold; color: #2196F3; }}
        .tests {{ margin: 20px 0; }}
        .test {{ padding: 10px; margin: 5px 0; border-radius: 4px; }}
        .test.passed {{ background: #e8f5e8; border-left: 4px solid #4caf50; }}
        .test.failed {{ background: #ffeaea; border-left: 4px solid #f44336; }}
        .footer {{ margin-top: 40px; padding: 20px; background: #f5f5f5; border-radius: 8px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{} Evaluation Report</h1>
        <p><strong>Generated:</strong> {}</p>
        <p><strong>Execution Time:</strong> {:.2}s</p>
    </div>
    
    <div class="metrics">
        <div class="metric">
            <h3>Success Rate</h3>
            <div class="value">{:.1}%</div>
        </div>
        <div class="metric">
            <h3>Tests Passed</h3>
            <div class="value">{}/{}</div>
        </div>
        <div class="metric">
            <h3>Average Score</h3>
            <div class="value">{:.2}</div>
        </div>
        <div class="metric">
            <h3>Failed Tests</h3>
            <div class="value">{}</div>
        </div>
    </div>
    
    <div class="tests">
        <h2>Test Results</h2>
        {}
    </div>
    
    <div class="footer">
        <h3>Metadata</h3>
        <pre>{}</pre>
    </div>
</body>
</html>"#,
            result.suite_name,
            result.suite_name,
            result.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            result.execution_time_ms as f64 / 1000.0,
            result.success_rate,
            result.passed_tests,
            result.total_tests,
            result.average_score,
            result.failed_tests,
            self.generate_test_results_html(&result.test_results),
            serde_json::to_string_pretty(&result.metadata)?
        );
        
        Ok(html)
    }
    
    /// Generate HTML for individual test results
    fn generate_test_results_html(&self, test_results: &[super::suites::TestResult]) -> String {
        test_results.iter().map(|test| {
            let status_class = if test.passed { "passed" } else { "failed" };
            let status_text = if test.passed { "✅ PASSED" } else { "❌ FAILED" };
            
            format!(
                r#"<div class="test {}">
                    <strong>{}</strong> - {} <span style="float: right;">{}</span>
                    <br><small>Score: {:.2} | Time: {}ms</small>
                    {}
                </div>"#,
                status_class,
                test.test_id,
                status_text,
                test.score,
                test.score,
                test.execution_time_ms,
                if let Some(error) = &test.error_message {
                    format!("<br><span style=\"color: #f44336;\">Error: {}</span>", error)
                } else {
                    String::new()
                }
            )
        }).collect::<Vec<_>>().join("")
    }
    
    /// Generate badge URL for evaluation results
    fn generate_badge_url(&self, result: &SuiteResult) -> String {
        let color = if result.success_rate >= 90.0 {
            "brightgreen"
        } else if result.success_rate >= 70.0 {
            "yellow"
        } else if result.success_rate >= 50.0 {
            "orange"
        } else {
            "red"
        };
        
        format!(
            "https://img.shields.io/badge/{}-{:.1}%25-{}",
            result.suite_name.replace(" ", "%20"),
            result.success_rate,
            color
        )
    }
    
    /// Update README with evaluation badges and links
    async fn update_readme_badges(&self, published_results: &[PublishedResults]) -> Result<()> {
        let readme_path = Path::new("README.md");
        
        if !readme_path.exists() {
            warn!("README.md not found, skipping badge update");
            return Ok(());
        }
        
        let readme_content = fs::read_to_string(readme_path)?;
        
        // Generate badges section
        let badges_section = self.generate_badges_section(published_results);
        
        // Update README content
        let updated_content = if readme_content.contains("## Evaluation Results") {
            // Replace existing section
            self.replace_eval_section(&readme_content, &badges_section)
        } else {
            // Add new section
            format!("{}\n\n{}", readme_content, badges_section)
        };
        
        fs::write(readme_path, updated_content)?;
        
        info!("README.md updated with evaluation badges");
        
        Ok(())
    }
    
    /// Generate badges section for README
    fn generate_badges_section(&self, published_results: &[PublishedResults]) -> String {
        let mut section = String::from("## Evaluation Results\n\n");
        
        for published in published_results {
            if let Some(badge_url) = &published.badge_url {
                let link_path = if let Some(html_path) = &published.html_path {
                    html_path.to_string_lossy().to_string()
                } else {
                    published.file_path.to_string_lossy().to_string()
                };
                
                section.push_str(&format!(
                    "[![{}]({})]({})\n",
                    published.suite_name,
                    badge_url,
                    link_path
                ));
            }
        }
        
        section.push_str("\n### Latest Results\n\n");
        
        for published in published_results {
            section.push_str(&format!(
                "- **{}** ({}): [JSON]({}) | [HTML]({})\n",
                published.suite_name,
                published.date,
                published.file_path.to_string_lossy(),
                published.html_path.as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "N/A".to_string())
            ));
        }
        
        section
    }
    
    /// Replace existing evaluation section in README
    fn replace_eval_section(&self, content: &str, new_section: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut in_eval_section = false;
        let mut skip_lines = false;
        
        for line in lines {
            if line.starts_with("## Evaluation Results") {
                in_eval_section = true;
                skip_lines = true;
                result.push(new_section.trim_end());
                continue;
            }
            
            if in_eval_section && line.starts_with("## ") && !line.starts_with("## Evaluation Results") {
                in_eval_section = false;
                skip_lines = false;
            }
            
            if !skip_lines {
                result.push(line);
            }
        }
        
        result.join("\n")
    }
}