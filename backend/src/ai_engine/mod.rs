use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{config::Config, CompletionRequest};

pub mod code_completion;
pub mod code_analysis;
pub mod model_manager;
pub mod multi_agent_system;
pub mod predictive_debugging;
pub mod real_time_completion;
pub mod emotional_ai_programming;
pub mod musical_code_composition;
pub mod quantum_code_optimization;
pub mod competitive_programming_arena;
pub mod code_smell_detector;
pub mod intelligent_autocomplete;
pub mod code_time_travel;
pub mod ai_pair_programming;
pub mod providers;

use model_manager::ModelManager;
use providers::{ProviderRouter, ProviderConfig, ProviderType, RoutingPolicy};

pub struct AIEngine {
    model_manager: Arc<RwLock<ModelManager>>,
    provider_router: Arc<ProviderRouter>,
    config: Arc<Config>,
}

impl AIEngine {
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Initializing AI Engine...");
        
        let model_manager = ModelManager::new(config).await?;
        
        // Initialize Provider Router with default configs
        let provider_configs = vec![
            ProviderConfig {
                provider_type: ProviderType::Ollama,
                endpoint: Some("http://localhost:11434".to_string()),
                model: Some("qwen2.5-coder:7b".to_string()),
                timeout_ms: 30000,
                max_retries: 3,
            },
            ProviderConfig {
                provider_type: ProviderType::Heuristic,
                endpoint: None,
                model: None,
                timeout_ms: 1000,
                max_retries: 1,
            },
        ];
        
        let routing_policy = RoutingPolicy::default();
        let provider_router = ProviderRouter::new(provider_configs, routing_policy).await?;
        
        info!("AI Engine initialized with Provider Router");
        
        Ok(Self {
            model_manager: Arc::new(RwLock::new(model_manager)),
            provider_router: Arc::new(provider_router),
            config: Arc::new(config.clone()),
        })
    }

    pub async fn is_model_loaded(&self) -> bool {
        // Check if either the model manager or provider router is available
        let manager = self.model_manager.read().await;
        let model_loaded = manager.is_loaded();
        
        // Also check provider router health
        match self.provider_router.health().await {
            Ok(health) => model_loaded || health.is_available,
            Err(_) => model_loaded,
        }
    }

    pub async fn complete_code(&self, request: &CompletionRequest) -> Result<Vec<String>> {
        // Extract context around cursor position
        let context = self.extract_context(request)?;
        
        // Build completion prompt
        let prompt = self.build_completion_prompt(&context, &request.language)?;
        
        // Use Provider Router for completion (with fallback support)
        match self.provider_router.complete(&prompt, Some(&request.language)).await {
            Ok(completions) => {
                // Post-process completions
                let filtered_completions = self.filter_completions(completions, request)?;
                Ok(filtered_completions)
            }
            Err(e) => {
                warn!("Provider router completion failed, trying model manager: {}", e);
                
                // Fallback to original model manager if provider router fails
                let manager = self.model_manager.read().await;
                if manager.is_loaded() {
                    let completions = manager.generate_completion(&prompt).await?;
                    let filtered_completions = self.filter_completions(completions, request)?;
                    Ok(filtered_completions)
                } else {
                    Err(anyhow!("All completion methods failed: {}", e))
                }
            }
        }
    }

    pub async fn analyze_code(&self, request: &CompletionRequest) -> Result<Value> {
        // Use Provider Router for analysis (with fallback support)
        match self.provider_router.analyze(&request.code, &request.language).await {
            Ok(analysis) => Ok(analysis),
            Err(e) => {
                warn!("Provider router analysis failed, trying model manager: {}", e);
                
                // Fallback to original model manager if provider router fails
                let manager = self.model_manager.read().await;
                if manager.is_loaded() {
                    let prompt = self.build_analysis_prompt(&request.code, &request.language)?;
                    let analysis = manager.generate_analysis(&prompt).await?;
                    Ok(analysis)
                } else {
                    Err(anyhow!("All analysis methods failed: {}", e))
                }
            }
        }
    }

    fn extract_context(&self, request: &CompletionRequest) -> Result<String> {
        let code = &request.code;
        let cursor_pos = request.cursor_position;
        
        // Extract lines around cursor position
        let lines: Vec<&str> = code.lines().collect();
        let mut line_pos = 0;
        let mut current_line = 0;
        
        // Find which line the cursor is on
        for (i, line) in lines.iter().enumerate() {
            if line_pos + line.len() >= cursor_pos {
                current_line = i;
                break;
            }
            line_pos += line.len() + 1; // +1 for newline
        }
        
        // Get context window (5 lines before and after)
        let start_line = current_line.saturating_sub(5);
        let end_line = std::cmp::min(current_line + 5, lines.len());
        
        let context_lines = &lines[start_line..end_line];
        Ok(context_lines.join("\n"))
    }

    fn build_completion_prompt(&self, context: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Complete the following {} code. Provide only the completion, no explanations:\n\n```{}\n{}\n```\n\nCompletion:",
            language, language, context
        );
        Ok(prompt)
    }

    fn build_analysis_prompt(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Analyze the following {} code for potential issues, improvements, and suggestions:\n\n```{}\n{}\n```\n\nProvide analysis in JSON format with fields: issues, suggestions, complexity, security_concerns.",
            language, language, code
        );
        Ok(prompt)
    }

    fn filter_completions(&self, completions: Vec<String>, _request: &CompletionRequest) -> Result<Vec<String>> {
        // Filter out empty or invalid completions
        let filtered: Vec<String> = completions
            .into_iter()
            .filter(|completion| {
                !completion.trim().is_empty() && 
                completion.len() < 1000 && // Reasonable length limit
                !completion.contains("```") // Remove code block markers
            })
            .take(5) // Limit to top 5 suggestions
            .collect();
        
        Ok(filtered)
    }
}