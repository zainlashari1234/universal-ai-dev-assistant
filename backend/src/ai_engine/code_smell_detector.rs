use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSmell {
    pub smell_type: CodeSmellType,
    pub severity: SmellSeverity,
    pub location: CodeLocation,
    pub description: String,
    pub refactoring_suggestion: String,
    pub estimated_effort: RefactoringEffort,
    pub impact_score: f64,
    pub examples: Vec<RefactoringExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeSmellType {
    // Method-level smells
    LongMethod,
    LongParameterList,
    DuplicatedCode,
    LargeClass,
    
    // Design smells
    FeatureEnvy,
    DataClumps,
    PrimitiveObsession,
    SwitchStatements,
    
    // Architecture smells
    GodClass,
    CircularDependency,
    TightCoupling,
    LowCohesion,
    
    // Naming smells
    UnclearNaming,
    InconsistentNaming,
    AbbreviationAbuse,
    
    // Performance smells
    InefficientAlgorithm,
    MemoryLeak,
    UnusedCode,
    PrematureOptimization,
    
    // Security smells
    HardcodedSecrets,
    WeakCryptography,
    InputValidationMissing,
    
    // Maintainability smells
    MagicNumbers,
    DeepNesting,
    CommentedCode,
    InconsistentFormatting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmellSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub function_name: Option<String>,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringEffort {
    Trivial,      // < 30 minutes
    Easy,         // 30 minutes - 2 hours
    Medium,       // 2 hours - 1 day
    Hard,         // 1 day - 1 week
    VeryHard,     // > 1 week
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringExample {
    pub before: String,
    pub after: String,
    pub explanation: String,
}

pub struct CodeSmellDetector {
    smell_patterns: HashMap<CodeSmellType, SmellPattern>,
    language_specific_rules: HashMap<String, Vec<LanguageRule>>,
}

#[derive(Debug, Clone)]
struct SmellPattern {
    detection_rules: Vec<DetectionRule>,
    severity_calculator: fn(&DetectionContext) -> SmellSeverity,
    refactoring_templates: Vec<RefactoringTemplate>,
}

#[derive(Debug, Clone)]
struct DetectionRule {
    pattern: String,
    threshold: Option<f64>,
    context_required: Vec<String>,
}

#[derive(Debug, Clone)]
struct DetectionContext {
    code: String,
    metrics: CodeMetrics,
    language: String,
    file_path: String,
}

#[derive(Debug, Clone)]
struct CodeMetrics {
    lines_of_code: u32,
    cyclomatic_complexity: u32,
    parameter_count: u32,
    nesting_depth: u32,
    method_count: u32,
    class_size: u32,
}

#[derive(Debug, Clone)]
struct LanguageRule {
    rule_type: String,
    pattern: String,
    severity: SmellSeverity,
}

#[derive(Debug, Clone)]
struct RefactoringTemplate {
    name: String,
    description: String,
    steps: Vec<String>,
    example: RefactoringExample,
}

impl CodeSmellDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            smell_patterns: HashMap::new(),
            language_specific_rules: HashMap::new(),
        };
        
        detector.initialize_patterns();
        detector.initialize_language_rules();
        detector
    }

    pub async fn detect_smells(&self, code: &str, language: &str, file_path: &str) -> Result<Vec<CodeSmell>> {
        let metrics = self.calculate_metrics(code, language)?;
        let context = DetectionContext {
            code: code.to_string(),
            metrics,
            language: language.to_string(),
            file_path: file_path.to_string(),
        };

        let mut smells = Vec::new();

        // Detect each type of smell
        for (smell_type, pattern) in &self.smell_patterns {
            if let Some(smell) = self.detect_specific_smell(smell_type, pattern, &context).await? {
                smells.push(smell);
            }
        }

        // Apply language-specific rules
        if let Some(rules) = self.language_specific_rules.get(language) {
            for rule in rules {
                if let Some(smell) = self.apply_language_rule(rule, &context).await? {
                    smells.push(smell);
                }
            }
        }

        // Sort by severity and impact
        smells.sort_by(|a, b| {
            let severity_order = |s: &SmellSeverity| match s {
                SmellSeverity::Critical => 5,
                SmellSeverity::High => 4,
                SmellSeverity::Medium => 3,
                SmellSeverity::Low => 2,
                SmellSeverity::Info => 1,
            };
            
            severity_order(&b.severity).cmp(&severity_order(&a.severity))
                .then(b.impact_score.partial_cmp(&a.impact_score).unwrap_or(std::cmp::Ordering::Equal))
        });

        Ok(smells)
    }

    async fn detect_specific_smell(
        &self,
        smell_type: &CodeSmellType,
        pattern: &SmellPattern,
        context: &DetectionContext,
    ) -> Result<Option<CodeSmell>> {
        // Check if any detection rules match
        for rule in &pattern.detection_rules {
            if self.rule_matches(rule, context) {
                let severity = (pattern.severity_calculator)(context);
                let location = self.find_smell_location(context, rule)?;
                
                let smell = CodeSmell {
                    smell_type: smell_type.clone(),
                    severity,
                    location,
                    description: self.generate_description(smell_type, context),
                    refactoring_suggestion: self.generate_refactoring_suggestion(smell_type, context),
                    estimated_effort: self.estimate_refactoring_effort(smell_type, context),
                    impact_score: self.calculate_impact_score(smell_type, context),
                    examples: self.get_refactoring_examples(smell_type),
                };
                
                return Ok(Some(smell));
            }
        }
        
        Ok(None)
    }

    fn rule_matches(&self, rule: &DetectionRule, context: &DetectionContext) -> bool {
        match rule.pattern.as_str() {
            "long_method" => {
                if let Some(threshold) = rule.threshold {
                    context.metrics.lines_of_code as f64 > threshold
                } else {
                    context.metrics.lines_of_code > 50 // Default threshold
                }
            }
            "high_complexity" => {
                if let Some(threshold) = rule.threshold {
                    context.metrics.cyclomatic_complexity as f64 > threshold
                } else {
                    context.metrics.cyclomatic_complexity > 10
                }
            }
            "too_many_parameters" => {
                if let Some(threshold) = rule.threshold {
                    context.metrics.parameter_count as f64 > threshold
                } else {
                    context.metrics.parameter_count > 5
                }
            }
            "deep_nesting" => {
                if let Some(threshold) = rule.threshold {
                    context.metrics.nesting_depth as f64 > threshold
                } else {
                    context.metrics.nesting_depth > 4
                }
            }
            "large_class" => {
                if let Some(threshold) = rule.threshold {
                    context.metrics.class_size as f64 > threshold
                } else {
                    context.metrics.class_size > 500
                }
            }
            "duplicated_code" => {
                self.detect_code_duplication(&context.code)
            }
            "magic_numbers" => {
                self.detect_magic_numbers(&context.code)
            }
            "hardcoded_secrets" => {
                self.detect_hardcoded_secrets(&context.code)
            }
            "unused_code" => {
                self.detect_unused_code(&context.code)
            }
            _ => false,
        }
    }

    fn detect_code_duplication(&self, code: &str) -> bool {
        let lines: Vec<&str> = code.lines().collect();
        let mut line_counts = HashMap::new();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.len() > 10 && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                *line_counts.entry(trimmed).or_insert(0) += 1;
            }
        }
        
        line_counts.values().any(|&count| count > 2)
    }

    fn detect_magic_numbers(&self, code: &str) -> bool {
        use regex::Regex;
        let number_regex = Regex::new(r"\b\d{2,}\b").unwrap();
        
        // Count numeric literals (excluding common ones like 0, 1, 2, etc.)
        let magic_numbers: Vec<_> = number_regex
            .find_iter(code)
            .filter(|m| {
                let num = m.as_str();
                !matches!(num, "0" | "1" | "2" | "10" | "100" | "1000")
            })
            .collect();
            
        magic_numbers.len() > 3
    }

    fn detect_hardcoded_secrets(&self, code: &str) -> bool {
        let secret_patterns = [
            "password", "secret", "key", "token", "api_key",
            "private_key", "access_token", "auth_token"
        ];
        
        for pattern in &secret_patterns {
            if code.to_lowercase().contains(pattern) && 
               (code.contains("=") || code.contains(":")) {
                return true;
            }
        }
        
        false
    }

    fn detect_unused_code(&self, code: &str) -> bool {
        // Simple heuristic: look for functions/variables that are defined but never used
        let lines: Vec<&str> = code.lines().collect();
        let mut definitions = Vec::new();
        let mut usages = Vec::new();
        
        for line in lines {
            if line.contains("def ") || line.contains("function ") || line.contains("fn ") {
                if let Some(name) = self.extract_function_name(line) {
                    definitions.push(name);
                }
            } else {
                // Count function calls
                for def in &definitions {
                    if line.contains(def) && !line.contains("def ") {
                        usages.push(def.clone());
                    }
                }
            }
        }
        
        // Check if any definitions are never used
        definitions.iter().any(|def| !usages.contains(def))
    }

    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple extraction - would be more sophisticated in real implementation
        if let Some(start) = line.find("def ") {
            let after_def = &line[start + 4..];
            if let Some(end) = after_def.find('(') {
                return Some(after_def[..end].trim().to_string());
            }
        }
        None
    }

    fn calculate_metrics(&self, code: &str, _language: &str) -> Result<CodeMetrics> {
        let lines: Vec<&str> = code.lines().collect();
        let lines_of_code = lines.len() as u32;
        
        // Calculate cyclomatic complexity
        let mut complexity = 1;
        for line in &lines {
            if line.contains("if ") || line.contains("while ") || 
               line.contains("for ") || line.contains("case ") ||
               line.contains("&&") || line.contains("||") {
                complexity += 1;
            }
        }
        
        // Calculate nesting depth
        let mut max_depth = 0;
        let mut current_depth = 0;
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.ends_with('{') || trimmed.ends_with(':') {
                current_depth += 1;
                max_depth = max_depth.max(current_depth);
            } else if trimmed.starts_with('}') {
                current_depth = current_depth.saturating_sub(1);
            }
        }
        
        // Count parameters (simplified)
        let parameter_count = code.matches('(').count().min(10) as u32;
        
        // Count methods
        let method_count = code.matches("def ").count() as u32 +
                          code.matches("function ").count() as u32 +
                          code.matches("fn ").count() as u32;
        
        Ok(CodeMetrics {
            lines_of_code,
            cyclomatic_complexity: complexity,
            parameter_count,
            nesting_depth: max_depth,
            method_count,
            class_size: lines_of_code, // Simplified
        })
    }

    fn find_smell_location(&self, context: &DetectionContext, _rule: &DetectionRule) -> Result<CodeLocation> {
        // Simplified location finding
        Ok(CodeLocation {
            file_path: context.file_path.clone(),
            start_line: 1,
            end_line: context.code.lines().count() as u32,
            function_name: None,
            class_name: None,
        })
    }

    fn generate_description(&self, smell_type: &CodeSmellType, context: &DetectionContext) -> String {
        match smell_type {
            CodeSmellType::LongMethod => {
                format!("Method is too long ({} lines). Consider breaking it into smaller methods.", 
                       context.metrics.lines_of_code)
            }
            CodeSmellType::LongParameterList => {
                format!("Method has too many parameters ({}). Consider using parameter objects or builder pattern.", 
                       context.metrics.parameter_count)
            }
            CodeSmellType::DuplicatedCode => {
                "Duplicated code detected. Extract common functionality into shared methods.".to_string()
            }
            CodeSmellType::LargeClass => {
                format!("Class is too large ({} lines). Consider splitting into multiple classes with single responsibilities.", 
                       context.metrics.class_size)
            }
            CodeSmellType::MagicNumbers => {
                "Magic numbers detected. Replace with named constants for better readability.".to_string()
            }
            CodeSmellType::DeepNesting => {
                format!("Code has deep nesting ({} levels). Consider extracting methods or using guard clauses.", 
                       context.metrics.nesting_depth)
            }
            CodeSmellType::HardcodedSecrets => {
                "Hardcoded secrets detected. Move sensitive data to environment variables or secure storage.".to_string()
            }
            CodeSmellType::UnusedCode => {
                "Unused code detected. Remove dead code to improve maintainability.".to_string()
            }
            _ => format!("Code smell detected: {:?}", smell_type),
        }
    }

    fn generate_refactoring_suggestion(&self, smell_type: &CodeSmellType, _context: &DetectionContext) -> String {
        match smell_type {
            CodeSmellType::LongMethod => {
                "1. Extract logical blocks into separate methods\n2. Use meaningful method names\n3. Keep methods focused on single responsibility".to_string()
            }
            CodeSmellType::LongParameterList => {
                "1. Group related parameters into objects\n2. Use builder pattern for complex construction\n3. Consider dependency injection".to_string()
            }
            CodeSmellType::DuplicatedCode => {
                "1. Extract common code into shared methods\n2. Use inheritance or composition\n3. Create utility classes for common operations".to_string()
            }
            CodeSmellType::MagicNumbers => {
                "1. Define constants with meaningful names\n2. Use enums for related constants\n3. Document the meaning of numbers".to_string()
            }
            CodeSmellType::DeepNesting => {
                "1. Use guard clauses for early returns\n2. Extract nested logic into methods\n3. Consider using polymorphism".to_string()
            }
            CodeSmellType::HardcodedSecrets => {
                "1. Use environment variables\n2. Implement secure configuration management\n3. Use secret management services".to_string()
            }
            _ => "Consider refactoring to improve code quality and maintainability.".to_string(),
        }
    }

    fn estimate_refactoring_effort(&self, smell_type: &CodeSmellType, context: &DetectionContext) -> RefactoringEffort {
        match smell_type {
            CodeSmellType::MagicNumbers | CodeSmellType::CommentedCode => RefactoringEffort::Trivial,
            CodeSmellType::LongParameterList | CodeSmellType::UnclearNaming => RefactoringEffort::Easy,
            CodeSmellType::LongMethod | CodeSmellType::DeepNesting => RefactoringEffort::Medium,
            CodeSmellType::LargeClass | CodeSmellType::GodClass => {
                if context.metrics.class_size > 1000 {
                    RefactoringEffort::VeryHard
                } else {
                    RefactoringEffort::Hard
                }
            }
            CodeSmellType::CircularDependency | CodeSmellType::TightCoupling => RefactoringEffort::VeryHard,
            _ => RefactoringEffort::Medium,
        }
    }

    fn calculate_impact_score(&self, smell_type: &CodeSmellType, context: &DetectionContext) -> f64 {
        let base_score = match smell_type {
            CodeSmellType::HardcodedSecrets => 0.9,
            CodeSmellType::CircularDependency => 0.85,
            CodeSmellType::GodClass => 0.8,
            CodeSmellType::LongMethod => 0.7,
            CodeSmellType::DuplicatedCode => 0.65,
            CodeSmellType::DeepNesting => 0.6,
            CodeSmellType::MagicNumbers => 0.4,
            CodeSmellType::CommentedCode => 0.3,
            _ => 0.5,
        };
        
        // Adjust based on code size and complexity
        let size_multiplier = (context.metrics.lines_of_code as f64 / 100.0).min(2.0);
        let complexity_multiplier = (context.metrics.cyclomatic_complexity as f64 / 10.0).min(2.0);
        
        (base_score * size_multiplier * complexity_multiplier).min(1.0)
    }

    fn get_refactoring_examples(&self, smell_type: &CodeSmellType) -> Vec<RefactoringExample> {
        match smell_type {
            CodeSmellType::LongMethod => vec![
                RefactoringExample {
                    before: "def process_order(order):\n    # 50 lines of code\n    validate_order()\n    calculate_total()\n    send_email()\n    update_inventory()".to_string(),
                    after: "def process_order(order):\n    validate_order(order)\n    total = calculate_total(order)\n    send_confirmation_email(order)\n    update_inventory(order)\n\ndef validate_order(order):\n    # validation logic\n\ndef calculate_total(order):\n    # calculation logic".to_string(),
                    explanation: "Break down long method into smaller, focused methods".to_string(),
                }
            ],
            CodeSmellType::MagicNumbers => vec![
                RefactoringExample {
                    before: "if age >= 18 and score > 85:\n    return True".to_string(),
                    after: "LEGAL_AGE = 18\nPASSING_SCORE = 85\n\nif age >= LEGAL_AGE and score > PASSING_SCORE:\n    return True".to_string(),
                    explanation: "Replace magic numbers with named constants".to_string(),
                }
            ],
            _ => Vec::new(),
        }
    }

    async fn apply_language_rule(&self, _rule: &LanguageRule, _context: &DetectionContext) -> Result<Option<CodeSmell>> {
        // Language-specific rule application would go here
        Ok(None)
    }

    fn initialize_patterns(&mut self) {
        // Initialize detection patterns for each smell type
        // This is a simplified version - real implementation would be more comprehensive
        
        self.smell_patterns.insert(
            CodeSmellType::LongMethod,
            SmellPattern {
                detection_rules: vec![
                    DetectionRule {
                        pattern: "long_method".to_string(),
                        threshold: Some(50.0),
                        context_required: vec![],
                    }
                ],
                severity_calculator: |context| {
                    if context.metrics.lines_of_code > 100 {
                        SmellSeverity::High
                    } else if context.metrics.lines_of_code > 75 {
                        SmellSeverity::Medium
                    } else {
                        SmellSeverity::Low
                    }
                },
                refactoring_templates: vec![],
            }
        );

        // Add more patterns...
    }

    fn initialize_language_rules(&mut self) {
        // Initialize language-specific rules
        self.language_specific_rules.insert(
            "python".to_string(),
            vec![
                LanguageRule {
                    rule_type: "pep8_violation".to_string(),
                    pattern: "long_line".to_string(),
                    severity: SmellSeverity::Low,
                }
            ]
        );
    }
}

impl Default for CodeSmellDetector {
    fn default() -> Self {
        Self::new()
    }
}