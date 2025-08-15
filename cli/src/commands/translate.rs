use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::client::{Client, CodeActionRequest};

pub async fn run(
    file: PathBuf,
    to: String,
    from: Option<String>,
    output: Option<PathBuf>,
    client: &Client,
) -> Result<()> {
    println!("{}", "🔄 AI Code Translator".bright_blue().bold());
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

    // Detect source language
    let source_language = from.unwrap_or_else(|| detect_language_from_extension(&file));
    let target_language = to.clone();
    
    // Generate output path if not provided
    let output_path = output.unwrap_or_else(|| generate_output_path(&file, &target_language));

    println!("{}", "🔄 Translation Info:".bright_cyan().bold());
    println!("  {} {}", "Source file:".bright_white(), file.display().to_string().bright_green());
    println!("  {} {} → {}", "Languages:".bright_white(), 
        source_language.bright_yellow(), 
        target_language.bright_magenta()
    );
    println!("  {} {}", "Output:".bright_white(), output_path.display().to_string().bright_cyan());
    println!("  {} {} lines", "Code size:".bright_white(), code.lines().count().to_string().bright_blue());
    println!();

    // Validate languages
    if source_language == target_language {
        println!("{} Source and target languages are the same", "⚠️".bright_yellow());
        return Ok(());
    }

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message(format!("Translating {} to {}...", source_language, target_language));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let instructions = format!(
        "Translate this {} code to {}. Guidelines:\n\
        - Preserve the original functionality exactly\n\
        - Use idiomatic {} patterns and conventions\n\
        - Include appropriate imports/dependencies\n\
        - Add comments explaining {} specific concepts\n\
        - Ensure the translated code follows {} best practices\n\
        - Handle language-specific features appropriately",
        source_language, target_language, target_language, target_language, target_language
    );

    let request = CodeActionRequest {
        code: code.clone(),
        language: source_language.clone(),
        action: "translate".to_string(),
        instructions: Some(instructions),
        target_language: Some(target_language.clone()),
    };

    match client.code_action(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            let translated_code = format_translated_code(&response.result, &target_language, &file, &source_language);
            
            // Write translated code
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output_path, &translated_code)?;
            
            println!("{} Code translated successfully!", "✅".bright_green());
            println!("  {} {}", "Translated file:".bright_white(), output_path.display().to_string().bright_cyan());
            
            // Show translation summary
            println!();
            println!("{}", "📊 Translation Summary:".bright_blue().bold());
            show_translation_summary(&code, &translated_code, &source_language, &target_language);
            
            // Show preview
            println!();
            println!("{}", "🔍 Translation Preview:".bright_yellow().bold());
            show_translation_preview(&translated_code, 15);
            
            // Show next steps
            println!();
            println!("{}", "🚀 Next Steps:".bright_magenta().bold());
            show_next_steps(&target_language, &output_path);

        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Translation failed: {}", "❌".bright_red().bold(), e);
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

fn generate_output_path(file: &PathBuf, target_language: &str) -> PathBuf {
    let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("translated");
    
    let extension = match target_language {
        "rust" => "rs",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        "go" => "go",
        "java" => "java",
        "cpp" => "cpp",
        "c" => "c",
        "php" => "php",
        "ruby" => "rb",
        "swift" => "swift",
        "kotlin" => "kt",
        "csharp" => "cs",
        _ => "txt",
    };
    
    if let Some(parent) = file.parent() {
        parent.join(format!("{}_translated.{}", file_stem, extension))
    } else {
        PathBuf::from(format!("{}_translated.{}", file_stem, extension))
    }
}

fn format_translated_code(content: &str, target_language: &str, original_file: &PathBuf, source_language: &str) -> String {
    let file_name = original_file.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown");
    
    let header = match target_language {
        "rust" => format!(
            "// Translated from {} to Rust\n\
            // Original file: {}\n\
            // Generated: {}\n\
            // Generated by UAIDA - Universal AI Development Assistant\n\n",
            source_language, file_name, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ),
        "python" => format!(
            "# Translated from {} to Python\n\
            # Original file: {}\n\
            # Generated: {}\n\
            # Generated by UAIDA - Universal AI Development Assistant\n\n",
            source_language, file_name, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ),
        "javascript" | "typescript" => format!(
            "// Translated from {} to {}\n\
            // Original file: {}\n\
            // Generated: {}\n\
            // Generated by UAIDA - Universal AI Development Assistant\n\n",
            source_language, target_language, file_name, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ),
        _ => format!(
            "/* Translated from {} to {} */\n\
            /* Original file: {} */\n\
            /* Generated: {} */\n\
            /* Generated by UAIDA - Universal AI Development Assistant */\n\n",
            source_language, target_language, file_name, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ),
    };
    
    format!("{}{}", header, content)
}

fn show_translation_summary(original: &str, translated: &str, source_lang: &str, target_lang: &str) {
    let original_lines = original.lines().count();
    let translated_lines = translated.lines().count();
    let original_chars = original.len();
    let translated_chars = translated.len();
    
    println!("  {} {} → {}", "Language:".bright_white(), 
        source_lang.bright_yellow(), 
        target_lang.bright_magenta()
    );
    
    println!("  {} {} → {} lines", "Lines:".bright_white(), 
        original_lines.to_string().bright_yellow(),
        translated_lines.to_string().bright_green()
    );
    
    println!("  {} {} → {} characters", "Characters:".bright_white(),
        original_chars.to_string().bright_yellow(),
        translated_chars.to_string().bright_green()
    );
    
    // Estimate complexity preservation
    let complexity_preserved = estimate_complexity_preservation(original, translated);
    println!("  {} {:.1}%", "Complexity preserved:".bright_white(),
        (complexity_preserved * 100.0).to_string().bright_cyan()
    );
}

fn estimate_complexity_preservation(original: &str, translated: &str) -> f64 {
    let original_complexity = count_control_structures(original);
    let translated_complexity = count_control_structures(translated);
    
    if original_complexity == 0 {
        1.0
    } else {
        let ratio = translated_complexity as f64 / original_complexity as f64;
        if ratio > 1.0 { 1.0 / ratio } else { ratio }
    }
}

fn count_control_structures(code: &str) -> usize {
    code.matches("if ").count() + 
    code.matches("for ").count() + 
    code.matches("while ").count() + 
    code.matches("match ").count() + 
    code.matches("switch ").count() +
    code.matches("try ").count()
}

fn show_translation_preview(content: &str, max_lines: usize) {
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

fn show_next_steps(target_language: &str, output_path: &PathBuf) {
    match target_language {
        "rust" => {
            println!("  {} Review and test the translated Rust code", "1.".bright_white());
            println!("  {} Run: {}", "2.".bright_white(), "cargo check".bright_green());
            println!("  {} Add to Cargo.toml if needed", "3.".bright_white());
            println!("  {} Run tests: {}", "4.".bright_white(), "cargo test".bright_green());
        }
        "python" => {
            println!("  {} Review the translated Python code", "1.".bright_white());
            println!("  {} Check syntax: {}", "2.".bright_white(), format!("python -m py_compile {}", output_path.display()).bright_green());
            println!("  {} Install dependencies if needed", "3.".bright_white());
            println!("  {} Run: {}", "4.".bright_white(), format!("python {}", output_path.display()).bright_green());
        }
        "javascript" => {
            println!("  {} Review the translated JavaScript code", "1.".bright_white());
            println!("  {} Run: {}", "2.".bright_white(), format!("node {}", output_path.display()).bright_green());
            println!("  {} Add to package.json if needed", "3.".bright_white());
            println!("  {} Test in browser if applicable", "4.".bright_white());
        }
        "go" => {
            println!("  {} Review the translated Go code", "1.".bright_white());
            println!("  {} Run: {}", "2.".bright_white(), "go run .".bright_green());
            println!("  {} Format: {}", "3.".bright_white(), "go fmt".bright_green());
            println!("  {} Test: {}", "4.".bright_white(), "go test".bright_green());
        }
        _ => {
            println!("  {} Review the translated code carefully", "1.".bright_white());
            println!("  {} Test functionality matches original", "2.".bright_white());
            println!("  {} Compile/run with appropriate tools", "3.".bright_white());
            println!("  {} Add language-specific optimizations", "4.".bright_white());
        }
    }
}