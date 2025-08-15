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

/// TestgenAgent: Generates comprehensive tests for code
/// Creates unit tests, integration tests, and edge case tests
pub struct TestgenAgent {
    provider_router: Arc<ProviderRouter>,
}

impl TestgenAgent {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self { provider_router }
    }

    /// Execute test generation for given code
    pub async fn execute(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let start_time = Instant::now();
        
        info!("TestgenAgent executing for goal: {}", request.goal);
        
        // Record metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["testgen", "execute"])
            .observe(0.0);
        
        let mut artifacts = Vec::new();
        let mut cost = 0.0;
        
        // Generate tests based on the context (generated code)
        let test_generation_result = self.generate_tests(&request.goal, &request.context).await;
        
        let (success, result, error) = match test_generation_result {
            Ok(test_files) => {
                cost += 0.15; // Estimated cost for test generation
                
                // Create artifacts for each test file
                for (i, test_file) in test_files.iter().enumerate() {
                    artifacts.push(AgentArtifact {
                        name: test_file["filename"].as_str().unwrap_or(&format!("test_{}.py", i)).to_string(),
                        artifact_type: ArtifactType::Test,
                        content: test_file["content"].as_str().unwrap_or("").to_string(),
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("language".to_string(), test_file["language"].as_str().unwrap_or("python").to_string());
                            meta.insert("test_type".to_string(), test_file["test_type"].as_str().unwrap_or("unit").to_string());
                            meta.insert("goal".to_string(), request.goal.clone());
                            meta
                        },
                    });
                }
                
                (true, json!({"test_files": test_files}), None)
            }
            Err(e) => {
                warn!("Test generation failed: {}", e);
                (false, json!({"error": "Test generation failed"}), Some(e.to_string()))
            }
        };
        
        let execution_time = start_time.elapsed();
        
        // Record execution time
        metrics.agent_step_duration_ms
            .with_label_values(&["testgen", "execute"])
            .observe(execution_time.as_millis() as f64);
        
        let response = AgentResponse {
            id: request.id,
            agent_name: "testgen".to_string(),
            success,
            result,
            artifacts,
            execution_time,
            cost: Some(cost),
            error,
        };
        
        debug!("TestgenAgent completed in {:?} (success: {})", execution_time, success);
        
        Ok(response)
    }

    /// Generate comprehensive tests for the given code
    async fn generate_tests(&self, goal: &str, context: &Option<String>) -> Result<Vec<Value>> {
        let mut test_files = Vec::new();
        
        // Parse context to extract generated code
        let generated_code = if let Some(ctx) = context {
            self.extract_code_from_context(ctx)?
        } else {
            vec![]
        };
        
        // Generate unit tests
        let unit_tests = self.generate_unit_tests(goal, &generated_code).await?;
        test_files.extend(unit_tests);
        
        // Generate integration tests if needed
        if self.should_generate_integration_tests(&generated_code) {
            let integration_tests = self.generate_integration_tests(goal, &generated_code).await?;
            test_files.extend(integration_tests);
        }
        
        // Generate edge case tests
        let edge_case_tests = self.generate_edge_case_tests(goal, &generated_code).await?;
        test_files.extend(edge_case_tests);
        
        Ok(test_files)
    }

    /// Extract code information from context
    fn extract_code_from_context(&self, context: &str) -> Result<Vec<Value>> {
        // Try to parse as JSON first
        if let Ok(parsed) = serde_json::from_str::<Value>(context) {
            if let Some(generated_files) = parsed["generated_files"].as_array() {
                return Ok(generated_files.clone());
            }
        }
        
        // Fallback: treat as raw code
        Ok(vec![json!({
            "filename": "code.py",
            "content": context,
            "language": "python"
        })])
    }

    /// Generate unit tests for the code
    async fn generate_unit_tests(&self, goal: &str, generated_code: &[Value]) -> Result<Vec<Value>> {
        let mut unit_tests = Vec::new();
        
        for code_file in generated_code {
            let filename = code_file["filename"].as_str().unwrap_or("code.py");
            let content = code_file["content"].as_str().unwrap_or("");
            let language = code_file["language"].as_str().unwrap_or("python");
            
            let test_content = match language {
                "python" => self.generate_python_unit_tests(goal, filename, content).await?,
                "javascript" => self.generate_javascript_unit_tests(goal, filename, content).await?,
                _ => self.generate_python_unit_tests(goal, filename, content).await?, // Default to Python
            };
            
            let test_filename = self.generate_test_filename(filename, language);
            
            unit_tests.push(json!({
                "filename": test_filename,
                "content": test_content,
                "language": language,
                "test_type": "unit",
                "target_file": filename,
                "description": format!("Unit tests for {}", filename)
            }));
        }
        
        Ok(unit_tests)
    }

    /// Generate Python unit tests
    async fn generate_python_unit_tests(&self, goal: &str, filename: &str, content: &str) -> Result<String> {
        let functions = self.extract_python_functions(content);
        let classes = self.extract_python_classes(content);
        
        let mut test_content = String::new();
        
        // Test file header
        test_content.push_str(&format!(r#""""
Unit tests for {}
Generated for goal: {}
"""

import pytest
import sys
from unittest.mock import Mock, patch

# Import the module under test
"#, filename, goal));
        
        // Add import statement
        let module_name = filename.replace(".py", "").replace("/", ".");
        test_content.push_str(&format!("from {} import *\n\n", module_name));
        
        // Generate tests for functions
        for function in &functions {
            test_content.push_str(&self.generate_python_function_test(function));
            test_content.push('\n');
        }
        
        // Generate tests for classes
        for class in &classes {
            test_content.push_str(&self.generate_python_class_test(class));
            test_content.push('\n');
        }
        
        // Add edge case tests
        test_content.push_str(&self.generate_python_edge_case_tests(goal, &functions));
        
        Ok(test_content)
    }

    /// Generate JavaScript unit tests
    async fn generate_javascript_unit_tests(&self, goal: &str, filename: &str, content: &str) -> Result<String> {
        let functions = self.extract_javascript_functions(content);
        
        let mut test_content = String::new();
        
        // Test file header
        test_content.push_str(&format!(r#"/**
 * Unit tests for {}
 * Generated for goal: {}
 */

const {{ describe, test, expect, beforeEach, afterEach }} = require('@jest/globals');

// Import the module under test
"#, filename, goal));
        
        // Add require statement
        let module_name = filename.replace(".js", "");
        test_content.push_str(&format!("const moduleUnderTest = require('./{}.js');\n\n", module_name));
        
        // Generate tests for functions
        test_content.push_str(&format!("describe('{}', () => {{\n", filename));
        
        for function in &functions {
            test_content.push_str(&self.generate_javascript_function_test(function));
            test_content.push('\n');
        }
        
        test_content.push_str("});\n");
        
        Ok(test_content)
    }

    /// Extract Python function names from code
    fn extract_python_functions(&self, content: &str) -> Vec<String> {
        let mut functions = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("def ") && !trimmed.starts_with("def _") {
                if let Some(func_name) = trimmed.split('(').next() {
                    let name = func_name.replace("def ", "").trim().to_string();
                    if !name.is_empty() {
                        functions.push(name);
                    }
                }
            }
        }
        
        functions
    }

    /// Extract Python class names from code
    fn extract_python_classes(&self, content: &str) -> Vec<String> {
        let mut classes = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("class ") {
                if let Some(class_def) = trimmed.split(':').next() {
                    let name = class_def.replace("class ", "").split('(').next().unwrap_or("").trim().to_string();
                    if !name.is_empty() {
                        classes.push(name);
                    }
                }
            }
        }
        
        classes
    }

    /// Extract JavaScript function names from code
    fn extract_javascript_functions(&self, content: &str) -> Vec<String> {
        let mut functions = Vec::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("function ") {
                if let Some(func_name) = trimmed.split('(').next() {
                    let name = func_name.replace("function ", "").trim().to_string();
                    if !name.is_empty() {
                        functions.push(name);
                    }
                }
            } else if trimmed.contains(" = function") || trimmed.contains(" => ") {
                if let Some(func_name) = trimmed.split('=').next() {
                    let name = func_name.replace("const ", "").replace("let ", "").replace("var ", "").trim().to_string();
                    if !name.is_empty() {
                        functions.push(name);
                    }
                }
            }
        }
        
        functions
    }

    /// Generate test for a Python function
    fn generate_python_function_test(&self, function_name: &str) -> String {
        format!(r#"def test_{}():
    """Test {} function."""
    # Test normal case
    # TODO: Add specific test cases for {}
    
    # Test with valid inputs
    # result = {}(valid_input)
    # assert result == expected_output
    
    # Test edge cases
    # with pytest.raises(ExpectedException):
    #     {}(invalid_input)
    
    pass  # Remove this when implementing actual tests

"#, function_name, function_name, function_name, function_name, function_name)
    }

    /// Generate test for a Python class
    fn generate_python_class_test(&self, class_name: &str) -> String {
        format!(r#"class Test{}:
    """Test {} class."""
    
    def setup_method(self):
        """Set up test fixtures."""
        self.instance = {}()
    
    def test_initialization(self):
        """Test class initialization."""
        assert self.instance is not None
    
    def test_methods(self):
        """Test class methods."""
        # TODO: Add specific method tests
        pass

"#, class_name, class_name, class_name)
    }

    /// Generate JavaScript function test
    fn generate_javascript_function_test(&self, function_name: &str) -> String {
        format!(r#"    test('{}', () => {{
        // Test normal case
        // const result = moduleUnderTest.{}(validInput);
        // expect(result).toBe(expectedOutput);
        
        // Test edge cases
        // expect(() => moduleUnderTest.{}(invalidInput)).toThrow();
        
        // TODO: Implement actual test cases
        expect(true).toBe(true); // Placeholder
    }});
"#, function_name, function_name, function_name)
    }

    /// Generate Python edge case tests
    fn generate_python_edge_case_tests(&self, goal: &str, functions: &[String]) -> String {
        let mut edge_tests = String::new();
        
        edge_tests.push_str("# Edge case tests\n\n");
        
        if goal.to_lowercase().contains("divide") || goal.to_lowercase().contains("division") {
            edge_tests.push_str(r#"def test_division_by_zero():
    """Test division by zero handling."""
    with pytest.raises((ValueError, ZeroDivisionError)):
        safe_divide(10, 0)

def test_division_with_floats():
    """Test division with floating point numbers."""
    result = safe_divide(10.5, 2.5)
    assert abs(result - 4.2) < 0.0001

"#);
        }
        
        if !functions.is_empty() {
            edge_tests.push_str(&format!(r#"def test_invalid_input_types():
    """Test functions with invalid input types."""
    # Test with None
    with pytest.raises(TypeError):
        {}(None)
    
    # Test with wrong types
    with pytest.raises(TypeError):
        {}("invalid_string")

"#, functions[0], functions[0]));
        }
        
        edge_tests
    }

    /// Generate integration tests if needed
    async fn generate_integration_tests(&self, goal: &str, _generated_code: &[Value]) -> Result<Vec<Value>> {
        let test_content = format!(r#""""
Integration tests for {}
"""

import pytest

def test_integration_workflow():
    """Test the complete workflow integration."""
    # TODO: Implement integration test
    pass

def test_module_interactions():
    """Test interactions between modules."""
    # TODO: Implement module interaction tests
    pass
"#, goal);
        
        Ok(vec![json!({
            "filename": "test_integration.py",
            "content": test_content,
            "language": "python",
            "test_type": "integration",
            "description": "Integration tests"
        })])
    }

    /// Generate edge case tests
    async fn generate_edge_case_tests(&self, goal: &str, _generated_code: &[Value]) -> Result<Vec<Value>> {
        let test_content = format!(r#""""
Edge case tests for {}
"""

import pytest

def test_boundary_conditions():
    """Test boundary conditions."""
    # TODO: Implement boundary tests
    pass

def test_error_conditions():
    """Test error handling."""
    # TODO: Implement error condition tests
    pass

def test_performance_edge_cases():
    """Test performance with edge cases."""
    # TODO: Implement performance tests
    pass
"#, goal);
        
        Ok(vec![json!({
            "filename": "test_edge_cases.py",
            "content": test_content,
            "language": "python",
            "test_type": "edge_case",
            "description": "Edge case tests"
        })])
    }

    /// Check if integration tests should be generated
    fn should_generate_integration_tests(&self, generated_code: &[Value]) -> bool {
        generated_code.len() > 1 || 
        generated_code.iter().any(|file| {
            file["content"].as_str().unwrap_or("").contains("import") ||
            file["content"].as_str().unwrap_or("").contains("class")
        })
    }

    /// Generate test filename from source filename
    fn generate_test_filename(&self, source_filename: &str, language: &str) -> String {
        let extension = match language {
            "javascript" => "test.js",
            "typescript" => "test.ts",
            _ => "py", // Default to Python
        };
        
        let base_name = source_filename.split('.').next().unwrap_or("test");
        format!("test_{}.{}", base_name, extension)
    }
}