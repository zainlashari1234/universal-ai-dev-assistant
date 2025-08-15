use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, Tree};
use tracing::{debug, info, warn};

use super::{CodeSpan, FileContext, SpanType, Symbol, SymbolReference, SymbolType, ReferenceType};

// Tree-sitter language declarations
extern "C" {
    fn tree_sitter_python() -> Language;
    fn tree_sitter_javascript() -> Language;
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_go() -> Language;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphNode {
    pub symbol: String,
    pub file_path: PathBuf,
    pub calls: Vec<String>,
    pub called_by: Vec<String>,
    pub complexity_score: f32,
}

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub nodes: HashMap<String, CallGraphNode>,
    pub edges: Vec<(String, String)>, // (caller, callee)
}

pub struct AstAnalyzer {
    parsers: HashMap<String, Parser>,
    call_graph: CallGraph,
}

impl AstAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();
        
        // Initialize parsers for supported languages
        Self::init_parser(&mut parsers, "python", unsafe { tree_sitter_python() })?;
        Self::init_parser(&mut parsers, "javascript", unsafe { tree_sitter_javascript() })?;
        Self::init_parser(&mut parsers, "typescript", unsafe { tree_sitter_typescript() })?;
        Self::init_parser(&mut parsers, "rust", unsafe { tree_sitter_rust() })?;
        Self::init_parser(&mut parsers, "go", unsafe { tree_sitter_go() })?;
        
        info!("AST Analyzer initialized with {} language parsers", parsers.len());
        
        Ok(Self {
            parsers,
            call_graph: CallGraph {
                nodes: HashMap::new(),
                edges: Vec::new(),
            },
        })
    }

    fn init_parser(parsers: &mut HashMap<String, Parser>, language: &str, ts_language: Language) -> Result<()> {
        let mut parser = Parser::new();
        parser.set_language(ts_language)
            .map_err(|e| anyhow!("Failed to set language {}: {}", language, e))?;
        parsers.insert(language.to_string(), parser);
        Ok(())
    }

    /// Analyze a single file and extract symbols
    pub async fn analyze_file(&mut self, file_path: &Path) -> Result<Vec<Symbol>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let language = self.detect_language(file_path);
        
        debug!("Analyzing file: {:?} ({})", file_path, language);
        
        let parser = self.parsers.get_mut(&language)
            .ok_or_else(|| anyhow!("No parser available for language: {}", language))?;
        
        let tree = parser.parse(&content, None)
            .ok_or_else(|| anyhow!("Failed to parse file: {:?}", file_path))?;
        
        let symbols = self.extract_symbols(&tree, &content, file_path, &language)?;
        
        // Update call graph
        self.update_call_graph(&symbols, &tree, &content, file_path, &language)?;
        
        debug!("Extracted {} symbols from {:?}", symbols.len(), file_path);
        Ok(symbols)
    }

    /// Extract symbols from parsed tree
    fn extract_symbols(&self, tree: &Tree, content: &str, file_path: &Path, language: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let root_node = tree.root_node();
        
        match language {
            "python" => self.extract_python_symbols(root_node, content, file_path, &mut symbols)?,
            "javascript" | "typescript" => self.extract_js_symbols(root_node, content, file_path, &mut symbols)?,
            "rust" => self.extract_rust_symbols(root_node, content, file_path, &mut symbols)?,
            "go" => self.extract_go_symbols(root_node, content, file_path, &mut symbols)?,
            _ => {
                warn!("Symbol extraction not implemented for language: {}", language);
                return Ok(symbols);
            }
        }
        
        Ok(symbols)
    }

    /// Extract Python symbols
    fn extract_python_symbols(&self, node: Node, content: &str, file_path: &Path, symbols: &mut Vec<Symbol>) -> Result<()> {
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Function,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                    }
                }
                "class_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Class,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                        
                        // Recursively extract class methods
                        self.extract_python_symbols(child, content, file_path, symbols)?;
                    }
                }
                "assignment" => {
                    // Extract variable assignments
                    if let Some(left_node) = child.child_by_field_name("left") {
                        if left_node.kind() == "identifier" {
                            let name = left_node.utf8_text(content.as_bytes())?;
                            let line = left_node.start_position().row;
                            let column = left_node.start_position().column;
                            
                            symbols.push(Symbol {
                                name: name.to_string(),
                                symbol_type: SymbolType::Variable,
                                file_path: file_path.to_path_buf(),
                                line,
                                column,
                                scope: "global".to_string(),
                                references: Vec::new(),
                            });
                        }
                    }
                }
                _ => {
                    // Recursively process other nodes
                    self.extract_python_symbols(child, content, file_path, symbols)?;
                }
            }
        }
        
        Ok(())
    }

    /// Extract JavaScript/TypeScript symbols
    fn extract_js_symbols(&self, node: Node, content: &str, file_path: &Path, symbols: &mut Vec<Symbol>) -> Result<()> {
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" | "function_expression" | "arrow_function" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Function,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                    }
                }
                "class_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Class,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                        
                        // Recursively extract class methods
                        self.extract_js_symbols(child, content, file_path, symbols)?;
                    }
                }
                "variable_declaration" | "lexical_declaration" => {
                    // Extract variable declarations
                    for declarator in child.children(&mut cursor) {
                        if declarator.kind() == "variable_declarator" {
                            if let Some(name_node) = declarator.child_by_field_name("name") {
                                let name = name_node.utf8_text(content.as_bytes())?;
                                let line = name_node.start_position().row;
                                let column = name_node.start_position().column;
                                
                                symbols.push(Symbol {
                                    name: name.to_string(),
                                    symbol_type: SymbolType::Variable,
                                    file_path: file_path.to_path_buf(),
                                    line,
                                    column,
                                    scope: "global".to_string(),
                                    references: Vec::new(),
                                });
                            }
                        }
                    }
                }
                _ => {
                    // Recursively process other nodes
                    self.extract_js_symbols(child, content, file_path, symbols)?;
                }
            }
        }
        
        Ok(())
    }

    /// Extract Rust symbols
    fn extract_rust_symbols(&self, node: Node, content: &str, file_path: &Path, symbols: &mut Vec<Symbol>) -> Result<()> {
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_item" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Function,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                    }
                }
                "struct_item" | "enum_item" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        let symbol_type = if child.kind() == "struct_item" {
                            SymbolType::Class
                        } else {
                            SymbolType::Enum
                        };
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                    }
                }
                "impl_item" => {
                    // Extract impl methods
                    self.extract_rust_symbols(child, content, file_path, symbols)?;
                }
                _ => {
                    // Recursively process other nodes
                    self.extract_rust_symbols(child, content, file_path, symbols)?;
                }
            }
        }
        
        Ok(())
    }

    /// Extract Go symbols
    fn extract_go_symbols(&self, node: Node, content: &str, file_path: &Path, symbols: &mut Vec<Symbol>) -> Result<()> {
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Function,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                    }
                }
                "type_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(content.as_bytes())?;
                        let line = name_node.start_position().row;
                        let column = name_node.start_position().column;
                        
                        symbols.push(Symbol {
                            name: name.to_string(),
                            symbol_type: SymbolType::Class,
                            file_path: file_path.to_path_buf(),
                            line,
                            column,
                            scope: "global".to_string(),
                            references: Vec::new(),
                        });
                    }
                }
                _ => {
                    // Recursively process other nodes
                    self.extract_go_symbols(child, content, file_path, symbols)?;
                }
            }
        }
        
        Ok(())
    }

    /// Update call graph with new symbols
    fn update_call_graph(&mut self, symbols: &[Symbol], tree: &Tree, content: &str, file_path: &Path, language: &str) -> Result<()> {
        // Extract function calls and build call graph
        for symbol in symbols {
            if matches!(symbol.symbol_type, SymbolType::Function) {
                let node_id = format!("{}::{}", file_path.display(), symbol.name);
                
                let calls = self.extract_function_calls(tree, content, &symbol.name, language)?;
                
                let call_graph_node = CallGraphNode {
                    symbol: symbol.name.clone(),
                    file_path: file_path.to_path_buf(),
                    calls: calls.clone(),
                    called_by: Vec::new(),
                    complexity_score: self.calculate_complexity_score(tree, content, &symbol.name, language),
                };
                
                self.call_graph.nodes.insert(node_id.clone(), call_graph_node);
                
                // Add edges for function calls
                for call in calls {
                    self.call_graph.edges.push((node_id.clone(), call));
                }
            }
        }
        
        Ok(())
    }

    /// Extract function calls from a function
    fn extract_function_calls(&self, tree: &Tree, content: &str, function_name: &str, _language: &str) -> Result<Vec<String>> {
        let mut calls = Vec::new();
        
        // This is a simplified implementation
        // In a real implementation, you'd use tree-sitter queries to find call expressions
        let root_node = tree.root_node();
        self.find_calls_recursive(root_node, content, &mut calls)?;
        
        Ok(calls)
    }

    fn find_calls_recursive(&self, node: Node, content: &str, calls: &mut Vec<String>) -> Result<()> {
        if node.kind() == "call_expression" || node.kind() == "call" {
            if let Some(function_node) = node.child(0) {
                if let Ok(function_name) = function_node.utf8_text(content.as_bytes()) {
                    calls.push(function_name.to_string());
                }
            }
        }
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_calls_recursive(child, content, calls)?;
        }
        
        Ok(())
    }

    /// Calculate complexity score for a function
    fn calculate_complexity_score(&self, tree: &Tree, content: &str, _function_name: &str, _language: &str) -> f32 {
        // Simplified complexity calculation
        let root_node = tree.root_node();
        let mut complexity = 1.0; // Base complexity
        
        // Count control flow statements
        complexity += self.count_control_flow_recursive(root_node, content) as f32 * 0.1;
        
        complexity
    }

    fn count_control_flow_recursive(&self, node: Node, _content: &str) -> usize {
        let mut count = 0;
        
        match node.kind() {
            "if_statement" | "while_statement" | "for_statement" | 
            "match_expression" | "switch_statement" => count += 1,
            _ => {}
        }
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            count += self.count_control_flow_recursive(child, _content);
        }
        
        count
    }

    /// Get relevant symbols based on query
    pub async fn get_relevant_symbols(&self, files: &[FileContext], query: &str) -> Result<Vec<Symbol>> {
        let mut relevant_symbols = Vec::new();
        
        for file in files {
            let symbols = self.analyze_file(&file.path).await?;
            
            for symbol in symbols {
                if self.is_symbol_relevant(&symbol, query) {
                    relevant_symbols.push(symbol);
                }
            }
        }
        
        Ok(relevant_symbols)
    }

    /// Get relevant code spans based on query
    pub async fn get_relevant_spans(&self, files: &[FileContext], query: &str) -> Result<Vec<CodeSpan>> {
        let mut relevant_spans = Vec::new();
        
        for file in files {
            let spans = self.extract_code_spans(&file.content, &file.path, query).await?;
            relevant_spans.extend(spans);
        }
        
        Ok(relevant_spans)
    }

    /// Extract code spans from file content
    async fn extract_code_spans(&self, content: &str, file_path: &Path, query: &str) -> Result<Vec<CodeSpan>> {
        let mut spans = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for (line_idx, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&query.to_lowercase()) {
                let start_line = line_idx.saturating_sub(2);
                let end_line = (line_idx + 3).min(lines.len());
                
                let span_content = lines[start_line..end_line].join("\n");
                
                spans.push(CodeSpan {
                    file_path: file_path.to_path_buf(),
                    start_line,
                    end_line,
                    content: span_content,
                    span_type: SpanType::Function, // Simplified
                    relevance_score: 0.8,
                });
            }
        }
        
        Ok(spans)
    }

    fn is_symbol_relevant(&self, symbol: &Symbol, query: &str) -> bool {
        symbol.name.to_lowercase().contains(&query.to_lowercase())
    }

    fn detect_language(&self, file_path: &Path) -> String {
        match file_path.extension().and_then(|s| s.to_str()) {
            Some("py") => "python".to_string(),
            Some("js") => "javascript".to_string(),
            Some("ts") => "typescript".to_string(),
            Some("rs") => "rust".to_string(),
            Some("go") => "go".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Get the current call graph
    pub fn get_call_graph(&self) -> &CallGraph {
        &self.call_graph
    }

    /// Find symbols that are called by a given symbol
    pub fn find_called_symbols(&self, symbol_name: &str) -> Vec<&CallGraphNode> {
        self.call_graph.nodes.values()
            .filter(|node| node.calls.contains(&symbol_name.to_string()))
            .collect()
    }

    /// Find symbols that call a given symbol
    pub fn find_calling_symbols(&self, symbol_name: &str) -> Vec<&CallGraphNode> {
        self.call_graph.nodes.values()
            .filter(|node| node.symbol == symbol_name)
            .flat_map(|node| &node.called_by)
            .filter_map(|caller| self.call_graph.nodes.get(caller))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_python_symbol_extraction() -> Result<()> {
        let mut analyzer = AstAnalyzer::new()?;
        
        let python_code = r#"
def hello_world():
    print("Hello, World!")

class MyClass:
    def method(self):
        pass

x = 42
"#;
        
        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::with_suffix(".py")?;
        tokio::fs::write(temp_file.path(), python_code).await?;
        
        let symbols = analyzer.analyze_file(temp_file.path()).await?;
        
        assert!(symbols.iter().any(|s| s.name == "hello_world" && matches!(s.symbol_type, SymbolType::Function)));
        assert!(symbols.iter().any(|s| s.name == "MyClass" && matches!(s.symbol_type, SymbolType::Class)));
        assert!(symbols.iter().any(|s| s.name == "x" && matches!(s.symbol_type, SymbolType::Variable)));
        
        Ok(())
    }
}