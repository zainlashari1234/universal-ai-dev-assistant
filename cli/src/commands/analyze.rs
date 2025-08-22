use anyhow::Result;
use colored::*;
use std::path::PathBuf;
use crate::client::Client;

pub async fn run(
    file: PathBuf,
    analysis_type: String,
    language: Option<String>,
    client: &Client,
) -> Result<()> {
    println!("{}", "🔍 AI Code Analysis".bright_blue().bold());
    println!();

    // Read file content
    let code = match std::fs::read_to_string(&file) {
        Ok(content) => content,
        Err(e) => {
            println!("{}", format!("❌ Failed to read file: {}", e).bright_red());
            return Err(e.into());
        }
    };

    // Detect language if not provided
    let detected_language = language.unwrap_or_else(|| {
        detect_language_from_extension(&file)
    });

    // Show analysis details
    println!("{}", "Analysis Details:".bright_white().bold());
    println!("  📁 File: {}", file.display().to_string().bright_cyan());
    println!("  🔤 Language: {}", detected_language.bright_green());
    println!("  🔍 Analysis Type: {}", analysis_type.bright_yellow());
    println!("  📏 Code Length: {} lines", code.lines().count().to_string().bright_white());
    println!();

    // Make analysis request
    println!("{}", "⏳ Analyzing code...".bright_yellow());
    
    let analysis_request = serde_json::json!({
        "code": code,
        "language": detected_language,
        "analysis_type": analysis_type,
        "provider_preference": null
    });

    match client.post("/api/v1/analyze", &analysis_request).await {
        Ok(response) => {
            println!("{}", "✅ Analysis completed successfully!".bright_green().bold());
            println!();
            
            // Show analysis type
            if let Some(analysis_type) = response.get("analysis_type").and_then(|t| t.as_str()) {
                println!("{}", format!("📊 Analysis Type: {}", analysis_type).bright_white().bold());
                println!();
            }
            
            // Show summary
            if let Some(summary) = response.get("summary").and_then(|s| s.as_str()) {
                println!("{}", "📋 Summary:".bright_white().bold());
                println!("{}", "─".repeat(50).bright_black());
                println!("{}", summary.bright_white());
                println!("{}", "─".repeat(50).bright_black());
                println!();
            }
            
            // Show findings
            if let Some(findings) = response.get("findings").and_then(|f| f.as_array()) {
                if !findings.is_empty() {
                    println!("{}", "🔍 Findings:".bright_red().bold());
                    for (i, finding) in findings.iter().enumerate() {
                        if let Some(text) = finding.as_str() {
                            println!("  {}. {}", (i + 1).to_string().bright_red(), text.bright_white());
                        }
                    }
                    println!();
                }
            }
            
            // Show suggestions
            if let Some(suggestions) = response.get("suggestions").and_then(|s| s.as_array()) {
                if !suggestions.is_empty() {
                    println!("{}", "💡 Suggestions:".bright_green().bold());
                    for (i, suggestion) in suggestions.iter().enumerate() {
                        if let Some(text) = suggestion.as_str() {
                            println!("  {}. {}", (i + 1).to_string().bright_green(), text.bright_white());
                        }
                    }
                    println!();
                }
            }
            
            // Show metadata
            println!("{}", "📊 Analysis Details:".bright_white().bold());
            
            if let Some(provider) = response.get("provider_used").and_then(|p| p.as_str()) {
                println!("  🔌 Provider: {}", provider.bright_magenta());
            }
            
            if let Some(confidence) = response.get("confidence_score").and_then(|c| c.as_f64()) {
                let confidence_percent = confidence * 100.0;
                let confidence_color = if confidence_percent >= 80.0 {
                    "bright_green"
                } else if confidence_percent >= 60.0 {
                    "bright_yellow"
                } else {
                    "bright_red"
                };
                
                match confidence_color {
                    "bright_green" => println!("  📈 Confidence: {:.1}%", confidence_percent.to_string().bright_green()),
                    "bright_yellow" => println!("  📈 Confidence: {:.1}%", confidence_percent.to_string().bright_yellow()),
                    _ => println!("  📈 Confidence: {:.1}%", confidence_percent.to_string().bright_red()),
                }
            }
            
            if let Some(time) = response.get("response_time_ms").and_then(|t| t.as_u64()) {
                println!("  ⏱️  Analysis Time: {}ms", time.to_string().bright_cyan());
            }
        }
        Err(e) => {
            println!("{}", "❌ Failed to analyze code".bright_red().bold());
            println!("Error: {}", e.to_string().bright_red());
            return Err(e);
        }
    }

    Ok(())
}

fn detect_language_from_extension(file: &PathBuf) -> String {
    match file.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust".to_string(),
        Some("py") => "python".to_string(),
        Some("js") => "javascript".to_string(),
        Some("ts") => "typescript".to_string(),
        Some("java") => "java".to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
        Some("c") => "c".to_string(),
        Some("go") => "go".to_string(),
        Some("php") => "php".to_string(),
        Some("rb") => "ruby".to_string(),
        Some("swift") => "swift".to_string(),
        Some("kt") => "kotlin".to_string(),
        Some("cs") => "csharp".to_string(),
        Some("html") => "html".to_string(),
        Some("css") => "css".to_string(),
        Some("sql") => "sql".to_string(),
        Some("sh") => "bash".to_string(),
        Some("yml") | Some("yaml") => "yaml".to_string(),
        Some("json") => "json".to_string(),
        Some("xml") => "xml".to_string(),
        _ => "text".to_string(),
    }
}