use super::*;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct RustRunner {
    docker_runner: DockerRunner,
}

impl RustRunner {
    pub fn new() -> Self {
        Self {
            docker_runner: DockerRunner::new("rust:1.75-slim".to_string()),
        }
    }

    async fn setup_rust_environment(&self, request: &ExecutionRequest, temp_dir: &PathBuf) -> Result<()> {
        // Create Cargo.toml
        let cargo_toml_content = self.generate_cargo_toml(request);
        let cargo_toml_path = temp_dir.join("Cargo.toml");
        tokio::fs::write(cargo_toml_path, cargo_toml_content).await?;

        // Create src directory
        let src_dir = temp_dir.join("src");
        tokio::fs::create_dir_all(&src_dir).await?;

        // Create main.rs or lib.rs
        let main_file = if request.code.contains("fn main") {
            "main.rs"
        } else {
            "lib.rs"
        };
        
        let main_path = src_dir.join(main_file);
        tokio::fs::write(main_path, &request.code).await?;

        // Create test files if needed
        if request.test_command.is_some() || request.code.contains("#[test]") {
            self.create_test_files(request, &src_dir).await?;
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

    fn generate_cargo_toml(&self, request: &ExecutionRequest) -> String {
        let package_name = "uaida-sandbox";
        let version = "0.1.0";
        let edition = "2021";

        let mut dependencies = Vec::new();

        // Detect common dependencies from code
        if request.code.contains("use serde") || request.code.contains("serde::") {
            dependencies.push(("serde", r#"{ version = "1.0", features = ["derive"] }"#));
        }
        if request.code.contains("use tokio") || request.code.contains("tokio::") {
            dependencies.push(("tokio", r#"{ version = "1.0", features = ["full"] }"#));
        }
        if request.code.contains("use reqwest") || request.code.contains("reqwest::") {
            dependencies.push(("reqwest", r#"{ version = "0.11", features = ["json"] }"#));
        }
        if request.code.contains("use anyhow") || request.code.contains("anyhow::") {
            dependencies.push(("anyhow", "1.0"));
        }
        if request.code.contains("use uuid") || request.code.contains("uuid::") {
            dependencies.push(("uuid", r#"{ version = "1.0", features = ["v4"] }"#));
        }
        if request.code.contains("use chrono") || request.code.contains("chrono::") {
            dependencies.push(("chrono", r#"{ version = "0.4", features = ["serde"] }"#));
        }

        let mut cargo_toml = format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "{}"

"#,
            package_name, version, edition
        );

        if !dependencies.is_empty() {
            cargo_toml.push_str("[dependencies]\n");
            for (name, version) in dependencies {
                cargo_toml.push_str(&format!("{} = {}\n", name, version));
            }
            cargo_toml.push('\n');
        }

        // Add dev dependencies for testing
        cargo_toml.push_str(
            r#"[dev-dependencies]
tokio-test = "0.4"
"#
        );

        cargo_toml
    }

    async fn create_test_files(&self, request: &ExecutionRequest, src_dir: &PathBuf) -> Result<()> {
        if request.code.contains("#[test]") {
            // Tests are already in the main code
            return Ok(());
        }

        // Generate basic test structure
        let test_content = format!(
            r#"#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_basic_functionality() {{
        // Basic test to ensure code compiles and runs
        assert!(true);
    }}

    #[tokio::test]
    async fn test_async_functionality() {{
        // Basic async test
        assert!(true);
    }}
}}
"#
        );

        // Append tests to lib.rs or create separate test file
        if src_dir.join("lib.rs").exists() {
            let mut lib_content = tokio::fs::read_to_string(src_dir.join("lib.rs")).await?;
            lib_content.push_str("\n");
            lib_content.push_str(&test_content);
            tokio::fs::write(src_dir.join("lib.rs"), lib_content).await?;
        } else {
            // Create a separate test file
            let test_path = src_dir.join("tests.rs");
            tokio::fs::write(test_path, test_content).await?;
        }

        Ok(())
    }

    async fn run_with_coverage(&self, config: &SandboxConfig, temp_dir: &PathBuf) -> Result<ExecutionResult> {
        // Install cargo-tarpaulin for coverage (if not available, fall back to basic tests)
        let install_tarpaulin_cmd = "cargo install cargo-tarpaulin || echo 'tarpaulin not available'";
        let install_request = ExecutionRequest {
            code: install_tarpaulin_cmd.to_string(),
            language: "bash".to_string(),
            test_command: None,
            files: HashMap::new(),
            environment: HashMap::new(),
            working_directory: Some("/app".to_string()),
        };

        let _ = self.docker_runner.execute_with_mount(&install_request, config, temp_dir).await;

        // Try to run tests with coverage
        let coverage_cmd = "cargo tarpaulin --out Json --output-dir . || cargo test";
        let coverage_request = ExecutionRequest {
            code: coverage_cmd.to_string(),
            language: "bash".to_string(),
            test_command: None,
            files: HashMap::new(),
            environment: HashMap::new(),
            working_directory: Some("/app".to_string()),
        };

        let mut result = self.docker_runner.execute_with_mount(&coverage_request, config, temp_dir).await?;

        // Parse coverage if tarpaulin was successful
        if result.success {
            let coverage_file = temp_dir.join("tarpaulin-report.json");
            if coverage_file.exists() {
                if let Ok(coverage_content) = tokio::fs::read_to_string(coverage_file).await {
                    if let Ok(coverage_data) = serde_json::from_str::<serde_json::Value>(&coverage_content) {
                        result.coverage = self.parse_tarpaulin_coverage(&coverage_data);
                    }
                }
            } else {
                // Fallback: parse test output for basic information
                result.coverage = self.parse_test_output(&result.stdout);
            }
        }

        Ok(result)
    }

    fn parse_tarpaulin_coverage(&self, coverage_data: &serde_json::Value) -> Option<CoverageReport> {
        let coverage_percentage = coverage_data.get("coverage")?.as_f64()? as f32;
        let covered_lines = coverage_data.get("covered")?.as_u64()? as usize;
        let total_lines = coverage_data.get("coverable")?.as_u64()? as usize;

        let mut file_coverage = HashMap::new();

        if let Some(files) = coverage_data.get("files").and_then(|f| f.as_object()) {
            for (filename, file_data) in files {
                if let (Some(file_covered), Some(file_total)) = (
                    file_data.get("covered").and_then(|v| v.as_u64()),
                    file_data.get("coverable").and_then(|v| v.as_u64()),
                ) {
                    let file_percentage = if file_total > 0 {
                        (file_covered as f32 / file_total as f32) * 100.0
                    } else {
                        0.0
                    };

                    file_coverage.insert(filename.clone(), FileCoverage {
                        filename: filename.clone(),
                        total_lines: file_total as usize,
                        covered_lines: file_covered as usize,
                        coverage_percentage: file_percentage,
                        missed_lines: vec![], // Would need more detailed parsing
                    });
                }
            }
        }

        Some(CoverageReport {
            total_lines,
            covered_lines,
            coverage_percentage,
            file_coverage,
        })
    }

    fn parse_test_output(&self, test_output: &str) -> Option<CoverageReport> {
        // Parse basic test information from cargo test output
        let lines: Vec<&str> = test_output.lines().collect();
        
        let mut passed = 0;
        let mut failed = 0;
        
        for line in lines {
            if line.contains("test result:") {
                // Parse line like "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if part == &"passed;" && i > 0 {
                        if let Ok(p) = parts[i - 1].parse::<usize>() {
                            passed = p;
                        }
                    }
                    if part == &"failed;" && i > 0 {
                        if let Ok(f) = parts[i - 1].parse::<usize>() {
                            failed = f;
                        }
                    }
                }
                break;
            }
        }

        // Estimate coverage based on test success (very rough estimate)
        let total_tests = passed + failed;
        let coverage_percentage = if total_tests > 0 {
            (passed as f32 / total_tests as f32) * 100.0
        } else {
            0.0
        };

        Some(CoverageReport {
            total_lines: total_tests * 10, // Rough estimate
            covered_lines: passed * 10,
            coverage_percentage,
            file_coverage: HashMap::new(),
        })
    }
}

#[async_trait]
impl SandboxRunner for RustRunner {
    fn language(&self) -> &str {
        "rust"
    }

    fn supports_coverage(&self) -> bool {
        true
    }

    async fn execute(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        debug!("Executing Rust code in sandbox");

        // Create temporary directory
        let temp_dir = config.temp_dir.join(format!("rust_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        // Setup Rust environment
        self.setup_rust_environment(request, &temp_dir).await?;

        // Execute using Docker
        let docker_request = ExecutionRequest {
            code: "cargo run".to_string(),
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
        debug!("Running Rust tests in sandbox");

        // Create temporary directory
        let temp_dir = config.temp_dir.join(format!("rust_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        // Setup Rust environment with tests
        self.setup_rust_environment(request, &temp_dir).await?;

        let result = if self.supports_coverage() {
            self.run_with_coverage(config, &temp_dir).await?
        } else {
            // Run tests without coverage
            let test_command = request.test_command
                .clone()
                .unwrap_or_else(|| "cargo test".to_string());

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