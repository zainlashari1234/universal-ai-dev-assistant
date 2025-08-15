use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::client::{Client, CodeActionRequest};

pub async fn run(
    file: PathBuf,
    output: Option<PathBuf>,
    framework: Option<String>,
    client: &Client,
) -> Result<()> {
    println!("{}", "🧪 AI Test Generator".bright_blue().bold());
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

    // Detect language and framework
    let language = detect_language_from_extension(&file);
    let test_framework = framework.unwrap_or_else(|| detect_test_framework(&language));
    
    // Generate output path if not provided
    let output_path = output.unwrap_or_else(|| generate_test_file_path(&file, &language));

    println!("{}", "📁 Test Generation Info:".bright_cyan().bold());
    println!("  {} {}", "Source file:".bright_white(), file.display().to_string().bright_green());
    println!("  {} {}", "Language:".bright_white(), language.bright_green());
    println!("  {} {}", "Framework:".bright_white(), test_framework.bright_magenta());
    println!("  {} {}", "Output:".bright_white(), output_path.display().to_string().bright_cyan());
    println!();

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message(format!("Generating {} tests...", test_framework));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let instructions = format!(
        "Generate comprehensive unit tests using {} framework. Include:\n\
        - Test cases for normal operation\n\
        - Edge cases and boundary conditions\n\
        - Error handling tests\n\
        - Mock objects where appropriate\n\
        - Setup and teardown if needed\n\
        - Clear test names and descriptions",
        test_framework
    );

    let request = CodeActionRequest {
        code: code.clone(),
        language: language.clone(),
        action: "test".to_string(),
        instructions: Some(instructions),
        target_language: None,
    };

    match client.code_action(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            let test_code = format_test_code(&response.result, &language, &test_framework, &file);
            
            // Write test file
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output_path, &test_code)?;
            
            println!("{} Test file generated: {}", 
                "✅".bright_green(), 
                output_path.display().to_string().bright_cyan()
            );
            
            // Show preview
            println!();
            println!("{}", "🔍 Test Preview:".bright_yellow().bold());
            show_test_preview(&test_code, 15);
            
            // Show statistics
            println!();
            println!("{}", "📊 Test Statistics:".bright_blue().bold());
            let test_functions = count_test_functions(&test_code, &language);
            println!("  {} {} lines", "Original code:".bright_white(), code.lines().count().to_string().bright_yellow());
            println!("  {} {} lines", "Test code:".bright_white(), test_code.lines().count().to_string().bright_green());
            println!("  {} {} functions", "Test functions:".bright_white(), test_functions.to_string().bright_cyan());
            
            // Show next steps
            println!();
            println!("{}", "🚀 Next Steps:".bright_magenta().bold());
            println!("  {} Review and customize the generated tests", "1.".bright_white());
            println!("  {} Run tests: {}", "2.".bright_white(), get_test_command(&language, &test_framework).bright_green());
            println!("  {} Add more specific test cases if needed", "3.".bright_white());

        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Test generation failed: {}", "❌".bright_red().bold(), e);
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

fn detect_test_framework(language: &str) -> String {
    match language {
        "rust" => "cargo test".to_string(),
        "python" => "pytest".to_string(),
        "javascript" | "typescript" => "jest".to_string(),
        "go" => "go test".to_string(),
        "java" => "junit".to_string(),
        "cpp" => "gtest".to_string(),
        "c" => "unity".to_string(),
        "php" => "phpunit".to_string(),
        "ruby" => "rspec".to_string(),
        "swift" => "xctest".to_string(),
        "kotlin" => "junit".to_string(),
        "csharp" => "nunit".to_string(),
        _ => "custom".to_string(),
    }
}

fn generate_test_file_path(file: &PathBuf, language: &str) -> PathBuf {
    let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("test");
    let extension = file.extension().and_then(|s| s.to_str()).unwrap_or("txt");
    
    match language {
        "rust" => {
            // Rust tests usually go in the same file or in tests/ directory
            if let Some(parent) = file.parent() {
                parent.join("tests").join(format!("{}_test.rs", file_stem))
            } else {
                PathBuf::from(format!("{}_test.rs", file_stem))
            }
        }
        "python" => {
            PathBuf::from(format!("test_{}.py", file_stem))
        }
        "javascript" | "typescript" => {
            let ext = if language == "typescript" { "ts" } else { "js" };
            PathBuf::from(format!("{}.test.{}", file_stem, ext))
        }
        "go" => {
            PathBuf::from(format!("{}_test.go", file_stem))
        }
        "java" => {
            PathBuf::from(format!("{}Test.java", file_stem))
        }
        _ => {
            PathBuf::from(format!("{}_test.{}", file_stem, extension))
        }
    }
}

fn format_test_code(content: &str, language: &str, framework: &str, original_file: &PathBuf) -> String {
    let file_name = original_file.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown");
    
    let header = format!(
        "// Generated tests for {}\n\
        // Framework: {}\n\
        // Generated: {}\n\
        // Generated by UAIDA - Universal AI Development Assistant\n\n",
        file_name,
        framework,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    
    match language {
        "rust" => format!("{}#[cfg(test)]\nmod tests {{\n    use super::*;\n\n{}\n}}", header, indent_code(content, 1)),
        "python" => format!("{}import unittest\nfrom {} import *\n\n{}", 
            header.replace("//", "#"), 
            original_file.file_stem().and_then(|s| s.to_str()).unwrap_or("module"),
            content
        ),
        _ => format!("{}{}", header, content),
    }
}

fn indent_code(code: &str, levels: usize) -> String {
    let indent = "    ".repeat(levels);
    code.lines()
        .map(|line| if line.trim().is_empty() { line.to_string() } else { format!("{}{}", indent, line) })
        .collect::<Vec<_>>()
        .join("\n")
}

fn show_test_preview(content: &str, max_lines: usize) {
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

fn count_test_functions(code: &str, language: &str) -> usize {
    match language {
        "rust" => code.matches("#[test]").count(),
        "python" => code.matches("def test_").count(),
        "javascript" | "typescript" => code.matches("test(").count() + code.matches("it(").count(),
        "go" => code.matches("func Test").count(),
        "java" => code.matches("@Test").count(),
        "cpp" => code.matches("TEST(").count(),
        _ => code.lines().filter(|line| line.contains("test")).count(),
    }
}

fn get_test_command(language: &str, framework: &str) -> String {
    match language {
        "rust" => "cargo test".to_string(),
        "python" => match framework {
            "pytest" => "pytest".to_string(),
            "unittest" => "python -m unittest".to_string(),
            _ => "python -m pytest".to_string(),
        },
        "javascript" | "typescript" => match framework {
            "jest" => "npm test".to_string(),
            "mocha" => "npm run test".to_string(),
            _ => "npm test".to_string(),
        },
        "go" => "go test".to_string(),
        "java" => "mvn test".to_string(),
        "cpp" => "make test".to_string(),
        _ => format!("{} test", framework),
    }
}