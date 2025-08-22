use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select, Confirm, MultiSelect};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub query_type: Option<String>,
    pub workspace_paths: Vec<String>,
    pub file_filters: Vec<FileFilter>,
    pub language_filters: Vec<String>,
    pub max_results: Option<usize>,
    pub similarity_threshold: Option<f32>,
    pub include_context: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileFilter {
    pub pattern: String,
    pub include: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub success: bool,
    pub response: SearchResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub results: Vec<CodeResult>,
    pub total_matches: usize,
    pub search_time_ms: u64,
    pub suggestions: Vec<SearchSuggestion>,
    pub related_queries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeResult {
    pub id: String,
    pub file_path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub relevance_score: f32,
    pub match_type: String,
    pub language: String,
    pub symbol_info: Option<SymbolInfo>,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: String,
    pub signature: Option<String>,
    pub complexity_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Highlight {
    pub start_offset: usize,
    pub end_offset: usize,
    pub highlight_type: String,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub suggestion: String,
    pub suggestion_type: String,
    pub confidence: f32,
    pub reason: String,
}

pub async fn run_search(
    query: String,
    workspace_path: Option<PathBuf>,
    language: Option<String>,
    file_type: Option<String>,
    max_results: Option<usize>,
    interactive: bool,
    client: &Client,
) -> Result<()> {
    println!("{}", "🔍 AI Kod Arama".bright_blue().bold());
    println!();

    let workspace_paths = if let Some(path) = workspace_path {
        vec![path.to_string_lossy().to_string()]
    } else {
        vec![std::env::current_dir()?.to_string_lossy().to_string()]
    };

    if interactive {
        run_interactive_search(client, workspace_paths).await
    } else {
        run_single_search(query, workspace_paths, language, file_type, max_results, client).await
    }
}

async fn run_interactive_search(client: &Client, workspace_paths: Vec<String>) -> Result<()> {
    println!("{}", "🎯 İnteraktif Arama Modu".bright_green().bold());
    println!("{}", "Çıkmak için 'exit' yazın, yardım için 'help' yazın".dimmed());
    println!();

    loop {
        // Ana sorgu al
        let query: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("🔍 Arama sorgusu")
            .allow_empty(false)
            .interact_text()?;

        if query.trim() == "exit" {
            println!("{}", "👋 Arama tamamlandı!".bright_blue());
            break;
        }

        if query.trim() == "help" {
            print_search_help();
            continue;
        }

        // Arama tipini belirle
        let search_type = determine_search_type(&query);
        println!("{} {}", "🎯 Tespit edilen arama tipi:".bright_cyan(), search_type.bright_white());

        // Gelişmiş seçenekler sor
        let use_advanced = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Gelişmiş arama seçeneklerini kullanmak istiyor musunuz?")
            .default(false)
            .interact()?;

        let mut request = SearchRequest {
            query: query.clone(),
            query_type: Some(search_type),
            workspace_paths: workspace_paths.clone(),
            file_filters: Vec::new(),
            language_filters: Vec::new(),
            max_results: Some(20),
            similarity_threshold: Some(0.7),
            include_context: Some(true),
        };

        if use_advanced {
            configure_advanced_options(&mut request).await?;
        }

        // Arama yap
        match perform_search(&request, client).await {
            Ok(response) => {
                display_search_results(&response, &query).await?;
                
                // Follow-up actions
                if !response.response.results.is_empty() {
                    handle_search_actions(&response, client).await?;
                }
            }
            Err(e) => {
                println!("{} {}", "❌ Arama hatası:".bright_red(), e);
            }
        }

        println!();
    }

    Ok(())
}

async fn run_single_search(
    query: String,
    workspace_paths: Vec<String>,
    language: Option<String>,
    file_type: Option<String>,
    max_results: Option<usize>,
    client: &Client,
) -> Result<()> {
    let mut request = SearchRequest {
        query: query.clone(),
        query_type: Some(determine_search_type(&query)),
        workspace_paths,
        file_filters: Vec::new(),
        language_filters: language.map(|l| vec![l]).unwrap_or_default(),
        max_results: max_results.or(Some(10)),
        similarity_threshold: Some(0.7),
        include_context: Some(true),
    };

    // File type filter ekle
    if let Some(ft) = file_type {
        request.file_filters.push(FileFilter {
            pattern: format!("*.{}", ft),
            include: true,
        });
    }

    println!("{} {}", "🔍 Aranan:".bright_blue(), query.bright_white());
    println!("{} {}", "📁 Workspace:".bright_blue(), request.workspace_paths.join(", ").dimmed());
    
    if !request.language_filters.is_empty() {
        println!("{} {}", "🌐 Diller:".bright_blue(), request.language_filters.join(", ").bright_yellow());
    }
    
    println!();

    match perform_search(&request, client).await {
        Ok(response) => {
            display_search_results(&response, &query).await?;
        }
        Err(e) => {
            println!("{} {}", "❌ Arama hatası:".bright_red(), e);
        }
    }

    Ok(())
}

fn determine_search_type(query: &str) -> String {
    let query_lower = query.to_lowercase();
    
    if query_lower.contains("similar") || query_lower.contains("benzer") {
        "semantic".to_string()
    } else if query_lower.contains("function") || query_lower.contains("fonksiyon") || query_lower.contains("method") {
        "symbol_name".to_string()
    } else if query_lower.contains("error") || query_lower.contains("hata") || query_lower.contains("exception") {
        "error_message".to_string()
    } else if query_lower.contains("doc") || query_lower.contains("readme") || query_lower.contains("documentation") {
        "documentation".to_string()
    } else if query.contains("(") || query.contains("->") || query.contains("fn ") {
        "function_signature".to_string()
    } else if query.contains("class ") || query.contains("struct ") || query.contains("interface ") {
        "code_pattern".to_string()
    } else {
        "natural_language".to_string()
    }
}

async fn configure_advanced_options(request: &mut SearchRequest) -> Result<()> {
    println!("{}", "⚙️  Gelişmiş Arama Seçenekleri".bright_cyan().bold());

    // Dil seçimi
    let languages = vec![
        "rust", "javascript", "typescript", "python", "java", "go", "cpp", "c", "csharp", "php", "ruby"
    ];
    
    let selected_languages = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi dillerde arama yapmak istiyorsunuz? (boş bırakırsanız tümü)")
        .items(&languages)
        .interact_opt()?;

    if let Some(indices) = selected_languages {
        request.language_filters = indices.into_iter()
            .map(|i| languages[i].to_string())
            .collect();
    }

    // Dosya filtreleri
    let add_file_filters = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Dosya filtreleri eklemek istiyor musunuz?")
        .default(false)
        .interact()?;

    if add_file_filters {
        loop {
            let pattern: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Dosya pattern'i (örn: *.rs, test/*, boş bırakırsanız çıkar)")
                .allow_empty(true)
                .interact_text()?;

            if pattern.trim().is_empty() {
                break;
            }

            let include = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Bu pattern'i dahil et (hayır = hariç tut)")
                .default(true)
                .interact()?;

            request.file_filters.push(FileFilter {
                pattern,
                include,
            });
        }
    }

    // Sonuç sayısı
    let max_results: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Maksimum sonuç sayısı")
        .default("20".to_string())
        .interact_text()?;

    if let Ok(num) = max_results.parse::<usize>() {
        request.max_results = Some(num);
    }

    // Similarity threshold
    let threshold: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Benzerlik eşiği (0.0-1.0)")
        .default("0.7".to_string())
        .interact_text()?;

    if let Ok(t) = threshold.parse::<f32>() {
        request.similarity_threshold = Some(t.clamp(0.0, 1.0));
    }

    Ok(())
}

async fn perform_search(request: &SearchRequest, client: &Client) -> Result<SearchResponse> {
    println!("{}", "🔄 Arama yapılıyor...".bright_yellow());
    
    let response: SearchResponse = client.post("/search", request).await?;
    Ok(response)
}

async fn display_search_results(response: &SearchResponse, query: &str) -> Result<()> {
    let results = &response.response;
    
    println!("{}", "📊 Arama Sonuçları".bright_green().bold());
    println!("{} {} {} {}ms", 
        format!("🔍 Sorgu: {}", query).bright_white(),
        format!("📈 {} sonuç", results.total_matches).bright_cyan(),
        format!("⏱️").bright_yellow(),
        results.search_time_ms
    );
    println!();

    if results.results.is_empty() {
        println!("{}", "😔 Sonuç bulunamadı.".bright_yellow());
        
        if !results.suggestions.is_empty() {
            println!();
            println!("{}", "💡 Öneriler:".bright_blue().bold());
            for suggestion in &results.suggestions {
                println!("  {} {}", "•".bright_blue(), suggestion.suggestion.bright_white());
                println!("    {}", suggestion.reason.dimmed());
            }
        }
        
        if !results.related_queries.is_empty() {
            println!();
            println!("{}", "🔗 İlgili aramalar:".bright_blue().bold());
            for related in &results.related_queries {
                println!("  {} {}", "•".bright_blue(), related.bright_white());
            }
        }
        
        return Ok(());
    }

    // Sonuçları göster
    for (i, result) in results.results.iter().enumerate() {
        display_single_result(i + 1, result)?;
        
        if i < results.results.len() - 1 {
            println!("{}", "─".repeat(80).dimmed());
        }
    }

    // Özet bilgi
    println!();
    println!("{}", "📈 Özet".bright_cyan().bold());
    println!("  {} {}", "📁 Toplam dosya:".bright_blue(), 
        results.results.iter().map(|r| &r.file_path).collect::<std::collections::HashSet<_>>().len());
    
    let avg_relevance = results.results.iter().map(|r| r.relevance_score).sum::<f32>() / results.results.len() as f32;
    println!("  {} {:.2}", "⭐ Ortalama relevance:".bright_blue(), avg_relevance);
    
    let languages: std::collections::HashSet<_> = results.results.iter().map(|r| &r.language).collect();
    println!("  {} {}", "🌐 Diller:".bright_blue(), languages.into_iter().collect::<Vec<_>>().join(", "));

    Ok(())
}

fn display_single_result(index: usize, result: &CodeResult) -> Result<()> {
    // Header
    println!("{} {} {}", 
        format!("{}.", index).bright_cyan().bold(),
        result.file_path.bright_green(),
        format!("({}:{})", result.start_line, result.end_line).dimmed()
    );

    // Symbol info
    if let Some(symbol) = &result.symbol_info {
        println!("  {} {} {} {}", 
            "🔧".bright_blue(),
            symbol.symbol_type.bright_yellow(),
            symbol.name.bright_white().bold(),
            format!("(complexity: {:.1})", symbol.complexity_score).dimmed()
        );
        
        if let Some(signature) = &symbol.signature {
            println!("  {} {}", "📝".bright_blue(), signature.dimmed());
        }
    }

    // Match info
    let match_icon = match result.match_type.as_str() {
        "ExactMatch" => "🎯",
        "SemanticMatch" => "🧠",
        "PatternMatch" => "🔍",
        "FuzzyMatch" => "🌟",
        _ => "📄",
    };
    
    println!("  {} {} {} {} {}", 
        match_icon,
        result.match_type.bright_magenta(),
        format!("({:.2})", result.relevance_score).bright_cyan(),
        result.language.bright_yellow(),
        format!("📏 {} chars", result.content.len()).dimmed()
    );

    // Content preview
    let content_preview = if result.content.len() > 200 {
        format!("{}...", &result.content[..200])
    } else {
        result.content.clone()
    };
    
    // Syntax highlighting (basit)
    let highlighted_content = apply_simple_highlighting(&content_preview, &result.language);
    println!();
    println!("{}", highlighted_content);
    
    // Highlights
    if !result.highlights.is_empty() {
        println!();
        println!("  {} {}", "🎨 Vurgular:".bright_blue(), 
            result.highlights.iter()
                .map(|h| format!("{} ({})", h.highlight_type, h.start_offset))
                .collect::<Vec<_>>()
                .join(", ")
                .dimmed()
        );
    }

    println!();
    Ok(())
}

fn apply_simple_highlighting(content: &str, language: &str) -> String {
    match language {
        "rust" => {
            content
                .replace("fn ", &format!("{} ", "fn".bright_blue()))
                .replace("pub ", &format!("{} ", "pub".bright_magenta()))
                .replace("async ", &format!("{} ", "async".bright_cyan()))
                .replace("await", &"await".bright_cyan().to_string())
        }
        "javascript" | "typescript" => {
            content
                .replace("function ", &format!("{} ", "function".bright_blue()))
                .replace("const ", &format!("{} ", "const".bright_magenta()))
                .replace("async ", &format!("{} ", "async".bright_cyan()))
                .replace("await ", &format!("{} ", "await".bright_cyan()))
        }
        "python" => {
            content
                .replace("def ", &format!("{} ", "def".bright_blue()))
                .replace("class ", &format!("{} ", "class".bright_magenta()))
                .replace("async ", &format!("{} ", "async".bright_cyan()))
                .replace("await ", &format!("{} ", "await".bright_cyan()))
        }
        _ => content.to_string(),
    }
}

async fn handle_search_actions(response: &SearchResponse, client: &Client) -> Result<()> {
    let actions = vec![
        "📖 Sonucu detaylı incele",
        "💬 AI ile bu kod hakkında konuş", 
        "🔍 Benzer kod ara",
        "📁 Dosyayı aç",
        "⭐ Geri bildirim ver",
        "❌ Hiçbiri",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Ne yapmak istiyorsunuz?")
        .items(&actions)
        .default(5)
        .interact_opt()?;

    if let Some(choice) = selection {
        match choice {
            0 => handle_detailed_view(response).await?,
            1 => handle_ai_chat(response, client).await?,
            2 => handle_similar_search(response, client).await?,
            3 => handle_open_file(response).await?,
            4 => handle_feedback(response, client).await?,
            _ => {}
        }
    }

    Ok(())
}

async fn handle_detailed_view(response: &SearchResponse) -> Result<()> {
    if response.response.results.is_empty() {
        return Ok(());
    }

    let file_options: Vec<String> = response.response.results.iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {} ({})", i + 1, r.file_path, r.symbol_info.as_ref().map(|s| &s.name).unwrap_or(&"file".to_string())))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi sonucu detaylı görmek istiyorsunuz?")
        .items(&file_options)
        .interact_opt()?;

    if let Some(idx) = selection {
        let result = &response.response.results[idx];
        
        println!();
        println!("{}", "📖 Detaylı Görünüm".bright_blue().bold());
        println!("{}", "=".repeat(60).bright_blue());
        
        println!("{} {}", "📁 Dosya:".bright_cyan(), result.file_path.bright_white());
        println!("{} {}:{}", "📍 Satır:".bright_cyan(), result.start_line, result.end_line);
        println!("{} {}", "🌐 Dil:".bright_cyan(), result.language.bright_yellow());
        println!("{} {:.2}", "⭐ Relevance:".bright_cyan(), result.relevance_score);
        println!("{} {}", "🎯 Match Type:".bright_cyan(), result.match_type.bright_magenta());
        
        if let Some(symbol) = &result.symbol_info {
            println!();
            println!("{}", "🔧 Sembol Bilgisi".bright_blue().bold());
            println!("{} {}", "📛 İsim:".bright_cyan(), symbol.name.bright_white().bold());
            println!("{} {}", "🏷️  Tip:".bright_cyan(), symbol.symbol_type.bright_yellow());
            println!("{} {:.1}", "📊 Complexity:".bright_cyan(), symbol.complexity_score);
            
            if let Some(signature) = &symbol.signature {
                println!("{} {}", "📝 Signature:".bright_cyan(), signature.dimmed());
            }
        }
        
        println!();
        println!("{}", "📄 Kod İçeriği".bright_blue().bold());
        println!("{}", "-".repeat(40).dimmed());
        println!("{}", apply_simple_highlighting(&result.content, &result.language));
        
        if !result.highlights.is_empty() {
            println!();
            println!("{}", "🎨 Vurgular".bright_blue().bold());
            for highlight in &result.highlights {
                println!("  {} {} {}", 
                    "•".bright_blue(),
                    highlight.highlight_type.bright_yellow(),
                    highlight.explanation.as_deref().unwrap_or("").dimmed()
                );
            }
        }
    }

    Ok(())
}

async fn handle_ai_chat(response: &SearchResponse, client: &Client) -> Result<()> {
    println!("{}", "💬 AI Chat özelliği yakında eklenecek!".bright_yellow());
    // TODO: Conversation API ile entegrasyon
    Ok(())
}

async fn handle_similar_search(response: &SearchResponse, client: &Client) -> Result<()> {
    if response.response.results.is_empty() {
        return Ok(());
    }

    let file_options: Vec<String> = response.response.results.iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {}", i + 1, r.file_path))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi koda benzer kod aramak istiyorsunuz?")
        .items(&file_options)
        .interact_opt()?;

    if let Some(idx) = selection {
        let result = &response.response.results[idx];
        
        println!("{} {}", "🔍 Benzer kod aranıyor:".bright_blue(), result.file_path.bright_white());
        
        // Similar search API call
        let similar_request = serde_json::json!({
            "code_snippet": result.content,
            "workspace_paths": [std::env::current_dir()?.to_string_lossy()]
        });

        match client.post::<serde_json::Value, _>("/search/similar", &similar_request).await {
            Ok(similar_response) => {
                println!("{}", "✅ Benzer kodlar bulundu!".bright_green());
                // TODO: Display similar results
            }
            Err(e) => {
                println!("{} {}", "❌ Benzer kod arama hatası:".bright_red(), e);
            }
        }
    }

    Ok(())
}

async fn handle_open_file(response: &SearchResponse) -> Result<()> {
    if response.response.results.is_empty() {
        return Ok(());
    }

    let file_options: Vec<String> = response.response.results.iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {}", i + 1, r.file_path))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Hangi dosyayı açmak istiyorsunuz?")
        .items(&file_options)
        .interact_opt()?;

    if let Some(idx) = selection {
        let result = &response.response.results[idx];
        
        // Try to open with default editor
        let editors = ["code", "vim", "nano", "gedit"];
        
        for editor in &editors {
            if std::process::Command::new("which")
                .arg(editor)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
            {
                println!("{} {} ile açılıyor: {}", "📝".bright_blue(), editor, result.file_path.bright_white());
                
                let mut cmd = std::process::Command::new(editor);
                cmd.arg(&result.file_path);
                
                // VSCode için satır numarası ekle
                if *editor == "code" {
                    cmd.arg("--goto").arg(format!("{}:{}", result.file_path, result.start_line));
                }
                
                match cmd.spawn() {
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
        println!("{}", result.file_path.bright_white());
    }

    Ok(())
}

async fn handle_feedback(response: &SearchResponse, client: &Client) -> Result<()> {
    println!("{}", "⭐ Geri Bildirim".bright_blue().bold());
    
    let satisfaction_options = vec![
        "😍 Mükemmel - tam istediğim",
        "😊 İyi - faydalı sonuçlar", 
        "😐 Orta - bazı sonuçlar faydalı",
        "😞 Kötü - istediğimi bulamadım",
        "😡 Çok kötü - hiç faydalı değil",
    ];

    let satisfaction = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Arama sonuçlarından ne kadar memnunsunuz?")
        .items(&satisfaction_options)
        .interact_opt()?;

    if let Some(score_idx) = satisfaction {
        let satisfaction_score = (5 - score_idx) as f32; // 5=mükemmel, 1=çok kötü
        
        let comments: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Ek yorumlarınız (opsiyonel)")
            .allow_empty(true)
            .interact_text()?;

        let feedback_request = serde_json::json!({
            "search_id": "temp-id", // TODO: Get actual search ID
            "feedback_type": "helpful",
            "satisfaction_score": satisfaction_score,
            "comments": if comments.is_empty() { None } else { Some(comments) }
        });

        match client.post::<serde_json::Value, _>("/search/feedback", &feedback_request).await {
            Ok(_) => {
                println!("{}", "✅ Geri bildiriminiz kaydedildi. Teşekkürler!".bright_green());
            }
            Err(e) => {
                println!("{} {}", "❌ Geri bildirim kaydedilemedi:".bright_red(), e);
            }
        }
    }

    Ok(())
}

fn print_search_help() {
    println!("{}", "💡 Arama Yardımı".bright_yellow().bold());
    println!();
    println!("{}", "🔍 Arama Tipleri:".bright_cyan().bold());
    println!("  {} - Doğal dil: 'HTTP client authentication'", "•".bright_blue());
    println!("  {} - Fonksiyon: 'authenticate function'", "•".bright_blue());
    println!("  {} - Hata: 'error borrow of moved value'", "•".bright_blue());
    println!("  {} - Benzer kod: 'similar to this function'", "•".bright_blue());
    println!("  {} - Dokümantasyon: 'README authentication'", "•".bright_blue());
    println!();
    println!("{}", "⚙️ Gelişmiş Özellikler:".bright_cyan().bold());
    println!("  {} - Dil filtreleme (rust, javascript, python...)", "•".bright_blue());
    println!("  {} - Dosya pattern'leri (*.rs, test/*, src/*)", "•".bright_blue());
    println!("  {} - Benzerlik eşiği ayarlama", "•".bright_blue());
    println!("  {} - Sonuç sayısı sınırlama", "•".bright_blue());
    println!();
    println!("{}", "🎯 İpuçları:".bright_cyan().bold());
    println!("  {} - Spesifik terimler kullanın", "•".bright_blue());
    println!("  {} - Kod parçacıkları ekleyin", "•".bright_blue());
    println!("  {} - Hata mesajlarını tam olarak yazın", "•".bright_blue());
    println!("  {} - Benzer kod için 'similar' kelimesini kullanın", "•".bright_blue());
    println!();
}