use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::client::{Client, CodeActionRequest};

pub async fn run(
    file: PathBuf,
    output: Option<PathBuf>,
    format: String,
    client: &Client,
) -> Result<()> {
    println!("{}", "📚 AI Documentation Generator".bright_blue().bold());
    println!();

    // Check if file exists
    if !file.exists() {
        println!("{} File not found: {}", "❌".bright_red(), file.display());
        return Ok(());
    }

    // Read file content
    let code = std::fs::read_to_string(&file)?;
    if code.trim().is_empty() {
        println!("{} File is empty", "⚠️".bright_yellow());
        return Ok(());
    }

    // Detect language
    let language = detect_language_from_extension(&file);

    println!("{}", "📁 File Information:".bright_cyan().bold());
    println!("  {} {}", "Input:".bright_white(), file.display().to_string().bright_green());
    println!("  {} {}", "Language:".bright_white(), language.bright_green());
    println!("  {} {}", "Format:".bright_white(), format.bright_magenta());
    
    if let Some(ref output_path) = output {
        println!("  {} {}", "Output:".bright_white(), output_path.display().to_string().bright_cyan());
    } else {
        println!("  {} {}", "Output:".bright_white(), "stdout".bright_cyan());
    }
    println!();

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Generating documentation...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let request = CodeActionRequest {
        code: code.clone(),
        language: language.clone(),
        action: "document".to_string(),
        instructions: Some(format!("Generate {} documentation", format)),
        target_language: None,
    };

    match client.code_action(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            let documentation = format_documentation(&response.result, &format, &file, &language);
            
            // Output documentation
            if let Some(output_path) = output {
                // Write to file
                std::fs::write(&output_path, &documentation)?;
                println!("{} Documentation saved to: {}", 
                    "✅".bright_green(), 
                    output_path.display().to_string().bright_cyan()
                );
                
                // Show preview
                println!();
                println!("{}", "📖 Preview:".bright_yellow().bold());
                show_preview(&documentation, 10);
                
            } else {
                // Output to stdout
                println!("{}", "📖 Generated Documentation:".bright_green().bold());
                println!();
                println!("{}", documentation.bright_white());
            }

            // Show statistics
            println!();
            println!("{}", "📊 Statistics:".bright_blue().bold());
            println!("  {} {} lines", "Original code:".bright_white(), code.lines().count().to_string().bright_yellow());
            println!("  {} {} lines", "Documentation:".bright_white(), documentation.lines().count().to_string().bright_green());
            println!("  {} {} words", "Word count:".bright_white(), documentation.split_whitespace().count().to_string().bright_cyan());

        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Documentation generation failed: {}", "❌".bright_red().bold(), e);
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
        Some("h") | Some("hpp") => "c".to_string(),
        Some("php") => "php".to_string(),
        Some("rb") => "ruby".to_string(),
        Some("swift") => "swift".to_string(),
        Some("kt") => "kotlin".to_string(),
        Some("cs") => "csharp".to_string(),
        Some("sh") => "bash".to_string(),
        _ => "text".to_string(),
    }
}

fn format_documentation(content: &str, format: &str, file: &PathBuf, language: &str) -> String {
    let file_name = file.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown");
    
    match format {
        "markdown" => format_as_markdown(content, file_name, language),
        "rst" => format_as_rst(content, file_name, language),
        "html" => format_as_html(content, file_name, language),
        _ => content.to_string(),
    }
}

fn format_as_markdown(content: &str, file_name: &str, language: &str) -> String {
    format!(
        "# Documentation for `{}`\n\n\
        **Language:** {}\n\
        **Generated:** {}\n\n\
        ---\n\n\
        {}\n\n\
        ---\n\n\
        *Generated by UAIDA - Universal AI Development Assistant*",
        file_name,
        language,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        content
    )
}

fn format_as_rst(content: &str, file_name: &str, language: &str) -> String {
    let title = format!("Documentation for {}", file_name);
    let title_underline = "=".repeat(title.len());
    
    format!(
        "{}\n{}\n\n\
        :Language: {}\n\
        :Generated: {}\n\n\
        {}\n\n\
        .. note::\n   Generated by UAIDA - Universal AI Development Assistant",
        title,
        title_underline,
        language,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        content
    )
}

fn format_as_html(content: &str, file_name: &str, language: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
        <html>\n\
        <head>\n\
            <title>Documentation for {}</title>\n\
            <meta charset=\"utf-8\">\n\
            <style>\n\
                body {{ font-family: Arial, sans-serif; margin: 40px; }}\n\
                .header {{ background: #f5f5f5; padding: 20px; border-radius: 5px; }}\n\
                .content {{ margin-top: 20px; line-height: 1.6; }}\n\
                .footer {{ margin-top: 40px; font-size: 0.9em; color: #666; }}\n\
                code {{ background: #f0f0f0; padding: 2px 4px; border-radius: 3px; }}\n\
                pre {{ background: #f8f8f8; padding: 15px; border-radius: 5px; overflow-x: auto; }}\n\
            </style>\n\
        </head>\n\
        <body>\n\
            <div class=\"header\">\n\
                <h1>Documentation for <code>{}</code></h1>\n\
                <p><strong>Language:</strong> {}</p>\n\
                <p><strong>Generated:</strong> {}</p>\n\
            </div>\n\
            <div class=\"content\">\n\
                {}\n\
            </div>\n\
            <div class=\"footer\">\n\
                <p><em>Generated by UAIDA - Universal AI Development Assistant</em></p>\n\
            </div>\n\
        </body>\n\
        </html>",
        file_name,
        file_name,
        language,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        content.replace("\n", "<br>\n")
    )
}

fn show_preview(content: &str, max_lines: usize) {
    let lines: Vec<&str> = content.lines().collect();
    let preview_lines = if lines.len() > max_lines {
        &lines[..max_lines]
    } else {
        &lines
    };
    
    for line in preview_lines {
        println!("  {}", line.bright_white());
    }
    
    if lines.len() > max_lines {
        println!("  {}", format!("... ({} more lines)", lines.len() - max_lines).bright_black().dimmed());
    }
}