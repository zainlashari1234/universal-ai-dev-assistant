use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::ai_engine::providers::ProviderRouter;
use crate::observability::get_metrics;

use super::{AgentRequest, AgentResponse, AgentArtifact, ArtifactType};

/// CodegenAgent: Generates code based on goals and context
/// Uses AI providers to generate high-quality, contextual code
pub struct CodegenAgent {
    provider_router: Arc<ProviderRouter>,
}

impl CodegenAgent {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self { provider_router }
    }

    /// Execute code generation for a given goal
    pub async fn execute(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let start_time = Instant::now();
        
        info!("CodegenAgent executing for goal: {}", request.goal);
        
        // Record metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["codegen", "execute"])
            .observe(0.0);
        
        let mut artifacts = Vec::new();
        let mut cost = 0.0;
        
        // Generate code based on goal and context
        let generation_result = self.generate_code(&request.goal, &request.context).await;
        
        let (success, result, error) = match generation_result {
            Ok(generated_files) => {
                cost += 0.2; // Estimated cost for code generation
                
                // Create artifacts for each generated file
                for (i, file) in generated_files.iter().enumerate() {
                    artifacts.push(AgentArtifact {
                        name: file["filename"].as_str().unwrap_or(&format!("generated_{}.py", i)).to_string(),
                        artifact_type: ArtifactType::Code,
                        content: file["content"].as_str().unwrap_or("").to_string(),
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("language".to_string(), file["language"].as_str().unwrap_or("python").to_string());
                            meta.insert("goal".to_string(), request.goal.clone());
                            meta.insert("lines".to_string(), file["content"].as_str().unwrap_or("").lines().count().to_string());
                            meta
                        },
                    });
                }
                
                (true, json!({"generated_files": generated_files}), None)
            }
            Err(e) => {
                warn!("Code generation failed: {}", e);
                (false, json!({"error": "Code generation failed"}), Some(e.to_string()))
            }
        };
        
        let execution_time = start_time.elapsed();
        
        // Record execution time
        metrics.agent_step_duration_ms
            .with_label_values(&["codegen", "execute"])
            .observe(execution_time.as_millis() as f64);
        
        let response = AgentResponse {
            id: request.id,
            agent_name: "codegen".to_string(),
            success,
            result,
            artifacts,
            execution_time,
            cost: Some(cost),
            error,
        };
        
        debug!("CodegenAgent completed in {:?} (success: {})", execution_time, success);
        
        Ok(response)
    }

    /// Generate code based on goal and context
    async fn generate_code(&self, goal: &str, context: &Option<String>) -> Result<Vec<Value>> {
        let code_prompt = self.build_code_generation_prompt(goal, context);
        
        let generation_result = self.provider_router
            .complete(&code_prompt, context.as_deref())
            .await?;
        
        if generation_result.is_empty() {
            return Ok(vec![self.create_fallback_code(goal)]);
        }
        
        // Parse and structure the generated code
        let structured_code = self.structure_generated_code(goal, &generation_result[0]).await?;
        
        Ok(structured_code)
    }

    /// Build prompt for code generation
    fn build_code_generation_prompt(&self, goal: &str, context: &Option<String>) -> String {
        let context_section = if let Some(ctx) = context {
            format!("\n\nExisting Code Context:\n{}", ctx)
        } else {
            String::new()
        };
        
        format!(
            r#"Generate high-quality Python code to achieve the following goal:

Goal: {}{}

Requirements:
1. Write clean, readable, and well-documented code
2. Follow Python best practices and PEP 8 style guide
3. Include proper error handling where appropriate
4. Add type hints if beneficial
5. Include docstrings for functions and classes
6. Consider edge cases and validation

Please provide the complete implementation that addresses the goal."#,
            goal, context_section
        )
    }

    /// Structure the generated code into files
    async fn structure_generated_code(&self, goal: &str, generated_code: &str) -> Result<Vec<Value>> {
        let mut files = Vec::new();
        
        // Determine the primary language and file structure
        let language = self.detect_language(generated_code);
        let file_extension = match language.as_str() {
            "python" => "py",
            "javascript" => "js",
            "typescript" => "ts",
            "rust" => "rs",
            _ => "py", // Default to Python
        };
        
        // Create main implementation file
        let main_filename = if goal.to_lowercase().contains("test") {
            format!("test_implementation.{}", file_extension)
        } else {
            format!("implementation.{}", file_extension)
        };
        
        files.push(json!({
            "filename": main_filename,
            "content": generated_code,
            "language": language,
            "file_type": "implementation",
            "description": format!("Implementation for: {}", goal)
        }));
        
        // Generate additional files if needed
        if self.should_generate_utils_file(generated_code) {
            let utils_content = self.generate_utils_file(&language).await?;
            files.push(json!({
                "filename": format!("utils.{}", file_extension),
                "content": utils_content,
                "language": language,
                "file_type": "utility",
                "description": "Utility functions and helpers"
            }));
        }
        
        if self.should_generate_config_file(goal) {
            let config_content = self.generate_config_file(&language).await?;
            files.push(json!({
                "filename": format!("config.{}", file_extension),
                "content": config_content,
                "language": language,
                "file_type": "configuration",
                "description": "Configuration and constants"
            }));
        }
        
        Ok(files)
    }

    /// Detect programming language from generated code
    fn detect_language(&self, code: &str) -> String {
        if code.contains("def ") || code.contains("import ") || code.contains("class ") {
            "python".to_string()
        } else if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            "javascript".to_string()
        } else if code.contains("fn ") || code.contains("struct ") || code.contains("impl ") {
            "rust".to_string()
        } else {
            "python".to_string() // Default
        }
    }

    /// Check if utils file should be generated
    fn should_generate_utils_file(&self, code: &str) -> bool {
        code.len() > 500 && (
            code.contains("helper") || 
            code.contains("utility") || 
            code.matches("def ").count() > 3
        )
    }

    /// Check if config file should be generated
    fn should_generate_config_file(&self, goal: &str) -> bool {
        let goal_lower = goal.to_lowercase();
        goal_lower.contains("config") || 
        goal_lower.contains("setting") || 
        goal_lower.contains("constant")
    }

    /// Generate utility file content
    async fn generate_utils_file(&self, language: &str) -> Result<String> {
        match language {
            "python" => Ok(r#""""
Utility functions and helpers.
"""

def validate_input(value, expected_type):
    """Validate input value against expected type."""
    if not isinstance(value, expected_type):
        raise TypeError(f"Expected {expected_type.__name__}, got {type(value).__name__}")
    return True

def safe_divide(a, b):
    """Safely divide two numbers with zero check."""
    if b == 0:
        raise ValueError("Division by zero is not allowed")
    return a / b

def format_error_message(error, context=""):
    """Format error message with context."""
    if context:
        return f"Error in {context}: {str(error)}"
    return str(error)
"#.to_string()),
            "javascript" => Ok(r#"/**
 * Utility functions and helpers.
 */

function validateInput(value, expectedType) {
    if (typeof value !== expectedType) {
        throw new TypeError(`Expected ${expectedType}, got ${typeof value}`);
    }
    return true;
}

function safeDivide(a, b) {
    if (b === 0) {
        throw new Error("Division by zero is not allowed");
    }
    return a / b;
}

function formatErrorMessage(error, context = "") {
    if (context) {
        return `Error in ${context}: ${error.message}`;
    }
    return error.message;
}

module.exports = {
    validateInput,
    safeDivide,
    formatErrorMessage
};
"#.to_string()),
            _ => Ok("// Utility functions\n".to_string()),
        }
    }

    /// Generate configuration file content
    async fn generate_config_file(&self, language: &str) -> Result<String> {
        match language {
            "python" => Ok(r#""""
Configuration and constants.
"""

# Application settings
DEBUG = False
VERSION = "1.0.0"

# Error handling settings
MAX_RETRIES = 3
TIMEOUT_SECONDS = 30

# Validation settings
STRICT_TYPE_CHECKING = True
ALLOW_ZERO_DIVISION = False

# Logging configuration
LOG_LEVEL = "INFO"
LOG_FORMAT = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
"#.to_string()),
            "javascript" => Ok(r#"/**
 * Configuration and constants.
 */

const config = {
    // Application settings
    DEBUG: false,
    VERSION: "1.0.0",
    
    // Error handling settings
    MAX_RETRIES: 3,
    TIMEOUT_SECONDS: 30,
    
    // Validation settings
    STRICT_TYPE_CHECKING: true,
    ALLOW_ZERO_DIVISION: false,
    
    // Logging configuration
    LOG_LEVEL: "INFO"
};

module.exports = config;
"#.to_string()),
            _ => Ok("// Configuration\n".to_string()),
        }
    }

    /// Create fallback code when AI generation fails
    fn create_fallback_code(&self, goal: &str) -> Value {
        let goal_lower = goal.to_lowercase();
        
        let content = if goal_lower.contains("divide") || goal_lower.contains("division") {
            r#"def safe_divide(a, b):
    """Safely divide two numbers with error handling."""
    if not isinstance(a, (int, float)) or not isinstance(b, (int, float)):
        raise TypeError("Both arguments must be numbers")
    
    if b == 0:
        raise ValueError("Division by zero is not allowed")
    
    return a / b

def main():
    """Example usage of safe_divide function."""
    try:
        result = safe_divide(10, 2)
        print(f"Result: {result}")
    except (TypeError, ValueError) as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()
"#
        } else {
            r#"def implement_goal():
    """Implementation for the specified goal."""
    # TODO: Implement the functionality
    pass

def main():
    """Main function."""
    implement_goal()

if __name__ == "__main__":
    main()
"#
        };
        
        json!({
            "filename": "implementation.py",
            "content": content,
            "language": "python",
            "file_type": "implementation",
            "description": format!("Fallback implementation for: {}", goal)
        })
    }
}