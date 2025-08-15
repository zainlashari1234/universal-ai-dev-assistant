pub mod repo_scan;
pub mod ast_graph;
pub mod embeddings;
pub mod selection;

pub use repo_scan::*;
pub use ast_graph::*;
pub use embeddings::*;
pub use selection::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub files: Vec<FileContext>,
    pub spans: Vec<CodeSpan>,
    pub symbols: Vec<Symbol>,
    pub related_tests: Vec<PathBuf>,
    pub total_tokens: usize,
    pub selection_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: PathBuf,
    pub content: String,
    pub language: String,
    pub relevance_score: f32,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSpan {
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub span_type: SpanType,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanType {
    Function,
    Class,
    Method,
    Import,
    Comment,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub scope: String,
    pub references: Vec<SymbolReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Variable,
    Constant,
    Module,
    Interface,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub reference_type: ReferenceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Definition,
    Usage,
    Call,
    Import,
}

pub struct ContextManager {
    repo_scanner: RepoScanner,
    ast_analyzer: AstAnalyzer,
    embedding_store: EmbeddingStore,
    selector: ContextSelector,
}

impl ContextManager {
    pub fn new(repo_path: PathBuf) -> Result<Self> {
        let repo_scanner = RepoScanner::new(repo_path.clone())?;
        let ast_analyzer = AstAnalyzer::new()?;
        let embedding_store = EmbeddingStore::new(repo_path.join(".uaida/embeddings"))?;
        let selector = ContextSelector::new();

        Ok(Self {
            repo_scanner,
            ast_analyzer,
            embedding_store,
            selector,
        })
    }

    pub async fn scan_repository(&mut self) -> Result<()> {
        // Scan files
        let files = self.repo_scanner.scan().await?;
        
        // Analyze ASTs and extract symbols
        for file in &files {
            if let Ok(symbols) = self.ast_analyzer.analyze_file(&file.path).await {
                // Store symbols for later use
                self.embedding_store.store_symbols(&file.path, &symbols).await?;
            }
        }

        // Generate embeddings
        self.embedding_store.generate_embeddings(&files).await?;

        Ok(())
    }

    pub async fn get_context(&self, query: &str, language: &str, max_tokens: usize) -> Result<ContextPackage> {
        // Find relevant files using embeddings
        let relevant_files = self.embedding_store.search_similar(query, 10).await?;
        
        // Get symbols and spans
        let symbols = self.ast_analyzer.get_relevant_symbols(&relevant_files, query).await?;
        let spans = self.ast_analyzer.get_relevant_spans(&relevant_files, query).await?;
        
        // Find related tests
        let related_tests = self.repo_scanner.find_related_tests(&relevant_files).await?;
        
        // Select best context within token limit
        let context = self.selector.select_context(
            relevant_files,
            spans,
            symbols,
            related_tests,
            max_tokens,
        ).await?;

        Ok(context)
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.scan_repository().await
    }
}