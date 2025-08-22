use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input};
use crate::client::Client;

pub async fn run(
    initial_message: Option<String>,
    mode: String,
    client: &Client,
) -> Result<()> {
    println!("{}", "💬 UAIDA Interactive Chat".bright_blue().bold());
    println!("{}", format!("Mode: {}", mode).bright_cyan());
    println!("{}", "Type 'exit' or 'quit' to end the session".bright_yellow());
    println!("{}", "─".repeat(50).bright_black());
    println!();

    let mut conversation_history = Vec::new();

    // Handle initial message if provided
    if let Some(message) = initial_message {
        println!("{} {}", "You:".bright_green().bold(), message);
        let response = send_message(&message, &mode, &conversation_history, client).await?;
        println!("{} {}", "UAIDA:".bright_blue().bold(), response);
        conversation_history.push(("user".to_string(), message));
        conversation_history.push(("assistant".to_string(), response));
        println!();
    }

    // Interactive chat loop
    loop {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("You")
            .interact_text()?;

        if input.trim().to_lowercase() == "exit" || input.trim().to_lowercase() == "quit" {
            println!("{}", "👋 Goodbye!".bright_yellow());
            break;
        }

        if input.trim().is_empty() {
            continue;
        }

        // Special commands
        if input.starts_with('/') {
            handle_special_command(&input, client).await?;
            continue;
        }

        // Send message and get response
        match send_message(&input, &mode, &conversation_history, client).await {
            Ok(response) => {
                println!("{} {}", "UAIDA:".bright_blue().bold(), response);
                conversation_history.push(("user".to_string(), input));
                conversation_history.push(("assistant".to_string(), response));
            }
            Err(e) => {
                println!("{} {}", "❌ Error:".bright_red().bold(), e);
            }
        }
        println!();
    }

    Ok(())
}

async fn send_message(
    message: &str,
    mode: &str,
    history: &[(String, String)],
    client: &Client,
) -> Result<String> {
    let chat_request = serde_json::json!({
        "message": message,
        "mode": mode,
        "history": history,
        "stream": false
    });

    let response = client.post("/api/v1/chat", &chat_request).await?;
    
    if let Some(reply) = response.get("response").and_then(|r| r.as_str()) {
        Ok(reply.to_string())
    } else {
        Ok("Sorry, I couldn't generate a response.".to_string())
    }
}

async fn handle_special_command(command: &str, client: &Client) -> Result<()> {
    match command {
        "/help" => {
            println!("{}", "Available commands:".bright_white().bold());
            println!("  /help     - Show this help");
            println!("  /status   - Show system status");
            println!("  /clear    - Clear conversation history");
            println!("  /mode     - Change chat mode");
            println!("  exit/quit - Exit chat");
        }
        "/status" => {
            let status = client.get("/health").await?;
            println!("{}", "System Status:".bright_white().bold());
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        "/clear" => {
            println!("{}", "🧹 Conversation history cleared".bright_yellow());
        }
        "/mode" => {
            println!("{}", "Available modes: code, general, debug".bright_white());
        }
        _ => {
            println!("{}", "Unknown command. Type /help for available commands.".bright_red());
        }
    }
    Ok(())
}