use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Select, Confirm, Input};
use std::path::PathBuf;
use tokio::fs;
use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct FixRequest {
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
pub struct FixResponse {
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
    pub old_content: Option<String>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorSearchRequest {
    pub error_message: String,
    pub workspace_paths: Vec<String>,
}

pub async fn run_fix(
    error_message: Option<String>,
    file_path: Option<PathBuf>,
    auto_apply: bool,
    search_solutions: bool,
    client: &Client,
) -> Result<()> {
    println!("{}", "🔧 AI Hata Düzeltici".bright_red().bold());
    println!();

    // Hata mesajını al
    let error_msg = if let Some(msg) = error_message {
        msg
    } else {
        get_error_message_interactively().await?
    };

    println!("{} {}", "❌ Hata mesajı:".bright_red(), error_msg.bright_white());
    println!();

    // Hata tipini analiz et
    let error_type = analyze_error_type(&error_msg);
    println!("{} {}", "🔍 Tespit edilen hata tipi:".bright_blue(), error_type.bright_yellow());

    // İlgili dosyayı belirle
    let target_file = if let Some(path) = file_path {
        Some(path)
    } else {
        extract_file_from_error(&error_msg).await?
    };

    if let Some(ref file) = target_file {
        println!("{} {}", "📁 İlgili dosya:".bright_blue(), file.display().to_string().bright_white());
    }

    // Benzer hata çözümlerini ara
    if search_solutions {
        search_error_solutions(&error_msg, client).await?;
    }

    // AI ile hata analizi ve çözüm önerisi
    let fix_response = get_fix_suggestions(&error_msg, &target_file, client).await?;
    
    // Çözüm önerilerini göster
    display_fix_suggestions(&fix_response, &error_msg).await?;

    // Kod değişikliklerini uygula
    if let Some(code_changes) = &fix_response.response.code_changes {
        if !code_changes.is_empty() {
            handle_code_changes(code_changes, auto_apply, &target_file).await?;
        }
    }

    // Follow-up actions
    handle_fix_actions(&fix_response, &error_msg, &target_file, client).await?;

    Ok(())
}

async fn get_error_message_interactively() -> Result<String> {
    println!("{}", "Hata mesajını nasıl almak istiyorsunuz?".bright_cyan());
    
    let options = vec![
        "⌨️  Manuel olarak gir",
        "📋 Clipboard'dan yapıştır",
        "📄 Log dosyasından oku",
        "🔄 Son build çıktısından al",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Seçim yapın")
        .items(&options)
        .interact()?;

    match selection {
        0 => {
            // Manuel giriş
            let error: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Hata mesajını girin")
                .allow_empty(false)
                .interact_text()?;
            Ok(error)
        }
        1 => {
            // Clipboard (basit implementasyon)
            println!("{}", "📋 Clipboard özelliği yakında eklenecek!".bright_yellow());
            println!("{}", "Şimdilik manuel olarak girin:".dimmed());
            let error: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Hata mesajını girin")
                .allow_empty(false)
                .interact_text()?;
            Ok(error)
        }
        2 => {
            // Log dosyasından oku
            read_error_from_log().await
        }
        3 => {
            // Son build çıktısından al
            get_last_build_error().await
        }
        _ => Err(anyhow::anyhow!("Geçersiz seçim")),
    }
}

async fn read_error_from_log() -> Result<String> {
    let log_files = find_log_files().await?;
    
    if log_files.is_empty() {
        println!("{}", "⚠️ Log dosyası bulunamadı.".bright_yellow());
        return get_error_message_interactively().await;
    }

    let file_names: Vec<String> = log_files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi log dosyasını okumak istiyorsunuz?")
        .items(&file_names)
        .interact()?;

    let content = fs::read_to_string(&log_files[selection]).await?;
    
    // Son hata mesajını bul
    let lines: Vec<&str> = content.lines().collect();
    for line in lines.iter().rev() {
        if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
            return Ok(line.to_string());
        }
    }

    Err(anyhow::anyhow!("Log dosyasında hata bulunamadı"))
}

async fn find_log_files() -> Result<Vec<PathBuf>> {
    let current_dir = std::env::current_dir()?;
    let mut log_files = Vec::new();

    // Yaygın log dosya isimleri
    let log_patterns = ["*.log", "error.log", "build.log", "output.log"];
    
    for pattern in &log_patterns {
        if let Ok(entries) = glob::glob(&format!("{}/**/{}", current_dir.display(), pattern)) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    log_files.push(entry);
                }
            }
        }
    }

    Ok(log_files)
}

async fn get_last_build_error() -> Result<String> {
    println!("{}", "🔄 Son build çıktısı kontrol ediliyor...".bright_yellow());

    // Yaygın build komutlarını dene
    let build_commands = [
        ("cargo", vec!["build"]),
        ("npm", vec!["run", "build"]),
        ("make", vec![]),
        ("mvn", vec!["compile"]),
        ("gradle", vec!["build"]),
    ];

    for (cmd, args) in &build_commands {
        if std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            println!("{} {} çalıştırılıyor...", "🔨".bright_blue(), cmd);
            
            let output = std::process::Command::new(cmd)
                .args(args)
                .output();

            if let Ok(result) = output {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let stdout = String::from_utf8_lossy(&result.stdout);
                
                // Hata mesajını bul
                for line in stderr.lines().chain(stdout.lines()) {
                    if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
                        return Ok(line.to_string());
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Build hatası bulunamadı"))
}

fn analyze_error_type(error_message: &str) -> String {
    let error_lower = error_message.to_lowercase();
    
    if error_lower.contains("syntax") {
        "Syntax Error".to_string()
    } else if error_lower.contains("type") || error_lower.contains("expected") {
        "Type Error".to_string()
    } else if error_lower.contains("borrow") || error_lower.contains("moved") {
        "Ownership Error (Rust)".to_string()
    } else if error_lower.contains("undefined") || error_lower.contains("not found") {
        "Reference Error".to_string()
    } else if error_lower.contains("import") || error_lower.contains("module") {
        "Import Error".to_string()
    } else if error_lower.contains("compile") || error_lower.contains("compilation") {
        "Compilation Error".to_string()
    } else if error_lower.contains("runtime") || error_lower.contains("exception") {
        "Runtime Error".to_string()
    } else if error_lower.contains("permission") || error_lower.contains("access") {
        "Permission Error".to_string()
    } else if error_lower.contains("network") || error_lower.contains("connection") {
        "Network Error".to_string()
    } else {
        "General Error".to_string()
    }
}

async fn extract_file_from_error(error_message: &str) -> Result<Option<PathBuf>> {
    // Hata mesajından dosya yolunu çıkarmaya çalış
    let file_patterns = [
        regex::Regex::new(r"([a-zA-Z0-9_/.-]+\.[a-zA-Z]+):(\d+):(\d+)").unwrap(),
        regex::Regex::new(r"at ([a-zA-Z0-9_/.-]+\.[a-zA-Z]+):(\d+)").unwrap(),
        regex::Regex::new(r"in ([a-zA-Z0-9_/.-]+\.[a-zA-Z]+)").unwrap(),
        regex::Regex::new(r"file://([a-zA-Z0-9_/.-]+\.[a-zA-Z]+)").unwrap(),
    ];

    for pattern in &file_patterns {
        if let Some(captures) = pattern.captures(error_message) {
            if let Some(file_path) = captures.get(1) {
                let path = PathBuf::from(file_path.as_str());
                if path.exists() {
                    return Ok(Some(path));
                }
            }
        }
    }

    // Dosya bulunamazsa kullanıcıdan sor
    let specify_file = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Hatayla ilgili dosyayı belirtmek istiyor musunuz?")
        .default(false)
        .interact()?;

    if specify_file {
        let file_path: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Dosya yolu")
            .interact_text()?;
        
        let path = PathBuf::from(file_path);
        if path.exists() {
            Ok(Some(path))
        } else {
            println!("{} Dosya bulunamadı: {}", "⚠️".bright_yellow(), path.display());
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

async fn search_error_solutions(error_message: &str, client: &Client) -> Result<()> {
    println!("{}", "🔍 Benzer hata çözümleri aranıyor...".bright_yellow());

    let search_request = ErrorSearchRequest {
        error_message: error_message.to_string(),
        workspace_paths: vec![std::env::current_dir()?.to_string_lossy().to_string()],
    };

    match client.post::<serde_json::Value, _>("/search/errors", &search_request).await {
        Ok(response) => {
            if let Some(results) = response["response"]["results"].as_array() {
                if !results.is_empty() {
                    println!();
                    println!("{}", "💡 Benzer Hata Çözümleri".bright_green().bold());
                    
                    for (i, result) in results.iter().take(3).enumerate() {
                        if let (Some(file_path), Some(content), Some(relevance)) = (
                            result["file_path"].as_str(),
                            result["content"].as_str(),
                            result["relevance_score"].as_f64()
                        ) {
                            println!("  {}. {} ({:.2})", 
                                i + 1, 
                                file_path.bright_white(), 
                                relevance
                            );
                            
                            // İlk 100 karakteri göster
                            let preview = if content.len() > 100 {
                                format!("{}...", &content[..100])
                            } else {
                                content.to_string()
                            };
                            println!("     {}", preview.dimmed());
                        }
                    }
                    println!();
                } else {
                    println!("{}", "ℹ️ Benzer hata çözümü bulunamadı.".bright_blue());
                }
            }
        }
        Err(e) => {
            println!("{} {}", "⚠️ Hata arama hatası:".bright_yellow(), e);
        }
    }

    Ok(())
}

async fn get_fix_suggestions(
    error_message: &str,
    file_path: &Option<PathBuf>,
    client: &Client,
) -> Result<FixResponse> {
    println!("{}", "🤖 AI hata analizi yapılıyor...".bright_yellow());

    let mut message = format!("Bu hatayı analiz et ve çözüm öner:\n\n{}", error_message);

    // Dosya içeriği varsa ekle
    if let Some(file) = file_path {
        if let Ok(content) = fs::read_to_string(file).await {
            message.push_str(&format!("\n\nİlgili dosya içeriği:\n```\n{}\n```", content));
        }
    }

    message.push_str("\n\nLütfen:\n1. Hatanın nedenini açıkla\n2. Adım adım çözüm öner\n3. Düzeltilmiş kod ver\n4. Gelecekte nasıl önlenebileceğini açıkla");

    let fix_request = FixRequest {
        session_id: None,
        message,
        current_file: file_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        selected_text: None,
        context_files: file_path.as_ref().map(|p| vec![p.to_string_lossy().to_string()]).unwrap_or_default(),
        intent_hint: Some("Debugging".to_string()),
    };

    let response: FixResponse = client.post("/conversation/message", &fix_request).await?;
    Ok(response)
}

async fn display_fix_suggestions(response: &FixResponse, error_message: &str) -> Result<()> {
    let conv_response = &response.response;
    
    println!("{}", "🔧 AI Hata Analizi ve Çözüm Önerileri".bright_green().bold());
    println!("{}", "=".repeat(80).bright_green());
    println!();
    
    // Ana analiz ve çözüm
    println!("{}", conv_response.ai_response);
    println!();
    
    // Güven skoru
    println!("{} {:.1}%", 
        "📊 Güven skoru:".bright_blue(), 
        conv_response.confidence_score * 100.0
    );
    
    // Önerilen aksiyonlar
    if !conv_response.suggested_actions.is_empty() {
        println!();
        println!("{}", "⚡ Önerilen Aksiyonlar".bright_yellow().bold());
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

async fn handle_code_changes(
    code_changes: &[CodeChange],
    auto_apply: bool,
    target_file: &Option<PathBuf>,
) -> Result<()> {
    println!("{}", "📝 Kod Değişiklikleri".bright_blue().bold());
    
    for (i, change) in code_changes.iter().enumerate() {
        println!("{}. {} - {}", 
            i + 1, 
            change.file_path.bright_white(), 
            change.description.bright_cyan()
        );
        
        println!("   {} {}", "🔄 Tip:".bright_blue(), change.change_type.bright_yellow());
        
        // Kod önizlemesi
        let preview = if change.new_content.len() > 200 {
            format!("{}...", &change.new_content[..200])
        } else {
            change.new_content.clone()
        };
        
        println!("   {} Yeni kod:", "📄".bright_blue());
        println!("{}", format_code_preview(&preview));
        
        if let Some(old_content) = &change.old_content {
            println!("   {} Eski kod:", "📄".bright_red());
            let old_preview = if old_content.len() > 200 {
                format!("{}...", &old_content[..200])
            } else {
                old_content.clone()
            };
            println!("{}", format_code_preview(&old_preview));
        }
        
        // Değişikliği uygula
        if auto_apply {
            apply_code_change(change).await?;
            println!("{} Değişiklik uygulandı!", "✅".bright_green());
        } else {
            let apply = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Bu değişikliği uygulamak istiyor musunuz?")
                .default(true)
                .interact()?;
            
            if apply {
                apply_code_change(change).await?;
                println!("{} Değişiklik uygulandı!", "✅".bright_green());
            } else {
                println!("{} Değişiklik atlandı.", "⏭️".bright_yellow());
            }
        }
        
        println!();
    }
    
    Ok(())
}

fn format_code_preview(code: &str) -> String {
    code.lines()
        .enumerate()
        .map(|(i, line)| format!("     {:2} │ {}", i + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn apply_code_change(change: &CodeChange) -> Result<()> {
    let file_path = PathBuf::from(&change.file_path);
    
    match change.change_type.as_str() {
        "Create" => {
            // Yeni dosya oluştur
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&file_path, &change.new_content).await?;
        }
        "Modify" => {
            // Mevcut dosyayı güncelle
            if file_path.exists() {
                // Backup oluştur
                let backup_path = format!("{}.backup", file_path.display());
                fs::copy(&file_path, &backup_path).await?;
                println!("   {} Backup oluşturuldu: {}", "💾".bright_blue(), backup_path.dimmed());
            }
            fs::write(&file_path, &change.new_content).await?;
        }
        "Delete" => {
            // Dosyayı sil
            if file_path.exists() {
                fs::remove_file(&file_path).await?;
            }
        }
        _ => {
            println!("   {} Bilinmeyen değişiklik tipi: {}", "⚠️".bright_yellow(), change.change_type);
        }
    }
    
    Ok(())
}

async fn handle_fix_actions(
    response: &FixResponse,
    error_message: &str,
    target_file: &Option<PathBuf>,
    client: &Client,
) -> Result<()> {
    let actions = vec![
        "🔄 Düzeltmeyi test et",
        "🔍 Benzer hataları ara",
        "💬 Bu hata hakkında soru sor",
        "📚 Hata hakkında daha fazla bilgi al",
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
                test_fix(target_file).await?;
            }
            1 => {
                search_similar_errors(error_message, client).await?;
            }
            2 => {
                println!("{}", "💬 Soru sorma özelliği yakında eklenecek!".bright_yellow());
                // TODO: Interactive Q&A about the error
            }
            3 => {
                get_more_error_info(error_message, client).await?;
            }
            4 => {
                if let Some(file) = target_file {
                    open_file_in_editor(file).await?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn test_fix(target_file: &Option<PathBuf>) -> Result<()> {
    println!("{}", "🧪 Düzeltme test ediliyor...".bright_yellow());

    // Proje tipine göre test komutu belirle
    let test_commands = [
        ("cargo", vec!["check"]),
        ("cargo", vec!["test"]),
        ("npm", vec!["test"]),
        ("python", vec!["-m", "pytest"]),
        ("mvn", vec!["test"]),
        ("make", vec!["test"]),
    ];

    for (cmd, args) in &test_commands {
        if std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            println!("{} {} çalıştırılıyor...", "🔨".bright_blue(), cmd);
            
            let output = std::process::Command::new(cmd)
                .args(args)
                .output();

            if let Ok(result) = output {
                if result.status.success() {
                    println!("{} Test başarılı!", "✅".bright_green());
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    if !stdout.trim().is_empty() {
                        println!("{}", stdout.trim().dimmed());
                    }
                    return Ok(());
                } else {
                    println!("{} Test başarısız:", "❌".bright_red());
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    if !stderr.trim().is_empty() {
                        println!("{}", stderr.trim().bright_red());
                    }
                }
            }
        }
    }

    println!("{}", "⚠️ Uygun test komutu bulunamadı.".bright_yellow());
    Ok(())
}

async fn search_similar_errors(error_message: &str, client: &Client) -> Result<()> {
    println!("{}", "🔍 Benzer hatalar aranıyor...".bright_yellow());
    search_error_solutions(error_message, client).await
}

async fn get_more_error_info(error_message: &str, client: &Client) -> Result<()> {
    println!("{}", "📚 Hata hakkında daha fazla bilgi alınıyor...".bright_yellow());
    
    let info_request = FixRequest {
        session_id: None,
        message: format!("Bu hata tipi hakkında detaylı bilgi ver: {}\n\nLütfen:\n1. Bu hatanın yaygın nedenlerini açıkla\n2. Önleme yöntemlerini anlat\n3. İlgili best practice'leri paylaş", error_message),
        current_file: None,
        selected_text: None,
        context_files: Vec::new(),
        intent_hint: Some("Documentation".to_string()),
    };

    match client.post::<FixResponse, _>("/conversation/message", &info_request).await {
        Ok(response) => {
            println!();
            println!("{}", "📖 Hata Hakkında Detaylı Bilgi".bright_blue().bold());
            println!("{}", "=".repeat(60).bright_blue());
            println!("{}", response.response.ai_response);
        }
        Err(e) => {
            println!("{} {}", "❌ Bilgi alınamadı:".bright_red(), e);
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