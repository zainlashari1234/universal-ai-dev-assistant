use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input};
use std::io::{self, Write};

use crate::client::{Client, CompletionRequest};

pub async fn run(
    initial_message: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    client: &Client,
) -> Result<()> {
    println!("{}", "💬 AI Chat Assistant".bright_blue().bold());
    println!("{}", "Type 'exit', 'quit', or press Ctrl+C to end the conversation".bright_white().dimmed());
    println!();

    let mut conversation_history = Vec::new();

    // Handle initial message if provided
    if let Some(message) = initial_message {
        process_message(message, &model, &provider, client, &mut conversation_history).await?;
    }

    // Interactive chat loop
    loop {
        print!("{} ", "You:".bright_green().bold());
        io::stdout().flush()?;

        let input: String = Input::with_theme(&ColorfulTheme::default())
            .allow_empty(false)
            .interact_text()?;

        if input.trim().to_lowercase() == "exit" 
            || input.trim().to_lowercase() == "quit" 
            || input.trim().to_lowercase() == "q" {
            println!("{}", "👋 Goodbye!".bright_blue());
            break;
        }

        if input.trim().to_lowercase() == "clear" {
            conversation_history.clear();
            println!("{}", "🧹 Conversation history cleared".bright_yellow());
            continue;
        }

        if input.trim().to_lowercase() == "help" {
            print_help();
            continue;
        }

        process_message(input, &model, &provider, client, &mut conversation_history).await?;
    }

    Ok(())
}

async fn process_message(
    message: String,
    model: &Option<String>,
    provider: &Option<String>,
    client: &Client,
    conversation_history: &mut Vec<(String, String)>,
) -> Result<()> {
    // Build context from conversation history
    let mut context = String::new();
    for (user_msg, ai_msg) in conversation_history.iter() {
        context.push_str(&format!("User: {}\nAssistant: {}\n\n", user_msg, ai_msg));
    }
    context.push_str(&format!("User: {}\nAssistant:", message));

    let request = CompletionRequest {
        prompt: context,
        language: None,
        model: model.clone(),
        provider: provider.clone(),
        max_tokens: Some(2000),
        temperature: Some(0.7),
        system_prompt: Some("You are a helpful AI assistant specialized in software development. Provide clear, accurate, and helpful responses. When discussing code, provide examples and explanations.".to_string()),
    };

    print!("{} ", "AI:".bright_blue().bold());
    io::stdout().flush()?;

    match client.complete(request).await {
        Ok(response) => {
            println!("{}", response.text.bright_white());
            
            // Add to conversation history
            conversation_history.push((message, response.text));
            
            // Keep only last 10 exchanges to manage context length
            if conversation_history.len() > 10 {
                conversation_history.remove(0);
            }
            
            println!();
        }
        Err(e) => {
            println!("{} {}", "❌ Error:".bright_red().bold(), e);
            println!();
        }
    }

    Ok(())
}

fn print_help() {
    println!("{}", "💡 Chat Commands:".bright_yellow().bold());
    println!("  {} - Exit the chat", "exit, quit, q".bright_green());
    println!("  {} - Clear conversation history", "clear".bright_green());
    println!("  {} - Show this help", "help".bright_green());
    println!();
    println!("{}", "💬 Tips:".bright_cyan().bold());
    println!("  • Ask about code, algorithms, debugging, or any programming topic");
    println!("  • Request code examples, explanations, or reviews");
    println!("  • The AI remembers the conversation context");
    println!("  • Use 'clear' to start a fresh conversation");
    println!();
}