use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::client::{Client, CodeActionRequest};

pub async fn run(
    file: PathBuf,
    instructions: String,
    output: Option<PathBuf>,
    backup: bool,
    client: &Client,
) -> Result<()> {
    println!("{}", "🔧 AI Code Refactorer".bright_blue().bold());
    println!();

    // Check if file exists
    if !file.exists() {
        println!("{} File not found: {}", "❌".bright_red(), file.display());
        return Ok(());
    }

    // Read file content
    let original_code = std::fs::read_to_string(&file)?;
    if original_code.trim().is_empty() {
        println!("{} File is empty", "⚠️".bright_yellow());
        return Ok(());
    }

    // Detect language
    let language = detect_language_from_extension(&file);
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| file.clone());
    let will_overwrite = output_path == file;

    println!("{}", "📁 Refactoring Info:".bright_cyan().bold());
    println!("  {} {}", "Source file:".bright_white(), file.display().to_string().bright_green());
    println!("  {} {}", "Language:".bright_white(), language.bright_green());
    println!("  {} {}", "Instructions:".bright_white(), instructions.bright_magenta());
    println!("  {} {}", "Output:".bright_white(), output_path.display().to_string().bright_cyan());
    println!("  {} {}", "Create backup:".bright_white(), 
        if backup { "Yes".bright_green() } else { "No".bright_red() }
    );
    println!();

    // Create backup if requested
    if backup && will_overwrite {
        let backup_path = create_backup(&file)?;
        println!("{} Backup created: {}", "💾".bright_blue(), backup_path.display().to_string().bright_cyan());
    }

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Refactoring code...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let detailed_instructions = format!(
        "Refactor this {} code according to these instructions: {}\n\
        \n\
        Guidelines:\n\
        - Preserve the original functionality\n\
        - Improve code readability and maintainability\n\
        - Follow {} best practices\n\
        - Add comments where helpful\n\
        - Optimize performance where possible\n\
        - Ensure the refactored code is well-structured",
        language, instructions, language
    );

    let request = CodeActionRequest {
        code: original_code.clone(),
        language: language.clone(),
        action: "refactor".to_string(),
        instructions: Some(detailed_instructions),
        target_language: None,
    };

    match client.code_action(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            let refactored_code = response.result;
            
            // Write refactored code
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output_path, &refactored_code)?;
            
            println!("{} Code refactored successfully!", "✅".bright_green());
            println!("  {} {}", "Output saved to:".bright_white(), output_path.display().to_string().bright_cyan());
            
            // Show comparison
            println!();
            println!("{}", "📊 Refactoring Summary:".bright_blue().bold());
            show_code_comparison(&original_code, &refactored_code);
            
            // Show diff preview
            println!();
            println!("{}", "🔍 Changes Preview:".bright_yellow().bold());
            show_diff_preview(&original_code, &refactored_code, 10);

        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Refactoring failed: {}", "❌".bright_red().bold(), e);
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
        Some("go") => "go".to_string(),
        Some("java") => "java".to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
        Some("c") => "c".to_string(),
        Some("php") => "php".to_string(),
        Some("rb") => "ruby".to_string(),
        Some("swift") => "swift".to_string(),
        Some("kt") => "kotlin".to_string(),
        Some("cs") => "csharp".to_string(),
        _ => "text".to_string(),
    }
}

fn create_backup(file: &PathBuf) -> Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = file.with_extension(format!("{}.backup.{}", 
        file.extension().and_then(|s| s.to_str()).unwrap_or("txt"), 
        timestamp
    ));
    
    std::fs::copy(file, &backup_path)?;
    Ok(backup_path)
}

fn show_code_comparison(original: &str, refactored: &str) {
    let original_lines = original.lines().count();
    let refactored_lines = refactored.lines().count();
    let original_chars = original.len();
    let refactored_chars = refactored.len();
    
    println!("  {} {} → {} lines", "Lines:".bright_white(), 
        original_lines.to_string().bright_yellow(),
        refactored_lines.to_string().bright_green()
    );
    
    println!("  {} {} → {} characters", "Characters:".bright_white(),
        original_chars.to_string().bright_yellow(),
        refactored_chars.to_string().bright_green()
    );
    
    let line_change = refactored_lines as i32 - original_lines as i32;
    let char_change = refactored_chars as i32 - original_chars as i32;
    
    if line_change != 0 {
        let change_color = if line_change > 0 { "bright_red" } else { "bright_green" };
        println!("  {} {} lines", "Change:".bright_white(),
            format!("{:+}", line_change).color(change_color)
        );
    }
    
    if char_change != 0 {
        let change_color = if char_change > 0 { "bright_red" } else { "bright_green" };
        println!("  {} {} characters", "Size change:".bright_white(),
            format!("{:+}", char_change).color(change_color)
        );
    }
}

fn show_diff_preview(original: &str, refactored: &str, max_lines: usize) {
    let original_lines: Vec<&str> = original.lines().collect();
    let refactored_lines: Vec<&str> = refactored.lines().collect();
    
    let mut shown_lines = 0;
    let max_len = std::cmp::max(original_lines.len(), refactored_lines.len());
    
    for i in 0..max_len {
        if shown_lines >= max_lines {
            println!("  {}", "... (more changes)".bright_black().dimmed());
            break;
        }
        
        let original_line = original_lines.get(i).unwrap_or(&"");
        let refactored_line = refactored_lines.get(i).unwrap_or(&"");
        
        if original_line != refactored_line {
            if !original_line.is_empty() {
                println!("  {} {}", "-".bright_red(), original_line.bright_red());
            }
            if !refactored_line.is_empty() {
                println!("  {} {}", "+".bright_green(), refactored_line.bright_green());
            }
            shown_lines += 1;
        }
    }
    
    if shown_lines == 0 {
        println!("  {}", "No significant changes detected in preview".bright_black().dimmed());
    }
}