use super::*;
use anyhow::{anyhow, Result};
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};
use uuid::Uuid;

pub struct DockerRunner {
    image: String,
    container_prefix: String,
}

impl DockerRunner {
    pub fn new(image: String) -> Self {
        Self {
            image,
            container_prefix: "uaida".to_string(),
        }
    }

    pub async fn execute(&self, request: &ExecutionRequest, config: &SandboxConfig) -> Result<ExecutionResult> {
        let container_name = format!("{}_{}", self.container_prefix, Uuid::new_v4());
        let start_time = Instant::now();

        // Prepare Docker command
        let mut docker_cmd = Command::new("docker");
        docker_cmd
            .arg("run")
            .arg("--rm")
            .arg("--name")
            .arg(&container_name)
            .arg("--memory")
            .arg(&config.memory_limit)
            .arg("--cpus")
            .arg(config.cpu_limit.to_string());

        // Network settings
        if !config.network_enabled {
            docker_cmd.arg("--network").arg("none");
        }

        // Environment variables
        for (key, value) in &request.environment {
            docker_cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        // Working directory
        if let Some(workdir) = &request.working_directory {
            docker_cmd.arg("-w").arg(workdir);
        }

        // Image and command
        docker_cmd.arg(&self.image);

        // Split command into parts
        let command_parts = self.parse_command(&request.code);
        for part in command_parts {
            docker_cmd.arg(part);
        }

        debug!("Running Docker command: {:?}", docker_cmd);

        // Execute with timeout
        let execution_future = docker_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();

        let output = match timeout(config.timeout, execution_future).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(anyhow!("Docker execution failed: {}", e)),
            Err(_) => {
                // Timeout occurred, kill the container
                self.kill_container(&container_name).await?;
                return Err(anyhow!("Execution timed out after {:?}", config.timeout));
            }
        };

        let execution_time = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            execution_time,
            memory_used: None, // Would need additional Docker stats
            coverage: None,
            artifacts: Vec::new(),
        })
    }

    pub async fn execute_with_mount(
        &self,
        request: &ExecutionRequest,
        config: &SandboxConfig,
        host_dir: &PathBuf,
    ) -> Result<ExecutionResult> {
        let container_name = format!("{}_{}", self.container_prefix, Uuid::new_v4());
        let start_time = Instant::now();

        // Prepare Docker command with volume mount
        let mut docker_cmd = Command::new("docker");
        docker_cmd
            .arg("run")
            .arg("--rm")
            .arg("--name")
            .arg(&container_name)
            .arg("--memory")
            .arg(&config.memory_limit)
            .arg("--cpus")
            .arg(config.cpu_limit.to_string())
            .arg("-v")
            .arg(format!("{}:/app", host_dir.to_string_lossy()));

        // Network settings
        if !config.network_enabled {
            docker_cmd.arg("--network").arg("none");
        }

        // Environment variables
        for (key, value) in &request.environment {
            docker_cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        // Working directory
        docker_cmd.arg("-w").arg("/app");

        // Image and command
        docker_cmd.arg(&self.image);

        // Use bash to run complex commands
        docker_cmd.arg("bash").arg("-c").arg(&request.code);

        debug!("Running Docker command with mount: {:?}", docker_cmd);

        // Execute with timeout
        let execution_future = docker_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();

        let output = match timeout(config.timeout, execution_future).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(anyhow!("Docker execution failed: {}", e)),
            Err(_) => {
                // Timeout occurred, kill the container
                self.kill_container(&container_name).await?;
                return Err(anyhow!("Execution timed out after {:?}", config.timeout));
            }
        };

        let execution_time = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Collect artifacts from the mounted directory
        let artifacts = self.collect_artifacts(host_dir).await.unwrap_or_default();

        Ok(ExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            execution_time,
            memory_used: None,
            coverage: None,
            artifacts,
        })
    }

    async fn kill_container(&self, container_name: &str) -> Result<()> {
        let mut kill_cmd = Command::new("docker");
        kill_cmd.arg("kill").arg(container_name);
        
        match kill_cmd.output().await {
            Ok(_) => debug!("Killed container: {}", container_name),
            Err(e) => warn!("Failed to kill container {}: {}", container_name, e),
        }
        
        Ok(())
    }

    async fn collect_artifacts(&self, dir: &PathBuf) -> Result<Vec<Artifact>> {
        let mut artifacts = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(metadata) = entry.metadata().await {
                        let artifact_type = self.determine_artifact_type(&path);
                        
                        artifacts.push(Artifact {
                            name: path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path: path.clone(),
                            artifact_type,
                            size_bytes: metadata.len(),
                        });
                    }
                }
            }
        }

        Ok(artifacts)
    }

    fn determine_artifact_type(&self, path: &PathBuf) -> ArtifactType {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension {
                "log" => ArtifactType::Log,
                "xml" | "json" if path.to_string_lossy().contains("coverage") => ArtifactType::Coverage,
                "xml" | "json" if path.to_string_lossy().contains("test") => ArtifactType::TestReport,
                "exe" | "bin" => ArtifactType::Binary,
                _ => ArtifactType::Output,
            }
        } else {
            ArtifactType::Output
        }
    }

    fn parse_command(&self, command: &str) -> Vec<String> {
        // Simple command parsing - in production, use a proper shell parser
        command
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    pub async fn check_docker_available(&self) -> Result<bool> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    pub async fn pull_image(&self) -> Result<()> {
        debug!("Pulling Docker image: {}", self.image);
        
        let output = Command::new("docker")
            .arg("pull")
            .arg(&self.image)
            .output()
            .await?;

        if output.status.success() {
            debug!("Successfully pulled image: {}", self.image);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to pull image {}: {}", self.image, stderr))
        }
    }
}