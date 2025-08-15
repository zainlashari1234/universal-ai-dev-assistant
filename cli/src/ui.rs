// UI utilities and components for the CLI
use colored::*;

pub fn print_header(title: &str) {
    println!("{}", title.bright_blue().bold());
    println!("{}", "=".repeat(title.len()).bright_blue());
    println!();
}

pub fn print_success(message: &str) {
    println!("{} {}", "✅".bright_green(), message.bright_white());
}

pub fn print_error(message: &str) {
    println!("{} {}", "❌".bright_red(), message.bright_red());
}

pub fn print_warning(message: &str) {
    println!("{} {}", "⚠️".bright_yellow(), message.bright_yellow());
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ️".bright_blue(), message.bright_white());
}

pub fn print_code_block(code: &str, language: Option<&str>) {
    if let Some(lang) = language {
        println!("{}", format!("```{}", lang).bright_black().dimmed());
    } else {
        println!("{}", "```".bright_black().dimmed());
    }
    
    // Simple syntax highlighting for common languages
    for line in code.lines() {
        if language == Some("rust") {
            print_rust_line(line);
        } else if language == Some("python") {
            print_python_line(line);
        } else {
            println!("{}", line.bright_white());
        }
    }
    
    println!("{}", "```".bright_black().dimmed());
}

fn print_rust_line(line: &str) {
    let line = line
        .replace("fn ", &format!("{} ", "fn".bright_blue().bold()))
        .replace("let ", &format!("{} ", "let".bright_blue().bold()))
        .replace("mut ", &format!("{} ", "mut".bright_blue().bold()))
        .replace("pub ", &format!("{} ", "pub".bright_blue().bold()));
    println!("{}", line);
}

fn print_python_line(line: &str) {
    let line = line
        .replace("def ", &format!("{} ", "def".bright_blue().bold()))
        .replace("class ", &format!("{} ", "class".bright_blue().bold()))
        .replace("import ", &format!("{} ", "import".bright_blue().bold()))
        .replace("from ", &format!("{} ", "from".bright_blue().bold()));
    println!("{}", line);
}

pub fn print_table_header(headers: &[&str]) {
    let header_line = headers
        .iter()
        .map(|h| format!("{:20}", h.bright_white().bold()))
        .collect::<Vec<_>>()
        .join(" ");
    
    println!("{}", header_line);
    println!("{}", "─".repeat(headers.len() * 21).bright_black());
}

pub fn print_table_row(values: &[&str]) {
    let row_line = values
        .iter()
        .map(|v| format!("{:20}", v.bright_white()))
        .collect::<Vec<_>>()
        .join(" ");
    
    println!("{}", row_line);
}