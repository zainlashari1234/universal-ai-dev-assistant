use super::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildDoctorAgent {
    supported_package_managers: Vec<PackageManager>,
    dependency_cache: HashMap<String, DependencyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Pip,
    Cargo,
    Maven,
    Gradle,
    Go,
    Composer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildAnalysisRequest {
    pub project_path: PathBuf,
    pub language: String,
    pub package_manager: Option<PackageManager>,
    pub build_command: Option<String>,
    pub target_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildAnalysisResponse {
    pub build_status: BuildStatus,
    pub dependency_conflicts: Vec<DependencyConflict>,
    pub build_failures: Vec<BuildFailure>,
    pub recommendations: Vec<BuildRecommendation>,
    pub fixes: Vec<BuildFix>,
    pub performance_metrics: BuildMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildStatus {
    Success,
    Warning,
    Failure,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    pub package_name: String,
    pub conflicting_versions: Vec<String>,
    pub required_by: Vec<String>,
    pub severity: ConflictSeverity,
    pub resolution_strategy: ResolutionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Critical,
    Major,
    Minor,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    UpdateToLatest,
    UpdateToCompatible,
    Downgrade,
    AddOverride,
    RemoveDuplicate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFailure {
    pub failure_type: BuildFailureType,
    pub error_message: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub suggested_fix: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildFailureType {
    MissingDependency,
    VersionConflict,
    SyntaxError,
    ConfigurationError,
    EnvironmentError,
    NetworkError,
    PermissionError,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecommendation {
    pub priority: RecommendationPriority,
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub estimated_impact: ImpactLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationCategory {
    Performance,
    Security,
    Maintainability,
    Compatibility,
    BestPractices,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFix {
    pub fix_id: String,
    pub fix_type: BuildFixType,
    pub description: String,
    pub commands: Vec<String>,
    pub file_changes: Vec<FileChange>,
    pub validation_command: Option<String>,
    pub rollback_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildFixType {
    DependencyUpdate,
    ConfigurationChange,
    FileModification,
    EnvironmentSetup,
    CommandExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub old_content: Option<String>,
    pub new_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
    Append,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetrics {
    pub build_time_ms: u64,
    pub dependency_count: usize,
    pub outdated_dependencies: usize,
    pub security_vulnerabilities: usize,
    pub cache_hit_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub is_outdated: bool,
    pub security_advisories: Vec<SecurityAdvisory>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAdvisory {
    pub id: String,
    pub severity: String,
    pub description: String,
    pub patched_versions: Vec<String>,
}

impl BuildDoctorAgent {
    pub fn new() -> Self {
        Self {
            supported_package_managers: vec![
                PackageManager::Npm,
                PackageManager::Pip,
                PackageManager::Cargo,
                PackageManager::Maven,
                PackageManager::Go,
            ],
            dependency_cache: HashMap::new(),
        }
    }

    pub async fn analyze_build(&self, request: &BuildAnalysisRequest) -> Result<BuildAnalysisResponse> {
        let start_time = Instant::now();
        info!("Starting build analysis for {} project", request.language);

        // Detect package manager if not specified
        let package_manager = if let Some(pm) = &request.package_manager {
            pm.clone()
        } else {
            self.detect_package_manager(&request.project_path, &request.language).await?
        };

        // Analyze dependencies
        let dependency_conflicts = self.analyze_dependencies(&request.project_path, &package_manager).await?;

        // Run build and analyze failures
        let (build_status, build_failures) = self.analyze_build_failures(&request.project_path, &package_manager, &request.build_command).await?;

        // Generate recommendations
        let recommendations = self.generate_build_recommendations(&dependency_conflicts, &build_failures);

        // Generate fixes
        let fixes = self.generate_build_fixes(&dependency_conflicts, &build_failures, &package_manager).await?;

        // Calculate metrics
        let build_time = start_time.elapsed().as_millis() as u64;
        let performance_metrics = BuildMetrics {
            build_time_ms: build_time,
            dependency_count: self.count_dependencies(&request.project_path, &package_manager).await.unwrap_or(0),
            outdated_dependencies: dependency_conflicts.len(),
            security_vulnerabilities: 0, // Would be calculated from security scan
            cache_hit_rate: 0.8, // Placeholder
        };

        Ok(BuildAnalysisResponse {
            build_status,
            dependency_conflicts,
            build_failures,
            recommendations,
            fixes,
            performance_metrics,
        })
    }

    async fn detect_package_manager(&self, project_path: &PathBuf, language: &str) -> Result<PackageManager> {
        // Check for package manager files
        let package_files = [
            ("package.json", PackageManager::Npm),
            ("requirements.txt", PackageManager::Pip),
            ("Pipfile", PackageManager::Pip),
            ("Cargo.toml", PackageManager::Cargo),
            ("pom.xml", PackageManager::Maven),
            ("build.gradle", PackageManager::Gradle),
            ("go.mod", PackageManager::Go),
            ("composer.json", PackageManager::Composer),
        ];

        for (file_name, pm) in package_files {
            let file_path = project_path.join(file_name);
            if file_path.exists() {
                return Ok(pm);
            }
        }

        // Fallback based on language
        match language {
            "javascript" | "typescript" => Ok(PackageManager::Npm),
            "python" => Ok(PackageManager::Pip),
            "rust" => Ok(PackageManager::Cargo),
            "java" => Ok(PackageManager::Maven),
            "go" => Ok(PackageManager::Go),
            _ => Err(anyhow::anyhow!("Cannot detect package manager for language: {}", language)),
        }
    }

    async fn analyze_dependencies(&self, project_path: &PathBuf, package_manager: &PackageManager) -> Result<Vec<DependencyConflict>> {
        let mut conflicts = Vec::new();

        match package_manager {
            PackageManager::Npm => {
                conflicts.extend(self.analyze_npm_dependencies(project_path).await?);
            }
            PackageManager::Pip => {
                conflicts.extend(self.analyze_pip_dependencies(project_path).await?);
            }
            PackageManager::Cargo => {
                conflicts.extend(self.analyze_cargo_dependencies(project_path).await?);
            }
            PackageManager::Maven => {
                conflicts.extend(self.analyze_maven_dependencies(project_path).await?);
            }
            _ => {
                debug!("Dependency analysis not implemented for {:?}", package_manager);
            }
        }

        Ok(conflicts)
    }

    async fn analyze_npm_dependencies(&self, project_path: &PathBuf) -> Result<Vec<DependencyConflict>> {
        let mut conflicts = Vec::new();

        // Run npm ls to check for conflicts
        let output = Command::new("npm")
            .arg("ls")
            .arg("--json")
            .current_dir(project_path)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(npm_tree) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    conflicts.extend(self.parse_npm_conflicts(&npm_tree));
                }
            }
            Err(e) => {
                debug!("Failed to run npm ls: {}", e);
            }
        }

        // Check for outdated packages
        let outdated_output = Command::new("npm")
            .arg("outdated")
            .arg("--json")
            .current_dir(project_path)
            .output()
            .await;

        if let Ok(output) = outdated_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(outdated) = serde_json::from_str::<serde_json::Value>(&stdout) {
                conflicts.extend(self.parse_npm_outdated(&outdated));
            }
        }

        Ok(conflicts)
    }

    fn parse_npm_conflicts(&self, npm_tree: &serde_json::Value) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();

        if let Some(problems) = npm_tree.get("problems").and_then(|p| p.as_array()) {
            for problem in problems {
                if let Some(problem_str) = problem.as_str() {
                    if problem_str.contains("peer dep missing") || problem_str.contains("ERESOLVE") {
                        // Parse the conflict information
                        conflicts.push(DependencyConflict {
                            package_name: "unknown".to_string(),
                            conflicting_versions: vec!["unknown".to_string()],
                            required_by: vec!["npm".to_string()],
                            severity: ConflictSeverity::Major,
                            resolution_strategy: ResolutionStrategy::UpdateToCompatible,
                        });
                    }
                }
            }
        }

        conflicts
    }

    fn parse_npm_outdated(&self, outdated: &serde_json::Value) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();

        if let Some(packages) = outdated.as_object() {
            for (package_name, info) in packages {
                if let Some(current) = info.get("current").and_then(|v| v.as_str()) {
                    if let Some(latest) = info.get("latest").and_then(|v| v.as_str()) {
                        if current != latest {
                            conflicts.push(DependencyConflict {
                                package_name: package_name.clone(),
                                conflicting_versions: vec![current.to_string(), latest.to_string()],
                                required_by: vec!["package.json".to_string()],
                                severity: ConflictSeverity::Minor,
                                resolution_strategy: ResolutionStrategy::UpdateToLatest,
                            });
                        }
                    }
                }
            }
        }

        conflicts
    }

    async fn analyze_pip_dependencies(&self, project_path: &PathBuf) -> Result<Vec<DependencyConflict>> {
        let mut conflicts = Vec::new();

        // Check for pip conflicts using pip check
        let output = Command::new("pip")
            .arg("check")
            .current_dir(project_path)
            .output()
            .await;

        if let Ok(output) = output {
            let stderr = String::from_utf8_lossy(&output.stderr);
            conflicts.extend(self.parse_pip_conflicts(&stderr));
        }

        Ok(conflicts)
    }

    fn parse_pip_conflicts(&self, pip_output: &str) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();

        for line in pip_output.lines() {
            if line.contains("has requirement") && line.contains("but you have") {
                // Parse pip conflict line
                // Example: "package 1.0.0 has requirement dependency>=2.0.0, but you have dependency 1.5.0"
                conflicts.push(DependencyConflict {
                    package_name: "unknown".to_string(),
                    conflicting_versions: vec!["unknown".to_string()],
                    required_by: vec!["pip".to_string()],
                    severity: ConflictSeverity::Major,
                    resolution_strategy: ResolutionStrategy::UpdateToCompatible,
                });
            }
        }

        conflicts
    }

    async fn analyze_cargo_dependencies(&self, project_path: &PathBuf) -> Result<Vec<DependencyConflict>> {
        let mut conflicts = Vec::new();

        // Run cargo tree to check for conflicts
        let output = Command::new("cargo")
            .arg("tree")
            .arg("--duplicates")
            .current_dir(project_path)
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            conflicts.extend(self.parse_cargo_conflicts(&stdout));
        }

        Ok(conflicts)
    }

    fn parse_cargo_conflicts(&self, cargo_output: &str) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();

        for line in cargo_output.lines() {
            if line.contains("(*)") {
                // Cargo marks duplicate dependencies with (*)
                conflicts.push(DependencyConflict {
                    package_name: "duplicate_crate".to_string(),
                    conflicting_versions: vec!["multiple".to_string()],
                    required_by: vec!["Cargo.toml".to_string()],
                    severity: ConflictSeverity::Minor,
                    resolution_strategy: ResolutionStrategy::UpdateToCompatible,
                });
            }
        }

        conflicts
    }

    async fn analyze_maven_dependencies(&self, project_path: &PathBuf) -> Result<Vec<DependencyConflict>> {
        let mut conflicts = Vec::new();

        // Run mvn dependency:tree to check for conflicts
        let output = Command::new("mvn")
            .arg("dependency:tree")
            .arg("-Dverbose")
            .current_dir(project_path)
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            conflicts.extend(self.parse_maven_conflicts(&stdout));
        }

        Ok(conflicts)
    }

    fn parse_maven_conflicts(&self, maven_output: &str) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();

        for line in maven_output.lines() {
            if line.contains("omitted for conflict") || line.contains("omitted for duplicate") {
                conflicts.push(DependencyConflict {
                    package_name: "maven_artifact".to_string(),
                    conflicting_versions: vec!["conflict".to_string()],
                    required_by: vec!["pom.xml".to_string()],
                    severity: ConflictSeverity::Major,
                    resolution_strategy: ResolutionStrategy::UpdateToCompatible,
                });
            }
        }

        conflicts
    }

    async fn analyze_build_failures(&self, project_path: &PathBuf, package_manager: &PackageManager, build_command: &Option<String>) -> Result<(BuildStatus, Vec<BuildFailure>)> {
        let mut failures = Vec::new();

        let command = if let Some(cmd) = build_command {
            cmd.clone()
        } else {
            self.get_default_build_command(package_manager)
        };

        // Run the build command
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(project_path)
            .output()
            .await;

        let build_status = match output {
            Ok(output) if output.status.success() => BuildStatus::Success,
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                failures.extend(self.parse_build_failures(&stderr, package_manager));
                BuildStatus::Failure
            }
            Err(_) => BuildStatus::Unknown,
        };

        Ok((build_status, failures))
    }

    fn get_default_build_command(&self, package_manager: &PackageManager) -> String {
        match package_manager {
            PackageManager::Npm => "npm run build".to_string(),
            PackageManager::Pip => "python -m build".to_string(),
            PackageManager::Cargo => "cargo build".to_string(),
            PackageManager::Maven => "mvn compile".to_string(),
            PackageManager::Gradle => "gradle build".to_string(),
            PackageManager::Go => "go build".to_string(),
            PackageManager::Composer => "composer install".to_string(),
        }
    }

    fn parse_build_failures(&self, error_output: &str, package_manager: &PackageManager) -> Vec<BuildFailure> {
        let mut failures = Vec::new();

        for line in error_output.lines() {
            if let Some(failure) = self.classify_build_error(line, package_manager) {
                failures.push(failure);
            }
        }

        failures
    }

    fn classify_build_error(&self, error_line: &str, package_manager: &PackageManager) -> Option<BuildFailure> {
        let error_lower = error_line.to_lowercase();

        if error_lower.contains("module not found") || error_lower.contains("cannot find module") {
            return Some(BuildFailure {
                failure_type: BuildFailureType::MissingDependency,
                error_message: error_line.to_string(),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Install missing dependency".to_string()),
                confidence: 0.9,
            });
        }

        if error_lower.contains("version conflict") || error_lower.contains("incompatible") {
            return Some(BuildFailure {
                failure_type: BuildFailureType::VersionConflict,
                error_message: error_line.to_string(),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Resolve version conflicts".to_string()),
                confidence: 0.8,
            });
        }

        if error_lower.contains("syntax error") || error_lower.contains("parse error") {
            return Some(BuildFailure {
                failure_type: BuildFailureType::SyntaxError,
                error_message: error_line.to_string(),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Fix syntax errors in code".to_string()),
                confidence: 0.95,
            });
        }

        None
    }

    fn generate_build_recommendations(&self, conflicts: &[DependencyConflict], failures: &[BuildFailure]) -> Vec<BuildRecommendation> {
        let mut recommendations = Vec::new();

        if !conflicts.is_empty() {
            recommendations.push(BuildRecommendation {
                priority: RecommendationPriority::High,
                category: RecommendationCategory::Maintainability,
                title: "Resolve Dependency Conflicts".to_string(),
                description: format!("Found {} dependency conflicts that may cause build issues", conflicts.len()),
                action_items: vec![
                    "Review and update conflicting dependencies".to_string(),
                    "Use dependency resolution tools".to_string(),
                    "Consider using lock files".to_string(),
                ],
                estimated_impact: ImpactLevel::High,
            });
        }

        if !failures.is_empty() {
            recommendations.push(BuildRecommendation {
                priority: RecommendationPriority::Immediate,
                category: RecommendationCategory::Performance,
                title: "Fix Build Failures".to_string(),
                description: format!("Found {} build failures that prevent successful compilation", failures.len()),
                action_items: vec![
                    "Address all build errors".to_string(),
                    "Validate build configuration".to_string(),
                    "Test build in clean environment".to_string(),
                ],
                estimated_impact: ImpactLevel::High,
            });
        }

        recommendations
    }

    async fn generate_build_fixes(&self, conflicts: &[DependencyConflict], failures: &[BuildFailure], package_manager: &PackageManager) -> Result<Vec<BuildFix>> {
        let mut fixes = Vec::new();

        // Generate fixes for dependency conflicts
        for conflict in conflicts {
            if let Some(fix) = self.generate_dependency_fix(conflict, package_manager) {
                fixes.push(fix);
            }
        }

        // Generate fixes for build failures
        for failure in failures {
            if let Some(fix) = self.generate_failure_fix(failure, package_manager) {
                fixes.push(fix);
            }
        }

        Ok(fixes)
    }

    fn generate_dependency_fix(&self, conflict: &DependencyConflict, package_manager: &PackageManager) -> Option<BuildFix> {
        match conflict.resolution_strategy {
            ResolutionStrategy::UpdateToLatest => {
                let commands = match package_manager {
                    PackageManager::Npm => vec![format!("npm update {}", conflict.package_name)],
                    PackageManager::Pip => vec![format!("pip install --upgrade {}", conflict.package_name)],
                    PackageManager::Cargo => vec![format!("cargo update -p {}", conflict.package_name)],
                    _ => vec![],
                };

                Some(BuildFix {
                    fix_id: format!("update_{}", conflict.package_name),
                    fix_type: BuildFixType::DependencyUpdate,
                    description: format!("Update {} to resolve version conflict", conflict.package_name),
                    commands,
                    file_changes: vec![],
                    validation_command: Some(self.get_default_build_command(package_manager)),
                    rollback_commands: vec!["git checkout -- .".to_string()],
                })
            }
            _ => None,
        }
    }

    fn generate_failure_fix(&self, failure: &BuildFailure, package_manager: &PackageManager) -> Option<BuildFix> {
        match failure.failure_type {
            BuildFailureType::MissingDependency => {
                let commands = match package_manager {
                    PackageManager::Npm => vec!["npm install".to_string()],
                    PackageManager::Pip => vec!["pip install -r requirements.txt".to_string()],
                    PackageManager::Cargo => vec!["cargo build".to_string()],
                    _ => vec![],
                };

                Some(BuildFix {
                    fix_id: "install_dependencies".to_string(),
                    fix_type: BuildFixType::DependencyUpdate,
                    description: "Install missing dependencies".to_string(),
                    commands,
                    file_changes: vec![],
                    validation_command: Some(self.get_default_build_command(package_manager)),
                    rollback_commands: vec![],
                })
            }
            _ => None,
        }
    }

    async fn count_dependencies(&self, project_path: &PathBuf, package_manager: &PackageManager) -> Result<usize> {
        match package_manager {
            PackageManager::Npm => {
                let package_json_path = project_path.join("package.json");
                if let Ok(content) = tokio::fs::read_to_string(package_json_path).await {
                    if let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) {
                        let deps = package_json.get("dependencies").and_then(|d| d.as_object()).map(|o| o.len()).unwrap_or(0);
                        let dev_deps = package_json.get("devDependencies").and_then(|d| d.as_object()).map(|o| o.len()).unwrap_or(0);
                        return Ok(deps + dev_deps);
                    }
                }
            }
            PackageManager::Pip => {
                let requirements_path = project_path.join("requirements.txt");
                if let Ok(content) = tokio::fs::read_to_string(requirements_path).await {
                    return Ok(content.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')).count());
                }
            }
            _ => {}
        }

        Ok(0)
    }

    pub async fn apply_fix(&self, fix: &BuildFix, project_path: &PathBuf) -> Result<bool> {
        info!("Applying build fix: {}", fix.description);

        // Apply file changes
        for file_change in &fix.file_changes {
            match file_change.change_type {
                ChangeType::Create | ChangeType::Update => {
                    let file_path = project_path.join(&file_change.file_path);
                    tokio::fs::write(file_path, &file_change.new_content).await?;
                }
                ChangeType::Delete => {
                    let file_path = project_path.join(&file_change.file_path);
                    let _ = tokio::fs::remove_file(file_path).await;
                }
                ChangeType::Append => {
                    let file_path = project_path.join(&file_change.file_path);
                    let mut content = tokio::fs::read_to_string(&file_path).await.unwrap_or_default();
                    content.push_str(&file_change.new_content);
                    tokio::fs::write(file_path, content).await?;
                }
            }
        }

        // Execute commands
        for command in &fix.commands {
            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(project_path)
                .output()
                .await?;

            if !output.status.success() {
                warn!("Command failed: {}", command);
                return Ok(false);
            }
        }

        // Validate fix
        if let Some(validation_cmd) = &fix.validation_command {
            let output = Command::new("sh")
                .arg("-c")
                .arg(validation_cmd)
                .current_dir(project_path)
                .output()
                .await?;

            return Ok(output.status.success());
        }

        Ok(true)
    }
}