use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::client::Client;
use crate::commands::ProviderCommands;

pub async fn run(action: ProviderCommands, client: &Client) -> Result<()> {
    match action {
        ProviderCommands::List => list_providers(client).await,
        ProviderCommands::Test { provider } => test_providers(client, provider).await,
        ProviderCommands::Metrics { provider } => show_metrics(client, provider).await,
        ProviderCommands::Configure { provider } => configure_provider(provider).await,
    }
}

async fn list_providers(client: &Client) -> Result<()> {
    println!("{}", "🤖 AI Providers Status".bright_blue().bold());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Checking provider status...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    match client.providers().await {
        Ok(response) => {
            pb.finish_and_clear();
            
            println!("{}", "📊 Available Providers:".bright_green().bold());
            for provider in &response.available_providers {
                println!("  {} {}", "✅".bright_green(), provider.bright_white().bold());
            }
            
            if let Some(recommended) = &response.recommended_provider {
                println!();
                println!("{} {}", "🏆 Recommended:".bright_yellow().bold(), recommended.bright_green().bold());
            }
            
            println!();
            println!("{}", "📈 Provider Metrics:".bright_cyan().bold());
            
            for (name, metrics) in &response.provider_metrics {
                if metrics.total_requests > 0 {
                    println!();
                    println!("  {} {}", "🔹".bright_blue(), name.bright_white().bold());
                    println!("    {} {}", "Total requests:".bright_white(), metrics.total_requests.to_string().bright_yellow());
                    println!("    {} {:.1}%", "Success rate:".bright_white(), 
                        (metrics.successful_requests as f64 / metrics.total_requests as f64 * 100.0).to_string().bright_green());
                    println!("    {} {:.1}ms", "Avg response:".bright_white(), metrics.average_response_time_ms.to_string().bright_cyan());
                    println!("    {} {}", "Tokens processed:".bright_white(), metrics.tokens_processed.to_string().bright_magenta());
                    if metrics.cost_usd > 0.0 {
                        println!("    {} ${:.4}", "Total cost:".bright_white(), metrics.cost_usd.to_string().bright_yellow());
                    }
                }
            }
        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Failed to get provider information: {}", "❌".bright_red(), e);
        }
    }

    Ok(())
}

async fn test_providers(client: &Client, provider: Option<String>) -> Result<()> {
    println!("{}", "🧪 Provider Connectivity Test".bright_blue().bold());
    println!();

    if let Some(provider_name) = provider {
        test_single_provider(client, &provider_name).await
    } else {
        test_all_providers(client).await
    }
}

async fn test_single_provider(client: &Client, provider_name: &str) -> Result<()> {
    println!("Testing provider: {}", provider_name.bright_white().bold());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message(format!("Testing {}...", provider_name));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Test with a simple completion request
    let request = crate::client::CompletionRequest {
        prompt: "Hello, world!".to_string(),
        language: Some("text".to_string()),
        model: None,
        provider: Some(provider_name.to_string()),
        max_tokens: Some(10),
        temperature: Some(0.7),
        system_prompt: None,
    };

    let start_time = std::time::Instant::now();
    match client.complete(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            let response_time = start_time.elapsed().as_millis();
            
            println!("{} {} is working correctly", "✅".bright_green(), provider_name.bright_white().bold());
            println!("  {} {}ms", "Response time:".bright_white(), response_time.to_string().bright_cyan());
            println!("  {} {}", "Model used:".bright_white(), response.model.bright_green());
            println!("  {} {} characters", "Response length:".bright_white(), response.text.len().to_string().bright_yellow());
            
            if let Some(usage) = response.usage {
                println!("  {} {} tokens", "Tokens used:".bright_white(), usage.total_tokens.to_string().bright_magenta());
                if let Some(cost) = usage.cost_usd {
                    println!("  {} ${:.6}", "Cost:".bright_white(), cost.to_string().bright_yellow());
                }
            }
        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} {} failed: {}", "❌".bright_red(), provider_name.bright_white().bold(), e);
        }
    }

    Ok(())
}

async fn test_all_providers(client: &Client) -> Result<()> {
    let providers = match client.providers().await {
        Ok(response) => response.available_providers,
        Err(e) => {
            println!("{} Failed to get provider list: {}", "❌".bright_red(), e);
            return Ok(());
        }
    };

    for provider in providers {
        test_single_provider(client, &provider).await?;
        println!();
    }

    Ok(())
}

async fn show_metrics(client: &Client, provider: Option<String>) -> Result<()> {
    println!("{}", "📊 Provider Metrics".bright_blue().bold());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Fetching metrics...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    match client.metrics().await {
        Ok(metrics) => {
            pb.finish_and_clear();
            
            if let Some(provider_metrics) = metrics.get("provider_metrics") {
                if let Some(specific_provider) = provider {
                    show_single_provider_metrics(&specific_provider, provider_metrics);
                } else {
                    show_all_provider_metrics(provider_metrics);
                }
            }
            
            if let Some(system_info) = metrics.get("system_info") {
                println!();
                println!("{}", "🖥️ System Information:".bright_cyan().bold());
                if let Some(version) = system_info.get("version") {
                    println!("  {} {}", "Version:".bright_white(), version.as_str().unwrap_or("unknown").bright_green());
                }
                if let Some(total_requests) = system_info.get("total_requests") {
                    println!("  {} {}", "Total requests:".bright_white(), total_requests.as_u64().unwrap_or(0).to_string().bright_yellow());
                }
                if let Some(total_cost) = system_info.get("total_cost_usd") {
                    println!("  {} ${:.4}", "Total cost:".bright_white(), total_cost.as_f64().unwrap_or(0.0).to_string().bright_yellow());
                }
            }
        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Failed to get metrics: {}", "❌".bright_red(), e);
        }
    }

    Ok(())
}

fn show_single_provider_metrics(provider_name: &str, metrics: &serde_json::Value) {
    println!("📈 Metrics for {}", provider_name.bright_white().bold());
    
    if let Some(provider_data) = metrics.get(provider_name) {
        display_provider_metrics(provider_name, provider_data);
    } else {
        println!("  {} No metrics available for {}", "⚠️".bright_yellow(), provider_name);
    }
}

fn show_all_provider_metrics(metrics: &serde_json::Value) {
    println!("{}", "📈 All Provider Metrics:".bright_green().bold());
    
    if let Some(obj) = metrics.as_object() {
        for (provider_name, provider_data) in obj {
            println!();
            display_provider_metrics(provider_name, provider_data);
        }
    }
}

fn display_provider_metrics(provider_name: &str, data: &serde_json::Value) {
    println!("  {} {}", "🔹".bright_blue(), provider_name.bright_white().bold());
    
    if let Some(total_requests) = data.get("total_requests") {
        println!("    {} {}", "Total requests:".bright_white(), total_requests.as_u64().unwrap_or(0).to_string().bright_yellow());
    }
    
    if let Some(successful) = data.get("successful_requests") {
        if let Some(total) = data.get("total_requests") {
            let success_rate = successful.as_u64().unwrap_or(0) as f64 / total.as_u64().unwrap_or(1) as f64 * 100.0;
            println!("    {} {:.1}%", "Success rate:".bright_white(), success_rate.to_string().bright_green());
        }
    }
    
    if let Some(avg_time) = data.get("average_response_time_ms") {
        println!("    {} {:.1}ms", "Avg response time:".bright_white(), avg_time.as_f64().unwrap_or(0.0).to_string().bright_cyan());
    }
    
    if let Some(tokens) = data.get("total_tokens") {
        println!("    {} {}", "Tokens processed:".bright_white(), tokens.as_u64().unwrap_or(0).to_string().bright_magenta());
    }
    
    if let Some(cost) = data.get("total_cost_usd") {
        let cost_val = cost.as_f64().unwrap_or(0.0);
        if cost_val > 0.0 {
            println!("    {} ${:.4}", "Total cost:".bright_white(), cost_val.to_string().bright_yellow());
        }
    }
}

async fn configure_provider(provider: String) -> Result<()> {
    println!("{}", format!("⚙️ Configure {}", provider).bright_blue().bold());
    println!();
    
    println!("{} Provider configuration is managed through:", "ℹ️".bright_blue());
    println!("  {} Environment variables (.env file)", "1.".bright_white());
    println!("  {} Configuration file (uaida config)", "2.".bright_white());
    println!("  {} Interactive setup (uaida init)", "3.".bright_white());
    println!();
    
    match provider.as_str() {
        "openrouter" => {
            println!("{}", "🔧 OpenRouter Configuration:".bright_green().bold());
            println!("  {} Get API key from: {}", "•".bright_blue(), "https://openrouter.ai/keys".bright_cyan());
            println!("  {} Set environment variable: {}", "•".bright_blue(), "OPENROUTER_API_KEY=your_key".bright_green());
            println!("  {} Base URL: {}", "•".bright_blue(), "https://openrouter.ai/api/v1".bright_white());
        }
        "openai" => {
            println!("{}", "🔧 OpenAI Configuration:".bright_green().bold());
            println!("  {} Get API key from: {}", "•".bright_blue(), "https://platform.openai.com/api-keys".bright_cyan());
            println!("  {} Set environment variable: {}", "•".bright_blue(), "OPENAI_API_KEY=your_key".bright_green());
            println!("  {} Base URL: {}", "•".bright_blue(), "https://api.openai.com/v1".bright_white());
        }
        "anthropic" => {
            println!("{}", "🔧 Anthropic Configuration:".bright_green().bold());
            println!("  {} Get API key from: {}", "•".bright_blue(), "https://console.anthropic.com/".bright_cyan());
            println!("  {} Set environment variable: {}", "•".bright_blue(), "ANTHROPIC_API_KEY=your_key".bright_green());
            println!("  {} Base URL: {}", "•".bright_blue(), "https://api.anthropic.com".bright_white());
        }
        "google" => {
            println!("{}", "🔧 Google Gemini Configuration:".bright_green().bold());
            println!("  {} Get API key from: {}", "•".bright_blue(), "https://makersuite.google.com/app/apikey".bright_cyan());
            println!("  {} Set environment variable: {}", "•".bright_blue(), "GOOGLE_API_KEY=your_key".bright_green());
            println!("  {} Base URL: {}", "•".bright_blue(), "https://generativelanguage.googleapis.com/v1".bright_white());
        }
        "groq" => {
            println!("{}", "🔧 Groq Configuration:".bright_green().bold());
            println!("  {} Get API key from: {}", "•".bright_blue(), "https://console.groq.com/keys".bright_cyan());
            println!("  {} Set environment variable: {}", "•".bright_blue(), "GROQ_API_KEY=your_key".bright_green());
            println!("  {} Base URL: {}", "•".bright_blue(), "https://api.groq.com/openai/v1".bright_white());
        }
        "ollama" => {
            println!("{}", "🔧 Ollama Configuration:".bright_green().bold());
            println!("  {} Install Ollama: {}", "•".bright_blue(), "https://ollama.ai/download".bright_cyan());
            println!("  {} Start Ollama: {}", "•".bright_blue(), "ollama serve".bright_green());
            println!("  {} Pull models: {}", "•".bright_blue(), "ollama pull qwen2.5-coder:7b".bright_green());
            println!("  {} Base URL: {}", "•".bright_blue(), "http://localhost:11434".bright_white());
        }
        _ => {
            println!("{} Unknown provider: {}", "⚠️".bright_yellow(), provider);
            println!("Available providers: openrouter, openai, anthropic, google, groq, ollama");
        }
    }
    
    println!();
    println!("{}", "🚀 Quick Setup:".bright_magenta().bold());
    println!("  {} Run: {}", "1.".bright_white(), "uaida init".bright_green());
    println!("  {} Test: {}", "2.".bright_white(), format!("uaida providers test {}", provider).bright_green());
    println!("  {} Use: {}", "3.".bright_white(), format!("uaida complete 'hello world' --provider {}", provider).bright_green());

    Ok(())
}