use super::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnalyzer {
    semgrep_enabled: bool,
    codeql_enabled: bool,
    custom_rules_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnalysisRequest {
    pub code: String,
    pub language: String,
    pub file_path: Option<String>,
    pub analysis_types: Vec<SecurityAnalysisType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAnalysisType {
    StaticAnalysis,
    VulnerabilityDetection,
    ComplianceCheck,
    SecretDetection,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnalysisResponse {
    pub findings: Vec<SecurityFinding>,
    pub risk_score: f32,
    pub compliance_status: ComplianceStatus,
    pub recommendations: Vec<SecurityRecommendation>,
    pub analysis_metadata: AnalysisMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: SecuritySeverity,
    pub category: SecurityCategory,
    pub title: String,
    pub description: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub column_number: Option<usize>,
    pub code_snippet: Option<String>,
    pub cwe_id: Option<String>,
    pub owasp_category: Option<String>,
    pub fix_suggestion: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityCategory {
    Injection,
    Authentication,
    Authorization,
    DataExposure,
    Cryptography,
    InputValidation,
    OutputEncoding,
    ErrorHandling,
    Logging,
    Configuration,
    Dependencies,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub owasp_top_10_compliant: bool,
    pub sans_top_25_compliant: bool,
    pub custom_rules_compliant: bool,
    pub violations: Vec<ComplianceViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub rule_id: String,
    pub description: String,
    pub severity: SecuritySeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Immediate,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub analysis_time_ms: u64,
    pub tools_used: Vec<String>,
    pub rules_applied: usize,
    pub code_lines_analyzed: usize,
}

impl SecurityAnalyzer {
    pub fn new() -> Self {
        Self {
            semgrep_enabled: true,
            codeql_enabled: false, // Requires setup
            custom_rules_path: None,
        }
    }

    pub fn with_custom_rules(mut self, rules_path: PathBuf) -> Self {
        self.custom_rules_path = Some(rules_path);
        self
    }

    pub async fn analyze_security(&self, request: &SecurityAnalysisRequest) -> Result<SecurityAnalysisResponse> {
        let start_time = Instant::now();
        info!("Starting security analysis for {} code", request.language);

        let mut findings = Vec::new();
        let mut tools_used = Vec::new();

        // Run Semgrep analysis
        if self.semgrep_enabled {
            match self.run_semgrep_analysis(request).await {
                Ok(mut semgrep_findings) => {
                    findings.append(&mut semgrep_findings);
                    tools_used.push("semgrep".to_string());
                }
                Err(e) => {
                    warn!("Semgrep analysis failed: {}", e);
                }
            }
        }

        // Run CodeQL analysis (if enabled)
        if self.codeql_enabled {
            match self.run_codeql_analysis(request).await {
                Ok(mut codeql_findings) => {
                    findings.append(&mut codeql_findings);
                    tools_used.push("codeql".to_string());
                }
                Err(e) => {
                    warn!("CodeQL analysis failed: {}", e);
                }
            }
        }

        // Run built-in security checks
        let mut builtin_findings = self.run_builtin_security_checks(request).await?;
        findings.append(&mut builtin_findings);
        tools_used.push("builtin".to_string());

        // Calculate risk score
        let risk_score = self.calculate_risk_score(&findings);

        // Check compliance
        let compliance_status = self.check_compliance(&findings);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&findings);

        let analysis_time = start_time.elapsed().as_millis() as u64;
        let code_lines = request.code.lines().count();

        Ok(SecurityAnalysisResponse {
            findings,
            risk_score,
            compliance_status,
            recommendations,
            analysis_metadata: AnalysisMetadata {
                analysis_time_ms: analysis_time,
                tools_used,
                rules_applied: 50, // Placeholder
                code_lines_analyzed: code_lines,
            },
        })
    }

    async fn run_semgrep_analysis(&self, request: &SecurityAnalysisRequest) -> Result<Vec<SecurityFinding>> {
        debug!("Running Semgrep analysis");

        // Create temporary file for analysis
        let temp_dir = std::env::temp_dir().join(format!("semgrep_{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        let file_extension = match request.language.as_str() {
            "python" => "py",
            "javascript" => "js",
            "typescript" => "ts",
            "java" => "java",
            "go" => "go",
            "rust" => "rs",
            _ => "txt",
        };

        let file_path = temp_dir.join(format!("code.{}", file_extension));
        tokio::fs::write(&file_path, &request.code).await?;

        // Run Semgrep
        let output = Command::new("semgrep")
            .arg("--config=auto")
            .arg("--json")
            .arg("--no-git-ignore")
            .arg(&file_path)
            .output()
            .await;

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_semgrep_output(&stdout)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Semgrep failed: {}", stderr);
                Ok(Vec::new())
            }
            Err(e) => {
                debug!("Semgrep not available: {}", e);
                Ok(Vec::new())
            }
        }
    }

    fn parse_semgrep_output(&self, output: &str) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        if let Ok(semgrep_result) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(results) = semgrep_result.get("results").and_then(|r| r.as_array()) {
                for result in results {
                    if let Some(finding) = self.parse_semgrep_finding(result) {
                        findings.push(finding);
                    }
                }
            }
        }

        Ok(findings)
    }

    fn parse_semgrep_finding(&self, result: &serde_json::Value) -> Option<SecurityFinding> {
        let check_id = result.get("check_id")?.as_str()?;
        let message = result.get("message")?.as_str()?;
        let severity = result.get("extra")?.get("severity")?.as_str().unwrap_or("INFO");
        let line = result.get("start")?.get("line")?.as_u64()? as usize;
        let column = result.get("start")?.get("col")?.as_u64()? as usize;

        let security_severity = match severity.to_uppercase().as_str() {
            "ERROR" => SecuritySeverity::High,
            "WARNING" => SecuritySeverity::Medium,
            "INFO" => SecuritySeverity::Low,
            _ => SecuritySeverity::Low,
        };

        let category = self.categorize_security_finding(check_id);

        Some(SecurityFinding {
            id: check_id.to_string(),
            severity: security_severity,
            category,
            title: check_id.to_string(),
            description: message.to_string(),
            file_path: "analyzed_code".to_string(),
            line_number: Some(line),
            column_number: Some(column),
            code_snippet: None,
            cwe_id: None,
            owasp_category: None,
            fix_suggestion: None,
            confidence: 0.8,
        })
    }

    async fn run_codeql_analysis(&self, request: &SecurityAnalysisRequest) -> Result<Vec<SecurityFinding>> {
        debug!("Running CodeQL analysis");
        // CodeQL implementation would go here
        // For now, return empty results
        Ok(Vec::new())
    }

    async fn run_builtin_security_checks(&self, request: &SecurityAnalysisRequest) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        match request.language.as_str() {
            "python" => {
                findings.extend(self.check_python_security(&request.code));
            }
            "javascript" | "typescript" => {
                findings.extend(self.check_javascript_security(&request.code));
            }
            "java" => {
                findings.extend(self.check_java_security(&request.code));
            }
            _ => {
                findings.extend(self.check_generic_security(&request.code));
            }
        }

        Ok(findings)
    }

    fn check_python_security(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for eval() usage
        if code.contains("eval(") {
            findings.push(SecurityFinding {
                id: "python-eval-injection".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::Injection,
                title: "Code Injection via eval()".to_string(),
                description: "Use of eval() can lead to arbitrary code execution".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "eval("),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-94".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use ast.literal_eval() for safe evaluation or avoid eval() entirely".to_string()),
                confidence: 0.95,
            });
        }

        // Check for shell injection
        if code.contains("shell=True") {
            findings.push(SecurityFinding {
                id: "python-shell-injection".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Injection,
                title: "Command Injection via shell=True".to_string(),
                description: "Using shell=True can lead to command injection vulnerabilities".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "shell=True"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-78".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use shell=False and pass arguments as a list".to_string()),
                confidence: 0.9,
            });
        }

        // Check for hardcoded secrets
        if code.to_lowercase().contains("password") && (code.contains("=") || code.contains(":")) {
            findings.push(SecurityFinding {
                id: "python-hardcoded-secret".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::DataExposure,
                title: "Hardcoded Secret Detected".to_string(),
                description: "Potential hardcoded password or secret found".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "password"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-798".to_string()),
                owasp_category: Some("A07:2021 - Identification and Authentication Failures".to_string()),
                fix_suggestion: Some("Use environment variables or secure credential storage".to_string()),
                confidence: 0.7,
            });
        }

        findings
    }

    fn check_javascript_security(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for eval() usage
        if code.contains("eval(") {
            findings.push(SecurityFinding {
                id: "js-eval-injection".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::Injection,
                title: "Code Injection via eval()".to_string(),
                description: "Use of eval() can lead to arbitrary code execution".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "eval("),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-94".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use JSON.parse() for data or avoid eval() entirely".to_string()),
                confidence: 0.95,
            });
        }

        // Check for innerHTML usage
        if code.contains("innerHTML") {
            findings.push(SecurityFinding {
                id: "js-xss-innerhtml".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::Injection,
                title: "Potential XSS via innerHTML".to_string(),
                description: "Using innerHTML with user data can lead to XSS vulnerabilities".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "innerHTML"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-79".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use textContent or properly sanitize HTML content".to_string()),
                confidence: 0.8,
            });
        }

        findings
    }

    fn check_java_security(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for SQL injection patterns
        if code.contains("Statement") && code.contains("executeQuery") {
            findings.push(SecurityFinding {
                id: "java-sql-injection".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Injection,
                title: "Potential SQL Injection".to_string(),
                description: "Direct SQL query execution may be vulnerable to injection".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "executeQuery"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-89".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use PreparedStatement with parameterized queries".to_string()),
                confidence: 0.8,
            });
        }

        findings
    }

    fn check_generic_security(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for common security patterns across languages
        let patterns = [
            ("password", "Potential hardcoded credential"),
            ("secret", "Potential hardcoded secret"),
            ("api_key", "Potential hardcoded API key"),
            ("token", "Potential hardcoded token"),
        ];

        for (pattern, description) in patterns {
            if code.to_lowercase().contains(pattern) && code.contains("=") {
                findings.push(SecurityFinding {
                    id: format!("generic-hardcoded-{}", pattern),
                    severity: SecuritySeverity::Medium,
                    category: SecurityCategory::DataExposure,
                    title: "Potential Hardcoded Secret".to_string(),
                    description: description.to_string(),
                    file_path: "analyzed_code".to_string(),
                    line_number: self.find_line_number(code, pattern),
                    column_number: None,
                    code_snippet: None,
                    cwe_id: Some("CWE-798".to_string()),
                    owasp_category: Some("A07:2021 - Identification and Authentication Failures".to_string()),
                    fix_suggestion: Some("Use environment variables or secure credential storage".to_string()),
                    confidence: 0.6,
                });
            }
        }

        findings
    }

    fn find_line_number(&self, code: &str, pattern: &str) -> Option<usize> {
        code.lines()
            .enumerate()
            .find(|(_, line)| line.to_lowercase().contains(&pattern.to_lowercase()))
            .map(|(i, _)| i + 1)
    }

    fn categorize_security_finding(&self, check_id: &str) -> SecurityCategory {
        if check_id.contains("injection") || check_id.contains("sql") || check_id.contains("xss") {
            SecurityCategory::Injection
        } else if check_id.contains("auth") {
            SecurityCategory::Authentication
        } else if check_id.contains("crypto") {
            SecurityCategory::Cryptography
        } else if check_id.contains("input") {
            SecurityCategory::InputValidation
        } else {
            SecurityCategory::Other(check_id.to_string())
        }
    }

    fn calculate_risk_score(&self, findings: &[SecurityFinding]) -> f32 {
        let mut score = 0.0;
        
        for finding in findings {
            let severity_weight = match finding.severity {
                SecuritySeverity::Critical => 10.0,
                SecuritySeverity::High => 7.0,
                SecuritySeverity::Medium => 4.0,
                SecuritySeverity::Low => 2.0,
                SecuritySeverity::Info => 1.0,
            };
            
            score += severity_weight * finding.confidence;
        }

        // Normalize to 0-1 scale
        (score / 100.0).min(1.0)
    }

    fn check_compliance(&self, findings: &[SecurityFinding]) -> ComplianceStatus {
        let mut violations = Vec::new();
        
        // Check for critical security issues that violate compliance
        for finding in findings {
            if matches!(finding.severity, SecuritySeverity::Critical | SecuritySeverity::High) {
                violations.push(ComplianceViolation {
                    rule_id: finding.id.clone(),
                    description: finding.description.clone(),
                    severity: finding.severity.clone(),
                });
            }
        }

        ComplianceStatus {
            owasp_top_10_compliant: violations.is_empty(),
            sans_top_25_compliant: violations.is_empty(),
            custom_rules_compliant: true,
            violations,
        }
    }

    fn generate_recommendations(&self, findings: &[SecurityFinding]) -> Vec<SecurityRecommendation> {
        let mut recommendations = Vec::new();

        if findings.iter().any(|f| matches!(f.severity, SecuritySeverity::Critical)) {
            recommendations.push(SecurityRecommendation {
                priority: RecommendationPriority::Immediate,
                title: "Address Critical Security Vulnerabilities".to_string(),
                description: "Critical security issues found that require immediate attention".to_string(),
                action_items: vec![
                    "Review and fix all critical security findings".to_string(),
                    "Implement security testing in CI/CD pipeline".to_string(),
                    "Conduct security code review".to_string(),
                ],
                references: vec![
                    "https://owasp.org/www-project-top-ten/".to_string(),
                    "https://cwe.mitre.org/".to_string(),
                ],
            });
        }

        if findings.len() > 5 {
            recommendations.push(SecurityRecommendation {
                priority: RecommendationPriority::High,
                title: "Implement Comprehensive Security Review".to_string(),
                description: "Multiple security issues detected, suggesting need for systematic review".to_string(),
                action_items: vec![
                    "Conduct thorough security audit".to_string(),
                    "Implement security training for development team".to_string(),
                    "Establish secure coding guidelines".to_string(),
                ],
                references: vec![
                    "https://owasp.org/www-project-code-review-guide/".to_string(),
                ],
            });
        }

        recommendations
    }

    pub async fn should_block_patch(&self, analysis: &SecurityAnalysisResponse) -> bool {
        // Block patch if there are critical security findings
        analysis.findings.iter().any(|f| matches!(f.severity, SecuritySeverity::Critical))
    }
}