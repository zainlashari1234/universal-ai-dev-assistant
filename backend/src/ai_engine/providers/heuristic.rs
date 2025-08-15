use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::traits::{Provider, ProviderHealth, ProviderMetrics};

/// Fallback provider that uses simple heuristics when AI models are unavailable
pub struct HeuristicProvider {
    metrics: Arc<RwLock<ProviderMetrics>>,
    language_patterns: HashMap<String, Vec<String>>,
}

impl HeuristicProvider {
    pub fn new() -> Self {
        let mut language_patterns = HashMap::new();
        
        // Python patterns
        language_patterns.insert("python".to_string(), vec![
            "def ".to_string(),
            "class ".to_string(),
            "import ".to_string(),
            "from ".to_string(),
            "if __name__ == '__main__':".to_string(),
            "return ".to_string(),
            "print(".to_string(),
            "len(".to_string(),
            "range(".to_string(),
            "enumerate(".to_string(),
        ]);
        
        // JavaScript/TypeScript patterns
        language_patterns.insert("javascript".to_string(), vec![
            "function ".to_string(),
            "const ".to_string(),
            "let ".to_string(),
            "var ".to_string(),
            "=> ".to_string(),
            "console.log(".to_string(),
            "return ".to_string(),
            ".map(".to_string(),
            ".filter(".to_string(),
            ".reduce(".to_string(),
        ]);
        
        language_patterns.insert("typescript".to_string(), language_patterns["javascript"].clone());
        
        // Rust patterns
        language_patterns.insert("rust".to_string(), vec![
            "fn ".to_string(),
            "struct ".to_string(),
            "enum ".to_string(),
            "impl ".to_string(),
            "use ".to_string(),
            "let ".to_string(),
            "mut ".to_string(),
            "match ".to_string(),
            "Result<".to_string(),
            "Option<".to_string(),
        ]);
        
        // Go patterns
        language_patterns.insert("go".to_string(), vec![
            "func ".to_string(),
            "type ".to_string(),
            "struct ".to_string(),
            "interface ".to_string(),
            "package ".to_string(),
            "import ".to_string(),
            "var ".to_string(),
            "const ".to_string(),
            "if err != nil".to_string(),
            "make(".to_string(),
        ]);

        Self {
            metrics: Arc::new(RwLock::new(ProviderMetrics::default())),
            language_patterns,
        }
    }

    fn analyze_context(&self, code: &str, language: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let lines: Vec<&str> = code.lines().collect();
        
        if lines.is_empty() {
            return self.get_starter_suggestions(language);
        }
        
        let last_line = lines.last().unwrap_or(&"").trim();
        
        // Context-based suggestions
        if last_line.ends_with('{') {
            suggestions.extend(self.get_block_suggestions(language));
        } else if last_line.ends_with('(') {
            suggestions.extend(self.get_parameter_suggestions(language));
        } else if last_line.contains("for ") {
            suggestions.extend(self.get_loop_suggestions(language));
        } else if last_line.contains("if ") {
            suggestions.extend(self.get_condition_suggestions(language));
        } else {
            suggestions.extend(self.get_general_suggestions(language, last_line));
        }
        
        suggestions.truncate(5);
        suggestions
    }

    fn get_starter_suggestions(&self, language: &str) -> Vec<String> {
        match language {
            "python" => vec![
                "def main():".to_string(),
                "class MyClass:".to_string(),
                "import os".to_string(),
                "if __name__ == '__main__':".to_string(),
            ],
            "javascript" | "typescript" => vec![
                "function main() {".to_string(),
                "const result = ".to_string(),
                "console.log(".to_string(),
                "export function ".to_string(),
            ],
            "rust" => vec![
                "fn main() {".to_string(),
                "struct MyStruct {".to_string(),
                "use std::".to_string(),
                "pub fn ".to_string(),
            ],
            "go" => vec![
                "func main() {".to_string(),
                "type MyStruct struct {".to_string(),
                "package main".to_string(),
                "import \"fmt\"".to_string(),
            ],
            _ => vec![
                "// TODO: Implement".to_string(),
                "function main() {".to_string(),
            ],
        }
    }

    fn get_block_suggestions(&self, language: &str) -> Vec<String> {
        match language {
            "python" => vec![
                "    pass".to_string(),
                "    return".to_string(),
                "    print(".to_string(),
            ],
            "javascript" | "typescript" => vec![
                "    return;".to_string(),
                "    console.log();".to_string(),
                "    // TODO".to_string(),
            ],
            "rust" => vec![
                "    todo!()".to_string(),
                "    Ok(())".to_string(),
                "    return;".to_string(),
            ],
            "go" => vec![
                "    return".to_string(),
                "    fmt.Println()".to_string(),
                "    // TODO".to_string(),
            ],
            _ => vec![
                "    return;".to_string(),
                "    // TODO".to_string(),
            ],
        }
    }

    fn get_parameter_suggestions(&self, language: &str) -> Vec<String> {
        match language {
            "python" => vec![
                "self".to_string(),
                "*args".to_string(),
                "**kwargs".to_string(),
            ],
            "javascript" | "typescript" => vec![
                "event".to_string(),
                "...args".to_string(),
                "callback".to_string(),
            ],
            "rust" => vec![
                "&self".to_string(),
                "mut self".to_string(),
                "&mut self".to_string(),
            ],
            "go" => vec![
                "ctx context.Context".to_string(),
                "w http.ResponseWriter".to_string(),
                "r *http.Request".to_string(),
            ],
            _ => vec![
                "param".to_string(),
            ],
        }
    }

    fn get_loop_suggestions(&self, language: &str) -> Vec<String> {
        match language {
            "python" => vec![
                "for i in range(".to_string(),
                "for item in items:".to_string(),
                "for key, value in dict.items():".to_string(),
            ],
            "javascript" | "typescript" => vec![
                "for (let i = 0; i < ".to_string(),
                "for (const item of items) {".to_string(),
                "for (const [key, value] of Object.entries(".to_string(),
            ],
            "rust" => vec![
                "for item in items.iter() {".to_string(),
                "for (i, item) in items.iter().enumerate() {".to_string(),
                "for i in 0..".to_string(),
            ],
            "go" => vec![
                "for i := 0; i < ".to_string(),
                "for _, item := range items {".to_string(),
                "for key, value := range map {".to_string(),
            ],
            _ => vec![
                "for (int i = 0; i < ".to_string(),
            ],
        }
    }

    fn get_condition_suggestions(&self, language: &str) -> Vec<String> {
        match language {
            "python" => vec![
                "if condition:".to_string(),
                "elif ".to_string(),
                "else:".to_string(),
            ],
            "javascript" | "typescript" => vec![
                "if (condition) {".to_string(),
                "} else if (".to_string(),
                "} else {".to_string(),
            ],
            "rust" => vec![
                "if condition {".to_string(),
                "} else if ".to_string(),
                "} else {".to_string(),
            ],
            "go" => vec![
                "if condition {".to_string(),
                "} else if ".to_string(),
                "} else {".to_string(),
            ],
            _ => vec![
                "if (condition) {".to_string(),
            ],
        }
    }

    fn get_general_suggestions(&self, language: &str, context: &str) -> Vec<String> {
        if let Some(patterns) = self.language_patterns.get(language) {
            // Return patterns that might make sense in context
            patterns.iter()
                .filter(|pattern| !context.contains(*pattern))
                .take(3)
                .cloned()
                .collect()
        } else {
            vec![
                "// TODO: Implement".to_string(),
                "return;".to_string(),
            ]
        }
    }
}

#[async_trait::async_trait]
impl Provider for HeuristicProvider {
    async fn complete(&self, prompt: &str, context: Option<&str>) -> Result<Vec<String>> {
        debug!("HeuristicProvider: Generating completions for prompt");
        
        let start_time = std::time::Instant::now();
        
        // Extract language from context or default to "unknown"
        let language = context
            .and_then(|ctx| {
                if ctx.contains("python") || ctx.contains(".py") { Some("python") }
                else if ctx.contains("javascript") || ctx.contains(".js") { Some("javascript") }
                else if ctx.contains("typescript") || ctx.contains(".ts") { Some("typescript") }
                else if ctx.contains("rust") || ctx.contains(".rs") { Some("rust") }
                else if ctx.contains("go") || ctx.contains(".go") { Some("go") }
                else { None }
            })
            .unwrap_or("unknown");
        
        let suggestions = self.analyze_context(prompt, language);
        
        // Record metrics
        {
            let mut metrics = self.metrics.write().await;
            let latency = start_time.elapsed().as_millis() as u64;
            metrics.record_success(latency);
        }
        
        info!("HeuristicProvider: Generated {} suggestions", suggestions.len());
        Ok(suggestions)
    }

    async fn analyze(&self, code: &str, language: &str) -> Result<Value> {
        debug!("HeuristicProvider: Analyzing code");
        
        let start_time = std::time::Instant::now();
        
        let lines = code.lines().count();
        let chars = code.chars().count();
        let complexity = if lines > 100 { "high" } else if lines > 50 { "medium" } else { "low" };
        
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        
        // Basic heuristic analysis
        if code.contains("TODO") || code.contains("FIXME") {
            issues.push("Contains TODO or FIXME comments");
        }
        
        if lines > 200 {
            suggestions.push("Consider breaking this into smaller functions");
        }
        
        if language == "python" && !code.contains("def ") && lines > 10 {
            suggestions.push("Consider organizing code into functions");
        }
        
        let analysis = serde_json::json!({
            "provider": "heuristic",
            "complexity": complexity,
            "lines_of_code": lines,
            "character_count": chars,
            "issues": issues,
            "suggestions": suggestions,
            "security_concerns": [],
            "confidence": 0.3
        });
        
        // Record metrics
        {
            let mut metrics = self.metrics.write().await;
            let latency = start_time.elapsed().as_millis() as u64;
            metrics.record_success(latency);
        }
        
        Ok(analysis)
    }

    async fn health(&self) -> Result<ProviderHealth> {
        // Heuristic provider is always available
        Ok(ProviderHealth {
            is_available: true,
            latency_ms: Some(1), // Very fast
            error_message: None,
            model_loaded: true, // Always "loaded"
        })
    }

    async fn metrics(&self) -> ProviderMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    fn name(&self) -> &'static str {
        "heuristic"
    }

    fn priority(&self) -> u8 {
        10 // Low priority, fallback only
    }
}