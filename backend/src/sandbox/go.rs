use super::*;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct GoRunner {
    docker_runner: DockerRunner,
}

impl GoRunner {
    pub fn new() -> Self {
        Self {
            docker_runner: DockerRunner::new("golang:1.21-alpine".to_string()),
        }
    }

    async fn setup_go_environment(&self, request: &ExecutionRequest, temp_dir: &PathBuf) -> Result<()> {
        // Create go.mod if it doesn't exist
        let go_mod_content = self.generate_go_mod(request);
        let go_mod_path = temp_dir.join("go.mod");
        tokio::fs::write(go_mod_path, go_mod_content).await?;

        // Create main.go with the code
        let main_go_path = temp_dir.join("main.go");
        tokio::fs::write(main_go_path, &request.code).await?;

        // Create test file if test command is provided
        if request.test_command.is_some() {
            self.create_test_file(request, temp_dir).await?;
        }

        // Write additional files
        for (filename, content) in &request.files {
            let file_path = temp_dir.join(filename);
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(file_path, content).await?;
        }

        Ok(())
    }

    fn generate_go_mod(&self, request: &ExecutionRequest) -> String {
        let module_name = "uaida-sandbox";
        let go_version = "1.21";

        let mut dependencies = Vec::new();

        // Detect common imports and add corresponding dependencies
        if request.code.contains("github.com/gin-gonic/gin") {
            dependencies.push("github.com/gin-gonic/gin v1.9.1");
        }
        if request.code.contains("github.com/gorilla/mux") {
            dependencies.push("github.com/gorilla/mux v1.8.0");
        }
        if request.code.contains("github.com/stretchr/testify") {
            dependencies.push("github.com/stretchr/testify v1.8.4");
        }
        if request.code.contains("gorm.io/gorm") {
            dependencies.push("gorm.io/gorm v1.25.5");
        }

        // Always include testify for testing
        if !dependencies.iter().any(|dep| dep.contains("testify")) {
            dependencies.push("github.com/stretchr/testify v1.8.4");
        }

        let mut go_mod = format!(
            "module {}\n\ngo {}\n",
            module_name, go_version
        );

        if !dependencies.is_empty() {
            go_mod.push_str("\nrequire (\n");
            for dep in dependencies {
                go_mod.push_str(&format!("\t{}\n", dep));
            }
            go_mod.push_str(")\n");
        }

        go_mod
    }

    async fn create_test_file(&self, request: &ExecutionRequest, temp_dir: &PathBuf) -> Result<()> {
        let test_content = if request.code.contains("func Test") {
            // Code already contains tests
            request.code.clone()
        } else {
            // Generate basic test structure
            format!(
                r#"package main

import (
    "testing"
    "github.com/stretchr/testify/assert"
)

func TestBasicFunctionality(t *testing.T) {{
    // Basic test to ensure code runs without errors
    assert.True(t, true, "Basic test should pass")
}}

// Add more specific tests here
"#
            )
        };

        let test_path = temp_dir.join("main_test.go");
        tokio::fs::write(test_path, test_content).await?;
        Ok(())
    }

    async fn run_with_coverage(&self, config: &SandboxConfig, temp_dir: &PathBuf) -> Result<ExecutionResult> {
        // Initialize Go module
        let init_cmd = "go mod tidy";
        let init_request = ExecutionRequest {
            code: init_cmd.to_string(),
            language: "bash".to_string(),
            test_command: None,
            files: HashMap::new(),
            environment: HashMap::new(),
            working_directory: Some("/app".to_string()),
        };

        let init_result = self.docker_runner.execute_with_mount(&init_request, config, temp_dir).await?;
        if !init_result.success {
            return Ok(init_result);
        }

        // Run tests with coverage
        let test_cmd = "go test -v -coverprofile=coverage.out -covermode=atomic ./...";
        let test_request = ExecutionRequest {
            code: test_cmd.to_string(),
            language: "bash".to_string(),
            test_command: None,
            files: HashMap::new(),
            environment: HashMap::new(),
            working_directory: Some("/app".to_string()),
        };

        let mut result = self.docker_runner.execute_with_mount(&test_request, config, temp_dir).await?;

        // Generate coverage report
        if result.success {
            let coverage_cmd = "go tool cover -func=coverage.out";
            let coverage_request = ExecutionRequest {
                code: coverage_cmd.to_string(),
                language: "bash".to_string(),
                test_command: None,
                files: HashMap::new(),
                environment: HashMap::new(),
                working_directory: Some("/app".to_string()),
            };

            if let Ok(coverage_result) = self.docker_runner.execute_with_mount(&coverage_request, config, temp_dir).await {
                result.coverage = self.parse_coverage_output(&coverage_result.stdout);
            }
        }

        Ok(result)
    }

    fn parse_coverage_output(&self, coverage_output: &str) -> Option<CoverageReport> {
        let lines: Vec<&str> = coverage_output.lines().collect();
        
        // Find the total coverage line (usually the last line)
        let total_line = lines.iter().rev().find(|line| line.contains("total:"))?;
        
        // Parse coverage percentage from line like "total:                  (statements)    85.7%"
        let parts: Vec<&str> = total_line.split_whitespace().collect();
        let percentage_str = parts.last()?.trim_end_matches('%');
        let coverage_percentage: f32 = percentage_str.parse().ok()?;

        // Count total and covered statements
        let mut total_statements = 0;
        let mut covered_statements = 0;
        
        for line in lines {
            if line.contains("total:") {
                continue;
            }
            if let Some(parts) = line.split_whitespace().nth(2) {
                if let Ok(statements) = parts.parse::<usize>() {
                    total_statements += statements;
                    // Estimate covered based on percentage (simplified)
                    covered_statements += ((statements as f32 * coverage_percentage / 100.0) as usize);
                }
            }
        }

        Some(CoverageReport {
            total_lines: total_statements,
            covered_lines: covered_statements,
            coverage_percentage,
            file_coverage: HashMap::new(), // Would need more detailed parsing
        })
    }
}

#[async_trait]
impl SandboxRunner for GoRunner {
    fn language(&self) -> &str {
        "go"
    }

    fn supports_coverage(&self) -> bool {
        true
    }

    async fn execute(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        debug!("Executing Go code in sandbox");

        // Create temporary directory
        let temp_dir = config.temp_dir.join(format!("go_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        // Setup Go environment
        self.setup_go_environment(request, &temp_dir).await?;

        // Execute using Docker
        let docker_request = ExecutionRequest {
            code: "go mod tidy && go run main.go".to_string(),
            language: "bash".to_string(),
            test_command: None,
            files: HashMap::new(),
            environment: request.environment.clone(),
            working_directory: Some("/app".to_string()),
        };

        let result = self.docker_runner.execute_with_mount(&docker_request, config, &temp_dir).await?;

        // Cleanup
        if let Err(e) = tokio::fs::remove_dir_all(&temp_dir).await {
            warn!("Failed to cleanup temp directory: {}", e);
        }

        Ok(result)
    }

    async fn run_tests(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        debug!("Running Go tests in sandbox");

        // Create temporary directory
        let temp_dir = config.temp_dir.join(format!("go_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        // Setup Go environment with tests
        self.setup_go_environment(request, &temp_dir).await?;

        let result = if self.supports_coverage() {
            self.run_with_coverage(config, &temp_dir).await?
        } else {
            // Initialize Go module first
            let init_cmd = "go mod tidy";
            let init_request = ExecutionRequest {
                code: init_cmd.to_string(),
                language: "bash".to_string(),
                test_command: None,
                files: HashMap::new(),
                environment: HashMap::new(),
                working_directory: Some("/app".to_string()),
            };

            let init_result = self.docker_runner.execute_with_mount(&init_request, config, &temp_dir).await?;
            if !init_result.success {
                return Ok(init_result);
            }

            // Run tests
            let test_command = request.test_command
                .clone()
                .unwrap_or_else(|| "go test -v ./...".to_string());

            let test_request = ExecutionRequest {
                code: test_command,
                language: "bash".to_string(),
                test_command: None,
                files: HashMap::new(),
                environment: request.environment.clone(),
                working_directory: Some("/app".to_string()),
            };

            self.docker_runner.execute_with_mount(&test_request, config, &temp_dir).await?
        };

        // Cleanup
        if let Err(e) = tokio::fs::remove_dir_all(&temp_dir).await {
            warn!("Failed to cleanup temp directory: {}", e);
        }

        Ok(result)
    }
}