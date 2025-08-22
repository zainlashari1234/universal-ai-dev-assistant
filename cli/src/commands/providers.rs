use anyhow::Result;
use colored::*;
use crate::client::Client;

pub async fn run(
    action: String,
    name: Option<String>,
    key: Option<String>,
    client: &Client,
) -> Result<()> {
    match action.as_str() {
        "list" => list_providers(client).await,
        "test" => test_providers(name, client).await,
        "add" => add_provider(name, key).await,
        "remove" => remove_provider(name).await,
        _ => {
            println!("{}", "❌ Unknown action. Available: list, test, add, remove".bright_red());
            Ok(())
        }
    }
}

async fn list_providers(client: &Client) -> Result<()> {
    println!("{}", "🔌 AI Providers".bright_blue().bold());
    println!();

    match client.get("/api/v1/providers").await {
        Ok(response) => {
            if let Some(providers) = response.as_array() {
                for provider in providers {
                    if let Some(name) = provider.get("name").and_then(|n| n.as_str()) {
                        let status = provider.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
                        let enabled = provider.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                        
                        let status_icon = if enabled {
                            match status {
                                "healthy" => "✅".bright_green(),
                                "degraded" => "⚠️".bright_yellow(), 
                                "unhealthy" => "❌".bright_red(),
                                _ => "❓".bright_white(),
                            }
                        } else {
                            "⏸️".bright_black()
                        };

                        println!("{} {} ({})", status_icon, name.bright_cyan().bold(), status);
                        println!();
                    }
                }
            }
        }
        Err(e) => {
            println!("{} Failed to get providers: {}", "❌".bright_red(), e);
        }
    }

    Ok(())
}

async fn test_providers(name: Option<String>, client: &Client) -> Result<()> {
    println!("{}", "🧪 Testing Providers".bright_blue().bold());
    println!();

    let test_prompt = "Hello, respond with 'Test successful'";
    
    if let Some(provider_name) = name {
        test_single_provider(&provider_name, test_prompt, client).await?;
    } else {
        println!("Testing all enabled providers...");
    }

    Ok(())
}

async fn test_single_provider(name: &str, prompt: &str, client: &Client) -> Result<()> {
    print!("Testing {}... ", name.bright_cyan());
    
    let test_request = serde_json::json!({
        "prompt": prompt,
        "provider": name,
        "max_tokens": 50
    });

    match client.post("/api/v1/complete", &test_request).await {
        Ok(_) => {
            println!("{}", "✅ Success".bright_green());
        }
        Err(e) => {
            println!("{} {}", "❌ Failed:".bright_red(), e);
        }
    }

    Ok(())
}

async fn add_provider(name: Option<String>, key: Option<String>) -> Result<()> {
    let provider_name = name.ok_or_else(|| anyhow::anyhow!("Provider name required"))?;
    let api_key = key.ok_or_else(|| anyhow::anyhow!("API key required"))?;

    println!("{}", "➕ Adding Provider".bright_blue().bold());
    
    let mut config = crate::config::Config::load(None)?;
    config.set_provider_api_key(&provider_name, api_key);
    
    let config_path = crate::config::Config::default_config_path()?;
    config.save(&config_path)?;
    
    println!("{} Provider {} added successfully", "✅".bright_green(), provider_name.bright_cyan());
    
    Ok(())
}

async fn remove_provider(name: Option<String>) -> Result<()> {
    let provider_name = name.ok_or_else(|| anyhow::anyhow!("Provider name required"))?;

    println!("{}", "➖ Removing Provider".bright_blue().bold());
    
    let mut config = crate::config::Config::load(None)?;
    
    if let Some(provider_config) = config.providers.get_mut(&provider_name) {
        provider_config.enabled = false;
        provider_config.api_key = None;
        
        let config_path = crate::config::Config::default_config_path()?;
        config.save(&config_path)?;
        
        println!("{} Provider {} removed", "✅".bright_green(), provider_name.bright_cyan());
    } else {
        println!("{} Provider {} not found", "❌".bright_red(), provider_name.bright_cyan());
    }
    
    Ok(())
}