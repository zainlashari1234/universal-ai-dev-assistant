use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::client::{Client, AnalysisRequest};

pub async fn run(
    file: PathBuf,
    analysis_type: String,
    language: Option<String>,
    client: &Client,
) -> Result<()> {
    println!("{}", "🔍 AI Code Analysis".bright_blue().bold());
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

    // Detect language if not provided
    let detected_language = language.unwrap_or_else(|| {
        detect_language_from_extension(&file)
    });

    println!("{}", "📁 File Information:".bright_cyan().bold());
    println!("  {} {}", "Path:".bright_white(), file.display().to_string().bright_green());
    println!("  {} {}", "Language:".bright_white(), detected_language.bright_green());
    println!("  {} {} lines", "Size:".bright_white(), code.lines().count().to_string().bright_yellow());
    println!("  {} {}", "Analysis:".bright_white(), analysis_type.bright_magenta());
    println!();

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message(format!("Analyzing {} code...", detected_language));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let request = AnalysisRequest {
        code: code.clone(),
        language: detected_language.clone(),
        analysis_type: analysis_type.clone(),
        context: Some(format!("File: {}", file.display())),
    };

    match client.analyze(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            // Display results
            println!("{}", format!("📊 {} Analysis Results", 
                match analysis_type.as_str() {
                    "security" => "🔒 Security",
                    "performance" => "⚡ Performance", 
                    "quality" => "✨ Quality",
                    "bugs" => "🐛 Bug",
                    "suggestions" => "💡 Suggestion",
                    "documentation" => "📚 Documentation",
                    "testing" => "🧪 Testing",
                    "refactoring" => "🔧 Refactoring",
                    _ => "📋 General"
                }
            ).bright_blue().bold());
            println!();

            // Show confidence score
            let confidence_color = if response.confidence_score > 0.8 {
                "bright_green"
            } else if response.confidence_score > 0.6 {
                "bright_yellow"
            } else {
                "bright_red"
            };
            
            println!("{} {:.1}%", 
                "🎯 Confidence:".bright_white().bold(), 
                (response.confidence_score * 100.0).to_string().color(confidence_color)
            );
            println!();

            // Show findings if any
            if !response.findings.is_empty() {
                println!("{}", "🔍 Detailed Findings:".bright_yellow().bold());
                for (i, finding) in response.findings.iter().enumerate() {
                    let severity_icon = match finding.severity.as_str() {
                        "Critical" => "🚨",
                        "High" => "🔴",
                        "Medium" => "🟡",
                        "Low" => "🟢",
                        _ => "ℹ️"
                    };
                    
                    println!("  {} {} {}", 
                        format!("{}.", i + 1).bright_white().dimmed(),
                        severity_icon,
                        finding.title.bright_white().bold()
                    );
                    
                    if let Some(line) = finding.line_number {
                        println!("     {} Line {}", "📍".bright_blue(), line.to_string().bright_cyan());
                    }
                    
                    println!("     {}", finding.description.bright_white());
                    
                    if let Some(fix) = &finding.fix_suggestion {
                        println!("     {} {}", "💡 Fix:".bright_green(), fix.bright_green());
                    }
                    println!();
                }
            }

            // Show main summary
            println!("{}", "📝 Analysis Summary:".bright_green().bold());
            println!("{}", response.summary.bright_white());
            println!();

            // Show suggestions if any
            if !response.suggestions.is_empty() {
                println!("{}", "💡 Recommendations:".bright_magenta().bold());
                for (i, suggestion) in response.suggestions.iter().enumerate() {
                    println!("  {} {}", 
                        format!("{}.", i + 1).bright_white().dimmed(),
                        suggestion.title.bright_white().bold()
                    );
                    println!("     {}", suggestion.description.bright_white());
                    println!("     {} {} | {} {}", 
                        "Impact:".bright_blue(), suggestion.impact.bright_cyan(),
                        "Effort:".bright_blue(), suggestion.effort.bright_cyan()
                    );
                    
                    if let Some(example) = &suggestion.code_example {
                        println!("     {} Example:", "📝".bright_green());
                        print_code_snippet(example, &detected_language);
                    }
                    println!();
                }
            }

            // Show code snippet with syntax highlighting if there are line-specific findings
            let has_line_findings = response.findings.iter().any(|f| f.line_number.is_some());
            if has_line_findings {
                println!("{}", "📄 Code Context:".bright_cyan().bold());
                print_code_with_highlights(&code, &detected_language, &response.findings);
            }

        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Analysis failed: {}", "❌".bright_red().bold(), e);
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
        Some("sql") => "sql".to_string(),
        Some("html") => "html".to_string(),
        Some("css") => "css".to_string(),
        Some("json") => "json".to_string(),
        Some("yaml") | Some("yml") => "yaml".to_string(),
        Some("toml") => "toml".to_string(),
        Some("xml") => "xml".to_string(),
        Some("md") => "markdown".to_string(),
        _ => "text".to_string(),
    }
}

fn print_code_snippet(code: &str, language: &str) {
    // Simple syntax highlighting
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    
    let syntax = ps.find_syntax_by_extension(language)
        .or_else(|| ps.find_syntax_by_name(language))
        .unwrap_or_else(|| ps.find_syntax_plain_text());
    
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    
    println!("       {}", "```".bright_black().dimmed());
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("       {}", escaped);
    }
    println!("       {}", "```".bright_black().dimmed());
}

fn print_code_with_highlights(code: &str, language: &str, findings: &[crate::client::Finding]) {
    let lines: Vec<&str> = code.lines().collect();
    let finding_lines: std::collections::HashSet<u32> = findings
        .iter()
        .filter_map(|f| f.line_number)
        .collect();

    println!("       {}", "```".bright_black().dimmed());
    for (i, line) in lines.iter().enumerate() {
        let line_num = (i + 1) as u32;
        let line_prefix = if finding_lines.contains(&line_num) {
            format!("  {} {} ", "⚠️".bright_red(), format!("{:3}", line_num).bright_red())
        } else {
            format!("     {} ", format!("{:3}", line_num).bright_black().dimmed())
        };
        
        println!("{}{}", line_prefix, line.bright_white());
    }
    println!("       {}", "```".bright_black().dimmed());
}