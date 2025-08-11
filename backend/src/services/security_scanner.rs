use anyhow::Result;
use crate::models::{SecurityIssue, SecuritySeverity};
use std::collections::HashMap;
use regex::Regex;

pub struct SecurityScanner {
    rules: HashMap<String, Vec<SecurityRule>>,
}

#[derive(Debug, Clone)]
struct SecurityRule {
    pattern: Regex,
    severity: SecuritySeverity,
    category: String,
    message: String,
    suggestion: Option<String>,
}

impl SecurityScanner {
    pub fn new() -> Self {
        let mut scanner = Self {
            rules: HashMap::new(),
        };
        scanner.load_default_rules();
        scanner
    }

    fn load_default_rules(&mut self) {
        // Python security rules
        let python_rules = vec![
            SecurityRule {
                pattern: Regex::new(r"eval\s*\(").unwrap(),
                severity: SecuritySeverity::Critical,
                category: "Code Injection".to_string(),
                message: "Use of eval() can lead to code injection vulnerabilities".to_string(),
                suggestion: Some("Consider using ast.literal_eval() for safe evaluation".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"exec\s*\(").unwrap(),
                severity: SecuritySeverity::Critical,
                category: "Code Injection".to_string(),
                message: "Use of exec() can lead to code injection vulnerabilities".to_string(),
                suggestion: Some("Avoid dynamic code execution".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"pickle\.loads?\s*\(").unwrap(),
                severity: SecuritySeverity::High,
                category: "Deserialization".to_string(),
                message: "Pickle deserialization can execute arbitrary code".to_string(),
                suggestion: Some("Use JSON or other safe serialization formats".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"subprocess\.call\s*\(.*shell\s*=\s*True").unwrap(),
                severity: SecuritySeverity::High,
                category: "Command Injection".to_string(),
                message: "Shell injection vulnerability in subprocess call".to_string(),
                suggestion: Some("Use shell=False and pass arguments as a list".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"random\.random\s*\(").unwrap(),
                severity: SecuritySeverity::Medium,
                category: "Weak Cryptography".to_string(),
                message: "random.random() is not cryptographically secure".to_string(),
                suggestion: Some("Use secrets module for cryptographic purposes".to_string()),
            },
        ];
        self.rules.insert("python".to_string(), python_rules);

        // JavaScript security rules
        let javascript_rules = vec![
            SecurityRule {
                pattern: Regex::new(r"eval\s*\(").unwrap(),
                severity: SecuritySeverity::Critical,
                category: "Code Injection".to_string(),
                message: "Use of eval() can lead to code injection vulnerabilities".to_string(),
                suggestion: Some("Use JSON.parse() for parsing JSON or avoid dynamic code execution".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"innerHTML\s*=").unwrap(),
                severity: SecuritySeverity::High,
                category: "XSS".to_string(),
                message: "Direct innerHTML assignment can lead to XSS vulnerabilities".to_string(),
                suggestion: Some("Use textContent or sanitize HTML content".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"document\.write\s*\(").unwrap(),
                severity: SecuritySeverity::High,
                category: "XSS".to_string(),
                message: "document.write() can lead to XSS vulnerabilities".to_string(),
                suggestion: Some("Use modern DOM manipulation methods".to_string()),
            },
            SecurityRule {
                pattern: Regex::new(r"Math\.random\s*\(").unwrap(),
                severity: SecuritySeverity::Medium,
                category: "Weak Cryptography".to_string(),
                message: "Math.random() is not cryptographically secure".to_string(),
                suggestion: Some("Use crypto.getRandomValues() for cryptographic purposes".to_string()),
            },
        ];
        self.rules.insert("javascript".to_string(), javascript_rules);

        // SQL injection patterns (generic)
        let sql_rules = vec![
            SecurityRule {
                pattern: Regex::new(r#"["'].*\+.*["']"#).unwrap(),
                severity: SecuritySeverity::Critical,
                category: "SQL Injection".to_string(),
                message: "Potential SQL injection vulnerability from string concatenation".to_string(),
                suggestion: Some("Use parameterized queries or prepared statements".to_string()),
            },
        ];
        self.rules.insert("sql".to_string(), sql_rules);
    }

    pub async fn scan_code(&self, code: &str, language: &str) -> Result<Vec<SecurityIssue>> {
        let mut issues = Vec::new();
        
        if let Some(rules) = self.rules.get(language) {
            let lines: Vec<&str> = code.lines().collect();
            
            for (line_num, line) in lines.iter().enumerate() {
                for rule in rules {
                    if let Some(mat) = rule.pattern.find(line) {
                        issues.push(SecurityIssue {
                            severity: rule.severity.clone(),
                            category: rule.category.clone(),
                            message: rule.message.clone(),
                            line: line_num + 1,
                            column: mat.start() + 1, // Exact column position
                            suggestion: rule.suggestion.clone(),
                        });
                    }
                }
            }
        }

        // Generic patterns that apply to all languages
        issues.extend(self.scan_generic_patterns(code).await?);
        
        Ok(issues)
    }

    async fn scan_generic_patterns(&self, code: &str) -> Result<Vec<SecurityIssue>> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        // Check for hardcoded secrets
        let secret_patterns = vec![
            (Regex::new(r"password\s*=\s*[\"'][^\"']+[\"']").unwrap(), "Hardcoded password"),
            (Regex::new(r"api_key\s*=\s*[\"'][^\"']+[\"']").unwrap(), "Hardcoded API key"),
            (Regex::new(r"secret\s*=\s*[\"'][^\"']+[\"']").unwrap(), "Hardcoded secret"),
            (Regex::new(r"token\s*=\s*[\"'][^\"']+[\"']").unwrap(), "Hardcoded token"),
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for (pattern, message) in &secret_patterns {
                if let Some(mat) = pattern.find(line) {
                    issues.push(SecurityIssue {
                        severity: SecuritySeverity::High,
                        category: "Hardcoded Secrets".to_string(),
                        message: format!("{} detected", message),
                        line: line_num + 1,
                        column: mat.start() + 1,
                        suggestion: Some("Use environment variables or secure configuration".to_string()),
                    });
                }
            }
        }

        Ok(issues)
    }

    pub fn add_custom_rule(&mut self, language: &str, rule: SecurityRule) {
        self.rules.entry(language.to_string()).or_insert_with(Vec::new).push(rule);
    }
}