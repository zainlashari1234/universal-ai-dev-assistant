use anyhow::Result;
use colored::*;
use crate::client::Client;

pub async fn run(
    prompt: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    max_tokens: u32,
    temperature: f32,
    client: &Client,
) -> Result<()> {
    println!("{}", "🤖 AI Code Completion".bright_blue().bold());
    println!();

    // Show request details
    println!("{}", "Request Details:".bright_white().bold());
    println!("  📝 Prompt: {}", prompt.bright_cyan());
    if let Some(lang) = &language {
        println!("  🔤 Language: {}", lang.bright_green());
    }
    if let Some(m) = &model {
        println!("  🧠 Model: {}", m.bright_yellow());
    }
    if let Some(p) = &provider {
        println!("  🔌 Provider: {}", p.bright_magenta());
    }
    println!("  🎛️  Max Tokens: {}", max_tokens.to_string().bright_white());
    println!("  🌡️  Temperature: {}", temperature.to_string().bright_white());
    println!();

    // Make completion request
    println!("{}", "⏳ Generating completion...".bright_yellow());
    
    let completion_request = serde_json::json!({
        "prompt": prompt,
        "language": language,
        "model": model,
        "provider": provider,
        "max_tokens": max_tokens,
        "temperature": temperature
    });

    match client.post("/api/v1/complete", &completion_request).await {
        Ok(response) => {
            println!("{}", "✅ Completion generated successfully!".bright_green().bold());
            println!();
            
            if let Some(code) = response.get("completion").and_then(|c| c.as_str()) {
                println!("{}", "Generated Code:".bright_white().bold());
                println!("{}", "─".repeat(50).bright_black());
                
                // Syntax highlighting would go here in a real implementation
                println!("{}", code.bright_white());
                
                println!("{}", "─".repeat(50).bright_black());
            }
            
            // Show metadata
            println!();
            println!("{}", "📊 Completion Details:".bright_white().bold());
            
            if let Some(provider) = response.get("provider_used").and_then(|p| p.as_str()) {
                println!("  🔌 Provider: {}", provider.bright_magenta());
            }
            
            if let Some(cost) = response.get("cost").and_then(|c| c.as_f64()) {
                println!("  💰 Cost: ${:.4}", cost.to_string().bright_green());
            }
            
            if let Some(time) = response.get("response_time_ms").and_then(|t| t.as_u64()) {
                println!("  ⏱️  Response Time: {}ms", time.to_string().bright_cyan());
            }
            
            if let Some(tokens) = response.get("tokens_used").and_then(|t| t.as_u64()) {
                println!("  🔢 Tokens Used: {}", tokens.to_string().bright_cyan());
            }
            
            if let Some(confidence) = response.get("confidence").and_then(|c| c.as_f64()) {
                println!("  📈 Confidence: {:.1}%", (confidence * 100.0).to_string().bright_yellow());
            }
            
            // Show suggestions if available
            if let Some(suggestions) = response.get("suggestions").and_then(|s| s.as_array()) {
                if !suggestions.is_empty() {
                    println!();
                    println!("{}", "💡 Alternative suggestions:".bright_yellow());
                    for (i, suggestion) in suggestions.iter().enumerate() {
                        if let Some(text) = suggestion.as_str() {
                            println!("  {}. {}", i + 1, text.chars().take(100).collect::<String>().bright_white());
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("{}", "❌ Failed to generate completion".bright_red().bold());
            println!("Error: {}", e.to_string().bright_red());
            return Err(e);
        }
    }

    Ok(())
}