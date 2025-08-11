pub mod code_parser;
pub mod security_scanner;
pub mod performance_analyzer;
pub mod documentation_generator;
pub mod test_generator;

use anyhow::Result;
use crate::models::*;

pub trait CodeAnalysisService {
    async fn analyze_code(&self, code: &str, language: &str) -> Result<CodeContext>;
    async fn detect_security_issues(&self, code: &str, language: &str) -> Result<Vec<SecurityIssue>>;
    async fn suggest_improvements(&self, code: &str, language: &str) -> Result<Vec<String>>;
}

pub trait DocumentationService {
    async fn generate_docstring(&self, function: &FunctionInfo, language: &str) -> Result<String>;
    async fn generate_readme(&self, project: &Project) -> Result<String>;
    async fn generate_api_docs(&self, code: &str, language: &str) -> Result<String>;
}

pub trait TestGenerationService {
    async fn generate_unit_tests(&self, function: &FunctionInfo, language: &str) -> Result<String>;
    async fn generate_integration_tests(&self, code: &str, language: &str) -> Result<String>;
    async fn suggest_test_cases(&self, function: &FunctionInfo) -> Result<Vec<String>>;
}