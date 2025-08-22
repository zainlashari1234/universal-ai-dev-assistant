use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexRequest {
    pub workspace_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexResponse {
    pub success: bool,
    pub stats: IndexStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_files: i64,
    pub total_symbols: i64,
    pub languages_count: i64,
    pub avg_complexity: f32,
}

pub async fn run_index(
    workspace_path: Option<PathBuf>,
    force: bool,
    verbose: bool,
    client: &Client,
) -> Result<()> {
    println!("{}", "📚 Workspace Indexer".bright_blue().bold());
    println!();

    let workspace = if let Some(path) = workspace_path {
        path
    } else {
        std::env::current_dir()?
    };

    if !workspace.exists() {
        println!("{} Workspace bulunamadı: {}", "❌".bright_red(), workspace.display());
        return Ok(());
    }

    println!("{} {}", "📁 Workspace:".bright_blue(), workspace.display().to_string().bright_white());

    // Mevcut index durumunu kontrol et
    if !force {
        match check_existing_index(&workspace, client).await {
            Ok(Some(stats)) => {
                println!();
                println!("{}", "ℹ️ Mevcut Index Bulundu".bright_cyan().bold());
                display_index_stats(&stats);
                
                let reindex = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Yeniden indexlemek istiyor musunuz?")
                    .default(false)
                    .interact()?;
                
                if !reindex {
                    println!("{}", "✅ Mevcut index korundu.".bright_green());
                    return Ok(());
                }
            }
            Ok(None) => {
                println!("{}", "ℹ️ Bu workspace daha önce indexlenmemiş.".bright_blue());
            }
            Err(e) => {
                if verbose {
                    println!("{} Index durumu kontrol edilemedi: {}", "⚠️".bright_yellow(), e);
                }
            }
        }
    }

    // Indexleme öncesi analiz
    if verbose {
        analyze_workspace_before_indexing(&workspace).await?;
    }

    // Indexleme başlat
    println!();
    println!("{}", "🔄 Indexleme başlatılıyor...".bright_yellow());
    
    let start_time = std::time::Instant::now();
    
    let index_request = IndexRequest {
        workspace_path: workspace.to_string_lossy().to_string(),
    };

    match perform_indexing(&index_request, verbose, client).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            
            println!();
            println!("{}", "✅ Indexleme Tamamlandı!".bright_green().bold());
            println!("{} {:.2}s", "⏱️ Süre:".bright_blue(), duration.as_secs_f64());
            println!();
            
            display_index_stats(&response.stats);
            
            // Başarı önerileri
            display_post_index_suggestions(&workspace, &response.stats).await?;
        }
        Err(e) => {
            println!("{} Indexleme hatası: {}", "❌".bright_red(), e);
            
            // Hata durumunda öneriler
            display_error_suggestions(&e).await?;
        }
    }

    Ok(())
}

async fn check_existing_index(workspace: &PathBuf, client: &Client) -> Result<Option<IndexStats>> {
    let workspace_path = workspace.to_string_lossy().to_string();
    let encoded_path = urlencoding::encode(&workspace_path);
    
    match client.get::<IndexResponse>(&format!("/search/stats/{}", encoded_path)).await {
        Ok(response) => Ok(Some(response.stats)),
        Err(_) => Ok(None),
    }
}

async fn analyze_workspace_before_indexing(workspace: &PathBuf) -> Result<()> {
    println!("{}", "🔍 Workspace Analizi".bright_cyan().bold());
    
    let mut file_count = 0;
    let mut language_counts = std::collections::HashMap::new();
    let mut total_size = 0u64;

    // Desteklenen dosya uzantıları
    let supported_extensions = [
        "rs", "js", "ts", "jsx", "tsx", "py", "java", "go", 
        "cpp", "cc", "cxx", "c", "h", "hpp", "cs", "php", "rb"
    ];

    if let Ok(entries) = std::fs::read_dir(workspace) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Some(extension) = entry.path().extension() {
                        let ext = extension.to_string_lossy().to_lowercase();
                        if supported_extensions.contains(&ext.as_str()) {
                            file_count += 1;
                            total_size += metadata.len();
                            *language_counts.entry(ext).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    println!("  {} {} dosya", "📄".bright_blue(), file_count);
    println!("  {} {:.2} MB", "💾".bright_blue(), total_size as f64 / 1024.0 / 1024.0);
    
    if !language_counts.is_empty() {
        println!("  {} Diller:", "🌐".bright_blue());
        for (lang, count) in language_counts {
            println!("    {} {} dosya", lang.bright_yellow(), count);
        }
    }
    
    // Tahmini süre
    let estimated_seconds = (file_count as f64 * 0.1).max(5.0);
    println!("  {} ~{:.0}s", "⏱️ Tahmini süre:".bright_blue(), estimated_seconds);
    
    println!();
    Ok(())
}

async fn perform_indexing(request: &IndexRequest, verbose: bool, client: &Client) -> Result<IndexResponse> {
    if verbose {
        println!("{}", "📊 Indexleme detayları:".bright_cyan());
        println!("  {} Dosyalar taranıyor...", "🔍".bright_blue());
        println!("  {} Semboller çıkarılıyor...", "🔧".bright_blue());
        println!("  {} AI embeddings oluşturuluyor...", "🧠".bright_blue());
        println!("  {} Veritabanına kaydediliyor...", "💾".bright_blue());
        println!();
    }

    let response: IndexResponse = client.post("/search/index", request).await?;
    Ok(response)
}

fn display_index_stats(stats: &IndexStats) {
    println!("{}", "📊 Index İstatistikleri".bright_cyan().bold());
    println!("  {} {} dosya", "📄".bright_blue(), stats.total_files);
    println!("  {} {} sembol", "🔧".bright_blue(), stats.total_symbols);
    println!("  {} {} dil", "🌐".bright_blue(), stats.languages_count);
    println!("  {} {:.1}", "📊 Ortalama complexity:".bright_blue(), stats.avg_complexity);
    
    // Performans metrikleri
    if stats.total_files > 0 {
        let symbols_per_file = stats.total_symbols as f64 / stats.total_files as f64;
        println!("  {} {:.1} sembol/dosya", "📈".bright_blue(), symbols_per_file);
    }
}

async fn display_post_index_suggestions(workspace: &PathBuf, stats: &IndexStats) -> Result<()> {
    println!("{}", "💡 Öneriler".bright_yellow().bold());
    
    // Arama önerileri
    println!("  {} Artık kod arayabilirsiniz:", "🔍".bright_blue());
    println!("    {} uaida search \"authentication functions\"", "💻".bright_cyan());
    println!("    {} uaida search \"HTTP client\" --language rust", "💻".bright_cyan());
    
    // Açıklama önerileri
    println!("  {} Kod açıklaması alabilirsiniz:", "📖".bright_blue());
    println!("    {} uaida explain src/main.rs", "💻".bright_cyan());
    println!("    {} uaida explain --function authenticate", "💻".bright_cyan());
    
    // Chat önerileri
    println!("  {} AI ile sohbet edebilirsiniz:", "💬".bright_blue());
    println!("    {} uaida chat --search \"Bu projede authentication nasıl çalışıyor?\"", "💻".bright_cyan());
    
    // Performans önerileri
    if stats.total_files > 1000 {
        println!("  {} Büyük proje için:", "⚡".bright_yellow());
        println!("    {} Arama filtrelerini kullanın (--language, --file-type)", "💡".bright_blue());
        println!("    {} Spesifik terimler kullanın", "💡".bright_blue());
    }
    
    if stats.avg_complexity > 7.0 {
        println!("  {} Yüksek complexity:", "🧠".bright_yellow());
        println!("    {} uaida explain ile karmaşık fonksiyonları anlayın", "💡".bright_blue());
        println!("    {} Refactoring önerileri alın", "💡".bright_blue());
    }
    
    println!();
    Ok(())
}

async fn display_error_suggestions(error: &anyhow::Error) -> Result<()> {
    println!();
    println!("{}", "🔧 Sorun Giderme Önerileri".bright_yellow().bold());
    
    let error_str = error.to_string().to_lowercase();
    
    if error_str.contains("permission") || error_str.contains("access") {
        println!("  {} Dosya izinlerini kontrol edin:", "🔒".bright_red());
        println!("    {} chmod -R 755 {}", "💻".bright_cyan(), "workspace_path");
        println!("    {} sudo ile çalıştırmayı deneyin", "💻".bright_cyan());
    }
    
    if error_str.contains("network") || error_str.contains("connection") {
        println!("  {} Ağ bağlantısını kontrol edin:", "🌐".bright_red());
        println!("    {} Backend servisinin çalıştığından emin olun", "💻".bright_cyan());
        println!("    {} Firewall ayarlarını kontrol edin", "💻".bright_cyan());
    }
    
    if error_str.contains("space") || error_str.contains("disk") {
        println!("  {} Disk alanını kontrol edin:", "💾".bright_red());
        println!("    {} df -h", "💻".bright_cyan());
        println!("    {} Gereksiz dosyaları temizleyin", "💻".bright_cyan());
    }
    
    if error_str.contains("timeout") {
        println!("  {} Timeout sorunu:", "⏱️".bright_red());
        println!("    {} Daha küçük workspace'leri deneyin", "💻".bright_cyan());
        println!("    {} --force ile yeniden deneyin", "💻".bright_cyan());
    }
    
    // Genel öneriler
    println!("  {} Genel çözümler:", "🛠️".bright_blue());
    println!("    {} --verbose ile detaylı log alın", "💻".bright_cyan());
    println!("    {} Backend loglarını kontrol edin", "💻".bright_cyan());
    println!("    {} Workspace yolunun doğru olduğundan emin olun", "💻".bright_cyan());
    
    println!();
    Ok(())
}

pub async fn run_interactive_index(client: &Client) -> Result<()> {
    println!("{}", "📚 İnteraktif Workspace Indexer".bright_blue().bold());
    println!();

    // Workspace seçimi
    let workspace_options = vec![
        "📁 Mevcut dizin",
        "🔍 Dizin seç",
        "📋 Son kullanılan workspace'ler",
    ];

    let workspace_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi workspace'i indexlemek istiyorsunuz?")
        .items(&workspace_options)
        .interact()?;

    let workspace = match workspace_choice {
        0 => std::env::current_dir()?,
        1 => {
            let path: String = dialoguer::Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Workspace yolu")
                .interact_text()?;
            PathBuf::from(path)
        }
        2 => {
            // TODO: Son kullanılan workspace'leri göster
            println!("{}", "Son kullanılan workspace'ler özelliği yakında eklenecek!".bright_yellow());
            std::env::current_dir()?
        }
        _ => std::env::current_dir()?,
    };

    // Seçenekler
    let force = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Mevcut index'i zorla yenile?")
        .default(false)
        .interact()?;

    let verbose = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Detaylı çıktı göster?")
        .default(true)
        .interact()?;

    // Indexleme çalıştır
    run_index(Some(workspace), force, verbose, client).await
}

pub async fn show_index_status(workspace_path: Option<PathBuf>, client: &Client) -> Result<()> {
    println!("{}", "📊 Index Durumu".bright_blue().bold());
    println!();

    let workspace = workspace_path.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    match check_existing_index(&workspace, client).await {
        Ok(Some(stats)) => {
            println!("{} {}", "📁 Workspace:".bright_blue(), workspace.display().to_string().bright_white());
            println!("{} İndexlenmiş", "✅".bright_green());
            println!();
            display_index_stats(&stats);
        }
        Ok(None) => {
            println!("{} {}", "📁 Workspace:".bright_blue(), workspace.display().to_string().bright_white());
            println!("{} İndexlenmemiş", "❌".bright_red());
            println!();
            println!("{}", "💡 Indexlemek için: uaida index".bright_yellow());
        }
        Err(e) => {
            println!("{} Index durumu kontrol edilemedi: {}", "❌".bright_red(), e);
        }
    }

    Ok(())
}