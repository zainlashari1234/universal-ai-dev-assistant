use anyhow::Result;
use colored::*;
use crate::client::Client;

pub async fn run(detailed: bool, health: bool, client: &Client) -> Result<()> {
    println!("{}", "📊 UAIDA System Status".bright_blue().bold());
    println!();

    // Basic health check
    println!("{}", "🏥 Health Check".bright_white().bold());
    match client.get("/health").await {
        Ok(response) => {
            println!("  {} {}", "Status:".bright_white(), "Healthy".bright_green().bold());
            
            if let Some(version) = response.get("version").and_then(|v| v.as_str()) {
                println!("  {} {}", "Version:".bright_white(), version.bright_cyan());
            }
            
            if let Some(uptime) = response.get("uptime_seconds").and_then(|u| u.as_u64()) {
                let hours = uptime / 3600;
                let minutes = (uptime % 3600) / 60;
                let seconds = uptime % 60;
                println!("  {} {}h {}m {}s", "Uptime:".bright_white(), hours, minutes, seconds);
            }
        }
        Err(e) => {
            println!("  {} {}", "Status:".bright_white(), "Unhealthy".bright_red().bold());
            println!("  {} {}", "Error:".bright_white(), e.to_string().bright_red());
            return Ok(());
        }
    }

    println!();

    // Provider status
    if health {
        println!("{}", "🔌 Provider Health".bright_white().bold());
        match client.get("/api/v1/providers/health").await {
            Ok(providers) => {
                if let Some(provider_list) = providers.as_array() {
                    for provider in provider_list {
                        if let (Some(name), Some(status)) = (
                            provider.get("name").and_then(|n| n.as_str()),
                            provider.get("status").and_then(|s| s.as_str())
                        ) {
                            let status_icon = match status {
                                "healthy" => "✅".bright_green(),
                                "degraded" => "⚠️".bright_yellow(),
                                "unhealthy" => "❌".bright_red(),
                                _ => "❓".bright_white(),
                            };
                            println!("  {} {} ({})", status_icon, name.bright_cyan(), status);
                            
                            if detailed {
                                if let Some(latency) = provider.get("latency_ms").and_then(|l| l.as_u64()) {
                                    println!("    {} {}ms", "Latency:".bright_white(), latency);
                                }
                                if let Some(success_rate) = provider.get("success_rate").and_then(|s| s.as_f64()) {
                                    println!("    {} {:.1}%", "Success Rate:".bright_white(), success_rate * 100.0);
                                }
                            }
                        }
                    }
                } else {
                    println!("  {} No provider data available", "⚠️".bright_yellow());
                }
            }
            Err(e) => {
                println!("  {} Failed to get provider status: {}", "❌".bright_red(), e);
            }
        }
        println!();
    }

    // System metrics
    if detailed {
        println!("{}", "📈 System Metrics".bright_white().bold());
        match client.get("/metrics").await {
            Ok(_) => {
                println!("  {} Metrics endpoint available", "✅".bright_green());
                println!("  {} View at: {}", "🔗".bright_blue(), "http://localhost:8080/metrics".bright_cyan());
            }
            Err(_) => {
                println!("  {} Metrics endpoint unavailable", "❌".bright_red());
            }
        }
        println!();

        // Configuration status
        println!("{}", "⚙️ Configuration".bright_white().bold());
        
        // Check if config file exists
        match crate::config::Config::default_config_path() {
            Ok(config_path) => {
                if config_path.exists() {
                    println!("  {} Configuration file found", "✅".bright_green());
                    println!("    {} {}", "Path:".bright_white(), config_path.display().to_string().bright_cyan());
                    
                    // Load and show basic config info
                    if let Ok(config) = crate::config::Config::load(None) {
                        let enabled_providers = config.get_enabled_providers();
                        println!("    {} {}", "Enabled Providers:".bright_white(), enabled_providers.len());
                        for provider in enabled_providers {
                            println!("      • {}", provider.bright_cyan());
                        }
                    }
                } else {
                    println!("  {} No configuration file found", "⚠️".bright_yellow());
                    println!("    {} Run 'uaida init' to create configuration", "💡".bright_blue());
                }
            }
            Err(e) => {
                println!("  {} Failed to check configuration: {}", "❌".bright_red(), e);
            }
        }
    }

    Ok(())
}