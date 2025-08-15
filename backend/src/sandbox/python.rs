use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

use crate::observability::get_metrics;
use super::{
    Artifact, ArtifactType, CoverageReport, ExecutionRequest, ExecutionResult, 
    FileCoverage, SandboxConfig, SandboxRunner
};

pub struct PythonSandboxRunner {
    docker_image: String,
}

impl PythonSandboxRunner {
    pub fn new() -> Self {
        Self {
            docker_image: "python:3.11-slim".to_string(),
        }
    }

    pub fn with_image(docker_image: String) -> Self {
        Self { docker_image }
    }

    /// Create isolated execution environment
    async fn create_execution_environment(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<PathBuf> {
        let execution_id = Uuid::new_v4().to_string();
        let execution_dir = config.temp_dir.join(format!("python_{}", execution_id));
        
        // Create execution directory
        fs::create_dir_all(&execution_dir).await?;
        
        // Write main code file
        let main_file = execution_dir.join("main.py");
        fs::write(&main_file, &request.code).await?;
        
        // Write additional files
        for (filename, content) in &request.files {
            let file_path = execution_dir.join(filename);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&file_path, content).await?;
        }
        
        // Create requirements.txt if not provided
        if !request.files.contains_key("requirements.txt") {
            let requirements = self.generate_requirements(&request.code);
            fs::write(execution_dir.join("requirements.txt"), requirements).await?;
        }
        
        // Create Dockerfile for this execution
        let dockerfile_content = self.generate_dockerfile(&request.environment);
        fs::write(execution_dir.join("Dockerfile"), dockerfile_content).await?;
        
        debug!("Created Python execution environment at: {:?}", execution_dir);
        Ok(execution_dir)
    }

    /// Generate requirements.txt based on code analysis
    fn generate_requirements(&self, code: &str) -> String {
        let mut requirements = Vec::new();
        
        // Basic requirements for testing and coverage
        requirements.push("pytest>=7.0.0".to_string());
        requirements.push("pytest-cov>=4.0.0".to_string());
        requirements.push("coverage>=7.0.0".to_string());
        
        // Analyze imports and add common packages
        let lines: Vec<&str> = code.lines().collect();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                if trimmed.contains("requests") {
                    requirements.push("requests>=2.28.0".to_string());
                }
                if trimmed.contains("numpy") {
                    requirements.push("numpy>=1.24.0".to_string());
                }
                if trimmed.contains("pandas") {
                    requirements.push("pandas>=1.5.0".to_string());
                }
                if trimmed.contains("flask") {
                    requirements.push("flask>=2.2.0".to_string());
                }
                if trimmed.contains("fastapi") {
                    requirements.push("fastapi>=0.95.0".to_string());
                }
            }
        }
        
        requirements.join("\n")
    }

    /// Generate Dockerfile for Python execution
    fn generate_dockerfile(&self, environment: &HashMap<String, String>) -> String {
        let mut dockerfile = format!(
            r#"FROM {}

# Set working directory
WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    gcc \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

# Set environment variables
"#,
            self.docker_image
        );
        
        // Add custom environment variables
        for (key, value) in environment {
            dockerfile.push_str(&format!("ENV {}={}\n", key, value));
        }
        
        dockerfile.push_str("\n# Default command\nCMD [\"python\", \"main.py\"]\n");
        
        dockerfile
    }

    /// Execute code in Docker container with resource limits
    async fn run_docker_container(&self, execution_dir: &PathBuf, command: &[&str], config: &SandboxConfig) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        // Record execution metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["sandbox", "python_execute"])
            .observe(0.0); // Will update at the end
        
        // Build Docker image
        let image_tag = format!("uaida-python-{}", Uuid::new_v4());
        let build_output = AsyncCommand::new("docker")
            .args(&["build", "-t", &image_tag, "."])
            .current_dir(execution_dir)
            .output()
            .await?;
        
        if !build_output.status.success() {
            let build_error = String::from_utf8_lossy(&build_output.stderr);
            error!("Docker build failed: {}", build_error);
            return Err(anyhow!("Docker build failed: {}", build_error));
        }
        
        debug!("Built Docker image: {}", image_tag);
        
        // Prepare Docker run command with security and resource limits
        let mut docker_args = vec![
            "run",
            "--rm",
            "--network=none", // No network access by default
            "--user=1000:1000", // Non-root user
            "--read-only", // Read-only filesystem
            "--tmpfs=/tmp:rw,noexec,nosuid,size=100m", // Temporary filesystem
            "--memory", &config.memory_limit,
            "--cpus", &config.cpu_limit.to_string(),
            "--ulimit", "nproc=64:64", // Limit processes
            "--ulimit", "fsize=10485760:10485760", // Limit file size to 10MB
            "--security-opt=no-new-privileges", // Security
        ];
        
        // Add timeout (Docker doesn't have built-in timeout)
        let timeout_seconds = config.timeout.as_secs();
        docker_args.extend(&["--stop-timeout", &timeout_seconds.to_string()]);
        
        // Add the image and command
        docker_args.push(&image_tag);
        docker_args.extend(command);
        
        debug!("Running Docker container with command: {:?}", docker_args);
        
        // Execute with timeout
        let execution_future = AsyncCommand::new("docker")
            .args(&docker_args)
            .current_dir(execution_dir)
            .output();
        
        let output = match tokio::time::timeout(config.timeout, execution_future).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                error!("Docker execution failed: {}", e);
                return Err(anyhow!("Docker execution failed: {}", e));
            }
            Err(_) => {
                warn!("Docker execution timed out after {:?}", config.timeout);
                // Kill the container
                let _ = AsyncCommand::new("docker")
                    .args(&["kill", &image_tag])
                    .output()
                    .await;
                
                return Ok(ExecutionResult {
                    success: false,
                    exit_code: 124, // Timeout exit code
                    stdout: String::new(),
                    stderr: format!("Execution timed out after {:?}", config.timeout),
                    execution_time: config.timeout,
                    memory_used: None,
                    coverage: None,
                    artifacts: Vec::new(),
                });
            }
        };
        
        let execution_time = start_time.elapsed();
        
        // Record execution time
        metrics.agent_step_duration_ms
            .with_label_values(&["sandbox", "python_execute"])
            .observe(execution_time.as_millis() as f64);
        
        // Clean up Docker image
        let _ = AsyncCommand::new("docker")
            .args(&["rmi", &image_tag])
            .output()
            .await;
        
        // Parse coverage if available
        let coverage = self.parse_coverage_report(execution_dir).await.ok();
        
        // Collect artifacts
        let artifacts = self.collect_artifacts(execution_dir).await?;
        
        let result = ExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            memory_used: None, // TODO: Extract from Docker stats
            coverage,
            artifacts,
        };
        
        info!("Python execution completed in {:?} (success: {})", 
              execution_time, result.success);
        
        Ok(result)
    }

    /// Parse coverage report from pytest-cov output
    async fn parse_coverage_report(&self, execution_dir: &PathBuf) -> Result<CoverageReport> {
        let coverage_file = execution_dir.join("coverage.json");
        
        if !coverage_file.exists() {
            return Err(anyhow!("Coverage report not found"));
        }
        
        let coverage_content = fs::read_to_string(&coverage_file).await?;
        let coverage_data: Value = serde_json::from_str(&coverage_content)?;
        
        let mut total_lines = 0;
        let mut covered_lines = 0;
        let mut file_coverage = HashMap::new();
        
        if let Some(files) = coverage_data["files"].as_object() {
            for (filename, file_data) in files {
                if let Some(summary) = file_data["summary"].as_object() {
                    let file_total = summary["num_statements"].as_u64().unwrap_or(0) as usize;
                    let file_covered = summary["covered_lines"].as_u64().unwrap_or(0) as usize;
                    let file_missed = file_data["missing_lines"].as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
                        .unwrap_or_default();
                    
                    total_lines += file_total;
                    covered_lines += file_covered;
                    
                    let file_percentage = if file_total > 0 {
                        (file_covered as f32 / file_total as f32) * 100.0
                    } else {
                        0.0
                    };
                    
                    file_coverage.insert(filename.clone(), FileCoverage {
                        filename: filename.clone(),
                        total_lines: file_total,
                        covered_lines: file_covered,
                        coverage_percentage: file_percentage,
                        missed_lines: file_missed,
                    });
                }
            }
        }
        
        let coverage_percentage = if total_lines > 0 {
            (covered_lines as f32 / total_lines as f32) * 100.0
        } else {
            0.0
        };
        
        Ok(CoverageReport {
            total_lines,
            covered_lines,
            coverage_percentage,
            file_coverage,
        })
    }

    /// Collect execution artifacts
    async fn collect_artifacts(&self, execution_dir: &PathBuf) -> Result<Vec<Artifact>> {
        let mut artifacts = Vec::new();
        
        // Collect log files
        if let Ok(mut entries) = fs::read_dir(execution_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                let artifact_type = match filename {
                    f if f.ends_with(".log") => Some(ArtifactType::Log),
                    "coverage.json" | "coverage.xml" => Some(ArtifactType::Coverage),
                    "pytest_report.xml" | "test_results.xml" => Some(ArtifactType::TestReport),
                    f if f.ends_with(".txt") || f.ends_with(".out") => Some(ArtifactType::Output),
                    _ => None,
                };
                
                if let Some(artifact_type) = artifact_type {
                    let metadata = fs::metadata(&path).await?;
                    artifacts.push(Artifact {
                        name: filename.to_string(),
                        path: path.clone(),
                        artifact_type,
                        size_bytes: metadata.len(),
                    });
                }
            }
        }
        
        debug!("Collected {} artifacts", artifacts.len());
        Ok(artifacts)
    }
}

#[async_trait::async_trait]
impl SandboxRunner for PythonSandboxRunner {
    fn language(&self) -> &str {
        "python"
    }

    fn supports_coverage(&self) -> bool {
        true
    }

    async fn execute(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        info!("Executing Python code in sandbox");
        
        let execution_dir = self.create_execution_environment(request, config).await?;
        
        let command = if let Some(custom_command) = &request.test_command {
            vec!["sh", "-c", custom_command]
        } else {
            vec!["python", "main.py"]
        };
        
        let result = self.run_docker_container(&execution_dir, &command, config).await?;
        
        // Cleanup
        let _ = fs::remove_dir_all(&execution_dir).await;
        
        Ok(result)
    }

    async fn run_tests(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        info!("Running Python tests in sandbox");
        
        let execution_dir = self.create_execution_environment(request, config).await?;
        
        // Create a test runner script
        let test_script = r#"#!/bin/bash
set -e

# Run tests with coverage
pytest --cov=. --cov-report=json --cov-report=term-missing --junit-xml=pytest_report.xml -v

# Generate coverage report
coverage report --show-missing

echo "Test execution completed"
"#;
        
        fs::write(execution_dir.join("run_tests.sh"), test_script).await?;
        
        let command = vec!["sh", "run_tests.sh"];
        let result = self.run_docker_container(&execution_dir, &command, config).await?;
        
        // Cleanup
        let _ = fs::remove_dir_all(&execution_dir).await;
        
        Ok(result)
    }
}

impl Default for PythonSandboxRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_python_execution() -> Result<()> {
        let runner = PythonSandboxRunner::new();
        let config = SandboxConfig::default();
        
        let request = ExecutionRequest {
            code: r#"
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

print(f"Fibonacci(10) = {fibonacci(10)}")
"#.to_string(),
            language: "python".to_string(),
            test_command: None,
            files: HashMap::new(),
            environment: HashMap::new(),
            working_directory: None,
        };
        
        let result = runner.execute(&request, &config).await?;
        
        assert!(result.success);
        assert!(result.stdout.contains("55")); // Fibonacci(10) = 55
        
        Ok(())
    }

    #[tokio::test]
    async fn test_python_test_execution() -> Result<()> {
        let runner = PythonSandboxRunner::new();
        let config = SandboxConfig::default();
        
        let mut files = HashMap::new();
        files.insert("test_fibonacci.py".to_string(), r#"
import pytest

def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

def test_fibonacci():
    assert fibonacci(0) == 0
    assert fibonacci(1) == 1
    assert fibonacci(10) == 55

def test_fibonacci_negative():
    assert fibonacci(-1) == -1
"#.to_string());
        
        let request = ExecutionRequest {
            code: "# Main file for testing".to_string(),
            language: "python".to_string(),
            test_command: None,
            files,
            environment: HashMap::new(),
            working_directory: None,
        };
        
        let result = runner.run_tests(&request, &config).await?;
        
        assert!(result.success);
        assert!(result.coverage.is_some());
        
        if let Some(coverage) = result.coverage {
            assert!(coverage.coverage_percentage >= 0.0);
        }
        
        Ok(())
    }

    #[test]
    fn test_requirements_generation() {
        let runner = PythonSandboxRunner::new();
        
        let code = r#"
import requests
import numpy as np
from pandas import DataFrame
import custom_module
"#;
        
        let requirements = runner.generate_requirements(code);
        
        assert!(requirements.contains("requests"));
        assert!(requirements.contains("numpy"));
        assert!(requirements.contains("pandas"));
        assert!(requirements.contains("pytest"));
    }
}