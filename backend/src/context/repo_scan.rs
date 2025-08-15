use anyhow::Result;
use ignore::{DirEntry, Walk, WalkBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

use super::{FileContext, SpanType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoScanConfig {
    pub max_file_size_mb: usize,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_files: usize,
}

impl Default for RepoScanConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: 10,
            include_patterns: vec![
                "*.rs".to_string(),
                "*.py".to_string(),
                "*.js".to_string(),
                "*.ts".to_string(),
                "*.jsx".to_string(),
                "*.tsx".to_string(),
                "*.go".to_string(),
                "*.java".to_string(),
                "*.c".to_string(),
                "*.cpp".to_string(),
                "*.h".to_string(),
                "*.hpp".to_string(),
            ],
            exclude_patterns: vec![
                "target/*".to_string(),
                "node_modules/*".to_string(),
                ".git/*".to_string(),
                "*.lock".to_string(),
                "*.log".to_string(),
                "dist/*".to_string(),
                "build/*".to_string(),
                ".next/*".to_string(),
                ".cache/*".to_string(),
                "coverage/*".to_string(),
            ],
            max_files: 1000,
        }
    }
}

pub struct RepoScanner {
    repo_path: PathBuf,
    config: RepoScanConfig,
}

impl RepoScanner {
    pub fn new(repo_path: PathBuf) -> Result<Self> {
        let config = RepoScanConfig::default();
        
        info!("Initializing RepoScanner for path: {:?}", repo_path);
        debug!("Scanner config: max_file_size={}MB, max_files={}", 
               config.max_file_size_mb, config.max_files);
        
        Ok(Self {
            repo_path,
            config,
        })
    }

    pub fn with_config(repo_path: PathBuf, config: RepoScanConfig) -> Result<Self> {
        Ok(Self {
            repo_path,
            config,
        })
    }

    /// Scan repository and return file contexts
    pub async fn scan(&self) -> Result<Vec<FileContext>> {
        info!("Starting repository scan at: {:?}", self.repo_path);
        
        let mut files = Vec::new();
        let mut file_count = 0;
        
        // Build walker with gitignore support
        let walker = WalkBuilder::new(&self.repo_path)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(true)
            .build();

        for result in walker {
            if file_count >= self.config.max_files {
                warn!("Reached maximum file limit ({}), stopping scan", self.config.max_files);
                break;
            }

            let entry = match result {
                Ok(entry) => entry,
                Err(e) => {
                    warn!("Error walking directory: {}", e);
                    continue;
                }
            };

            if let Some(file_context) = self.process_file_entry(&entry).await? {
                files.push(file_context);
                file_count += 1;
                
                if file_count % 100 == 0 {
                    debug!("Scanned {} files so far...", file_count);
                }
            }
        }

        info!("Repository scan completed. Found {} relevant files", files.len());
        Ok(files)
    }

    /// Process a single file entry
    async fn process_file_entry(&self, entry: &DirEntry) -> Result<Option<FileContext>> {
        let path = entry.path();
        
        // Skip directories
        if path.is_dir() {
            return Ok(None);
        }

        // Check if file matches our patterns
        if !self.should_include_file(path) {
            return Ok(None);
        }

        // Check file size
        let metadata = match fs::metadata(path).await {
            Ok(metadata) => metadata,
            Err(e) => {
                warn!("Could not read metadata for {:?}: {}", path, e);
                return Ok(None);
            }
        };

        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        if size_mb > self.config.max_file_size_mb as f64 {
            debug!("Skipping large file {:?} ({:.1}MB)", path, size_mb);
            return Ok(None);
        }

        // Read file content
        let content = match fs::read_to_string(path).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Could not read file {:?}: {}", path, e);
                return Ok(None);
            }
        };

        // Detect language
        let language = self.detect_language(path);
        
        // Calculate relevance score (basic implementation)
        let relevance_score = self.calculate_file_relevance(path, &content);

        let file_context = FileContext {
            path: path.to_path_buf(),
            content,
            language,
            relevance_score,
            last_modified: chrono::DateTime::from(metadata.modified()?),
            size_bytes: metadata.len() as usize,
        };

        debug!("Processed file: {:?} ({})", path, language);
        Ok(Some(file_context))
    }

    /// Check if file should be included based on patterns
    fn should_include_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check exclude patterns first
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return false;
            }
        }

        // Check include patterns
        for pattern in &self.config.include_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple pattern matching (supports basic wildcards)
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple wildcard support
            if pattern.starts_with("*.") {
                let extension = &pattern[2..];
                return path.ends_with(&format!(".{}", extension));
            } else if pattern.ends_with("/*") {
                let prefix = &pattern[..pattern.len() - 2];
                return path.contains(prefix);
            }
        }
        
        path.contains(pattern)
    }

    /// Detect programming language from file extension
    fn detect_language(&self, path: &Path) -> String {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("py") => "python".to_string(),
            Some("js") => "javascript".to_string(),
            Some("ts") => "typescript".to_string(),
            Some("jsx") => "jsx".to_string(),
            Some("tsx") => "tsx".to_string(),
            Some("go") => "go".to_string(),
            Some("java") => "java".to_string(),
            Some("c") => "c".to_string(),
            Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
            Some("h") | Some("hpp") => "header".to_string(),
            Some("md") => "markdown".to_string(),
            Some("toml") => "toml".to_string(),
            Some("yaml") | Some("yml") => "yaml".to_string(),
            Some("json") => "json".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Calculate basic relevance score for a file
    fn calculate_file_relevance(&self, path: &Path, content: &str) -> f32 {
        let mut score = 0.5; // Base score
        
        // Boost for test files
        if self.is_test_file(path) {
            score += 0.2;
        }
        
        // Boost for main/entry files
        if self.is_entry_file(path) {
            score += 0.3;
        }
        
        // Boost for configuration files
        if self.is_config_file(path) {
            score += 0.1;
        }
        
        // Content-based scoring
        let line_count = content.lines().count();
        if line_count > 50 && line_count < 500 {
            score += 0.1; // Sweet spot for meaningful files
        }
        
        // Check for important keywords
        let important_keywords = ["TODO", "FIXME", "BUG", "HACK", "NOTE"];
        for keyword in &important_keywords {
            if content.contains(keyword) {
                score += 0.05;
            }
        }
        
        score.min(1.0)
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        path_str.contains("test") || path_str.contains("spec")
    }

    fn is_entry_file(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        matches!(file_name.as_str(), 
            "main.rs" | "main.py" | "index.js" | "index.ts" | 
            "app.py" | "app.js" | "server.py" | "server.js" |
            "lib.rs" | "mod.rs"
        )
    }

    fn is_config_file(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        matches!(file_name.as_str(),
            "cargo.toml" | "package.json" | "requirements.txt" |
            "setup.py" | "makefile" | "dockerfile" | "docker-compose.yml" |
            "config.py" | "config.js" | "settings.py"
        )
    }

    /// Find test files related to given source files
    pub async fn find_related_tests(&self, source_files: &[FileContext]) -> Result<Vec<PathBuf>> {
        let mut test_files = HashSet::new();
        
        // Scan for test files that might be related
        let all_files = self.scan().await?;
        
        for file in &all_files {
            if self.is_test_file(&file.path) {
                // Check if test file is related to any source file
                for source_file in source_files {
                    if self.are_files_related(&source_file.path, &file.path) {
                        test_files.insert(file.path.clone());
                    }
                }
            }
        }
        
        Ok(test_files.into_iter().collect())
    }

    /// Check if two files are related (e.g., test and source)
    fn are_files_related(&self, source_path: &Path, test_path: &Path) -> bool {
        let source_stem = source_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let test_name = test_path.to_string_lossy();
        
        // Simple heuristic: test file contains source file name
        test_name.contains(source_stem) || 
        test_name.contains(&source_stem.replace('_', "-")) ||
        test_name.contains(&source_stem.replace('-', "_"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_repo_scanner_basic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();
        
        // Create test files
        fs::write(repo_path.join("main.rs"), "fn main() {}").await?;
        fs::write(repo_path.join("lib.py"), "def hello(): pass").await?;
        fs::write(repo_path.join("test.txt"), "should be ignored").await?;
        
        let scanner = RepoScanner::new(repo_path)?;
        let files = scanner.scan().await?;
        
        assert_eq!(files.len(), 2); // Only .rs and .py files
        assert!(files.iter().any(|f| f.path.file_name().unwrap() == "main.rs"));
        assert!(files.iter().any(|f| f.path.file_name().unwrap() == "lib.py"));
        
        Ok(())
    }

    #[test]
    fn test_language_detection() {
        let scanner = RepoScanner::new(PathBuf::from(".")).unwrap();
        
        assert_eq!(scanner.detect_language(Path::new("file.rs")), "rust");
        assert_eq!(scanner.detect_language(Path::new("file.py")), "python");
        assert_eq!(scanner.detect_language(Path::new("file.js")), "javascript");
        assert_eq!(scanner.detect_language(Path::new("file.unknown")), "unknown");
    }

    #[test]
    fn test_pattern_matching() {
        let scanner = RepoScanner::new(PathBuf::from(".")).unwrap();
        
        assert!(scanner.matches_pattern("file.rs", "*.rs"));
        assert!(scanner.matches_pattern("src/main.rs", "*.rs"));
        assert!(scanner.matches_pattern("target/debug/app", "target/*"));
        assert!(!scanner.matches_pattern("file.py", "*.rs"));
    }
}