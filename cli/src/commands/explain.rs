use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Select, Confirm};
use std::path::PathBuf;
use tokio::fs;
use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplainRequest {
    pub session_id: Option<String>,
    pub message: String,
    pub current_file: Option<String>,
    pub selected_text: Option<TextSelection>,
    pub context_files: Vec<String>,
    pub intent_hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextSelection {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplainResponse {
    pub success: bool,
    pub response: ConversationResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub session_id: String,
    pub ai_response: String,
    pub intent: String,
    pub confidence_score: f32,
    pub code_changes: Option<Vec<CodeChange>>,
    pub suggested_actions: Vec<SuggestedAction>,
    pub file_references: Vec<String>,
    pub follow_up_questions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub change_type: String,
    pub new_content: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: String,
    pub description: String,
    pub command: Option<String>,
    pub priority: String,
}

pub async fn run_explain(
    file_path: Option<PathBuf>,
    line_range: Option<String>,
    function_name: Option<String>,
    with_examples: bool,
    search_similar: bool,
    client: &Client,
) -> Result<()> {
    println!("{}", "📖 AI Kod Açıklayıcı".bright_blue().bold());
    println!();

    let target_file = if let Some(path) = file_path {
        path
    } else {
        // Mevcut dizindeki dosyaları listele
        select_file_interactively().await?
    };

    if !target_file.exists() {
        println!("{} Dosya bulunamadı: {}", "❌".bright_red(), target_file.display());
        return Ok(());
    }

    println!("{} {}", "📁 Açıklanacak dosya:".bright_blue(), target_file.display().to_string().bright_white());

    // Dosya içeriğini oku
    let content = fs::read_to_string(&target_file).await?;
    let lines: Vec<&str> = content.lines().collect();

    // Açıklanacak kısmı belirle
    let (selected_content, start_line, end_line) = if let Some(func_name) = function_name {
        extract_function(&content, &func_name)?
    } else if let Some(range) = line_range {
        extract_line_range(&lines, &range)?
    } else {
        // İnteraktif seçim
        select_code_section(&content, &target_file).await?
    };

    println!("{} {}:{}", "📍 Seçilen bölüm:".bright_blue(), start_line, end_line);
    println!();

    // Kod önizlemesi göster
    display_code_preview(&selected_content, start_line);

    // Açıklama isteği oluştur
    let mut message = format!("Bu kodu detaylı olarak açıkla:\n\n```\n{}\n```", selected_content);
    
    if with_examples {
        message.push_str("\n\nLütfen kod örnekleri ve kullanım senaryoları da ekle.");
    }

    let explain_request = ExplainRequest {
        session_id: None,
        message,
        current_file: Some(target_file.to_string_lossy().to_string()),
        selected_text: Some(TextSelection {
            start_line,
            start_column: 0,
            end_line,
            end_column: 0,
            text: selected_content.clone(),
        }),
        context_files: vec![target_file.to_string_lossy().to_string()],
        intent_hint: Some("CodeExplanation".to_string()),
    };

    // AI açıklaması al
    println!("{}", "🤖 AI açıklama oluşturuyor...".bright_yellow());
    
    match get_explanation(&explain_request, client).await {
        Ok(response) => {
            display_explanation(&response, &selected_content).await?;
            
            // Benzer kod arama
            if search_similar {
                search_similar_code(&selected_content, &target_file, client).await?;
            }
            
            // Follow-up actions
            handle_explanation_actions(&response, &target_file, client).await?;
        }
        Err(e) => {
            println!("{} {}", "❌ Açıklama alınamadı:".bright_red(), e);
        }
    }

    Ok(())
}

async fn select_file_interactively() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let mut entries = fs::read_dir(&current_dir).await?;
    let mut files = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                let ext = extension.to_string_lossy().to_lowercase();
                if matches!(ext.as_str(), "rs" | "js" | "ts" | "py" | "java" | "go" | "cpp" | "c" | "cs") {
                    files.push(path);
                }
            }
        }
    }

    if files.is_empty() {
        return Err(anyhow::anyhow!("Mevcut dizinde desteklenen kod dosyası bulunamadı"));
    }

    let file_names: Vec<String> = files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi dosyayı açıklamak istiyorsunuz?")
        .items(&file_names)
        .interact()?;

    Ok(files[selection].clone())
}

async fn select_code_section(content: &str, file_path: &PathBuf) -> Result<(String, usize, usize)> {
    let options = vec![
        "📄 Tüm dosyayı açıkla",
        "🔧 Belirli bir fonksiyonu seç",
        "📏 Satır aralığı belirle",
        "🎯 İnteraktif seçim",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Neyi açıklamak istiyorsunuz?")
        .items(&options)
        .interact()?;

    match selection {
        0 => {
            // Tüm dosya
            let line_count = content.lines().count();
            Ok((content.to_string(), 1, line_count))
        }
        1 => {
            // Fonksiyon seçimi
            let functions = extract_functions(content, file_path)?;
            if functions.is_empty() {
                println!("{}", "⚠️ Bu dosyada fonksiyon bulunamadı.".bright_yellow());
                Ok((content.to_string(), 1, content.lines().count()))
            } else {
                let func_names: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
                let func_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Hangi fonksiyonu açıklamak istiyorsunuz?")
                    .items(&func_names)
                    .interact()?;
                
                let selected_func = &functions[func_selection];
                Ok((selected_func.content.clone(), selected_func.start_line, selected_func.end_line))
            }
        }
        2 => {
            // Satır aralığı
            println!("{}", "📏 Satır aralığı formatı: 'başlangıç-bitiş' (örn: 10-25)".dimmed());
            let range: String = dialoguer::Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Satır aralığı")
                .interact_text()?;
            
            let lines: Vec<&str> = content.lines().collect();
            extract_line_range(&lines, &range)
        }
        3 => {
            // İnteraktif seçim - fonksiyonları listele
            interactive_code_selection(content, file_path).await
        }
        _ => Ok((content.to_string(), 1, content.lines().count())),
    }
}

fn extract_function(content: &str, function_name: &str) -> Result<(String, usize, usize)> {
    let lines: Vec<&str> = content.lines().collect();
    
    // Basit fonksiyon arama (dil agnostik)
    for (i, line) in lines.iter().enumerate() {
        if line.contains(function_name) && 
           (line.contains("fn ") || line.contains("function ") || line.contains("def ") || 
            line.contains("func ") || line.contains("public ") || line.contains("private ")) {
            
            // Fonksiyon sonunu bul
            let mut brace_count = 0;
            let mut end_line = i;
            let mut found_opening = false;
            
            for (j, check_line) in lines.iter().enumerate().skip(i) {
                for ch in check_line.chars() {
                    match ch {
                        '{' => {
                            brace_count += 1;
                            found_opening = true;
                        }
                        '}' => {
                            brace_count -= 1;
                            if found_opening && brace_count == 0 {
                                end_line = j;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if found_opening && brace_count == 0 {
                    break;
                }
            }
            
            let function_content = lines[i..=end_line].join("\n");
            return Ok((function_content, i + 1, end_line + 1));
        }
    }
    
    Err(anyhow::anyhow!("Fonksiyon bulunamadı: {}", function_name))
}

fn extract_line_range(lines: &[&str], range: &str) -> Result<(String, usize, usize)> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Geçersiz satır aralığı formatı. Kullanım: 'başlangıç-bitiş'"));
    }
    
    let start: usize = parts[0].trim().parse()?;
    let end: usize = parts[1].trim().parse()?;
    
    if start == 0 || end == 0 || start > end || end > lines.len() {
        return Err(anyhow::anyhow!("Geçersiz satır aralığı: {}-{}", start, end));
    }
    
    let selected_lines = lines[(start-1)..end].join("\n");
    Ok((selected_lines, start, end))
}

#[derive(Debug)]
struct FunctionInfo {
    name: String,
    content: String,
    start_line: usize,
    end_line: usize,
}

fn extract_functions(content: &str, file_path: &PathBuf) -> Result<Vec<FunctionInfo>> {
    let mut functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    let function_patterns = match extension {
        "rs" => vec![r"fn\s+(\w+)", r"pub\s+fn\s+(\w+)", r"async\s+fn\s+(\w+)"],
        "js" | "ts" => vec![r"function\s+(\w+)", r"const\s+(\w+)\s*=", r"(\w+)\s*:\s*function"],
        "py" => vec![r"def\s+(\w+)", r"async\s+def\s+(\w+)"],
        "java" | "cs" => vec![r"public\s+\w+\s+(\w+)\s*\(", r"private\s+\w+\s+(\w+)\s*\("],
        "go" => vec![r"func\s+(\w+)", r"func\s+\(\w+\s+\*?\w+\)\s+(\w+)"],
        "cpp" | "c" => vec![r"\w+\s+(\w+)\s*\("],
        _ => vec![r"(\w+)\s*\("],
    };
    
    for pattern_str in function_patterns {
        let pattern = regex::Regex::new(pattern_str)?;
        
        for (i, line) in lines.iter().enumerate() {
            if let Some(captures) = pattern.captures(line) {
                if let Some(func_name) = captures.get(1) {
                    // Fonksiyon sonunu bul
                    let (content, end_line) = extract_function_body(&lines, i, extension);
                    
                    functions.push(FunctionInfo {
                        name: func_name.as_str().to_string(),
                        content,
                        start_line: i + 1,
                        end_line: end_line + 1,
                    });
                }
            }
        }
    }
    
    // Duplicates'leri kaldır
    functions.sort_by(|a, b| a.start_line.cmp(&b.start_line));
    functions.dedup_by(|a, b| a.name == b.name && a.start_line == b.start_line);
    
    Ok(functions)
}

fn extract_function_body(lines: &[&str], start_line: usize, extension: &str) -> (String, usize) {
    let mut end_line = start_line;
    let mut brace_count = 0;
    let mut found_opening = false;
    
    // Python için özel handling
    if extension == "py" {
        let base_indent = lines[start_line].len() - lines[start_line].trim_start().len();
        
        for (i, line) in lines.iter().enumerate().skip(start_line + 1) {
            if line.trim().is_empty() {
                continue;
            }
            
            let current_indent = line.len() - line.trim_start().len();
            if current_indent <= base_indent && !line.trim().is_empty() {
                end_line = i - 1;
                break;
            }
            end_line = i;
        }
    } else {
        // Brace-based languages
        for (i, line) in lines.iter().enumerate().skip(start_line) {
            for ch in line.chars() {
                match ch {
                    '{' => {
                        brace_count += 1;
                        found_opening = true;
                    }
                    '}' => {
                        brace_count -= 1;
                        if found_opening && brace_count == 0 {
                            end_line = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if found_opening && brace_count == 0 {
                break;
            }
        }
    }
    
    let content = lines[start_line..=end_line].join("\n");
    (content, end_line)
}

async fn interactive_code_selection(content: &str, file_path: &PathBuf) -> Result<(String, usize, usize)> {
    let functions = extract_functions(content, file_path)?;
    
    if functions.is_empty() {
        println!("{}", "ℹ️ Bu dosyada fonksiyon bulunamadı, tüm dosya gösterilecek.".bright_blue());
        return Ok((content.to_string(), 1, content.lines().count()));
    }
    
    println!("{}", "🔧 Bulunan fonksiyonlar:".bright_blue().bold());
    for (i, func) in functions.iter().enumerate() {
        println!("  {}. {} ({}:{})", 
            i + 1, 
            func.name.bright_white(), 
            func.start_line, 
            func.end_line
        );
    }
    
    let mut options: Vec<String> = functions.iter()
        .map(|f| format!("{} ({}:{})", f.name, f.start_line, f.end_line))
        .collect();
    options.push("📄 Tüm dosya".to_string());
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi kısmı açıklamak istiyorsunuz?")
        .items(&options)
        .interact()?;
    
    if selection == functions.len() {
        // Tüm dosya seçildi
        Ok((content.to_string(), 1, content.lines().count()))
    } else {
        let selected_func = &functions[selection];
        Ok((selected_func.content.clone(), selected_func.start_line, selected_func.end_line))
    }
}

fn display_code_preview(content: &str, start_line: usize) {
    println!("{}", "📄 Kod Önizlemesi".bright_blue().bold());
    println!("{}", "─".repeat(60).dimmed());
    
    for (i, line) in content.lines().enumerate() {
        let line_num = start_line + i;
        println!("{:4} │ {}", 
            line_num.to_string().dimmed(), 
            line
        );
        
        // İlk 10 satırdan fazlaysa kısalt
        if i >= 10 && content.lines().count() > 15 {
            let remaining = content.lines().count() - i - 1;
            println!("{}", format!("     │ ... ({} satır daha)", remaining).dimmed());
            break;
        }
    }
    
    println!("{}", "─".repeat(60).dimmed());
    println!();
}

async fn get_explanation(request: &ExplainRequest, client: &Client) -> Result<ExplainResponse> {
    let response: ExplainResponse = client.post("/conversation/message", request).await?;
    Ok(response)
}

async fn display_explanation(response: &ExplainResponse, code: &str) -> Result<()> {
    let conv_response = &response.response;
    
    println!("{}", "🤖 AI Açıklaması".bright_green().bold());
    println!("{}", "=".repeat(80).bright_green());
    println!();
    
    // Ana açıklama
    println!("{}", conv_response.ai_response);
    println!();
    
    // Güven skoru
    println!("{} {:.1}%", 
        "📊 Güven skoru:".bright_blue(), 
        conv_response.confidence_score * 100.0
    );
    
    // Dosya referansları
    if !conv_response.file_references.is_empty() {
        println!("{} {}", 
            "📁 İlgili dosyalar:".bright_blue(), 
            conv_response.file_references.join(", ").bright_white()
        );
    }
    
    // Önerilen aksiyonlar
    if !conv_response.suggested_actions.is_empty() {
        println!();
        println!("{}", "💡 Önerilen Aksiyonlar".bright_yellow().bold());
        for action in &conv_response.suggested_actions {
            let priority_icon = match action.priority.as_str() {
                "High" => "🔴",
                "Medium" => "🟡", 
                "Low" => "🟢",
                _ => "⚪",
            };
            
            println!("  {} {} {}", 
                priority_icon, 
                action.description.bright_white(),
                action.action_type.dimmed()
            );
            
            if let Some(command) = &action.command {
                println!("    {} {}", "💻".bright_blue(), command.bright_cyan());
            }
        }
    }
    
    // Follow-up sorular
    if !conv_response.follow_up_questions.is_empty() {
        println!();
        println!("{}", "❓ İlgili Sorular".bright_cyan().bold());
        for question in &conv_response.follow_up_questions {
            println!("  {} {}", "•".bright_cyan(), question.bright_white());
        }
    }
    
    println!();
    Ok(())
}

async fn search_similar_code(code: &str, file_path: &PathBuf, client: &Client) -> Result<()> {
    println!("{}", "🔍 Benzer kod aranıyor...".bright_yellow());
    
    let search_request = serde_json::json!({
        "code_snippet": code,
        "workspace_paths": [std::env::current_dir()?.to_string_lossy()]
    });
    
    match client.post::<serde_json::Value, _>("/search/similar", &search_request).await {
        Ok(response) => {
            if let Some(results) = response["response"]["results"].as_array() {
                if !results.is_empty() {
                    println!();
                    println!("{}", "🎯 Benzer Kodlar Bulundu".bright_green().bold());
                    
                    for (i, result) in results.iter().take(3).enumerate() {
                        if let (Some(file_path), Some(relevance)) = (
                            result["file_path"].as_str(),
                            result["relevance_score"].as_f64()
                        ) {
                            println!("  {}. {} ({:.2})", 
                                i + 1, 
                                file_path.bright_white(), 
                                relevance
                            );
                        }
                    }
                } else {
                    println!("{}", "ℹ️ Benzer kod bulunamadı.".bright_blue());
                }
            }
        }
        Err(e) => {
            println!("{} {}", "⚠️ Benzer kod arama hatası:".bright_yellow(), e);
        }
    }
    
    Ok(())
}

async fn handle_explanation_actions(
    response: &ExplainResponse, 
    file_path: &PathBuf, 
    client: &Client
) -> Result<()> {
    let actions = vec![
        "💬 Bu kod hakkında soru sor",
        "🔍 Benzer kodları ara", 
        "🔧 Kod iyileştirme önerileri al",
        "📝 Bu kodu test et",
        "📁 Dosyayı editörde aç",
        "❌ Hiçbiri",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Ne yapmak istiyorsunuz?")
        .items(&actions)
        .default(5)
        .interact_opt()?;

    if let Some(choice) = selection {
        match choice {
            0 => {
                println!("{}", "💬 Soru sorma özelliği yakında eklenecek!".bright_yellow());
                // TODO: Interactive Q&A session
            }
            1 => {
                println!("{}", "🔍 Benzer kod arama özelliği yakında eklenecek!".bright_yellow());
                // TODO: Similar code search
            }
            2 => {
                println!("{}", "🔧 Kod iyileştirme özelliği yakında eklenecek!".bright_yellow());
                // TODO: Code improvement suggestions
            }
            3 => {
                println!("{}", "📝 Test oluşturma özelliği yakında eklenecek!".bright_yellow());
                // TODO: Test generation
            }
            4 => {
                // Dosyayı editörde aç
                open_file_in_editor(file_path).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn open_file_in_editor(file_path: &PathBuf) -> Result<()> {
    let editors = ["code", "vim", "nano", "gedit"];
    
    for editor in &editors {
        if std::process::Command::new("which")
            .arg(editor)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            println!("{} {} ile açılıyor: {}", 
                "📝".bright_blue(), 
                editor, 
                file_path.display().to_string().bright_white()
            );
            
            match std::process::Command::new(editor)
                .arg(file_path)
                .spawn()
            {
                Ok(_) => {
                    println!("{}", "✅ Dosya açıldı!".bright_green());
                    return Ok(());
                }
                Err(e) => {
                    println!("{} {} ile açılamadı: {}", "⚠️".bright_yellow(), editor, e);
                }
            }
        }
    }
    
    println!("{}", "❌ Uygun editör bulunamadı. Dosya yolu:".bright_red());
    println!("{}", file_path.display().to_string().bright_white());
    Ok(())
}