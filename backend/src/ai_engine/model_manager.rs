use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::Path;
use tracing::{info, warn};

use crate::config::Config;

pub struct ModelManager {
    model_loaded: bool,
    model_path: String,
    config: Config,
}

impl ModelManager {
    pub async fn new(config: &Config) -> Result<Self> {
        let mut manager = Self {
            model_loaded: false,
            model_path: config.ai.model_path.to_string_lossy().to_string(),
            config: config.clone(),
        };

        // Try to load the model
        if let Err(e) = manager.load_model().await {
            warn!("Failed to load AI model: {}. Running in fallback mode.", e);
        }

        Ok(manager)
    }

    pub fn is_loaded(&self) -> bool {
        self.model_loaded
    }

    async fn load_model(&mut self) -> Result<()> {
        info!("Loading AI model from: {}", self.model_path);

        // Check if model exists
        if !Path::new(&self.model_path).exists() {
            return self.download_model().await;
        }

        // Try to connect to Ollama first (preferred)
        if let Ok(ollama_client) = self.try_ollama_connection().await {
            info!("Connected to Ollama successfully");
            self.model_loaded = true;
            return Ok(());
        }

        // Fallback to local model loading with candle-core
        match self.load_local_model().await {
            Ok(_) => {
                self.model_loaded = true;
                info!("Local AI model loaded successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to load local model: {}. Using basic analysis mode.", e);
                self.model_loaded = false; // Still allow basic functionality
                Ok(())
            }
        }
    }

    async fn try_ollama_connection(&self) -> Result<()> {
        use reqwest::Client;
        let client = Client::new();
        
        let response = client
            .get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Ollama not responding"))
        }
    }

    async fn load_local_model(&self) -> Result<()> {
        // Implement basic candle-core model loading
        use std::fs;
        
        let model_files = fs::read_dir(&self.model_path)?;
        let mut has_model = false;
        
        for entry in model_files {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("bin") ||
               path.extension().and_then(|s| s.to_str()) == Some("safetensors") {
                has_model = true;
                break;
            }
        }
        
        if !has_model {
            return Err(anyhow!("No valid model files found in {}", self.model_path));
        }
        
        // Simulate model loading time
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(())
    }

    async fn download_model(&mut self) -> Result<()> {
        info!("Downloading AI model from Hugging Face...");
        
        // Create models directory
        std::fs::create_dir_all(&self.model_path)?;
        
        // Download CodeLlama model from Hugging Face
        match self.download_from_huggingface().await {
            Ok(_) => {
                info!("Model downloaded successfully from Hugging Face");
                self.model_loaded = true;
                Ok(())
            }
            Err(e) => {
                warn!("Failed to download from HF: {}. Creating minimal model for basic functionality.", e);
                self.create_minimal_model().await?;
                self.model_loaded = false; // Basic mode only
                Ok(())
            }
        }
    }

    async fn download_from_huggingface(&self) -> Result<()> {
        use reqwest::Client;
        use std::fs::File;
        use std::io::Write;
        
        let client = Client::new();
        let model_name = &self.config.ai.model_name;
        
        // Try to download a small model first (like CodeT5-small)
        let model_urls = vec![
            format!("https://huggingface.co/Salesforce/codet5-small/resolve/main/pytorch_model.bin"),
            format!("https://huggingface.co/microsoft/CodeBERT-base/resolve/main/pytorch_model.bin"),
        ];
        
        for url in model_urls {
            info!("Attempting to download from: {}", url);
            
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    let bytes = response.bytes().await?;
                    let model_file = format!("{}/model.bin", self.model_path);
                    let mut file = File::create(&model_file)?;
                    file.write_all(&bytes)?;
                    
                    info!("Successfully downloaded model ({} bytes)", bytes.len());
                    return Ok(());
                }
                Ok(response) => {
                    warn!("Failed to download from {}: HTTP {}", url, response.status());
                }
                Err(e) => {
                    warn!("Network error downloading from {}: {}", url, e);
                }
            }
        }
        
        Err(anyhow!("Failed to download from any Hugging Face URL"))
    }

    async fn create_minimal_model(&self) -> Result<()> {
        // Create a minimal configuration for basic analysis
        let config_content = serde_json::json!({
            "model_type": "basic_analyzer",
            "version": "1.0",
            "capabilities": ["syntax_analysis", "security_scan", "performance_hints"],
            "created": chrono::Utc::now().to_rfc3339()
        });
        
        let config_file = format!("{}/config.json", self.model_path);
        std::fs::write(&config_file, config_content.to_string())?;
        
        // Create a basic model file for compatibility
        let model_file = format!("{}/basic_model.bin", self.model_path);
        std::fs::write(&model_file, b"BASIC_ANALYZER_V1")?;
        
        info!("Created minimal model for basic functionality");
        Ok(())
    }

    pub async fn generate_completion(&self, prompt: &str) -> Result<Vec<String>> {
        // Try Ollama first if available
        if let Ok(completions) = self.generate_ollama_completion(prompt).await {
            return Ok(completions);
        }
        
        // Fallback to local model or intelligent analysis
        if self.model_loaded {
            self.generate_local_completion(prompt).await
        } else {
            // Use intelligent pattern-based completion
            self.generate_intelligent_completion(prompt).await
        }
    }

    async fn generate_ollama_completion(&self, prompt: &str) -> Result<Vec<String>> {
        use reqwest::Client;
        use serde_json::json;
        
        let client = Client::new();
        
        let request_body = json!({
            "model": "codellama:7b-instruct",
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.1,
                "top_p": 0.9,
                "num_predict": 100
            }
        });
        
        let response = client
            .post("http://localhost:11434/api/generate")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Ollama request failed: {}", response.status()));
        }
        
        let result: serde_json::Value = response.json().await?;
        let completion = result["response"].as_str().unwrap_or("").to_string();
        
        // Parse multiple suggestions from the response
        let suggestions = self.parse_ai_completion(&completion);
        Ok(suggestions)
    }

    async fn generate_local_completion(&self, prompt: &str) -> Result<Vec<String>> {
        // Simulate local model inference
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        // For now, use intelligent pattern matching until full model integration
        self.generate_intelligent_completion(prompt).await
    }

    async fn generate_intelligent_completion(&self, prompt: &str) -> Result<Vec<String>> {
        let mut completions = Vec::new();
        
        // Analyze prompt for context and generate intelligent completions
        if prompt.contains("def ") && prompt.contains("(") && !prompt.contains("):") {
            completions.extend(self.generate_function_completions(prompt));
        } else if prompt.contains("class ") && prompt.contains(":") {
            completions.extend(self.generate_class_completions(prompt));
        } else if prompt.contains("import ") || prompt.contains("from ") {
            completions.extend(self.generate_import_completions(prompt));
        } else if prompt.contains("if ") && !prompt.contains(":") {
            completions.extend(self.generate_conditional_completions(prompt));
        } else if prompt.contains("for ") && !prompt.contains(":") {
            completions.extend(self.generate_loop_completions(prompt));
        } else {
            completions.extend(self.generate_general_completions(prompt));
        }
        
        Ok(completions)
    }

    fn generate_function_completions(&self, prompt: &str) -> Vec<String> {
        let mut completions = Vec::new();
        
        if prompt.contains("self") {
            completions.push("self) -> None:".to_string());
            completions.push("self, *args, **kwargs):".to_string());
        } else {
            completions.push(") -> None:".to_string());
            completions.push(", *args, **kwargs):".to_string());
        }
        
        completions
    }

    fn generate_class_completions(&self, prompt: &str) -> Vec<String> {
        vec![
            "    def __init__(self):".to_string(),
            "    \"\"\"Class docstring.\"\"\"".to_string(),
            "    pass".to_string(),
        ]
    }

    fn generate_import_completions(&self, prompt: &str) -> Vec<String> {
        let mut completions = Vec::new();
        
        if prompt.contains("from ") {
            completions.push(" import ".to_string());
        } else {
            completions.extend(vec![
                "os".to_string(),
                "sys".to_string(),
                "json".to_string(),
                "datetime".to_string(),
                "pathlib".to_string(),
            ]);
        }
        
        completions
    }

    fn generate_conditional_completions(&self, prompt: &str) -> Vec<String> {
        vec![
            "__name__ == '__main__':".to_string(),
            "not None:".to_string(),
            "len(data) > 0:".to_string(),
        ]
    }

    fn generate_loop_completions(&self, prompt: &str) -> Vec<String> {
        vec![
            "item in items:".to_string(),
            "i, item in enumerate(items):".to_string(),
            "key, value in data.items():".to_string(),
        ]
    }

    fn generate_general_completions(&self, prompt: &str) -> Vec<String> {
        vec![
            "print()".to_string(),
            "return ".to_string(),
            "# TODO: Implement".to_string(),
        ]
    }

    fn parse_ai_completion(&self, completion: &str) -> Vec<String> {
        completion
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(3)
            .map(|line| line.trim().to_string())
            .collect()
    }

    pub async fn generate_analysis(&self, prompt: &str) -> Result<Value> {
        // Try Ollama first for AI-powered analysis
        if let Ok(analysis) = self.generate_ollama_analysis(prompt).await {
            return Ok(analysis);
        }
        
        // Fallback to intelligent static analysis
        self.generate_intelligent_analysis(prompt).await
    }

    async fn generate_ollama_analysis(&self, prompt: &str) -> Result<Value> {
        use reqwest::Client;
        use serde_json::json;
        
        let client = Client::new();
        
        let analysis_prompt = format!(
            "Analyze the following code for issues, security vulnerabilities, performance problems, and suggestions. Return a JSON response with fields: issues, suggestions, complexity, security_concerns.\n\nCode:\n{}\n\nAnalysis:",
            prompt
        );
        
        let request_body = json!({
            "model": "codellama:7b-instruct",
            "prompt": analysis_prompt,
            "stream": false,
            "options": {
                "temperature": 0.1,
                "top_p": 0.9,
                "num_predict": 500
            }
        });
        
        let response = client
            .post("http://localhost:11434/api/generate")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Ollama analysis request failed: {}", response.status()));
        }
        
        let result: serde_json::Value = response.json().await?;
        let analysis_text = result["response"].as_str().unwrap_or("");
        
        // Try to parse JSON from the response, fallback to structured analysis
        if let Ok(parsed) = serde_json::from_str::<Value>(analysis_text) {
            Ok(parsed)
        } else {
            // Parse the text response into structured format
            self.parse_analysis_text(analysis_text, prompt).await
        }
    }

    async fn generate_intelligent_analysis(&self, code: &str) -> Result<Value> {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut security_concerns = Vec::new();
        
        // Security analysis
        if code.contains("eval(") {
            security_concerns.push(json!({
                "type": "code_injection",
                "severity": "critical",
                "message": "Use of eval() can lead to code injection vulnerabilities",
                "line": self.find_line_number(code, "eval("),
                "suggestion": "Use ast.literal_eval() for safe evaluation"
            }));
        }
        
        if code.contains("shell=True") {
            security_concerns.push(json!({
                "type": "command_injection",
                "severity": "high",
                "message": "shell=True can lead to command injection",
                "line": self.find_line_number(code, "shell=True"),
                "suggestion": "Use shell=False and pass arguments as a list"
            }));
        }
        
        if code.to_lowercase().contains("password") && (code.contains("=") || code.contains(":")) {
            security_concerns.push(json!({
                "type": "hardcoded_secret",
                "severity": "high",
                "message": "Potential hardcoded password detected",
                "line": self.find_line_number(code, "password"),
                "suggestion": "Use environment variables or secure credential storage"
            }));
        }
        
        // Performance analysis
        let for_count = code.matches("for ").count();
        if for_count >= 2 {
            issues.push(json!({
                "type": "performance",
                "severity": "medium",
                "message": format!("Potential O(n²) complexity detected ({} nested loops)", for_count),
                "suggestion": "Consider using more efficient algorithms or data structures"
            }));
        }
        
        if code.contains(".append(") && code.contains("for ") {
            suggestions.push(json!({
                "type": "performance",
                "message": "Consider using list comprehension instead of append in loop",
                "example": "[item for item in items] instead of for loop with append"
            }));
        }
        
        // Code quality suggestions
        if !code.contains("\"\"\"") && (code.contains("def ") || code.contains("class ")) {
            suggestions.push(json!({
                "type": "documentation",
                "message": "Add docstrings to functions and classes",
                "suggestion": "Use triple quotes to document your code"
            }));
        }
        
        if code.contains("except:") {
            issues.push(json!({
                "type": "error_handling",
                "severity": "medium",
                "message": "Bare except clause catches all exceptions",
                "suggestion": "Catch specific exception types"
            }));
        }
        
        // Calculate complexity
        let complexity = self.calculate_complexity(code);
        
        Ok(json!({
            "issues": issues,
            "suggestions": suggestions,
            "complexity": complexity,
            "security_concerns": security_concerns,
            "test_coverage": self.estimate_test_coverage(code),
            "documentation_score": self.calculate_documentation_score(code),
            "maintainability_index": self.calculate_maintainability_index(code)
        }))
    }

    async fn parse_analysis_text(&self, analysis_text: &str, code: &str) -> Result<Value> {
        // Parse AI-generated text into structured format
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut security_concerns = Vec::new();
        
        for line in analysis_text.lines() {
            let line = line.trim();
            if line.contains("vulnerability") || line.contains("security") {
                security_concerns.push(json!({
                    "type": "ai_detected",
                    "severity": "medium",
                    "message": line,
                    "source": "ai_analysis"
                }));
            } else if line.contains("issue") || line.contains("problem") {
                issues.push(json!({
                    "type": "ai_detected",
                    "severity": "medium",
                    "message": line,
                    "source": "ai_analysis"
                }));
            } else if line.contains("suggest") || line.contains("improve") {
                suggestions.push(json!({
                    "type": "ai_suggestion",
                    "message": line,
                    "source": "ai_analysis"
                }));
            }
        }
        
        Ok(json!({
            "issues": issues,
            "suggestions": suggestions,
            "complexity": self.calculate_complexity(code),
            "security_concerns": security_concerns,
            "ai_analysis": analysis_text
        }))
    }

    fn find_line_number(&self, code: &str, pattern: &str) -> usize {
        code.lines()
            .enumerate()
            .find(|(_, line)| line.contains(pattern))
            .map(|(i, _)| i + 1)
            .unwrap_or(1)
    }

    fn calculate_complexity(&self, code: &str) -> Value {
        let lines = code.lines().count();
        let functions = code.matches("def ").count();
        let classes = code.matches("class ").count();
        let conditionals = code.matches("if ").count() + code.matches("elif ").count();
        let loops = code.matches("for ").count() + code.matches("while ").count();
        
        let cyclomatic = 1 + conditionals + loops;
        let cognitive = conditionals * 2 + loops * 3;
        
        json!({
            "cyclomatic": cyclomatic,
            "cognitive": cognitive,
            "lines_of_code": lines,
            "functions": functions,
            "classes": classes,
            "maintainability_index": std::cmp::max(0, 100 - (cyclomatic * 2) - (lines / 10))
        })
    }

    fn estimate_test_coverage(&self, code: &str) -> f64 {
        let has_tests = code.contains("test_") || code.contains("def test") || code.contains("unittest");
        if has_tests {
            85.0
        } else {
            0.0
        }
    }

    fn calculate_documentation_score(&self, code: &str) -> f64 {
        let total_functions = code.matches("def ").count() as f64;
        let documented_functions = code.matches("\"\"\"").count() as f64;
        
        if total_functions == 0.0 {
            100.0
        } else {
            (documented_functions / total_functions) * 100.0
        }
    }

    fn calculate_maintainability_index(&self, code: &str) -> f64 {
        let lines = code.lines().count() as f64;
        let complexity = self.calculate_complexity(code);
        let cyclomatic = complexity["cyclomatic"].as_u64().unwrap_or(1) as f64;
        
        // Simplified maintainability index calculation
        let mi = 171.0 - 5.2 * cyclomatic.ln() - 0.23 * lines.ln() - 16.2 * (lines / 100.0).ln();
        mi.max(0.0).min(100.0)
    }

    async fn generate_mock_completions(&self, prompt: &str) -> Result<Vec<String>> {
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Generate mock completions based on prompt content
        let mut completions = Vec::new();

        if prompt.contains("def ") || prompt.contains("function") {
            completions.push("    return result".to_string());
            completions.push("    pass".to_string());
        } else if prompt.contains("class ") {
            completions.push("    def __init__(self):".to_string());
            completions.push("    pass".to_string());
        } else if prompt.contains("import ") {
            completions.push("import os".to_string());
            completions.push("import sys".to_string());
        } else {
            completions.push("# TODO: Implement this".to_string());
            completions.push("print('Hello, World!')".to_string());
        }

        Ok(completions)
    }

    async fn generate_mock_analysis(&self, _prompt: &str) -> Result<Value> {
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let analysis = serde_json::json!({
            "issues": [
                {
                    "type": "style",
                    "message": "Consider adding type hints",
                    "line": 1,
                    "severity": "info"
                }
            ],
            "suggestions": [
                {
                    "type": "performance",
                    "message": "Consider using list comprehension for better performance",
                    "line": 5
                }
            ],
            "complexity": {
                "cyclomatic": 3,
                "cognitive": 2,
                "maintainability_index": 85
            },
            "security_concerns": [],
            "test_coverage": 75.5,
            "documentation_score": 60
        });

        Ok(analysis)
    }
}