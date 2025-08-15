use super::*;
use crate::agents::security_analyzer::*;
use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct MultiLanguageSecurityAnalyzer {
    base_analyzer: SecurityAnalyzer,
}

impl MultiLanguageSecurityAnalyzer {
    pub fn new() -> Self {
        Self {
            base_analyzer: SecurityAnalyzer::new(),
        }
    }

    pub async fn analyze_go_security(&self, code: &str) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        // Go-specific security checks
        findings.extend(self.check_go_sql_injection(code));
        findings.extend(self.check_go_command_injection(code));
        findings.extend(self.check_go_path_traversal(code));
        findings.extend(self.check_go_unsafe_operations(code));
        findings.extend(self.check_go_crypto_issues(code));
        findings.extend(self.check_go_race_conditions(code));
        findings.extend(self.check_go_memory_issues(code));

        Ok(findings)
    }

    pub async fn analyze_rust_security(&self, code: &str) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        // Rust-specific security checks
        findings.extend(self.check_rust_unsafe_blocks(code));
        findings.extend(self.check_rust_panic_conditions(code));
        findings.extend(self.check_rust_integer_overflow(code));
        findings.extend(self.check_rust_memory_safety(code));
        findings.extend(self.check_rust_crypto_issues(code));
        findings.extend(self.check_rust_deserialization(code));
        findings.extend(self.check_rust_ffi_safety(code));

        Ok(findings)
    }

    // Go Security Checks
    fn check_go_sql_injection(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for string concatenation in SQL queries
        if code.contains("fmt.Sprintf") && (code.contains("SELECT") || code.contains("INSERT") || code.contains("UPDATE")) {
            findings.push(SecurityFinding {
                id: "go-sql-injection-sprintf".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Injection,
                title: "SQL Injection via fmt.Sprintf".to_string(),
                description: "Using fmt.Sprintf to build SQL queries can lead to SQL injection".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "fmt.Sprintf"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-89".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use parameterized queries with database/sql package".to_string()),
                confidence: 0.9,
            });
        }

        // Check for direct string concatenation in queries
        if code.contains("\"SELECT") && code.contains("+") {
            findings.push(SecurityFinding {
                id: "go-sql-injection-concat".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Injection,
                title: "SQL Injection via String Concatenation".to_string(),
                description: "String concatenation in SQL queries can lead to injection attacks".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "\"SELECT"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-89".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Use prepared statements with placeholders".to_string()),
                confidence: 0.8,
            });
        }

        findings
    }

    fn check_go_command_injection(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for exec.Command with user input
        if code.contains("exec.Command") && (code.contains("fmt.Sprintf") || code.contains("+")) {
            findings.push(SecurityFinding {
                id: "go-command-injection".to_string(),
                severity: SecuritySeverity::Critical,
                category: SecurityCategory::Injection,
                title: "Command Injection via exec.Command".to_string(),
                description: "Using user input in exec.Command can lead to command injection".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "exec.Command"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-78".to_string()),
                owasp_category: Some("A03:2021 - Injection".to_string()),
                fix_suggestion: Some("Validate and sanitize input, use exec.CommandContext with separate arguments".to_string()),
                confidence: 0.85,
            });
        }

        findings
    }

    fn check_go_path_traversal(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for filepath.Join with user input without validation
        if code.contains("filepath.Join") && !code.contains("filepath.Clean") {
            findings.push(SecurityFinding {
                id: "go-path-traversal".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::InputValidation,
                title: "Potential Path Traversal".to_string(),
                description: "Using filepath.Join without validation can lead to path traversal attacks".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "filepath.Join"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-22".to_string()),
                owasp_category: Some("A01:2021 - Broken Access Control".to_string()),
                fix_suggestion: Some("Use filepath.Clean and validate paths against allowed directories".to_string()),
                confidence: 0.6,
            });
        }

        findings
    }

    fn check_go_unsafe_operations(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for unsafe package usage
        if code.contains("import \"unsafe\"") || code.contains("unsafe.") {
            findings.push(SecurityFinding {
                id: "go-unsafe-usage".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Other("Memory Safety".to_string()),
                title: "Unsafe Package Usage".to_string(),
                description: "Using unsafe package can lead to memory safety issues".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "unsafe"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-119".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Avoid unsafe operations or ensure proper bounds checking".to_string()),
                confidence: 0.9,
            });
        }

        findings
    }

    fn check_go_crypto_issues(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for weak crypto algorithms
        if code.contains("md5.") || code.contains("sha1.") {
            findings.push(SecurityFinding {
                id: "go-weak-crypto".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::Cryptography,
                title: "Weak Cryptographic Algorithm".to_string(),
                description: "MD5 and SHA1 are cryptographically weak".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "md5.").or_else(|| self.find_line_number(code, "sha1.")),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-327".to_string()),
                owasp_category: Some("A02:2021 - Cryptographic Failures".to_string()),
                fix_suggestion: Some("Use SHA-256 or stronger algorithms".to_string()),
                confidence: 0.9,
            });
        }

        findings
    }

    fn check_go_race_conditions(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for potential race conditions
        if code.contains("go func") && !code.contains("sync.Mutex") && !code.contains("sync.RWMutex") {
            findings.push(SecurityFinding {
                id: "go-race-condition".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::Other("Concurrency".to_string()),
                title: "Potential Race Condition".to_string(),
                description: "Goroutines without synchronization can lead to race conditions".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "go func"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-362".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Use sync.Mutex, channels, or other synchronization primitives".to_string()),
                confidence: 0.5,
            });
        }

        findings
    }

    fn check_go_memory_issues(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for potential memory leaks with goroutines
        if code.contains("for {") && code.contains("go func") {
            findings.push(SecurityFinding {
                id: "go-goroutine-leak".to_string(),
                severity: SecuritySeverity::Low,
                category: SecurityCategory::Other("Resource Management".to_string()),
                title: "Potential Goroutine Leak".to_string(),
                description: "Infinite loops with goroutines can cause memory leaks".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "for {"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-401".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Use context.Context for cancellation and proper cleanup".to_string()),
                confidence: 0.6,
            });
        }

        findings
    }

    // Rust Security Checks
    fn check_rust_unsafe_blocks(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        if code.contains("unsafe {") || code.contains("unsafe fn") {
            findings.push(SecurityFinding {
                id: "rust-unsafe-block".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Other("Memory Safety".to_string()),
                title: "Unsafe Block Usage".to_string(),
                description: "Unsafe blocks bypass Rust's safety guarantees".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "unsafe"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-119".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Minimize unsafe usage and ensure proper invariants are maintained".to_string()),
                confidence: 0.9,
            });
        }

        findings
    }

    fn check_rust_panic_conditions(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for unwrap() usage which can panic
        if code.contains(".unwrap()") {
            findings.push(SecurityFinding {
                id: "rust-unwrap-panic".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::ErrorHandling,
                title: "Potential Panic with unwrap()".to_string(),
                description: "Using unwrap() can cause panics on None/Err values".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, ".unwrap()"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-248".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Use proper error handling with match, if let, or expect()".to_string()),
                confidence: 0.7,
            });
        }

        // Check for expect() with generic messages
        if code.contains(".expect(\"") {
            findings.push(SecurityFinding {
                id: "rust-expect-usage".to_string(),
                severity: SecuritySeverity::Low,
                category: SecurityCategory::ErrorHandling,
                title: "expect() Usage".to_string(),
                description: "expect() can cause panics and may leak information".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, ".expect("),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-248".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Use proper error propagation with ? operator".to_string()),
                confidence: 0.5,
            });
        }

        findings
    }

    fn check_rust_integer_overflow(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for potential integer overflow in arithmetic operations
        if code.contains("wrapping_add") || code.contains("wrapping_sub") || code.contains("wrapping_mul") {
            findings.push(SecurityFinding {
                id: "rust-integer-overflow".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::Other("Integer Overflow".to_string()),
                title: "Explicit Integer Overflow Handling".to_string(),
                description: "Wrapping arithmetic operations may hide overflow issues".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "wrapping_"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-190".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Use checked arithmetic or saturating operations".to_string()),
                confidence: 0.6,
            });
        }

        findings
    }

    fn check_rust_memory_safety(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for raw pointer usage
        if code.contains("*const") || code.contains("*mut") {
            findings.push(SecurityFinding {
                id: "rust-raw-pointer".to_string(),
                severity: SecuritySeverity::High,
                category: SecurityCategory::Other("Memory Safety".to_string()),
                title: "Raw Pointer Usage".to_string(),
                description: "Raw pointers bypass Rust's ownership system".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "*const").or_else(|| self.find_line_number(code, "*mut")),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-119".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Use safe references or smart pointers when possible".to_string()),
                confidence: 0.8,
            });
        }

        findings
    }

    fn check_rust_crypto_issues(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for weak random number generation
        if code.contains("rand::random") && !code.contains("rand::rngs::OsRng") {
            findings.push(SecurityFinding {
                id: "rust-weak-rng".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::Cryptography,
                title: "Weak Random Number Generation".to_string(),
                description: "Using non-cryptographic RNG for security purposes".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "rand::random"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-338".to_string()),
                owasp_category: Some("A02:2021 - Cryptographic Failures".to_string()),
                fix_suggestion: Some("Use OsRng or other cryptographically secure RNG".to_string()),
                confidence: 0.8,
            });
        }

        findings
    }

    fn check_rust_deserialization(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for unsafe deserialization
        if code.contains("serde_json::from_str") && !code.contains("deserialize_with") {
            findings.push(SecurityFinding {
                id: "rust-unsafe-deserialization".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::InputValidation,
                title: "Potentially Unsafe Deserialization".to_string(),
                description: "Deserializing untrusted data without validation".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "serde_json::from_str"),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-502".to_string()),
                owasp_category: Some("A08:2021 - Software and Data Integrity Failures".to_string()),
                fix_suggestion: Some("Validate input and use custom deserializers for untrusted data".to_string()),
                confidence: 0.6,
            });
        }

        findings
    }

    fn check_rust_ffi_safety(&self, code: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // Check for FFI usage
        if code.contains("extern \"C\"") || code.contains("std::ffi::") {
            findings.push(SecurityFinding {
                id: "rust-ffi-usage".to_string(),
                severity: SecuritySeverity::Medium,
                category: SecurityCategory::Other("FFI Safety".to_string()),
                title: "Foreign Function Interface Usage".to_string(),
                description: "FFI calls can bypass Rust's safety guarantees".to_string(),
                file_path: "analyzed_code".to_string(),
                line_number: self.find_line_number(code, "extern \"C\"").or_else(|| self.find_line_number(code, "std::ffi::")),
                column_number: None,
                code_snippet: None,
                cwe_id: Some("CWE-119".to_string()),
                owasp_category: None,
                fix_suggestion: Some("Ensure proper validation of FFI inputs and outputs".to_string()),
                confidence: 0.7,
            });
        }

        findings
    }

    fn find_line_number(&self, code: &str, pattern: &str) -> Option<usize> {
        code.lines()
            .enumerate()
            .find(|(_, line)| line.contains(pattern))
            .map(|(i, _)| i + 1)
    }
}

// Integration with existing SecurityAnalyzer
impl SecurityAnalyzer {
    pub async fn analyze_multi_language_security(&self, request: &SecurityAnalysisRequest) -> Result<SecurityAnalysisResponse> {
        let multi_lang_analyzer = MultiLanguageSecurityAnalyzer::new();
        let mut all_findings = Vec::new();

        // Run base analysis
        let base_response = self.analyze_security(request).await?;
        all_findings.extend(base_response.findings);

        // Add language-specific analysis
        match request.language.as_str() {
            "go" => {
                let go_findings = multi_lang_analyzer.analyze_go_security(&request.code).await?;
                all_findings.extend(go_findings);
            }
            "rust" => {
                let rust_findings = multi_lang_analyzer.analyze_rust_security(&request.code).await?;
                all_findings.extend(rust_findings);
            }
            _ => {
                // Use base analysis for other languages
            }
        }

        // Recalculate risk score with additional findings
        let risk_score = self.calculate_risk_score(&all_findings);
        let compliance_status = self.check_compliance(&all_findings);
        let recommendations = self.generate_recommendations(&all_findings);

        Ok(SecurityAnalysisResponse {
            findings: all_findings,
            risk_score,
            compliance_status,
            recommendations,
            analysis_metadata: base_response.analysis_metadata,
        })
    }
}