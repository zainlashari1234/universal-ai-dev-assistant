use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::client::{Client, CodeActionRequest};

pub async fn run(
    file: PathBuf,
    symbol: Option<String>,
    client: &Client,
) -> Result<()> {
    println!("{}", "🧠 AI Code Explainer".bright_blue().bold());
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

    // Extract specific symbol if requested
    let (code_to_explain, explanation_scope) = if let Some(ref symbol_name) = symbol {
        match extract_symbol(&code, symbol_name, &language) {
            Some(extracted) => (extracted, format!("symbol '{}'", symbol_name)),
            None => {
                println!("{} Symbol '{}' not found in file", "⚠️".bright_yellow(), symbol_name);
                (code.clone(), "entire file".to_string())
            }
        }
    } else {
        (code.clone(), "entire file".to_string())
    };

    println!("{}", "📁 Explanation Target:".bright_cyan().bold());
    println!("  {} {}", "File:".bright_white(), file.display().to_string().bright_green());
    println!("  {} {}", "Language:".bright_white(), language.bright_green());
    println!("  {} {}", "Scope:".bright_white(), explanation_scope.bright_magenta());
    println!("  {} {} lines", "Code size:".bright_white(), code_to_explain.lines().count().to_string().bright_yellow());
    println!();

    // Show the code being explained with syntax highlighting
    println!("{}", "📄 Code to Explain:".bright_cyan().bold());
    print_code_with_syntax_highlighting(&code_to_explain, &language);
    println!();

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Analyzing and explaining code...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let instructions = format!(
        "Explain this {} code in detail. Include:\n\
        - What the code does (high-level purpose)\n\
        - How it works (step-by-step breakdown)\n\
        - Key concepts and algorithms used\n\
        - Input/output behavior\n\
        - Important design decisions\n\
        - Potential edge cases or limitations\n\
        - Best practices demonstrated or violated",
        language
    );

    let request = CodeActionRequest {
        code: code_to_explain.clone(),
        language: language.clone(),
        action: "explain".to_string(),
        instructions: Some(instructions),
        target_language: None,
    };

    match client.code_action(request).await {
        Ok(response) => {
            pb.finish_and_clear();
            
            // Display explanation
            println!("{}", "🧠 Code Explanation:".bright_green().bold());
            println!();
            
            // Format and display the explanation
            display_formatted_explanation(&response.result);
            
            // Show code complexity analysis
            println!();
            println!("{}", "📊 Code Analysis:".bright_blue().bold());
            analyze_code_complexity(&code_to_explain, &language);
            
            // Show related concepts
            println!();
            println!("{}", "🔗 Related Concepts:".bright_magenta().bold());
            suggest_related_concepts(&code_to_explain, &language);

        }
        Err(e) => {
            pb.finish_and_clear();
            println!("{} Code explanation failed: {}", "❌".bright_red().bold(), e);
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

fn extract_symbol(code: &str, symbol_name: &str, language: &str) -> Option<String> {
    match language {
        "rust" => extract_rust_symbol(code, symbol_name),
        "python" => extract_python_symbol(code, symbol_name),
        "javascript" | "typescript" => extract_js_symbol(code, symbol_name),
        "java" => extract_java_symbol(code, symbol_name),
        "cpp" | "c" => extract_c_symbol(code, symbol_name),
        _ => extract_generic_symbol(code, symbol_name),
    }
}

fn extract_rust_symbol(code: &str, symbol_name: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();
    let mut in_function = false;
    let mut brace_count = 0;
    
    for line in lines {
        if line.contains(&format!("fn {}", symbol_name)) || 
           line.contains(&format!("struct {}", symbol_name)) ||
           line.contains(&format!("enum {}", symbol_name)) ||
           line.contains(&format!("impl {}", symbol_name)) {
            in_function = true;
            brace_count = 0;
        }
        
        if in_function {
            result.push(line);
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            
            if brace_count == 0 && line.contains('}') {
                break;
            }
        }
    }
    
    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn extract_python_symbol(code: &str, symbol_name: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();
    let mut in_function = false;
    let mut base_indent = 0;
    
    for line in lines {
        let current_indent = line.len() - line.trim_start().len();
        
        if line.trim_start().starts_with(&format!("def {}", symbol_name)) ||
           line.trim_start().starts_with(&format!("class {}", symbol_name)) {
            in_function = true;
            base_indent = current_indent;
            result.push(line);
            continue;
        }
        
        if in_function {
            if line.trim().is_empty() || current_indent > base_indent {
                result.push(line);
            } else {
                break;
            }
        }
    }
    
    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn extract_js_symbol(code: &str, symbol_name: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();
    let mut in_function = false;
    let mut brace_count = 0;
    
    for line in lines {
        if line.contains(&format!("function {}", symbol_name)) ||
           line.contains(&format!("const {} =", symbol_name)) ||
           line.contains(&format!("let {} =", symbol_name)) ||
           line.contains(&format!("var {} =", symbol_name)) ||
           line.contains(&format!("class {}", symbol_name)) {
            in_function = true;
            brace_count = 0;
        }
        
        if in_function {
            result.push(line);
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            
            if brace_count == 0 && line.contains('}') {
                break;
            }
        }
    }
    
    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn extract_java_symbol(code: &str, symbol_name: &str) -> Option<String> {
    extract_c_symbol(code, symbol_name) // Similar brace-based extraction
}

fn extract_c_symbol(code: &str, symbol_name: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();
    let mut in_function = false;
    let mut brace_count = 0;
    
    for line in lines {
        if line.contains(symbol_name) && (line.contains('(') || line.contains("struct") || line.contains("class")) {
            in_function = true;
            brace_count = 0;
        }
        
        if in_function {
            result.push(line);
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            
            if brace_count == 0 && line.contains('}') {
                break;
            }
        }
    }
    
    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn extract_generic_symbol(code: &str, symbol_name: &str) -> Option<String> {
    // Simple line-based extraction for unknown languages
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();
    
    for (i, line) in lines.iter().enumerate() {
        if line.contains(symbol_name) {
            // Include some context around the symbol
            let start = i.saturating_sub(2);
            let end = std::cmp::min(i + 5, lines.len());
            result.extend_from_slice(&lines[start..end]);
            break;
        }
    }
    
    if result.is_empty() { None } else { Some(result.join("\n")) }
}

fn print_code_with_syntax_highlighting(code: &str, language: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    
    let syntax = ps.find_syntax_by_extension(language)
        .or_else(|| ps.find_syntax_by_name(language))
        .unwrap_or_else(|| ps.find_syntax_plain_text());
    
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    
    println!("  {}", "```".bright_black().dimmed());
    for (i, line) in LinesWithEndings::from(code).enumerate() {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("  {:3} {}", (i + 1).to_string().bright_black().dimmed(), escaped);
    }
    println!("  {}", "```".bright_black().dimmed());
}

fn display_formatted_explanation(explanation: &str) {
    // Split explanation into sections and format nicely
    let sections = explanation.split("\n\n");
    
    for section in sections {
        if section.trim().is_empty() {
            continue;
        }
        
        // Check if this looks like a heading
        if section.lines().count() == 1 && section.len() < 100 {
            println!("  {} {}", "▶".bright_blue(), section.bright_white().bold());
        } else {
            // Regular paragraph
            for line in section.lines() {
                println!("    {}", line.bright_white());
            }
        }
        println!();
    }
}

fn analyze_code_complexity(code: &str, language: &str) {
    let lines = code.lines().count();
    let chars = code.len();
    let functions = count_functions(code, language);
    let complexity = estimate_complexity(code, language);
    
    println!("  {} {} lines", "Lines of code:".bright_white(), lines.to_string().bright_yellow());
    println!("  {} {} characters", "Characters:".bright_white(), chars.to_string().bright_yellow());
    println!("  {} {} functions", "Functions:".bright_white(), functions.to_string().bright_cyan());
    println!("  {} {}", "Complexity:".bright_white(), 
        match complexity {
            1..=3 => "Low".bright_green(),
            4..=7 => "Medium".bright_yellow(),
            _ => "High".bright_red(),
        }
    );
}

fn count_functions(code: &str, language: &str) -> usize {
    match language {
        "rust" => code.matches("fn ").count(),
        "python" => code.matches("def ").count(),
        "javascript" | "typescript" => code.matches("function ").count() + code.matches(" => ").count(),
        "java" | "cpp" | "c" => code.matches("(").count(), // Rough estimate
        _ => 0,
    }
}

fn estimate_complexity(code: &str, _language: &str) -> usize {
    let control_structures = code.matches("if ").count() + 
                           code.matches("for ").count() + 
                           code.matches("while ").count() + 
                           code.matches("match ").count() + 
                           code.matches("switch ").count();
    
    std::cmp::max(1, control_structures)
}

fn suggest_related_concepts(code: &str, language: &str) {
    let mut concepts = Vec::new();
    
    // Language-specific concepts
    match language {
        "rust" => {
            if code.contains("Option") || code.contains("Result") { concepts.push("Error handling"); }
            if code.contains("Vec") || code.contains("HashMap") { concepts.push("Collections"); }
            if code.contains("async") || code.contains("await") { concepts.push("Async programming"); }
            if code.contains("impl") { concepts.push("Traits and implementations"); }
        }
        "python" => {
            if code.contains("class ") { concepts.push("Object-oriented programming"); }
            if code.contains("def ") { concepts.push("Functions and methods"); }
            if code.contains("import ") { concepts.push("Modules and packages"); }
            if code.contains("try:") { concepts.push("Exception handling"); }
        }
        "javascript" | "typescript" => {
            if code.contains("async") || code.contains("Promise") { concepts.push("Asynchronous programming"); }
            if code.contains("class ") { concepts.push("ES6 classes"); }
            if code.contains("=>") { concepts.push("Arrow functions"); }
            if code.contains("import ") { concepts.push("ES6 modules"); }
        }
        _ => {}
    }
    
    // General programming concepts
    if code.contains("loop") || code.contains("for") || code.contains("while") {
        concepts.push("Iteration and loops");
    }
    if code.contains("if") || code.contains("else") {
        concepts.push("Conditional logic");
    }
    if code.contains("return") {
        concepts.push("Function return values");
    }
    
    for concept in concepts {
        println!("  {} {}", "•".bright_blue(), concept.bright_white());
    }
    
    if concepts.is_empty() {
        println!("  {} No specific concepts detected", "•".bright_black().dimmed());
    }
}