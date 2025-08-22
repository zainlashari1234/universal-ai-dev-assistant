use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::fs;
use tracing::{info, debug, warn};

use super::{
    WorkspaceContext, CodeContext, ConversationSession, ConversationTurn,
    OpenFile, FunctionInfo, ImportInfo, SymbolInfo, TextSelection,
    workspace_analyzer::WorkspaceAnalyzer
};

pub struct ContextManager {
    workspace_analyzer: WorkspaceAnalyzer,
    file_cache: Arc<tokio::sync::RwLock<HashMap<String, CachedFile>>>,
}

#[derive(Debug, Clone)]
struct CachedFile {
    content: String,
    last_modified: chrono::DateTime<chrono::Utc>,
    symbols: Vec<SymbolInfo>,
    functions: Vec<FunctionInfo>,
    imports: Vec<ImportInfo>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            workspace_analyzer: WorkspaceAnalyzer::new(),
            file_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_workspace_context(
        &self,
        session: &mut ConversationSession,
        workspace_path: Option<&str>,
    ) -> Result<()> {
        if let Some(path) = workspace_path {
            info!("Updating workspace context for: {}", path);
            
            let new_context = self.workspace_analyzer.analyze_workspace(path).await?;
            session.update_workspace_context(new_context);
            
            debug!("Workspace context updated for session: {}", session.id);
        }
        
        Ok(())
    }

    pub async fn update_code_context(
        &self,
        session: &mut ConversationSession,
        current_file: Option<&str>,
        selected_text: Option<TextSelection>,
        open_files: Vec<String>,
    ) -> Result<()> {
        debug!("Updating code context for session: {}", session.id);

        // Mevcut dosya bilgisini güncelle
        if let Some(file_path) = current_file {
            session.code_context.current_file = Some(file_path.to_string());
            session.add_active_file(file_path.to_string());

            // Dosya içeriğini analiz et
            if let Ok(cached_file) = self.get_or_cache_file(file_path).await {
                session.code_context.symbols = cached_file.symbols;
                session.code_context.recent_functions = cached_file.functions;
                session.code_context.imports = cached_file.imports;
            }
        }

        // Seçili metin
        session.code_context.selected_text = selected_text;

        // Açık dosyaları güncelle
        let mut open_file_objects = Vec::new();
        for file_path in open_files {
            if let Ok(open_file) = self.create_open_file_info(&file_path).await {
                open_file_objects.push(open_file);
            }
        }
        session.code_context.open_files = open_file_objects;

        Ok(())
    }

    pub async fn get_relevant_context(
        &self,
        session: &ConversationSession,
        message: &str,
        intent: &super::MessageIntent,
    ) -> Result<RelevantContext> {
        debug!("Getting relevant context for intent: {:?}", intent);

        let mut context = RelevantContext::default();

        // Intent'e göre ilgili bağlamı topla
        match intent {
            super::MessageIntent::CodeGeneration => {
                context = self.get_code_generation_context(session, message).await?;
            }
            super::MessageIntent::CodeExplanation => {
                context = self.get_code_explanation_context(session, message).await?;
            }
            super::MessageIntent::Debugging => {
                context = self.get_debugging_context(session, message).await?;
            }
            super::MessageIntent::Refactoring => {
                context = self.get_refactoring_context(session, message).await?;
            }
            super::MessageIntent::Testing => {
                context = self.get_testing_context(session, message).await?;
            }
            super::MessageIntent::FileOperation => {
                context = self.get_file_operation_context(session, message).await?;
            }
            _ => {
                context = self.get_general_context(session, message).await?;
            }
        }

        Ok(context)
    }

    async fn get_code_generation_context(
        &self,
        session: &ConversationSession,
        message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Mevcut dosya içeriği
        if let Some(current_file) = &session.code_context.current_file {
            if let Ok(content) = fs::read_to_string(current_file).await {
                context.current_file_content = Some(content);
                context.relevant_files.push(current_file.clone());
            }
        }

        // Benzer fonksiyonları bul
        context.similar_functions = self.find_similar_functions(session, message).await?;

        // İlgili import'ları ekle
        context.relevant_imports = session.code_context.imports.clone();

        // Proje yapısı bilgisi
        context.project_structure = self.get_project_structure_summary(&session.workspace_context).await?;

        Ok(context)
    }

    async fn get_code_explanation_context(
        &self,
        session: &ConversationSession,
        _message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Seçili kod varsa
        if let Some(selected_text) = &session.code_context.selected_text {
            context.selected_code = Some(selected_text.text.clone());
        }

        // Mevcut dosya içeriği
        if let Some(current_file) = &session.code_context.current_file {
            if let Ok(content) = fs::read_to_string(current_file).await {
                context.current_file_content = Some(content);
            }
        }

        // İlgili semboller
        context.relevant_symbols = session.code_context.symbols.clone();

        Ok(context)
    }

    async fn get_debugging_context(
        &self,
        session: &ConversationSession,
        message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Hata mesajından dosya adı çıkarmaya çalış
        let error_files = self.extract_file_names_from_error(message);
        for file_path in error_files {
            if let Ok(content) = fs::read_to_string(&file_path).await {
                context.error_related_files.insert(file_path, content);
            }
        }

        // Mevcut dosya
        if let Some(current_file) = &session.code_context.current_file {
            if let Ok(content) = fs::read_to_string(current_file).await {
                context.current_file_content = Some(content);
            }
        }

        // Son değişiklikler
        context.recent_changes = session.workspace_context.recent_changes.clone();

        Ok(context)
    }

    async fn get_refactoring_context(
        &self,
        session: &ConversationSession,
        _message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Seçili kod
        if let Some(selected_text) = &session.code_context.selected_text {
            context.selected_code = Some(selected_text.text.clone());
        }

        // Mevcut dosya
        if let Some(current_file) = &session.code_context.current_file {
            if let Ok(content) = fs::read_to_string(current_file).await {
                context.current_file_content = Some(content);
            }
        }

        // İlgili fonksiyonlar
        context.related_functions = session.code_context.recent_functions.clone();

        // Proje pattern'leri
        context.project_patterns = self.analyze_project_patterns(&session.workspace_context).await?;

        Ok(context)
    }

    async fn get_testing_context(
        &self,
        session: &ConversationSession,
        _message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Test edilecek kod
        if let Some(current_file) = &session.code_context.current_file {
            if let Ok(content) = fs::read_to_string(current_file).await {
                context.current_file_content = Some(content);
            }
        }

        // Mevcut test dosyalarını bul
        context.existing_tests = self.find_existing_tests(&session.workspace_context).await?;

        // Test framework bilgisi
        context.test_framework = self.detect_test_framework(&session.workspace_context);

        Ok(context)
    }

    async fn get_file_operation_context(
        &self,
        session: &ConversationSession,
        message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Mesajdan dosya adlarını çıkar
        let mentioned_files = self.extract_file_names_from_message(message);
        for file_path in mentioned_files {
            context.relevant_files.push(file_path);
        }

        // Mevcut dizin yapısı
        context.directory_structure = self.get_directory_structure(&session.workspace_context.root_path).await?;

        Ok(context)
    }

    async fn get_general_context(
        &self,
        session: &ConversationSession,
        _message: &str,
    ) -> Result<RelevantContext> {
        let mut context = RelevantContext::default();

        // Genel proje bilgisi
        context.project_structure = self.get_project_structure_summary(&session.workspace_context).await?;

        // Son konuşma bağlamı
        if let Some(last_turn) = session.conversation_history.last() {
            context.last_conversation = Some(last_turn.clone());
        }

        Ok(context)
    }

    async fn get_or_cache_file(&self, file_path: &str) -> Result<CachedFile> {
        let cache = self.file_cache.read().await;
        
        // Cache'de var mı kontrol et
        if let Some(cached) = cache.get(file_path) {
            // Dosya değişmiş mi kontrol et
            if let Ok(metadata) = fs::metadata(file_path).await {
                let modified = chrono::DateTime::from(metadata.modified()?);
                if modified <= cached.last_modified {
                    return Ok(cached.clone());
                }
            }
        }
        
        drop(cache);

        // Dosyayı yeniden analiz et
        let content = fs::read_to_string(file_path).await?;
        let symbols = self.workspace_analyzer.analyze_file_symbols(file_path).await?;
        let functions = self.extract_functions(&content, file_path).await?;
        let imports = self.extract_imports(&content, file_path).await?;

        let cached_file = CachedFile {
            content,
            last_modified: chrono::Utc::now(),
            symbols,
            functions,
            imports,
        };

        // Cache'e ekle
        let mut cache = self.file_cache.write().await;
        cache.insert(file_path.to_string(), cached_file.clone());

        Ok(cached_file)
    }

    async fn create_open_file_info(&self, file_path: &str) -> Result<OpenFile> {
        let content = fs::read_to_string(file_path).await?;
        let metadata = fs::metadata(file_path).await?;
        let modified = chrono::DateTime::from(metadata.modified()?);

        let language = self.detect_file_language(file_path);
        let preview = if content.len() > 500 {
            format!("{}...", &content[..500])
        } else {
            content
        };

        Ok(OpenFile {
            path: file_path.to_string(),
            language,
            content_preview: preview,
            last_modified: modified,
            is_dirty: false, // Bu bilgiyi IDE'den alacağız
        })
    }

    async fn find_similar_functions(
        &self,
        session: &ConversationSession,
        message: &str,
    ) -> Result<Vec<FunctionInfo>> {
        // Mesajdan fonksiyon tipini tahmin et
        let keywords = self.extract_keywords_from_message(message);
        
        let mut similar_functions = Vec::new();
        for func in &session.code_context.recent_functions {
            for keyword in &keywords {
                if func.name.to_lowercase().contains(&keyword.to_lowercase()) ||
                   func.signature.to_lowercase().contains(&keyword.to_lowercase()) {
                    similar_functions.push(func.clone());
                    break;
                }
            }
        }

        Ok(similar_functions)
    }

    async fn get_project_structure_summary(&self, workspace_context: &WorkspaceContext) -> Result<String> {
        let mut summary = Vec::new();
        
        summary.push(format!("Proje tipi: {:?}", workspace_context.project_type));
        summary.push(format!("Build sistem: {:?}", workspace_context.build_system));
        summary.push(format!("Ana dosyalar: {:?}", workspace_context.main_files));
        summary.push(format!("Bağımlılık sayısı: {}", workspace_context.dependencies.len()));

        if let Some(git_info) = &workspace_context.git_info {
            summary.push(format!("Git branch: {}", git_info.branch));
            summary.push(format!("Değiştirilmiş dosyalar: {}", git_info.modified_files.len()));
        }

        Ok(summary.join("\n"))
    }

    async fn extract_functions(&self, content: &str, file_path: &str) -> Result<Vec<FunctionInfo>> {
        // Basit function extraction (gerçek projede tree-sitter kullanılmalı)
        let mut functions = Vec::new();
        
        if file_path.ends_with(".rs") {
            // Rust functions
            for (line_num, line) in content.lines().enumerate() {
                if line.trim().starts_with("fn ") || line.trim().starts_with("pub fn ") {
                    if let Some(func_name) = self.extract_rust_function_name(line) {
                        functions.push(FunctionInfo {
                            name: func_name,
                            file_path: file_path.to_string(),
                            line_number: line_num + 1,
                            signature: line.trim().to_string(),
                            doc_string: None,
                            complexity_score: 1.0, // Basit hesaplama
                        });
                    }
                }
            }
        }

        Ok(functions)
    }

    async fn extract_imports(&self, content: &str, file_path: &str) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();

        if file_path.ends_with(".rs") {
            // Rust imports
            for line in content.lines() {
                if line.trim().starts_with("use ") {
                    if let Some(import) = self.parse_rust_import(line) {
                        imports.push(ImportInfo {
                            module: import,
                            alias: None,
                            items: Vec::new(),
                            file_path: file_path.to_string(),
                        });
                    }
                }
            }
        }

        Ok(imports)
    }

    fn extract_rust_function_name(&self, line: &str) -> Option<String> {
        let line = line.trim();
        if let Some(start) = line.find("fn ") {
            let after_fn = &line[start + 3..];
            if let Some(end) = after_fn.find('(') {
                return Some(after_fn[..end].trim().to_string());
            }
        }
        None
    }

    fn parse_rust_import(&self, line: &str) -> Option<String> {
        let line = line.trim();
        if let Some(start) = line.find("use ") {
            let after_use = &line[start + 4..];
            if let Some(end) = after_use.find(';') {
                return Some(after_use[..end].trim().to_string());
            }
        }
        None
    }

    fn extract_file_names_from_error(&self, error_message: &str) -> Vec<String> {
        // Basit file path extraction
        let mut files = Vec::new();
        
        // Common patterns: "file.rs:line:col" or "at file.rs"
        let patterns = [
            regex::Regex::new(r"(\w+\.\w+):\d+:\d+").unwrap(),
            regex::Regex::new(r"at (\w+\.\w+)").unwrap(),
            regex::Regex::new(r"in (\w+\.\w+)").unwrap(),
        ];

        for pattern in &patterns {
            for cap in pattern.captures_iter(error_message) {
                if let Some(file) = cap.get(1) {
                    files.push(file.as_str().to_string());
                }
            }
        }

        files
    }

    fn extract_file_names_from_message(&self, message: &str) -> Vec<String> {
        // Mesajdan dosya adlarını çıkar
        let file_pattern = regex::Regex::new(r"\b\w+\.\w+\b").unwrap();
        file_pattern
            .find_iter(message)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    fn extract_keywords_from_message(&self, message: &str) -> Vec<String> {
        // Önemli kelimeleri çıkar
        let words: Vec<String> = message
            .split_whitespace()
            .filter(|word| word.len() > 3)
            .map(|word| word.to_lowercase())
            .collect();
        
        words
    }

    async fn analyze_project_patterns(&self, workspace_context: &WorkspaceContext) -> Result<Vec<String>> {
        // Proje pattern'lerini analiz et
        let mut patterns = Vec::new();
        
        if let Some(project_type) = &workspace_context.project_type {
            match project_type.as_str() {
                "rust" => {
                    patterns.push("Rust ownership patterns".to_string());
                    patterns.push("Error handling with Result<T, E>".to_string());
                }
                "node" => {
                    patterns.push("Async/await patterns".to_string());
                    patterns.push("Promise-based APIs".to_string());
                }
                _ => {}
            }
        }

        Ok(patterns)
    }

    async fn find_existing_tests(&self, workspace_context: &WorkspaceContext) -> Result<Vec<String>> {
        // Mevcut test dosyalarını bul
        let mut test_files = Vec::new();
        
        // Common test directories and patterns
        let test_patterns = ["test", "tests", "__tests__", "spec"];
        
        for pattern in &test_patterns {
            let test_dir = std::path::Path::new(&workspace_context.root_path).join(pattern);
            if test_dir.exists() {
                test_files.push(format!("{}/", pattern));
            }
        }

        Ok(test_files)
    }

    fn detect_test_framework(&self, workspace_context: &WorkspaceContext) -> Option<String> {
        for dep in &workspace_context.dependencies {
            match dep.name.as_str() {
                "jest" => return Some("jest".to_string()),
                "mocha" => return Some("mocha".to_string()),
                "pytest" => return Some("pytest".to_string()),
                "junit" => return Some("junit".to_string()),
                _ => {}
            }
        }

        // Default frameworks by project type
        match workspace_context.project_type.as_deref() {
            Some("rust") => Some("cargo test".to_string()),
            Some("node") => Some("jest".to_string()),
            Some("python") => Some("pytest".to_string()),
            Some("java") => Some("junit".to_string()),
            _ => None,
        }
    }

    async fn get_directory_structure(&self, root_path: &str) -> Result<Vec<String>> {
        let mut structure = Vec::new();
        
        if let Ok(mut entries) = fs::read_dir(root_path).await {
            while let Some(entry) = entries.next_entry().await? {
                if let Some(name) = entry.file_name().to_str() {
                    if entry.file_type().await?.is_dir() {
                        structure.push(format!("{}/", name));
                    } else {
                        structure.push(name.to_string());
                    }
                }
            }
        }

        structure.sort();
        Ok(structure)
    }

    fn detect_file_language(&self, file_path: &str) -> String {
        if let Some(extension) = std::path::Path::new(file_path).extension() {
            match extension.to_str() {
                Some("rs") => "rust".to_string(),
                Some("js") => "javascript".to_string(),
                Some("ts") => "typescript".to_string(),
                Some("py") => "python".to_string(),
                Some("java") => "java".to_string(),
                Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
                Some("c") => "c".to_string(),
                Some("go") => "go".to_string(),
                Some("php") => "php".to_string(),
                Some("rb") => "ruby".to_string(),
                Some("html") => "html".to_string(),
                Some("css") => "css".to_string(),
                Some("json") => "json".to_string(),
                Some("yaml") | Some("yml") => "yaml".to_string(),
                Some("toml") => "toml".to_string(),
                Some("md") => "markdown".to_string(),
                _ => "text".to_string(),
            }
        } else {
            "text".to_string()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RelevantContext {
    pub current_file_content: Option<String>,
    pub selected_code: Option<String>,
    pub relevant_files: Vec<String>,
    pub similar_functions: Vec<FunctionInfo>,
    pub relevant_imports: Vec<ImportInfo>,
    pub relevant_symbols: Vec<SymbolInfo>,
    pub project_structure: String,
    pub error_related_files: HashMap<String, String>,
    pub recent_changes: Vec<super::FileChange>,
    pub related_functions: Vec<FunctionInfo>,
    pub project_patterns: Vec<String>,
    pub existing_tests: Vec<String>,
    pub test_framework: Option<String>,
    pub directory_structure: Vec<String>,
    pub last_conversation: Option<ConversationTurn>,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}