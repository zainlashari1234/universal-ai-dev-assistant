use anyhow::Result;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::fs;
use tracing::{info, debug, warn, error};
use uuid::Uuid;
use chrono::Utc;
use regex::Regex;

use super::{
    CodeIndex, IndexedSymbol, SymbolInfo, SymbolType, Parameter, Visibility,
    IndexMetadata, SymbolReference, ReferenceType, EmbeddingRequest, EmbeddingType
};
use super::embedding_manager::EmbeddingManager;

pub struct CodeIndexer {
    embedding_manager: Arc<EmbeddingManager>,
    language_parsers: HashMap<String, Box<dyn LanguageParser + Send + Sync>>,
    ignore_patterns: Vec<Regex>,
}

#[async_trait::async_trait]
pub trait LanguageParser {
    async fn parse_file(&self, content: &str, file_path: &str) -> Result<Vec<ParsedSymbol>>;
    fn get_language(&self) -> &str;
    fn get_file_extensions(&self) -> Vec<&str>;
    fn extract_imports(&self, content: &str) -> Vec<String>;
    fn extract_comments(&self, content: &str) -> Vec<String>;
    fn calculate_complexity(&self, content: &str) -> f32;
}

#[derive(Debug, Clone)]
pub struct ParsedSymbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub references: Vec<SymbolReference>,
}

impl CodeIndexer {
    pub fn new(embedding_manager: Arc<EmbeddingManager>) -> Self {
        let mut indexer = Self {
            embedding_manager,
            language_parsers: HashMap::new(),
            ignore_patterns: Self::create_ignore_patterns(),
        };
        
        // Language parser'ları kaydet
        indexer.register_parsers();
        indexer
    }

    fn register_parsers(&mut self) {
        self.language_parsers.insert("rust".to_string(), Box::new(RustParser::new()));
        self.language_parsers.insert("javascript".to_string(), Box::new(JavaScriptParser::new()));
        self.language_parsers.insert("typescript".to_string(), Box::new(TypeScriptParser::new()));
        self.language_parsers.insert("python".to_string(), Box::new(PythonParser::new()));
        self.language_parsers.insert("java".to_string(), Box::new(JavaParser::new()));
        self.language_parsers.insert("go".to_string(), Box::new(GoParser::new()));
        self.language_parsers.insert("cpp".to_string(), Box::new(CppParser::new()));
    }

    fn create_ignore_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"node_modules").unwrap(),
            Regex::new(r"target").unwrap(),
            Regex::new(r"\.git").unwrap(),
            Regex::new(r"\.vscode").unwrap(),
            Regex::new(r"\.idea").unwrap(),
            Regex::new(r"__pycache__").unwrap(),
            Regex::new(r"\.pyc$").unwrap(),
            Regex::new(r"\.o$").unwrap(),
            Regex::new(r"\.exe$").unwrap(),
            Regex::new(r"\.dll$").unwrap(),
            Regex::new(r"\.so$").unwrap(),
            Regex::new(r"build/").unwrap(),
            Regex::new(r"dist/").unwrap(),
            Regex::new(r"out/").unwrap(),
        ]
    }

    pub async fn index_workspace(&self, workspace_path: &str) -> Result<Vec<CodeIndex>> {
        info!("Starting workspace indexing: {}", workspace_path);
        
        let files = self.discover_files(workspace_path).await?;
        info!("Found {} files to index", files.len());
        
        let mut indices = Vec::new();
        let mut processed = 0;
        
        for file_path in files {
            match self.index_file(&file_path).await {
                Ok(index) => {
                    indices.push(index);
                    processed += 1;
                    
                    if processed % 100 == 0 {
                        info!("Indexed {} files", processed);
                    }
                }
                Err(e) => {
                    warn!("Failed to index file {}: {}", file_path, e);
                }
            }
        }
        
        info!("Workspace indexing completed. Indexed {} files", processed);
        Ok(indices)
    }

    pub async fn index_file(&self, file_path: &str) -> Result<CodeIndex> {
        debug!("Indexing file: {}", file_path);
        
        let content = fs::read_to_string(file_path).await?;
        let language = self.detect_language(file_path);
        
        // Content hash hesapla
        let content_hash = self.calculate_content_hash(&content);
        
        // Dosya embedding'i oluştur
        let file_embedding_request = EmbeddingRequest {
            text: content.clone(),
            context: Some(format!("File: {}", file_path)),
            embedding_type: EmbeddingType::Code,
        };
        
        let file_embedding_response = self.embedding_manager
            .generate_embedding(file_embedding_request).await?;
        
        // Sembolleri parse et
        let symbols = if let Some(parser) = self.language_parsers.get(&language) {
            self.parse_symbols_with_embeddings(parser.as_ref(), &content, file_path).await?
        } else {
            self.parse_symbols_generic(&content, file_path).await?
        };
        
        // Metadata oluştur
        let metadata = self.create_metadata(&content, &language, &symbols);
        
        Ok(CodeIndex {
            id: Uuid::new_v4(),
            file_path: file_path.to_string(),
            content_hash,
            embedding: file_embedding_response.embedding,
            symbols,
            metadata,
            indexed_at: Utc::now(),
            last_updated: Utc::now(),
        })
    }

    async fn parse_symbols_with_embeddings(
        &self,
        parser: &dyn LanguageParser,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<IndexedSymbol>> {
        let parsed_symbols = parser.parse_file(content, file_path).await?;
        let mut indexed_symbols = Vec::new();
        
        // Her sembol için embedding oluştur
        for parsed_symbol in parsed_symbols {
            let symbol_text = format!(
                "{} {} {}",
                parsed_symbol.name,
                parsed_symbol.signature.as_deref().unwrap_or(""),
                parsed_symbol.documentation.as_deref().unwrap_or("")
            );
            
            let embedding_request = EmbeddingRequest {
                text: symbol_text,
                context: Some(format!("Symbol in {}", file_path)),
                embedding_type: EmbeddingType::Symbol,
            };
            
            let embedding_response = self.embedding_manager
                .generate_embedding(embedding_request).await?;
            
            let signature_hash = self.calculate_signature_hash(&parsed_symbol);
            
            indexed_symbols.push(IndexedSymbol {
                name: parsed_symbol.name,
                symbol_type: parsed_symbol.symbol_type,
                line_start: parsed_symbol.line_start,
                line_end: parsed_symbol.line_end,
                content: parsed_symbol.content,
                embedding: embedding_response.embedding,
                signature_hash,
                references: parsed_symbol.references,
            });
        }
        
        Ok(indexed_symbols)
    }

    async fn parse_symbols_generic(&self, content: &str, file_path: &str) -> Result<Vec<IndexedSymbol>> {
        // Generic parser - basit pattern matching
        let mut symbols = Vec::new();
        
        // Function patterns
        let function_patterns = [
            r"function\s+(\w+)\s*\(",
            r"def\s+(\w+)\s*\(",
            r"fn\s+(\w+)\s*\(",
            r"public\s+\w+\s+(\w+)\s*\(",
            r"private\s+\w+\s+(\w+)\s*\(",
        ];
        
        for pattern in &function_patterns {
            let regex = Regex::new(pattern)?;
            for (line_num, line) in content.lines().enumerate() {
                if let Some(captures) = regex.captures(line) {
                    if let Some(func_name) = captures.get(1) {
                        let symbol_text = format!("function {}", func_name.as_str());
                        
                        let embedding_request = EmbeddingRequest {
                            text: symbol_text.clone(),
                            context: Some(format!("Generic symbol in {}", file_path)),
                            embedding_type: EmbeddingType::Symbol,
                        };
                        
                        let embedding_response = self.embedding_manager
                            .generate_embedding(embedding_request).await?;
                        
                        symbols.push(IndexedSymbol {
                            name: func_name.as_str().to_string(),
                            symbol_type: SymbolType::Function,
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            content: line.to_string(),
                            embedding: embedding_response.embedding,
                            signature_hash: self.calculate_simple_hash(&symbol_text),
                            references: Vec::new(),
                        });
                    }
                }
            }
        }
        
        Ok(symbols)
    }

    async fn discover_files(&self, workspace_path: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.discover_files_recursive(Path::new(workspace_path), &mut files).await?;
        Ok(files)
    }

    async fn discover_files_recursive(&self, dir: &Path, files: &mut Vec<String>) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Ignore patterns kontrolü
            let path_str = path.to_string_lossy();
            if self.ignore_patterns.iter().any(|pattern| pattern.is_match(&path_str)) {
                continue;
            }
            
            if path.is_dir() {
                self.discover_files_recursive(&path, files).await?;
            } else if self.is_supported_file(&path) {
                files.push(path.to_string_lossy().to_string());
            }
        }
        
        Ok(())
    }

    fn is_supported_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), 
                "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "go" | 
                "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" | "cs" | "php" | 
                "rb" | "swift" | "kt" | "scala" | "clj" | "hs" | "ml" | "fs"
            )
        } else {
            false
        }
    }

    fn detect_language(&self, file_path: &str) -> String {
        if let Some(extension) = Path::new(file_path).extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "rs" => "rust".to_string(),
                "js" | "jsx" => "javascript".to_string(),
                "ts" | "tsx" => "typescript".to_string(),
                "py" => "python".to_string(),
                "java" => "java".to_string(),
                "go" => "go".to_string(),
                "cpp" | "cc" | "cxx" => "cpp".to_string(),
                "c" => "c".to_string(),
                "cs" => "csharp".to_string(),
                "php" => "php".to_string(),
                "rb" => "ruby".to_string(),
                "swift" => "swift".to_string(),
                "kt" => "kotlin".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }

    fn calculate_content_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn calculate_signature_hash(&self, symbol: &ParsedSymbol) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        symbol.name.hash(&mut hasher);
        symbol.symbol_type.hash(&mut hasher);
        if let Some(sig) = &symbol.signature {
            sig.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    fn calculate_simple_hash(&self, text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn create_metadata(&self, content: &str, language: &str, symbols: &[IndexedSymbol]) -> IndexMetadata {
        let line_count = content.lines().count();
        let file_size = content.len() as u64;
        let symbol_count = symbols.len();
        
        let complexity_score = if let Some(parser) = self.language_parsers.get(language) {
            parser.calculate_complexity(content)
        } else {
            self.calculate_generic_complexity(content)
        };
        
        let quality_score = self.calculate_quality_score(content, symbols);
        let tags = self.extract_tags(content, language);
        let categories = self.categorize_file(content, language);
        
        IndexMetadata {
            language: language.to_string(),
            file_size,
            line_count,
            symbol_count,
            complexity_score,
            quality_score,
            tags,
            categories,
        }
    }

    fn calculate_generic_complexity(&self, content: &str) -> f32 {
        let lines = content.lines().count() as f32;
        let control_structures = content.matches("if ").count() + 
                               content.matches("for ").count() + 
                               content.matches("while ").count() + 
                               content.matches("switch ").count();
        
        let base_complexity = lines / 100.0;
        let control_complexity = control_structures as f32 * 0.5;
        
        (base_complexity + control_complexity).min(10.0)
    }

    fn calculate_quality_score(&self, content: &str, symbols: &[IndexedSymbol]) -> f32 {
        let mut score = 5.0; // Base score
        
        // Comment ratio
        let comment_lines = content.lines().filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*")
        }).count();
        
        let total_lines = content.lines().count();
        if total_lines > 0 {
            let comment_ratio = comment_lines as f32 / total_lines as f32;
            score += comment_ratio * 2.0; // Max +2 for good commenting
        }
        
        // Symbol documentation ratio
        let documented_symbols = symbols.iter()
            .filter(|s| !s.content.is_empty())
            .count();
        
        if !symbols.is_empty() {
            let doc_ratio = documented_symbols as f32 / symbols.len() as f32;
            score += doc_ratio * 2.0; // Max +2 for good documentation
        }
        
        // Line length penalty
        let long_lines = content.lines().filter(|line| line.len() > 120).count();
        if total_lines > 0 {
            let long_line_ratio = long_lines as f32 / total_lines as f32;
            score -= long_line_ratio * 1.0; // Max -1 for long lines
        }
        
        score.clamp(0.0, 10.0)
    }

    fn extract_tags(&self, content: &str, language: &str) -> Vec<String> {
        let mut tags = Vec::new();
        
        // Language tag
        tags.push(language.to_string());
        
        // Framework detection
        if content.contains("React") || content.contains("useState") {
            tags.push("react".to_string());
        }
        if content.contains("async") || content.contains("await") {
            tags.push("async".to_string());
        }
        if content.contains("test") || content.contains("Test") {
            tags.push("test".to_string());
        }
        if content.contains("API") || content.contains("http") {
            tags.push("api".to_string());
        }
        if content.contains("database") || content.contains("sql") {
            tags.push("database".to_string());
        }
        
        tags
    }

    fn categorize_file(&self, content: &str, language: &str) -> Vec<String> {
        let mut categories = Vec::new();
        
        // File type categories
        if content.contains("test") || content.contains("Test") || content.contains("spec") {
            categories.push("test".to_string());
        } else if content.contains("main") || content.contains("Main") {
            categories.push("entry-point".to_string());
        } else if content.contains("config") || content.contains("Config") {
            categories.push("configuration".to_string());
        } else if content.contains("util") || content.contains("helper") {
            categories.push("utility".to_string());
        } else {
            categories.push("implementation".to_string());
        }
        
        // Functionality categories
        if content.contains("http") || content.contains("request") || content.contains("response") {
            categories.push("networking".to_string());
        }
        if content.contains("database") || content.contains("sql") || content.contains("query") {
            categories.push("data-access".to_string());
        }
        if content.contains("auth") || content.contains("login") || content.contains("password") {
            categories.push("authentication".to_string());
        }
        if content.contains("ui") || content.contains("component") || content.contains("render") {
            categories.push("user-interface".to_string());
        }
        
        categories
    }

    pub async fn update_index(&self, existing_index: &CodeIndex) -> Result<CodeIndex> {
        // Dosya değişmiş mi kontrol et
        let content = fs::read_to_string(&existing_index.file_path).await?;
        let new_hash = self.calculate_content_hash(&content);
        
        if existing_index.content_hash == new_hash {
            // Değişmemiş, mevcut index'i döndür
            return Ok(existing_index.clone());
        }
        
        // Yeniden index et
        let mut new_index = self.index_file(&existing_index.file_path).await?;
        new_index.id = existing_index.id; // ID'yi koru
        new_index.indexed_at = existing_index.indexed_at; // İlk indexleme zamanını koru
        
        Ok(new_index)
    }

    pub fn should_reindex(&self, index: &CodeIndex, file_modified_time: chrono::DateTime<Utc>) -> bool {
        index.last_updated < file_modified_time
    }
}

// Language-specific parsers (simplified implementations)
pub struct RustParser;
pub struct JavaScriptParser;
pub struct TypeScriptParser;
pub struct PythonParser;
pub struct JavaParser;
pub struct GoParser;
pub struct CppParser;

impl RustParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LanguageParser for RustParser {
    async fn parse_file(&self, content: &str, _file_path: &str) -> Result<Vec<ParsedSymbol>> {
        let mut symbols = Vec::new();
        
        // Rust function parsing
        let fn_regex = Regex::new(r"(?m)^(?:\s*pub\s+)?fn\s+(\w+)\s*\((.*?)\)(?:\s*->\s*([^{]+))?\s*\{")?;
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = fn_regex.captures(line) {
                let name = captures.get(1).unwrap().as_str().to_string();
                let params_str = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                let return_type = captures.get(3).map(|m| m.as_str().trim().to_string());
                
                let visibility = if line.contains("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                
                symbols.push(ParsedSymbol {
                    name,
                    symbol_type: SymbolType::Function,
                    line_start: line_num + 1,
                    line_end: line_num + 1, // Simplified
                    content: line.to_string(),
                    signature: Some(line.trim().to_string()),
                    documentation: None, // TODO: Parse doc comments
                    parameters: self.parse_rust_parameters(params_str),
                    return_type,
                    visibility,
                    references: Vec::new(),
                });
            }
        }
        
        Ok(symbols)
    }

    fn get_language(&self) -> &str {
        "rust"
    }

    fn get_file_extensions(&self) -> Vec<&str> {
        vec!["rs"]
    }

    fn extract_imports(&self, content: &str) -> Vec<String> {
        let use_regex = Regex::new(r"use\s+([^;]+);").unwrap();
        use_regex.captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn extract_comments(&self, content: &str) -> Vec<String> {
        let comment_regex = Regex::new(r"//(.*)").unwrap();
        comment_regex.captures_iter(content)
            .map(|cap| cap[1].trim().to_string())
            .collect()
    }

    fn calculate_complexity(&self, content: &str) -> f32 {
        let control_keywords = ["if", "else", "match", "for", "while", "loop"];
        let mut complexity = 1.0;
        
        for keyword in &control_keywords {
            complexity += content.matches(keyword).count() as f32 * 0.5;
        }
        
        complexity.min(10.0)
    }
}

impl RustParser {
    fn parse_rust_parameters(&self, params_str: &str) -> Vec<Parameter> {
        if params_str.trim().is_empty() {
            return Vec::new();
        }
        
        params_str.split(',')
            .map(|param| {
                let parts: Vec<&str> = param.trim().split(':').collect();
                if parts.len() >= 2 {
                    Parameter {
                        name: parts[0].trim().to_string(),
                        param_type: parts[1].trim().to_string(),
                        default_value: None,
                        description: None,
                    }
                } else {
                    Parameter {
                        name: param.trim().to_string(),
                        param_type: "unknown".to_string(),
                        default_value: None,
                        description: None,
                    }
                }
            })
            .collect()
    }
}

// Simplified implementations for other languages
macro_rules! impl_basic_parser {
    ($parser:ident, $lang:expr, $exts:expr) => {
        impl $parser {
            pub fn new() -> Self {
                Self
            }
        }

        #[async_trait::async_trait]
        impl LanguageParser for $parser {
            async fn parse_file(&self, _content: &str, _file_path: &str) -> Result<Vec<ParsedSymbol>> {
                // TODO: Implement language-specific parsing
                Ok(Vec::new())
            }

            fn get_language(&self) -> &str {
                $lang
            }

            fn get_file_extensions(&self) -> Vec<&str> {
                $exts
            }

            fn extract_imports(&self, _content: &str) -> Vec<String> {
                Vec::new()
            }

            fn extract_comments(&self, _content: &str) -> Vec<String> {
                Vec::new()
            }

            fn calculate_complexity(&self, content: &str) -> f32 {
                content.lines().count() as f32 / 50.0
            }
        }
    };
}

impl_basic_parser!(JavaScriptParser, "javascript", vec!["js", "jsx"]);
impl_basic_parser!(TypeScriptParser, "typescript", vec!["ts", "tsx"]);
impl_basic_parser!(PythonParser, "python", vec!["py"]);
impl_basic_parser!(JavaParser, "java", vec!["java"]);
impl_basic_parser!(GoParser, "go", vec!["go"]);
impl_basic_parser!(CppParser, "cpp", vec!["cpp", "cc", "cxx", "c", "h", "hpp"]);