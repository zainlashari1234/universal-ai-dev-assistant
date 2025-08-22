use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select, Confirm};
use std::io::{self, Write};
use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalSuggestRequest {
    pub query: String,
    pub query_type: String,
    pub session_id: Option<String>,
    pub workspace_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalSuggestResponse {
    pub session_id: String,
    pub suggestions: Vec<CommandSuggestion>,
    pub execution_result: Option<ExecutionResult>,
    pub explanation: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandSuggestion {
    pub command: String,
    pub explanation: String,
    pub confidence: f32,
    pub safety_level: String,
    pub category: String,
    pub estimated_time: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub command: String,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: i32,
    pub execution_time_ms: u64,
}

pub async fn run_interactive_terminal(client: &Client) -> Result<()> {
    println!("{}", "🖥️  AI Destekli Terminal".bright_blue().bold());
    println!("{}", "Komutları yazın veya doğal dilde ne yapmak istediğinizi açıklayın".bright_white().dimmed());
    println!("{}", "Komutlar: 'exit' (çıkış), 'help' (yardım), 'history' (geçmiş), 'stats' (istatistikler)".bright_white().dimmed());
    println!();

    let workspace_path = std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let mut session_id: Option<String> = None;
    let mut command_count = 0;

    loop {
        print!("{} ", "❯".bright_green().bold());
        io::stdout().flush()?;

        let input: String = Input::with_theme(&ColorfulTheme::default())
            .allow_empty(false)
            .interact_text()?;

        let input = input.trim();

        match input {
            "exit" | "quit" | "q" => {
                println!("{}", "👋 Terminal oturumu sonlandırıldı!".bright_blue());
                break;
            }
            "help" => {
                print_help();
                continue;
            }
            "history" => {
                show_command_history(client, &session_id).await?;
                continue;
            }
            "stats" => {
                show_statistics(client).await?;
                continue;
            }
            "clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
                continue;
            }
            _ => {}
        }

        if input.is_empty() {
            continue;
        }

        // Doğal dil mi yoksa komut mu kontrol et
        if is_natural_language(input) {
            // Doğal dil isteği - komut önerileri al
            match get_command_suggestions(client, input, &session_id, &workspace_path).await {
                Ok(response) => {
                    session_id = Some(response.session_id.clone());
                    display_suggestions(&response).await?;
                }
                Err(e) => {
                    println!("{} {}", "❌ Hata:".bright_red().bold(), e);
                }
            }
        } else {
            // Direkt komut çalıştırma
            match execute_command(client, input, &session_id, &workspace_path).await {
                Ok(response) => {
                    session_id = Some(response.session_id.clone());
                    display_execution_result(&response);
                    command_count += 1;
                }
                Err(e) => {
                    println!("{} {}", "❌ Hata:".bright_red().bold(), e);
                }
            }
        }

        // Her 10 komutta bir istatistik göster
        if command_count > 0 && command_count % 10 == 0 {
            println!("\n{}", format!("📊 {} komut çalıştırıldı", command_count).bright_cyan());
        }
    }

    Ok(())
}

async fn get_command_suggestions(
    client: &Client,
    query: &str,
    session_id: &Option<String>,
    workspace_path: &Option<String>,
) -> Result<TerminalSuggestResponse> {
    let request = TerminalSuggestRequest {
        query: query.to_string(),
        query_type: "natural_language".to_string(),
        session_id: session_id.clone(),
        workspace_path: workspace_path.clone(),
    };

    let response = client.post("/terminal/suggest", &request).await?;
    Ok(response)
}

async fn execute_command(
    client: &Client,
    command: &str,
    session_id: &Option<String>,
    workspace_path: &Option<String>,
) -> Result<TerminalSuggestResponse> {
    let request = TerminalSuggestRequest {
        query: command.to_string(),
        query_type: "command_execution".to_string(),
        session_id: session_id.clone(),
        workspace_path: workspace_path.clone(),
    };

    let response = client.post("/terminal/execute", &request).await?;
    Ok(response)
}

async fn display_suggestions(response: &TerminalSuggestResponse) -> Result<()> {
    if !response.warnings.is_empty() {
        for warning in &response.warnings {
            println!("{} {}", "⚠️".bright_yellow(), warning.bright_yellow());
        }
        println!();
    }

    if let Some(explanation) = &response.explanation {
        println!("{} {}", "💡".bright_blue(), explanation.bright_white());
        println!();
    }

    if response.suggestions.is_empty() {
        println!("{}", "Komut önerisi bulunamadı.".bright_yellow());
        return Ok(());
    }

    println!("{}", "🤖 Komut Önerileri:".bright_blue().bold());
    
    let suggestion_items: Vec<String> = response.suggestions
        .iter()
        .enumerate()
        .map(|(i, suggestion)| {
            let safety_icon = match suggestion.safety_level.as_str() {
                "safe" => "✅",
                "caution" => "⚠️",
                "dangerous" => "🚨",
                _ => "❓",
            };
            
            let time_info = suggestion.estimated_time
                .map(|t| format!(" (~{}s)", t))
                .unwrap_or_default();
            
            format!(
                "{} {} {} (güven: {:.1}{})",
                safety_icon,
                suggestion.command.bright_green(),
                suggestion.explanation.dimmed(),
                suggestion.confidence * 100.0,
                time_info
            )
        })
        .collect();

    suggestion_items.push("❌ Hiçbirini çalıştırma".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi komutu çalıştırmak istiyorsunuz?")
        .items(&suggestion_items)
        .default(0)
        .interact()?;

    if selection < response.suggestions.len() {
        let selected_suggestion = &response.suggestions[selection];
        
        // Tehlikeli komutlar için onay al
        if selected_suggestion.safety_level == "dangerous" {
            println!("{}", "🚨 TEHLİKELİ KOMUT UYARISI!".bright_red().bold());
            println!("{}", "Bu komut sisteminize zarar verebilir.".bright_red());
            
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Yine de çalıştırmak istiyor musunuz?")
                .default(false)
                .interact()?;
                
            if !confirm {
                println!("{}", "Komut iptal edildi.".bright_yellow());
                return Ok(());
            }
        }

        // Komutu çalıştır
        println!("\n{} {}", "🚀 Çalıştırılıyor:".bright_blue(), selected_suggestion.command.bright_green());
        
        // Burada gerçek komut çalıştırma API'sini çağıracağız
        // Şimdilik simüle edelim
        println!("{}", "Komut çalıştırıldı (simülasyon)".bright_green());
    }

    Ok(())
}

fn display_execution_result(response: &TerminalSuggestResponse) {
    if !response.warnings.is_empty() {
        for warning in &response.warnings {
            println!("{} {}", "⚠️".bright_yellow(), warning.bright_yellow());
        }
    }

    if let Some(result) = &response.execution_result {
        if result.exit_code == 0 {
            println!("{} Komut başarıyla tamamlandı", "✅".bright_green());
        } else {
            println!("{} Komut hata ile sonlandı (kod: {})", "❌".bright_red(), result.exit_code);
        }

        if !result.output.is_empty() {
            println!("\n{}", "📤 Çıktı:".bright_blue().bold());
            println!("{}", result.output);
        }

        if let Some(error) = &result.error {
            if !error.is_empty() {
                println!("\n{}", "❌ Hata:".bright_red().bold());
                println!("{}", error.bright_red());
            }
        }

        println!("\n{} {}ms", "⏱️".bright_cyan(), result.execution_time_ms);
    }
}

async fn show_command_history(client: &Client, session_id: &Option<String>) -> Result<()> {
    println!("{}", "📚 Komut Geçmişi".bright_blue().bold());
    
    if session_id.is_none() {
        println!("{}", "Henüz aktif bir oturum yok.".bright_yellow());
        return Ok(());
    }

    // API'den geçmişi al (şimdilik simüle edelim)
    println!("{}", "Geçmiş komutlar yükleniyor...".dimmed());
    
    // Gerçek implementasyonda burada API çağrısı yapılacak
    println!("{}", "Geçmiş komutlar burada görünecek.".bright_white());
    
    Ok(())
}

async fn show_statistics(client: &Client) -> Result<()> {
    println!("{}", "📊 Terminal İstatistikleri".bright_blue().bold());
    
    // API'den istatistikleri al (şimdilik simüle edelim)
    println!("{}", "İstatistikler yükleniyor...".dimmed());
    
    // Gerçek implementasyonda burada API çağrısı yapılacak
    println!("📈 Toplam komut: 42");
    println!("✅ Başarı oranı: %85");
    println!("🤖 AI önerisi kullanım: %60");
    println!("⏰ En aktif saat: 14:00");
    
    Ok(())
}

fn is_natural_language(input: &str) -> bool {
    let input_lower = input.to_lowercase();
    
    // Türkçe doğal dil göstergeleri
    let nl_indicators = [
        "nasıl", "nedir", "ne", "hangi", "göster", "bul", "listele", 
        "yap", "oluştur", "sil", "kopyala", "taşı", "yardım",
        "how", "what", "show", "find", "list", "create", "delete",
        "copy", "move", "help", "explain"
    ];
    
    // Soru kelimeleri
    let question_words = ["nasıl", "nedir", "ne", "hangi", "nerede", "ne zaman"];
    
    // Komut benzeri değilse (/ veya - ile başlamıyorsa) ve doğal dil göstergesi varsa
    let has_nl_indicator = nl_indicators.iter().any(|&indicator| input_lower.contains(indicator));
    let has_question = question_words.iter().any(|&word| input_lower.starts_with(word));
    let is_command_like = input.starts_with('/') || input.starts_with('-') || input.contains('=');
    
    (has_nl_indicator || has_question) && !is_command_like
}

fn print_help() {
    println!("{}", "💡 AI Destekli Terminal Yardımı".bright_yellow().bold());
    println!();
    println!("{}", "Komutlar:".bright_cyan().bold());
    println!("  {} - Terminali kapat", "exit, quit, q".bright_green());
    println!("  {} - Bu yardımı göster", "help".bright_green());
    println!("  {} - Komut geçmişini göster", "history".bright_green());
    println!("  {} - İstatistikleri göster", "stats".bright_green());
    println!("  {} - Ekranı temizle", "clear".bright_green());
    println!();
    println!("{}", "Kullanım:".bright_cyan().bold());
    println!("  • Direkt komut yazın: {}", "ls -la".bright_green());
    println!("  • Doğal dil kullanın: {}", "tüm python dosyalarını listele".bright_green());
    println!("  • Komut açıklaması: {}", "git status ne yapar?".bright_green());
    println!();
    println!("{}", "Güvenlik Seviyeleri:".bright_cyan().bold());
    println!("  {} Güvenli komutlar", "✅".bright_green());
    println!("  {} Dikkatli kullanılması gereken komutlar", "⚠️".bright_yellow());
    println!("  {} Tehlikeli komutlar (onay gerektirir)", "🚨".bright_red());
    println!();
}