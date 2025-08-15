use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use uuid::Uuid;

use crate::ai_engine::providers::ProviderRouter;
use crate::observability::get_metrics;

use super::{AgentRequest, AgentResponse, AgentArtifact, ArtifactType};

/// ReviewerAgent: Reviews code quality and provides feedback
/// Analyzes code for best practices, maintainability, and quality
pub struct ReviewerAgent {
    provider_router: Arc<ProviderRouter>,
}

impl ReviewerAgent {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self { provider_router }
    }

    /// Execute code review for generated code
    pub async fn execute(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let start_time = Instant::now();
        
        info!("ReviewerAgent executing for goal: {}", request.goal);
        
        // Record metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["reviewer", "execute"])
            .observe(0.0);
        
        let mut artifacts = Vec::new();
        let cost = 0.1; // Cost for review
        
        // Perform code review
        let review_result = self.perform_code_review(&request.goal, &request.context).await?;
        
        // Create review artifact
        artifacts.push(AgentArtifact {
            name: "code_review_report.json".to_string(),
            artifact_type: ArtifactType::Report,
            content: serde_json::to_string_pretty(&review_result)?,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("goal".to_string(), request.goal.clone());
                meta.insert("overall_score".to_string(), review_result["overall_score"].as_f64().unwrap_or(0.0).to_string());
                meta
            },
        });
        
        let execution_time = start_time.elapsed();
        
        // Record execution time
        metrics.agent_step_duration_ms
            .with_label_values(&["reviewer", "execute"])
            .observe(execution_time.as_millis() as f64);
        
        let response = AgentResponse {
            id: request.id,
            agent_name: "reviewer".to_string(),
            success: true,
            result: review_result,
            artifacts,
            execution_time,
            cost: Some(cost),
            error: None,
        };
        
        debug!("ReviewerAgent completed in {:?}", execution_time);
        
        Ok(response)
    }

    /// Perform comprehensive code review
    async fn perform_code_review(&self, goal: &str, context: &Option<String>) -> Result<Value> {
        let generated_code = if let Some(ctx) = context {
            self.extract_code_from_context(ctx)?
        } else {
            vec![]
        };
        
        let mut review_scores = HashMap::new();
        let mut suggestions = Vec::new();
        let mut issues = Vec::new();
        
        // Review each file
        for code_file in &generated_code {
            let filename = code_file["filename"].as_str().unwrap_or("unknown");
            let content = code_file["content"].as_str().unwrap_or("");
            let language = code_file["language"].as_str().unwrap_or("python");
            
            let file_review = self.review_file(filename, content, language).await?;
            review_scores.insert(filename.to_string(), file_review);
        }
        
        // Generate overall suggestions
        suggestions.extend(self.generate_suggestions(goal, &generated_code).await?);
        
        // Identify issues
        issues.extend(self.identify_issues(&generated_code).await?);
        
        // Calculate overall scores
        let overall_score = self.calculate_overall_score(&review_scores);
        let code_quality = self.calculate_code_quality_score(&generated_code);
        let maintainability = self.calculate_maintainability_score(&generated_code);
        
        let review_result = json!({
            "overall_score": overall_score,
            "code_quality": code_quality,
            "maintainability": maintainability,
            "file_scores": review_scores,
            "suggestions": suggestions,
            "issues": issues,
            "reviewed_files": generated_code.len(),
            "review_timestamp": chrono::Utc::now().to_rfc3339(),
            "goal": goal
        });
        
        Ok(review_result)
    }

    /// Extract code from context
    fn extract_code_from_context(&self, context: &str) -> Result<Vec<Value>> {
        if let Ok(parsed) = serde_json::from_str::<Value>(context) {
            if let Some(generated_files) = parsed["generated_files"].as_array() {
                return Ok(generated_files.clone());
            }
        }
        
        // Fallback
        Ok(vec![json!({
            "filename": "code.py",
            "content": context,
            "language": "python"
        })])
    }

    /// Review individual file
    async fn review_file(&self, filename: &str, content: &str, language: &str) -> Result<Value> {
        let mut score = 10.0; // Start with perfect score
        let mut deductions = Vec::new();
        
        // Check code length
        let line_count = content.lines().count();
        if line_count > 100 {
            score -= 0.5;
            deductions.push("File is quite long, consider splitting".to_string());
        }
        
        // Language-specific checks
        match language {
            "python" => {
                score -= self.check_python_issues(content, &mut deductions);
            }
            "javascript" => {
                score -= self.check_javascript_issues(content, &mut deductions);
            }
            _ => {}
        }
        
        // Check for documentation
        if !content.contains("\"\"\"") && !content.contains("/**") {
            score -= 1.0;
            deductions.push("Missing documentation/docstrings".to_string());
        }
        
        // Check for error handling
        if !content.contains("try") && !content.contains("except") && 
           !content.contains("catch") && !content.contains("throw") {
            score -= 0.5;
            deductions.push("Consider adding error handling".to_string());
        }
        
        Ok(json!({
            "filename": filename,
            "score": score.max(0.0),
            "line_count": line_count,
            "deductions": deductions,
            "language": language
        }))
    }

    /// Check Python-specific issues
    fn check_python_issues(&self, content: &str, deductions: &mut Vec<String>) -> f64 {
        let mut penalty = 0.0;
        
        // Check for PEP 8 violations (simplified)
        for line in content.lines() {
            if line.len() > 120 {
                penalty += 0.1;
                deductions.push("Line too long (>120 chars)".to_string());
                break;
            }
        }
        
        // Check for proper imports
        let import_lines: Vec<&str> = content.lines()
            .filter(|line| line.trim().starts_with("import ") || line.trim().starts_with("from "))
            .collect();
        
        if import_lines.len() > 10 {
            penalty += 0.3;
            deductions.push("Too many imports, consider organizing".to_string());
        }
        
        // Check for function complexity (simplified)
        let function_count = content.matches("def ").count();
        let total_lines = content.lines().count();
        
        if function_count > 0 && total_lines / function_count > 20 {
            penalty += 0.5;
            deductions.push("Functions might be too complex".to_string());
        }
        
        penalty
    }

    /// Check JavaScript-specific issues
    fn check_javascript_issues(&self, content: &str, deductions: &mut Vec<String>) -> f64 {
        let mut penalty = 0.0;
        
        // Check for var usage (prefer let/const)
        if content.contains("var ") {
            penalty += 0.3;
            deductions.push("Use 'let' or 'const' instead of 'var'".to_string());
        }
        
        // Check for == usage (prefer ===)
        if content.contains(" == ") && !content.contains(" === ") {
            penalty += 0.2;
            deductions.push("Use '===' instead of '==' for comparison".to_string());
        }
        
        // Check for semicolons
        let lines_without_semicolon = content.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && 
                !trimmed.starts_with("//") && 
                !trimmed.ends_with(';') && 
                !trimmed.ends_with('{') && 
                !trimmed.ends_with('}')
            })
            .count();
        
        if lines_without_semicolon > 2 {
            penalty += 0.2;
            deductions.push("Missing semicolons".to_string());
        }
        
        penalty
    }

    /// Generate improvement suggestions
    async fn generate_suggestions(&self, goal: &str, generated_code: &[Value]) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();
        
        // Goal-specific suggestions
        let goal_lower = goal.to_lowercase();
        
        if goal_lower.contains("error") || goal_lower.contains("exception") {
            suggestions.push("Consider adding comprehensive error handling with specific exception types".to_string());
            suggestions.push("Add logging for error cases to aid debugging".to_string());
        }
        
        if goal_lower.contains("performance") || goal_lower.contains("optimize") {
            suggestions.push("Consider adding performance benchmarks".to_string());
            suggestions.push("Profile the code to identify bottlenecks".to_string());
        }
        
        // Code structure suggestions
        if generated_code.len() == 1 {
            suggestions.push("Consider splitting functionality into multiple modules for better organization".to_string());
        }
        
        // Documentation suggestions
        suggestions.push("Add comprehensive docstrings with parameter and return type information".to_string());
        suggestions.push("Consider adding usage examples in documentation".to_string());
        
        // Testing suggestions
        suggestions.push("Ensure test coverage includes edge cases and error conditions".to_string());
        suggestions.push("Consider adding integration tests for complete workflows".to_string());
        
        Ok(suggestions)
    }

    /// Identify code issues
    async fn identify_issues(&self, generated_code: &[Value]) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        for code_file in generated_code {
            let content = code_file["content"].as_str().unwrap_or("");
            let filename = code_file["filename"].as_str().unwrap_or("unknown");
            
            // Check for potential security issues
            if content.contains("eval(") || content.contains("exec(") {
                issues.push(format!("{}: Potential security risk with eval/exec usage", filename));
            }
            
            // Check for hardcoded values
            if content.contains("password") || content.contains("secret") || content.contains("key") {
                issues.push(format!("{}: Potential hardcoded sensitive information", filename));
            }
            
            // Check for TODO/FIXME comments
            if content.contains("TODO") || content.contains("FIXME") {
                issues.push(format!("{}: Contains TODO/FIXME comments that need attention", filename));
            }
            
            // Check for empty functions
            if content.contains("pass") && content.matches("def ").count() > 0 {
                issues.push(format!("{}: Contains empty functions with 'pass' statements", filename));
            }
        }
        
        Ok(issues)
    }

    /// Calculate overall score
    fn calculate_overall_score(&self, review_scores: &HashMap<String, Value>) -> f64 {
        if review_scores.is_empty() {
            return 0.0;
        }
        
        let total_score: f64 = review_scores.values()
            .filter_map(|score| score["score"].as_f64())
            .sum();
        
        total_score / review_scores.len() as f64
    }

    /// Calculate code quality score
    fn calculate_code_quality_score(&self, generated_code: &[Value]) -> f64 {
        let mut score = 8.0; // Base score
        
        for code_file in generated_code {
            let content = code_file["content"].as_str().unwrap_or("");
            
            // Bonus for documentation
            if content.contains("\"\"\"") || content.contains("/**") {
                score += 0.5;
            }
            
            // Bonus for error handling
            if content.contains("try") || content.contains("catch") {
                score += 0.3;
            }
            
            // Penalty for very short or very long files
            let line_count = content.lines().count();
            if line_count < 5 {
                score -= 0.5;
            } else if line_count > 200 {
                score -= 0.3;
            }
        }
        
        score.min(10.0).max(0.0)
    }

    /// Calculate maintainability score
    fn calculate_maintainability_score(&self, generated_code: &[Value]) -> f64 {
        let mut score = 8.5; // Base score
        
        // Multiple files indicate better organization
        if generated_code.len() > 1 {
            score += 0.5;
        }
        
        // Check for consistent naming and structure
        let mut has_consistent_style = true;
        for code_file in generated_code {
            let content = code_file["content"].as_str().unwrap_or("");
            
            // Check for consistent indentation (simplified)
            let lines: Vec<&str> = content.lines().collect();
            if lines.len() > 5 {
                let indented_lines: Vec<&str> = lines.iter()
                    .filter(|line| line.starts_with("    ") || line.starts_with("\t"))
                    .cloned()
                    .collect();
                
                if !indented_lines.is_empty() {
                    let uses_spaces = indented_lines[0].starts_with("    ");
                    let consistent = indented_lines.iter()
                        .all(|line| line.starts_with("    ") == uses_spaces);
                    
                    if !consistent {
                        has_consistent_style = false;
                    }
                }
            }
        }
        
        if !has_consistent_style {
            score -= 1.0;
        }
        
        score.min(10.0).max(0.0)
    }
}