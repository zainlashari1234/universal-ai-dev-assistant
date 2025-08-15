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

pub struct NodeSandboxRunner {
    docker_image: String,
}

impl NodeSandboxRunner {
    pub fn new() -> Self {
        Self {
            docker_image: "node:18-slim".to_string(),
        }
    }

    pub fn with_image(docker_image: String) -> Self {
        Self { docker_image }
    }

    /// Create isolated execution environment for Node.js
    async fn create_execution_environment(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<PathBuf> {
        let execution_id = Uuid::new_v4().to_string();
        let execution_dir = config.temp_dir.join(format!("node_{}", execution_id));
        
        // Create execution directory
        fs::create_dir_all(&execution_dir).await?;
        
        // Write main code file
        let main_file = execution_dir.join("index.js");
        fs::write(&main_file, &request.code).await?;
        
        // Write additional files
        for (filename, content) in &request.files {
            let file_path = execution_dir.join(filename);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&file_path, content).await?;
        }
        
        // Create package.json if not provided
        if !request.files.contains_key("package.json") {
            let package_json = self.generate_package_json(&request.code);
            fs::write(execution_dir.join("package.json"), package_json).await?;
        }
        
        // Create Dockerfile for this execution
        let dockerfile_content = self.generate_dockerfile(&request.environment);
        fs::write(execution_dir.join("Dockerfile"), dockerfile_content).await?;
        
        debug!("Created Node.js execution environment at: {:?}", execution_dir);
        Ok(execution_dir)
    }

    /// Generate package.json based on code analysis
    fn generate_package_json(&self, code: &str) -> String {
        let mut dependencies = HashMap::new();
        let mut dev_dependencies = HashMap::new();
        
        // Basic testing dependencies
        dev_dependencies.insert("jest", "^29.0.0");
        dev_dependencies.insert("@jest/globals", "^29.0.0");
        dev_dependencies.insert("c8", "^7.0.0"); // Coverage tool
        
        // Analyze imports/requires and add common packages
        let lines: Vec<&str> = code.lines().collect();
        for line in lines {
            let trimmed = line.trim();
            
            // Check for require statements
            if let Some(package) = self.extract_require_package(trimmed) {
                match package.as_str() {
                    "express" => { dependencies.insert("express", "^4.18.0"); }
                    "axios" => { dependencies.insert("axios", "^1.0.0"); }
                    "lodash" => { dependencies.insert("lodash", "^4.17.0"); }
                    "moment" => { dependencies.insert("moment", "^2.29.0"); }
                    "uuid" => { dependencies.insert("uuid", "^9.0.0"); }
                    "fs-extra" => { dependencies.insert("fs-extra", "^11.0.0"); }
                    "dotenv" => { dependencies.insert("dotenv", "^16.0.0"); }
                    _ => {}
                }
            }
            
            // Check for ES6 imports
            if let Some(package) = self.extract_import_package(trimmed) {
                match package.as_str() {
                    "express" => { dependencies.insert("express", "^4.18.0"); }
                    "axios" => { dependencies.insert("axios", "^1.0.0"); }
                    "lodash" => { dependencies.insert("lodash", "^4.17.0"); }
                    "uuid" => { dependencies.insert("uuid", "^9.0.0"); }
                    "fs-extra" => { dependencies.insert("fs-extra", "^11.0.0"); }
                    "dotenv" => { dependencies.insert("dotenv", "^16.0.0"); }
                    _ => {}
                }
            }
        }
        
        let deps_json = if dependencies.is_empty() {
            "{}".to_string()
        } else {
            serde_json::to_string_pretty(&dependencies).unwrap_or("{}".to_string())
        };
        
        let dev_deps_json = serde_json::to_string_pretty(&dev_dependencies).unwrap_or("{}".to_string());
        
        format!(
            r#"{{
  "name": "uaida-execution",
  "version": "1.0.0",
  "description": "UAIDA sandbox execution environment",
  "main": "index.js",
  "scripts": {{
    "start": "node index.js",
    "test": "jest --coverage --coverageReporters=json --coverageReporters=text",
    "test:watch": "jest --watch",
    "coverage": "c8 --reporter=json --reporter=text npm test"
  }},
  "dependencies": {},
  "devDependencies": {},
  "jest": {{
    "testEnvironment": "node",
    "collectCoverageFrom": [
      "**/*.js",
      "!node_modules/**",
      "!coverage/**"
    ],
    "coverageDirectory": "coverage",
    "coverageReporters": ["json", "text", "html"]
  }}
}}"#,
            deps_json, dev_deps_json
        )
    }

    /// Extract package name from require statement
    fn extract_require_package(&self, line: &str) -> Option<String> {
        if line.contains("require(") {
            // Extract package name from require('package') or require("package")
            if let Some(start) = line.find("require(") {
                let after_require = &line[start + 8..];
                if let Some(quote_start) = after_require.find(['\'', '"']) {
                    let quote_char = after_require.chars().nth(quote_start).unwrap();
                    let after_quote = &after_require[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find(quote_char) {
                        let package_name = &after_quote[..quote_end];
                        // Only return if it's not a relative path
                        if !package_name.starts_with('.') && !package_name.starts_with('/') {
                            return Some(package_name.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract package name from ES6 import statement
    fn extract_import_package(&self, line: &str) -> Option<String> {
        if line.starts_with("import ") && line.contains(" from ") {
            if let Some(from_pos) = line.find(" from ") {
                let after_from = &line[from_pos + 6..].trim();
                if let Some(quote_start) = after_from.find(['\'', '"']) {
                    let quote_char = after_from.chars().nth(quote_start).unwrap();
                    let after_quote = &after_from[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find(quote_char) {
                        let package_name = &after_quote[..quote_end];
                        // Only return if it's not a relative path
                        if !package_name.starts_with('.') && !package_name.starts_with('/') {
                            return Some(package_name.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Generate Dockerfile for Node.js execution
    fn generate_dockerfile(&self, environment: &HashMap<String, String>) -> String {
        let mut dockerfile = format!(
            r#"FROM {}

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci --only=production --silent

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
        
        dockerfile.push_str("\n# Default command\nCMD [\"npm\", \"start\"]\n");
        
        dockerfile
    }

    /// Execute code in Docker container with resource limits
    async fn run_docker_container(&self, execution_dir: &PathBuf, command: &[&str], config: &SandboxConfig) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        // Record execution metrics
        let metrics = get_metrics();
        metrics.agent_step_duration_ms
            .with_label_values(&["sandbox", "node_execute"])
            .observe(0.0); // Will update at the end
        
        // Build Docker image
        let image_tag = format!("uaida-node-{}", Uuid::new_v4());
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
            "--tmpfs=/app/node_modules:rw,noexec,nosuid,size=200m", // Node modules
            "--memory", &config.memory_limit,
            "--cpus", &config.cpu_limit.to_string(),
            "--ulimit", "nproc=64:64", // Limit processes
            "--ulimit", "fsize=10485760:10485760", // Limit file size to 10MB
            "--security-opt=no-new-privileges", // Security
        ];
        
        // Add timeout
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
            .with_label_values(&["sandbox", "node_execute"])
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
        
        info!("Node.js execution completed in {:?} (success: {})", 
              execution_time, result.success);
        
        Ok(result)
    }

    /// Parse coverage report from Jest output
    async fn parse_coverage_report(&self, execution_dir: &PathBuf) -> Result<CoverageReport> {
        let coverage_file = execution_dir.join("coverage").join("coverage-final.json");
        
        if !coverage_file.exists() {
            return Err(anyhow!("Coverage report not found"));
        }
        
        let coverage_content = fs::read_to_string(&coverage_file).await?;
        let coverage_data: Value = serde_json::from_str(&coverage_content)?;
        
        let mut total_lines = 0;
        let mut covered_lines = 0;
        let mut file_coverage = HashMap::new();
        
        if let Some(files) = coverage_data.as_object() {
            for (filename, file_data) in files {
                if let Some(s) = file_data["s"].as_object() { // Statement coverage
                    let file_total = s.len();
                    let file_covered = s.values().filter(|v| v.as_u64().unwrap_or(0) > 0).count();
                    
                    // Get uncovered lines
                    let uncovered_lines: Vec<usize> = if let Some(statement_map) = file_data["statementMap"].as_object() {
                        s.iter()
                            .filter(|(_, count)| count.as_u64().unwrap_or(0) == 0)
                            .filter_map(|(stmt_id, _)| {
                                statement_map.get(stmt_id)?
                                    .get("start")?
                                    .get("line")?
                                    .as_u64()
                                    .map(|n| n as usize)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    
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
                        missed_lines: uncovered_lines,
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
        
        // Collect files from execution directory and subdirectories
        let paths_to_check = vec![
            execution_dir.clone(),
            execution_dir.join("coverage"),
        ];
        
        for base_path in paths_to_check {
            if let Ok(mut entries) = fs::read_dir(&base_path).await {
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    let artifact_type = match filename {
                        f if f.ends_with(".log") => Some(ArtifactType::Log),
                        "coverage-final.json" | "lcov.info" => Some(ArtifactType::Coverage),
                        "jest-report.xml" | "test-results.xml" => Some(ArtifactType::TestReport),
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
        }
        
        debug!("Collected {} artifacts", artifacts.len());
        Ok(artifacts)
    }
}

#[async_trait::async_trait]
impl SandboxRunner for NodeSandboxRunner {
    fn language(&self) -> &str {
        "javascript"
    }

    fn supports_coverage(&self) -> bool {
        true
    }

    async fn execute(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        info!("Executing Node.js code in sandbox");
        
        let execution_dir = self.create_execution_environment(request, config).await?;
        
        let command = if let Some(custom_command) = &request.test_command {
            vec!["sh", "-c", custom_command]
        } else {
            vec!["npm", "start"]
        };
        
        let result = self.run_docker_container(&execution_dir, &command, config).await?;
        
        // Cleanup
        let _ = fs::remove_dir_all(&execution_dir).await;
        
        Ok(result)
    }

    async fn run_tests(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        info!("Running Node.js tests in sandbox");
        
        let execution_dir = self.create_execution_environment(request, config).await?;
        
        let command = vec!["npm", "test"];
        let result = self.run_docker_container(&execution_dir, &command, config).await?;
        
        // Cleanup
        let _ = fs::remove_dir_all(&execution_dir).await;
        
        Ok(result)
    }
}

impl Default for NodeSandboxRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_node_execution() -> Result<()> {
        let runner = NodeSandboxRunner::new();
        let config = SandboxConfig::default();
        
        let request = ExecutionRequest {
            code: r#"
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log(`Fibonacci(10) = ${fibonacci(10)}`);
"#.to_string(),
            language: "javascript".to_string(),
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
    async fn test_node_test_execution() -> Result<()> {
        let runner = NodeSandboxRunner::new();
        let config = SandboxConfig::default();
        
        let mut files = HashMap::new();
        files.insert("fibonacci.js".to_string(), r#"
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

module.exports = { fibonacci };
"#.to_string());
        
        files.insert("fibonacci.test.js".to_string(), r#"
const { fibonacci } = require('./fibonacci');

describe('Fibonacci Tests', () => {
    test('fibonacci(0) should return 0', () => {
        expect(fibonacci(0)).toBe(0);
    });
    
    test('fibonacci(1) should return 1', () => {
        expect(fibonacci(1)).toBe(1);
    });
    
    test('fibonacci(10) should return 55', () => {
        expect(fibonacci(10)).toBe(55);
    });
});
"#.to_string());
        
        let request = ExecutionRequest {
            code: "// Main file for testing".to_string(),
            language: "javascript".to_string(),
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
    fn test_package_extraction() {
        let runner = NodeSandboxRunner::new();
        
        // Test require extraction
        assert_eq!(runner.extract_require_package("const express = require('express');"), Some("express".to_string()));
        assert_eq!(runner.extract_require_package("const axios = require(\"axios\");"), Some("axios".to_string()));
        assert_eq!(runner.extract_require_package("const local = require('./local');"), None);
        
        // Test import extraction
        assert_eq!(runner.extract_import_package("import express from 'express';"), Some("express".to_string()));
        assert_eq!(runner.extract_import_package("import { Router } from 'express';"), Some("express".to_string()));
        assert_eq!(runner.extract_import_package("import local from './local';"), None);
    }

    #[test]
    fn test_package_json_generation() {
        let runner = NodeSandboxRunner::new();
        
        let code = r#"
const express = require('express');
import axios from 'axios';
const lodash = require('lodash');
"#;
        
        let package_json = runner.generate_package_json(code);
        
        assert!(package_json.contains("express"));
        assert!(package_json.contains("axios"));
        assert!(package_json.contains("lodash"));
        assert!(package_json.contains("jest"));
    }
}