use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use tokio::fs as async_fs;
use tracing::{info, warn, error};
use serde_json;
use regex::Regex;

use super::{WorkspaceContext, GitInfo, Dependency, DependencyType, BuildSystem, FileChange};

pub struct WorkspaceAnalyzer {
    ignore_patterns: Vec<Regex>,
}

impl WorkspaceAnalyzer {
    pub fn new() -> Self {
        let ignore_patterns = vec![
            Regex::new(r"node_modules").unwrap(),
            Regex::new(r"target").unwrap(),
            Regex::new(r"\.git").unwrap(),
            Regex::new(r"\.vscode").unwrap(),
            Regex::new(r"\.idea").unwrap(),
            Regex::new(r"__pycache__").unwrap(),
            Regex::new(r"\.pyc$").unwrap(),
            Regex::new(r"\.o$").unwrap(),
            Regex::new(r"\.exe$").unwrap(),
            Regex::new(r"\.dll$").unwrap(),
            Regex::new(r"\.so$").unwrap(),
        ];

        Self { ignore_patterns }
    }

    pub async fn analyze_workspace(&self, workspace_path: &str) -> Result<WorkspaceContext> {
        info!("Analyzing workspace: {}", workspace_path);
        
        let path = Path::new(workspace_path);
        if !path.exists() {
            return Err(anyhow::anyhow!("Workspace path does not exist: {}", workspace_path));
        }

        let mut context = WorkspaceContext::new(Some(workspace_path.to_string()));

        // Proje tipini tespit et
        context.project_type = self.detect_project_type(path).await?;
        info!("Detected project type: {:?}", context.project_type);

        // Ana dosyaları bul
        context.main_files = self.find_main_files(path).await?;
        info!("Found {} main files", context.main_files.len());

        // Git bilgilerini al
        context.git_info = self.analyze_git_info(path).await?;

        // Bağımlılıkları analiz et
        context.dependencies = self.analyze_dependencies(path).await?;
        info!("Found {} dependencies", context.dependencies.len());

        // Build sistemini tespit et
        context.build_system = self.detect_build_system(path).await?;

        // Son değişiklikleri al
        context.recent_changes = self.get_recent_changes(path).await?;

        Ok(context)
    }

    async fn detect_project_type(&self, path: &Path) -> Result<Option<String>> {
        // Rust projesi
        if path.join("Cargo.toml").exists() {
            return Ok(Some("rust".to_string()));
        }

        // Node.js projesi
        if path.join("package.json").exists() {
            return Ok(Some("node".to_string()));
        }

        // Python projesi
        if path.join("requirements.txt").exists() || 
           path.join("pyproject.toml").exists() || 
           path.join("setup.py").exists() {
            return Ok(Some("python".to_string()));
        }

        // Java projesi
        if path.join("pom.xml").exists() {
            return Ok(Some("java-maven".to_string()));
        }

        if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            return Ok(Some("java-gradle".to_string()));
        }

        // Go projesi
        if path.join("go.mod").exists() {
            return Ok(Some("go".to_string()));
        }

        // C/C++ projesi
        if path.join("CMakeLists.txt").exists() {
            return Ok(Some("cpp-cmake".to_string()));
        }

        if path.join("Makefile").exists() {
            return Ok(Some("c-make".to_string()));
        }

        // .NET projesi
        if self.find_files_with_extension(path, "csproj").await?.len() > 0 {
            return Ok(Some("dotnet".to_string()));
        }

        // PHP projesi
        if path.join("composer.json").exists() {
            return Ok(Some("php".to_string()));
        }

        // Ruby projesi
        if path.join("Gemfile").exists() {
            return Ok(Some("ruby".to_string()));
        }

        Ok(None)
    }

    async fn find_main_files(&self, path: &Path) -> Result<Vec<String>> {
        let mut main_files = Vec::new();

        // Yaygın ana dosya isimleri
        let main_file_names = [
            "main.rs", "lib.rs", "mod.rs",
            "index.js", "app.js", "server.js", "main.js",
            "main.py", "__init__.py", "app.py",
            "Main.java", "Application.java",
            "main.go",
            "main.cpp", "main.c",
            "index.html", "App.tsx", "App.jsx",
            "README.md", "README.txt",
        ];

        for file_name in &main_file_names {
            let file_path = path.join(file_name);
            if file_path.exists() {
                if let Some(relative_path) = file_path.strip_prefix(path).ok() {
                    main_files.push(relative_path.to_string_lossy().to_string());
                }
            }
        }

        // src/ dizinindeki dosyaları da kontrol et
        let src_path = path.join("src");
        if src_path.exists() {
            for file_name in &main_file_names {
                let file_path = src_path.join(file_name);
                if file_path.exists() {
                    if let Some(relative_path) = file_path.strip_prefix(path).ok() {
                        main_files.push(relative_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(main_files)
    }

    async fn analyze_git_info(&self, path: &Path) -> Result<Option<GitInfo>> {
        let git_path = path.join(".git");
        if !git_path.exists() {
            return Ok(None);
        }

        let mut git_info = GitInfo {
            branch: "main".to_string(),
            commit_hash: String::new(),
            has_uncommitted_changes: false,
            remote_url: None,
            last_commit_message: String::new(),
            modified_files: Vec::new(),
        };

        // Git branch bilgisi
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["branch", "--show-current"])
            .current_dir(path)
            .output()
            .await
        {
            if output.status.success() {
                git_info.branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }

        // Son commit hash
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(path)
            .output()
            .await
        {
            if output.status.success() {
                git_info.commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }

        // Değiştirilmiş dosyalar
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(path)
            .output()
            .await
        {
            if output.status.success() {
                let status_output = String::from_utf8_lossy(&output.stdout);
                git_info.has_uncommitted_changes = !status_output.trim().is_empty();
                
                for line in status_output.lines() {
                    if line.len() > 3 {
                        git_info.modified_files.push(line[3..].to_string());
                    }
                }
            }
        }

        // Remote URL
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .current_dir(path)
            .output()
            .await
        {
            if output.status.success() {
                git_info.remote_url = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Son commit mesajı
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["log", "-1", "--pretty=format:%s"])
            .current_dir(path)
            .output()
            .await
        {
            if output.status.success() {
                git_info.last_commit_message = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }

        Ok(Some(git_info))
    }

    async fn analyze_dependencies(&self, path: &Path) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Rust dependencies (Cargo.toml)
        if let Ok(cargo_deps) = self.parse_cargo_dependencies(path).await {
            dependencies.extend(cargo_deps);
        }

        // Node.js dependencies (package.json)
        if let Ok(npm_deps) = self.parse_npm_dependencies(path).await {
            dependencies.extend(npm_deps);
        }

        // Python dependencies (requirements.txt, pyproject.toml)
        if let Ok(python_deps) = self.parse_python_dependencies(path).await {
            dependencies.extend(python_deps);
        }

        Ok(dependencies)
    }

    async fn parse_cargo_dependencies(&self, path: &Path) -> Result<Vec<Dependency>> {
        let cargo_path = path.join("Cargo.toml");
        if !cargo_path.exists() {
            return Ok(Vec::new());
        }

        let content = async_fs::read_to_string(cargo_path).await?;
        let mut dependencies = Vec::new();

        // Basit TOML parsing (gerçek projede toml crate kullanılmalı)
        let lines: Vec<&str> = content.lines().collect();
        let mut in_dependencies = false;
        let mut in_dev_dependencies = false;

        for line in lines {
            let line = line.trim();
            
            if line == "[dependencies]" {
                in_dependencies = true;
                in_dev_dependencies = false;
                continue;
            } else if line == "[dev-dependencies]" {
                in_dependencies = false;
                in_dev_dependencies = true;
                continue;
            } else if line.starts_with('[') {
                in_dependencies = false;
                in_dev_dependencies = false;
                continue;
            }

            if (in_dependencies || in_dev_dependencies) && line.contains('=') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim().to_string();
                    let version = parts[1].trim().trim_matches('"').to_string();
                    
                    dependencies.push(Dependency {
                        name,
                        version,
                        dependency_type: if in_dev_dependencies {
                            DependencyType::Development
                        } else {
                            DependencyType::Runtime
                        },
                        source: "crates.io".to_string(),
                    });
                }
            }
        }

        Ok(dependencies)
    }

    async fn parse_npm_dependencies(&self, path: &Path) -> Result<Vec<Dependency>> {
        let package_path = path.join("package.json");
        if !package_path.exists() {
            return Ok(Vec::new());
        }

        let content = async_fs::read_to_string(package_path).await?;
        let package_json: serde_json::Value = serde_json::from_str(&content)?;
        let mut dependencies = Vec::new();

        // Runtime dependencies
        if let Some(deps) = package_json.get("dependencies").and_then(|d| d.as_object()) {
            for (name, version) in deps {
                dependencies.push(Dependency {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("unknown").to_string(),
                    dependency_type: DependencyType::Runtime,
                    source: "npm".to_string(),
                });
            }
        }

        // Dev dependencies
        if let Some(deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
            for (name, version) in deps {
                dependencies.push(Dependency {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("unknown").to_string(),
                    dependency_type: DependencyType::Development,
                    source: "npm".to_string(),
                });
            }
        }

        Ok(dependencies)
    }

    async fn parse_python_dependencies(&self, path: &Path) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // requirements.txt
        let req_path = path.join("requirements.txt");
        if req_path.exists() {
            let content = async_fs::read_to_string(req_path).await?;
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    let parts: Vec<&str> = line.split("==").collect();
                    let name = parts[0].trim().to_string();
                    let version = if parts.len() > 1 {
                        parts[1].trim().to_string()
                    } else {
                        "latest".to_string()
                    };

                    dependencies.push(Dependency {
                        name,
                        version,
                        dependency_type: DependencyType::Runtime,
                        source: "pypi".to_string(),
                    });
                }
            }
        }

        Ok(dependencies)
    }

    async fn detect_build_system(&self, path: &Path) -> Result<Option<BuildSystem>> {
        if path.join("Cargo.toml").exists() {
            return Ok(Some(BuildSystem::Cargo));
        }

        if path.join("package.json").exists() {
            return Ok(Some(BuildSystem::Npm));
        }

        if path.join("pom.xml").exists() {
            return Ok(Some(BuildSystem::Maven));
        }

        if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            return Ok(Some(BuildSystem::Gradle));
        }

        if path.join("Makefile").exists() {
            return Ok(Some(BuildSystem::Make));
        }

        if path.join("CMakeLists.txt").exists() {
            return Ok(Some(BuildSystem::CMake));
        }

        if path.join("pyproject.toml").exists() {
            return Ok(Some(BuildSystem::Poetry));
        }

        Ok(None)
    }

    async fn get_recent_changes(&self, path: &Path) -> Result<Vec<FileChange>> {
        let mut changes = Vec::new();

        // Git log ile son değişiklikleri al
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["log", "--oneline", "--name-status", "-10"])
            .current_dir(path)
            .output()
            .await
        {
            if output.status.success() {
                let log_output = String::from_utf8_lossy(&output.stdout);
                // Git log parsing (basitleştirilmiş)
                for line in log_output.lines() {
                    if line.starts_with('M') || line.starts_with('A') || line.starts_with('D') {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            changes.push(FileChange {
                                file_path: parts[1].to_string(),
                                change_type: match parts[0] {
                                    "M" => "modified".to_string(),
                                    "A" => "added".to_string(),
                                    "D" => "deleted".to_string(),
                                    _ => "unknown".to_string(),
                                },
                                timestamp: chrono::Utc::now(), // Gerçek timestamp alınmalı
                                lines_added: 0, // Git diff ile hesaplanmalı
                                lines_removed: 0,
                            });
                        }
                    }
                }
            }
        }

        Ok(changes)
    }

    async fn find_files_with_extension(&self, path: &Path, extension: &str) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut entries = async_fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == extension {
                        files.push(path);
                    }
                }
            } else if path.is_dir() {
                // Ignore patterns kontrolü
                let should_ignore = self.ignore_patterns.iter().any(|pattern| {
                    pattern.is_match(&path.to_string_lossy())
                });

                if !should_ignore {
                    let mut sub_files = self.find_files_with_extension(&path, extension).await?;
                    files.append(&mut sub_files);
                }
            }
        }

        Ok(files)
    }

    pub async fn get_file_content_preview(&self, file_path: &str, max_lines: usize) -> Result<String> {
        let content = async_fs::read_to_string(file_path).await?;
        let lines: Vec<&str> = content.lines().take(max_lines).collect();
        Ok(lines.join("\n"))
    }

    pub async fn analyze_file_symbols(&self, file_path: &str) -> Result<Vec<super::SymbolInfo>> {
        // Bu fonksiyon dosyadaki sembolleri analiz eder
        // Gerçek implementasyonda tree-sitter veya LSP kullanılmalı
        let mut symbols = Vec::new();
        
        let content = async_fs::read_to_string(file_path).await?;
        
        // Basit Rust function detection
        if file_path.ends_with(".rs") {
            let function_regex = Regex::new(r"fn\s+(\w+)\s*\(").unwrap();
            for (line_num, line) in content.lines().enumerate() {
                if let Some(captures) = function_regex.captures(line) {
                    if let Some(func_name) = captures.get(1) {
                        symbols.push(super::SymbolInfo {
                            name: func_name.as_str().to_string(),
                            symbol_type: super::SymbolType::Function,
                            file_path: file_path.to_string(),
                            line_number: line_num + 1,
                            scope: "global".to_string(),
                        });
                    }
                }
            }
        }

        Ok(symbols)
    }
}

impl Default for WorkspaceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}