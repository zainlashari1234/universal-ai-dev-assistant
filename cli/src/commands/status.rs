use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::client::Client;

pub async fn run(client: &Client) -> Result<()> {
    println!("{}", "📊 UAIDA System Status".bright_blue().bold());
    println!();

    // Show progress while checking
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Checking system status...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    match client.health().await {
        Ok(health) => {
            pb.finish_and_clear();
            
            // System status
            println!("{}", "🏥 System Health".bright_green().bold());
            println!("  {} {}", "Status:".bright_white(), 
                if health.status == "healthy" { 
                    health.status.bright_green() 
                } else { 
                    health.status.bright_red() 
                });
            println!("  {} {}", "Version:".bright_white(), health.version.bright_cyan());
            println!();
            
            // Provider status
            println!("{}", "🤖 AI Providers".bright_blue().bold());
            for (name, provider) in &health.providers {
                let status_icon = if provider.is_available { "✅" } else { "❌" };
                let status_text = if provider.is_available { 
                    "Available".bright_green() 
                } else { 
                    "Unavailable".bright_red() 
                };
                
                println!("  {} {} {}", status_icon, name.bright_white().bold(), status_text);
                
                if let Some(response_time) = provider.response_time_ms {
                    println!("    {} {}ms", "Response time:".bright_white().dimmed(), 
                        response_time.to_string().bright_yellow());
                }
                
                if !provider.models_available.is_empty() {
                    println!("    {} {}", "Models:".bright_white().dimmed(), 
                        provider.models_available.len().to_string().bright_yellow());
                }
                
                if let Some(error) = &provider.error_message {
                    println!("    {} {}", "Error:".bright_red().dimmed(), error.bright_red());
                }
            }
            println!();
            
            // Features
            println!("{}", "🎯 Available Features".bright_cyan().bold());
            for feature in &health.features {
                println!("  ✨ {}", feature.bright_white());
            }
            println!();
            
            // Get additional metrics
            if let Ok(providers_info) = client.providers().await {
                println!("{}", "📈 Provider Metrics".bright_magenta().bold());
                for (name, metrics) in &providers_info.provider_metrics {
                    if metrics.total_requests > 0 {
                        println!("  {} {}", name.bright_white().bold(), ":");
                        println!("    {} {}", "Total requests:".bright_white().dimmed(), 
                            metrics.total_requests.to_string().bright_yellow());
                        println!("    {} {:.1}%", "Success rate:".bright_white().dimmed(), 
                            (metrics.successful_requests as f64 / metrics.total_requests as f64 * 100.0).to_string().bright_green());
                        println!("    {} {:.1}ms", "Avg response:".bright_white().dimmed(), 
                            metrics.average_response_time_ms.to_string().bright_cyan());
                        if metrics.cost_usd > 0.0 {
                            println!("    {} ${:.4}", "Total cost:".bright_white().dimmed(), 
                                metrics.cost_usd.to_string().bright_yellow());
                        }
                    }
                }
                
                if let Some(recommended) = &providers_info.recommended_provider {
                    println!();
                    println!("{} {}", "🏆 Recommended provider:".bright_yellow().bold(), 
                        recommended.bright_green().bold());
                }
            }
            
        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Failed to connect to UAIDA server", "❌".bright_red());
            println!("   {}", e.to_string().bright_red());
            println!();
            println!("{}", "💡 Troubleshooting:".bright_yellow().bold());
            println!("  • Make sure the UAIDA server is running");
            println!("  • Check the server URL in your configuration");
            println!("  • Verify network connectivity");
            println!("  • Try: {}", "uaida config show".bright_green());
        }
    }

    Ok(())
}