pub mod python;
pub mod node;
pub mod rust;
pub mod go;
pub mod rust_runner;
pub mod docker_runner;

pub use python::*;
pub use node::*;
pub use rust::*;
pub use go::*;
pub use rust_runner::*;
pub use docker_runner::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub timeout: Duration,
    pub memory_limit: String,
    pub cpu_limit: f32,
    pub network_enabled: bool,
    pub temp_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub code: String,
    pub language: String,
    pub test_command: Option<String>,
    pub files: HashMap<String, String>, // filename -> content
    pub environment: HashMap<String, String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub memory_used: Option<u64>,
    pub coverage: Option<CoverageReport>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub total_lines: usize,
    pub covered_lines: usize,
    pub coverage_percentage: f32,
    pub file_coverage: HashMap<String, FileCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub filename: String,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub coverage_percentage: f32,
    pub missed_lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: PathBuf,
    pub artifact_type: ArtifactType,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    Log,
    Coverage,
    TestReport,
    Binary,
    Output,
}

pub trait SandboxRunner: Send + Sync {
    fn language(&self) -> &str;
    fn supports_coverage(&self) -> bool;
    async fn execute(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult>;
    async fn run_tests(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult>;
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            memory_limit: "512m".to_string(),
            cpu_limit: 1.0,
            network_enabled: false,
            temp_dir: std::env::temp_dir().join("uaida_sandbox"),
        }
    }
}