use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::client::{Client, CompletionRequest};

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

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Generating completion...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let request = CompletionRequest {
        prompt: prompt.clone(),
        language,
        model,
        provider,
        max_tokens: Some(max_tokens),
        temperature: Some(temperature),
        system_prompt: Some("You are an expert programmer. Provide clean, efficient, and well-commented code completions.".to_string()),
    };

    match client.complete(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            println!("{}", "📝 Input:".bright_yellow().bold());
            println!("{}", prompt.bright_white());
            println!();
            
            println!("{}", "✨ Completion:".bright_green().bold());
            println!("{}", response.text.bright_white());
            println!();
            
            println!("{}", "📊 Details:".bright_cyan().bold());
            println!("  {} {}", "Model:".bright_white(), response.model.bright_green());
            println!("  {} {}", "Provider:".bright_white(), response.provider.bright_green());
            
            if let Some(usage) = response.usage {
                println!("  {} {}", "Tokens:".bright_white(), format!("{}", usage.total_tokens).bright_yellow());
                if let Some(cost) = usage.cost_usd {
                    println!("  {} ${:.6}", "Cost:".bright_white(), cost);
                }
            }
        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} {}", "❌ Error:".bright_red().bold(), e);
        }
    }

    Ok(())
}