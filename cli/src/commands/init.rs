use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::collections::HashMap;

use crate::config::{Config, ProviderConfig};

pub async fn run(force: bool) -> Result<()> {
    println!("{}", "🚀 UAIDA Configuration Setup".bright_blue().bold());
    println!();

    let config_path = Config::default_config_path()?;
    
    if config_path.exists() && !force {
        let overwrite = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Configuration already exists. Overwrite?")
            .default(false)
            .interact()?;
        
        if !overwrite {
            println!("{}", "✅ Configuration setup cancelled".bright_yellow());
            return Ok(());
        }
    }

    println!("{}", "Let's configure your AI providers:".bright_white());
    println!();

    let mut config = Config::default();
    
    // Configure providers
    configure_provider("OpenRouter", "openrouter", &mut config).await?;
    configure_provider("OpenAI", "openai", &mut config).await?;
    configure_provider("Anthropic", "anthropic", &mut config).await?;
    configure_provider("Google", "google", &mut config).await?;
    configure_provider("Groq", "groq", &mut config).await?;
    
    // Configure preferences
    configure_preferences(&mut config).await?;
    
    // Save configuration
    config.save(&config_path)?;
    
    println!();
    println!("{}", "✅ Configuration saved successfully!".bright_green().bold());
    println!("📁 Config file: {}", config_path.display().to_string().bright_cyan());
    println!();
    println!("{}", "🎯 Next steps:".bright_yellow().bold());
    println!("  • Test your setup: {}", "uaida status".bright_green());
    println!("  • Start coding: {}", "uaida dev".bright_green());
    println!("  • Get help: {}", "uaida --help".bright_green());
    
    Ok(())
}

async fn configure_provider(name: &str, key: &str, config: &mut Config) -> Result<()> {
    println!("{} {}", "🔧 Configuring".bright_blue(), name.bright_white().bold());
    
    let enable = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enable {} provider?", name))
        .default(false)
        .interact()?;
    
    if enable {
        let api_key: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("{} API Key", name))
            .interact_text()?;
        
        if let Some(provider_config) = config.providers.get_mut(key) {
            provider_config.enabled = true;
            provider_config.api_key = Some(api_key);
        }
        
        println!("{} {} provider configured", "✅".bright_green(), name);
    } else {
        println!("{} {} provider skipped", "⏭️".bright_yellow(), name);
    }
    
    println!();
    Ok(())
}

async fn configure_preferences(config: &mut Config) -> Result<()> {
    println!("{}", "⚙️ Preferences".bright_blue().bold());
    
    let languages = vec![
        "Auto-detect",
        "Rust",
        "Python", 
        "JavaScript",
        "TypeScript",
        "Go",
        "Java",
        "C++",
        "C",
    ];
    
    let language_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Default programming language")
        .default(0)
        .items(&languages)
        .interact()?;
    
    if language_selection > 0 {
        config.preferences.default_language = Some(languages[language_selection].to_lowercase());
    }
    
    let max_tokens: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Default max tokens")
        .default(1000)
        .interact()?;
    
    config.preferences.max_tokens = max_tokens;
    
    let temperature: f32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Default temperature (0.0-1.0)")
        .default(0.7)
        .interact()?;
    
    config.preferences.temperature = temperature.clamp(0.0, 1.0);
    
    let auto_save = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Auto-save generated code?")
        .default(true)
        .interact()?;
    
    config.preferences.auto_save = auto_save;
    
    let create_backups = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create backups when modifying files?")
        .default(true)
        .interact()?;
    
    config.preferences.create_backups = create_backups;
    
    println!("{} Preferences configured", "✅".bright_green());
    println!();
    
    Ok(())
}