use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use uuid::Uuid;

use crate::ai_engine::providers::ProviderRouter;
use crate::observability::get_metrics;

use super::{AgentRequest, AgentResponse, AgentArtifact, ArtifactType, RiskLevel};

/// RiskAgent: Assesses security and performance risks
/// Analyzes code for potential security vulnerabilities and performance issues
pub struct RiskAgent {
    provider_router: Arc<ProviderRouter>,
}

impl RiskAgent {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self { provider_router }
    }

    /// Execute risk assessment for generated code
    pub async fn execute(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let start_time = Instant::now();
        
        info!("RiskAgent executing for goal: {}", request.goal);
        
        // Record metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["risk", "execute"])
            .observe(0.0);
        
        let mut artifacts = Vec::new();
        let cost = 0.1; // Cost for risk assessment
        
        // Perform risk assessment
        let risk_result = self.perform_risk_assessment(&request.goal, &request.context).await?;
        
        // Create risk assessment artifact
        artifacts.push(AgentArtifact {
            name: "risk_assessment_report.json".to_string(),
            artifact_type: ArtifactType::Report,
            content: serde_json::to_string_pretty(&risk_result)?,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("goal".to_string(), request.goal.clone());
                meta.insert("risk_level".to_string(), risk_result["risk_level"].as_str().unwrap_or("low").to_string());
                meta
            },
        });
        
        let execution_time = start_time.elapsed();
        
        // Record execution time
        metrics.agent_step_duration_ms
            .with_label_values(&["risk", "execute"])
            .observe(execution_time.as_millis() as f64);
        
        let response = AgentResponse {
            id: request.id,
            agent_name: "risk".to_string(),
            success: true,
            result: risk_result,
            artifacts,
            execution_time,
            cost: Some(cost),
            error: None,
        };
        
        debug!("RiskAgent completed in {:?}", execution_time);
        
        Ok(response)
    }

    /// Perform comprehensive risk assessment
    async fn perform_risk_assessment(&self, goal: &str, context: &Option<String>) -> Result<Value> {
        let generated_code = if let Some(ctx) = context {
            self.extract_code_from_context(ctx)?
        } else {
            vec![]
        };
        
        // Assess different risk categories
        let security_risks = self.assess_security_risks(&generated_code).await?;
        let performance_risks = self.assess_performance_risks(&generated_code).await?;
        let breaking_changes = self.assess_breaking_changes(goal, &generated_code).await?;
        let operational_risks = self.assess_operational_risks(&generated_code).await?;
        
        // Calculate overall risk level
        let risk_level = self.calculate_overall_risk_level(&security_risks, &performance_risks, &breaking_changes);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&security_risks, &performance_risks, &breaking_changes).await?;
        
        let risk_assessment = json!({
            "risk_level": risk_level,
            "security_risks": security_risks,
            "performance_risks": performance_risks,
            "breaking_changes": breaking_changes,
            "operational_risks": operational_risks,
            "recommendations": recommendations,
            "assessment_timestamp": chrono::Utc::now().to_rfc3339(),
            "goal": goal,
            "files_analyzed": generated_code.len()
        });
        
        Ok(risk_assessment)
    }

    /// Extract code from context
    fn extract_code_from_context(&self, context: &str) -> Result<Vec<Value>> {
        if let Ok(parsed) = serde_json::from_str::<Value>(context) {
            if let Some(generated_files) = parsed["generated_files"].as_array() {
                return Ok(generated_files.clone());
            }
        }
        
        Ok(vec![json!({
            "filename": "code.py",
            "content": context,
            "language": "python"
        })])
    }

    /// Assess security risks
    async fn assess_security_risks(&self, generated_code: &[Value]) -> Result<Vec<Value>> {
        let mut security_risks = Vec::new();
        
        for code_file in generated_code {
            let filename = code_file["filename"].as_str().unwrap_or("unknown");
            let content = code_file["content"].as_str().unwrap_or("");
            
            // Check for common security vulnerabilities
            
            // SQL Injection risks
            if content.contains("execute(") && content.contains("%s") {
                security_risks.push(json!({
                    "type": "sql_injection",
                    "severity": "high",
                    "file": filename,
                    "description": "Potential SQL injection vulnerability with string formatting",
                    "recommendation": "Use parameterized queries instead of string formatting"
                }));
            }
            
            // Command injection risks
            if content.contains("os.system(") || content.contains("subprocess.call(") {
                security_risks.push(json!({
                    "type": "command_injection",
                    "severity": "high",
                    "file": filename,
                    "description": "Potential command injection vulnerability",
                    "recommendation": "Validate and sanitize all inputs to system commands"
                }));
            }
            
            // Hardcoded secrets
            if self.contains_potential_secrets(content) {
                security_risks.push(json!({
                    "type": "hardcoded_secrets",
                    "severity": "medium",
                    "file": filename,
                    "description": "Potential hardcoded secrets or credentials",
                    "recommendation": "Use environment variables or secure credential storage"
                }));
            }
            
            // Unsafe deserialization
            if content.contains("pickle.loads(") || content.contains("eval(") {
                security_risks.push(json!({
                    "type": "unsafe_deserialization",
                    "severity": "high",
                    "file": filename,
                    "description": "Unsafe deserialization or code execution",
                    "recommendation": "Use safe serialization methods and avoid eval()"
                }));
            }
            
            // Path traversal
            if content.contains("../") && content.contains("open(") {
                security_risks.push(json!({
                    "type": "path_traversal",
                    "severity": "medium",
                    "file": filename,
                    "description": "Potential path traversal vulnerability",
                    "recommendation": "Validate and sanitize file paths"
                }));
            }
        }
        
        Ok(security_risks)
    }

    /// Check for potential secrets in code
    fn contains_potential_secrets(&self, content: &str) -> bool {
        let secret_patterns = [
            "password", "secret", "key", "token", "api_key",
            "private_key", "access_key", "auth_token"
        ];
        
        let content_lower = content.to_lowercase();
        
        for pattern in &secret_patterns {
            if content_lower.contains(pattern) && 
               (content_lower.contains("=") || content_lower.contains(":")) {
                // Check if it's not just a variable name
                if !content_lower.contains(&format!("{}_file", pattern)) &&
                   !content_lower.contains(&format!("{}_path", pattern)) {
                    return true;
                }
            }
        }
        
        false
    }

    /// Assess performance risks
    async fn assess_performance_risks(&self, generated_code: &[Value]) -> Result<Vec<Value>> {
        let mut performance_risks = Vec::new();
        
        for code_file in generated_code {
            let filename = code_file["filename"].as_str().unwrap_or("unknown");
            let content = code_file["content"].as_str().unwrap_or("");
            
            // Check for performance anti-patterns
            
            // Nested loops
            let loop_depth = self.calculate_loop_depth(content);
            if loop_depth > 2 {
                performance_risks.push(json!({
                    "type": "nested_loops",
                    "severity": "medium",
                    "file": filename,
                    "description": format!("Deep nested loops (depth: {})", loop_depth),
                    "recommendation": "Consider optimizing algorithm complexity"
                }));
            }
            
            // Large file operations
            if content.contains("read()") && !content.contains("readline()") {
                performance_risks.push(json!({
                    "type": "large_file_read",
                    "severity": "low",
                    "file": filename,
                    "description": "Reading entire file into memory",
                    "recommendation": "Consider streaming large files"
                }));
            }
            
            // Inefficient string concatenation
            if content.matches(" + ").count() > 3 && content.contains("str") {
                performance_risks.push(json!({
                    "type": "string_concatenation",
                    "severity": "low",
                    "file": filename,
                    "description": "Multiple string concatenations",
                    "recommendation": "Use join() or f-strings for better performance"
                }));
            }
            
            // Database queries in loops
            if content.contains("for ") && content.contains("execute(") {
                performance_risks.push(json!({
                    "type": "n_plus_one_query",
                    "severity": "high",
                    "file": filename,
                    "description": "Potential N+1 query problem",
                    "recommendation": "Use batch queries or eager loading"
                }));
            }
        }
        
        Ok(performance_risks)
    }

    /// Calculate maximum loop nesting depth
    fn calculate_loop_depth(&self, content: &str) -> usize {
        let mut max_depth = 0;
        let mut current_depth = 0;
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("for ") || trimmed.starts_with("while ") {
                current_depth += 1;
                max_depth = max_depth.max(current_depth);
            } else if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
                current_depth = 0;
            }
            
            // Simple heuristic for end of blocks
            if line.len() > 0 && !line.starts_with(' ') && !line.starts_with('\t') {
                current_depth = 0;
            }
        }
        
        max_depth
    }

    /// Assess breaking changes
    async fn assess_breaking_changes(&self, goal: &str, generated_code: &[Value]) -> Result<Vec<Value>> {
        let mut breaking_changes = Vec::new();
        
        let goal_lower = goal.to_lowercase();
        
        // Check for API changes
        if goal_lower.contains("api") || goal_lower.contains("interface") {
            breaking_changes.push(json!({
                "type": "api_change",
                "severity": "high",
                "description": "Potential API interface changes",
                "affected_components": ["external_clients", "integrations"],
                "recommendation": "Ensure backward compatibility or provide migration guide"
            }));
        }
        
        // Check for database schema changes
        if goal_lower.contains("database") || goal_lower.contains("schema") {
            breaking_changes.push(json!({
                "type": "schema_change",
                "severity": "high",
                "description": "Potential database schema changes",
                "affected_components": ["data_persistence", "migrations"],
                "recommendation": "Create migration scripts and test thoroughly"
            }));
        }
        
        // Check for configuration changes
        for code_file in generated_code {
            let content = code_file["content"].as_str().unwrap_or("");
            
            if content.contains("config") || content.contains("settings") {
                breaking_changes.push(json!({
                    "type": "configuration_change",
                    "severity": "medium",
                    "description": "Configuration changes detected",
                    "affected_components": ["deployment", "environment_setup"],
                    "recommendation": "Update deployment documentation and environment configs"
                }));
                break;
            }
        }
        
        Ok(breaking_changes)
    }

    /// Assess operational risks
    async fn assess_operational_risks(&self, generated_code: &[Value]) -> Result<Vec<Value>> {
        let mut operational_risks = Vec::new();
        
        for code_file in generated_code {
            let filename = code_file["filename"].as_str().unwrap_or("unknown");
            let content = code_file["content"].as_str().unwrap_or("");
            
            // Check for missing error handling
            if !content.contains("try") && !content.contains("except") && 
               !content.contains("catch") && content.len() > 100 {
                operational_risks.push(json!({
                    "type": "missing_error_handling",
                    "severity": "medium",
                    "file": filename,
                    "description": "Missing error handling in substantial code",
                    "recommendation": "Add comprehensive error handling and logging"
                }));
            }
            
            // Check for missing logging
            if !content.contains("log") && !content.contains("print") && content.len() > 100 {
                operational_risks.push(json!({
                    "type": "missing_logging",
                    "severity": "low",
                    "file": filename,
                    "description": "No logging detected",
                    "recommendation": "Add appropriate logging for monitoring and debugging"
                }));
            }
            
            // Check for resource management
            if content.contains("open(") && !content.contains("with ") {
                operational_risks.push(json!({
                    "type": "resource_leak",
                    "severity": "medium",
                    "file": filename,
                    "description": "Potential resource leak with file operations",
                    "recommendation": "Use context managers (with statements) for resource management"
                }));
            }
        }
        
        Ok(operational_risks)
    }

    /// Calculate overall risk level
    fn calculate_overall_risk_level(&self, security_risks: &[Value], performance_risks: &[Value], breaking_changes: &[Value]) -> String {
        let high_security = security_risks.iter().any(|r| r["severity"] == "high");
        let high_performance = performance_risks.iter().any(|r| r["severity"] == "high");
        let high_breaking = breaking_changes.iter().any(|r| r["severity"] == "high");
        
        if high_security || high_breaking {
            "high".to_string()
        } else if high_performance || security_risks.len() > 2 || breaking_changes.len() > 1 {
            "medium".to_string()
        } else if security_risks.is_empty() && performance_risks.is_empty() && breaking_changes.is_empty() {
            "low".to_string()
        } else {
            "low".to_string()
        }
    }

    /// Generate recommendations based on risks
    async fn generate_recommendations(&self, security_risks: &[Value], performance_risks: &[Value], breaking_changes: &[Value]) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();
        
        // Security recommendations
        if !security_risks.is_empty() {
            recommendations.push("Conduct a thorough security review before deployment".to_string());
            recommendations.push("Run security scanning tools (SAST/DAST)".to_string());
            
            if security_risks.iter().any(|r| r["severity"] == "high") {
                recommendations.push("Address high-severity security issues immediately".to_string());
            }
        }
        
        // Performance recommendations
        if !performance_risks.is_empty() {
            recommendations.push("Conduct performance testing under expected load".to_string());
            recommendations.push("Monitor application performance metrics after deployment".to_string());
            
            if performance_risks.iter().any(|r| r["type"] == "n_plus_one_query") {
                recommendations.push("Optimize database queries to prevent performance degradation".to_string());
            }
        }
        
        // Breaking change recommendations
        if !breaking_changes.is_empty() {
            recommendations.push("Create comprehensive migration documentation".to_string());
            recommendations.push("Test backward compatibility thoroughly".to_string());
            recommendations.push("Plan phased rollout to minimize impact".to_string());
        }
        
        // General recommendations
        recommendations.push("Ensure comprehensive test coverage including edge cases".to_string());
        recommendations.push("Set up monitoring and alerting for the new functionality".to_string());
        recommendations.push("Document any new dependencies or configuration changes".to_string());
        
        Ok(recommendations)
    }
}