use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub options: OllamaOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaOptions {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponse {
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub eval_count: Option<u32>,
}

impl OllamaClient {
    pub fn new(base_url: String, model_name: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            model_name,
        }
    }

    pub async fn generate_completion(&self, prompt: &str) -> Result<String> {
        let request = OllamaRequest {
            model: self.model_name.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: 0.1,
                top_p: 0.9,
                max_tokens: 2048,
            },
        };

        let response = self
            .client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Ollama request failed: {}", response.status()));
        }

        let ollama_response: OllamaResponse = response.json().await?;
        Ok(ollama_response.response)
    }

    pub async fn generate_code_completion(&self, code: &str, language: &str, cursor_position: usize) -> Result<Vec<String>> {
        let context = self.extract_context(code, cursor_position);
        
        let prompt = format!(
            "You are an expert {} programmer. Complete the following code. Provide only the completion, no explanations.\n\nCode:\n```{}\n{}\n```\n\nCompletion:",
            language, language, context
        );

        let completion = self.generate_completion(&prompt).await?;
        
        // Parse multiple suggestions from the response
        let suggestions = self.parse_completions(&completion);
        
        Ok(suggestions)
    }

    pub async fn analyze_code_security(&self, code: &str, language: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "You are a security expert. Analyze the following {} code for security vulnerabilities. List each vulnerability with a brief description.\n\nCode:\n```{}\n{}\n```\n\nSecurity Issues:",
            language, language, code
        );

        let analysis = self.generate_completion(&prompt).await?;
        let issues = self.parse_security_issues(&analysis);
        
        Ok(issues)
    }

    pub async fn generate_documentation(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Generate comprehensive documentation for the following {} code. Include function descriptions, parameters, return values, and examples.\n\nCode:\n```{}\n{}\n```\n\nDocumentation:",
            language, language, code
        );

        self.generate_completion(&prompt).await
    }

    pub async fn generate_tests(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Generate comprehensive unit tests for the following {} code. Include edge cases and error scenarios.\n\nCode:\n```{}\n{}\n```\n\nTests:",
            language, language, code
        );

        self.generate_completion(&prompt).await
    }

    pub async fn suggest_optimizations(&self, code: &str, language: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Analyze the following {} code and suggest performance optimizations. Focus on algorithmic improvements, memory usage, and best practices.\n\nCode:\n```{}\n{}\n```\n\nOptimizations:",
            language, language, code
        );

        let suggestions = self.generate_completion(&prompt).await?;
        let optimizations = self.parse_optimizations(&suggestions);
        
        Ok(optimizations)
    }

    pub async fn check_health(&self) -> Result<bool> {
        let response = self
            .client
            .get(&format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    fn extract_context(&self, code: &str, cursor_position: usize) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let mut char_count = 0;
        let mut target_line = 0;

        // Find the line containing the cursor
        for (i, line) in lines.iter().enumerate() {
            if char_count + line.len() >= cursor_position {
                target_line = i;
                break;
            }
            char_count += line.len() + 1; // +1 for newline
        }

        // Extract context (5 lines before and after)
        let start = target_line.saturating_sub(5);
        let end = std::cmp::min(target_line + 5, lines.len());
        
        lines[start..end].join("\n")
    }

    fn parse_completions(&self, response: &str) -> Vec<String> {
        // Simple parsing - in a real implementation, this would be more sophisticated
        response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(3) // Return top 3 suggestions
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn parse_security_issues(&self, response: &str) -> Vec<String> {
        response
            .lines()
            .filter(|line| !line.trim().is_empty() && line.contains("vulnerability"))
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn parse_optimizations(&self, response: &str) -> Vec<String> {
        response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect()
    }
}

// Integration with existing AI engine
impl crate::ai_engine::model_manager::ModelManager {
    pub async fn new_with_ollama(config: &crate::config::Config) -> Result<Self> {
        let ollama_client = OllamaClient::new(
            "http://localhost:11434".to_string(),
            "codellama:7b-instruct".to_string(),
        );

        // Check if Ollama is available
        if ollama_client.check_health().await.is_ok() {
            println!("✅ Ollama connection established");
        } else {
            println!("⚠️  Ollama not available, using fallback mode");
        }

        Ok(Self {
            model_loaded: true,
            model_path: config.ai.model_path.to_string_lossy().to_string(),
            config: config.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_client_creation() {
        let client = OllamaClient::new(
            "http://localhost:11434".to_string(),
            "codellama:7b-instruct".to_string(),
        );
        
        assert_eq!(client.model_name, "codellama:7b-instruct");
        assert_eq!(client.base_url, "http://localhost:11434");
    }

    #[tokio::test]
    async fn test_context_extraction() {
        let client = OllamaClient::new(
            "http://localhost:11434".to_string(),
            "codellama:7b-instruct".to_string(),
        );

        let code = "line1\nline2\nline3\nline4\nline5\nline6\nline7";
        let context = client.extract_context(code, 12); // Position in line3
        
        assert!(context.contains("line3"));
    }
}