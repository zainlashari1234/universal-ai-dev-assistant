use anyhow::Result;
use std::sync::Arc;
use std::path::Path;
use tokio::fs;
use tracing::{info, warn, error};

use crate::providers::{ProviderRouter, CompletionRequest};
use super::{
    CodeChange, ChangeType, MessageIntent, CodeContext, WorkspaceContext,
    FunctionInfo, ImportInfo, SymbolInfo, SymbolType, TextSelection, Position
};

pub struct CodeIntegrationService {
    provider_router: Arc<ProviderRouter>,
}

impl CodeIntegrationService {
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self { provider_router }
    }

    pub async fn generate_code(
        &self,
        request: &CodeGenerationRequest,
        workspace_context: &WorkspaceContext,
        code_context: &CodeContext,
    ) -> Result<CodeGenerationResult> {
        info!("Generating code for: {}", request.description);

        let context_info = self.build_code_context(workspace_context, code_context).await?;
        let prompt = self.build_code_generation_prompt(request, &context_info);

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(2000),
            temperature: Some(0.3),
            system_prompt: Some(self.get_code_generation_system_prompt(workspace_context)),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let result = self.parse_code_generation_response(&response.text, request)?;

        Ok(result)
    }

    pub async fn explain_code(
        &self,
        code: &str,
        file_path: Option<&str>,
        workspace_context: &WorkspaceContext,
    ) -> Result<CodeExplanation> {
        info!("Explaining code from: {:?}", file_path);

        let language = self.detect_language(code, file_path);
        let context_info = if let Some(path) = file_path {
            self.get_file_context(path, workspace_context).await?
        } else {
            String::new()
        };

        let prompt = format!(
            r#"Bu kodu detaylı olarak açıkla:

```{}
{}
```

Dosya bağlamı:
{}

Açıklaman şunları içersin:
1. Kodun genel amacı
2. Her fonksiyon/method'un ne yaptığı
3. Kullanılan algoritma veya pattern'ler
4. Potansiyel iyileştirmeler
5. Güvenlik veya performans notları

Açıklamayı Türkçe yap ve teknik terimleri açıkla."#,
            language,
            code,
            context_info
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(1500),
            temperature: Some(0.2),
            system_prompt: Some("Sen bir kod analiz uzmanısın. Kodları detaylı ve anlaşılır şekilde açıklıyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;

        Ok(CodeExplanation {
            explanation: response.text,
            language,
            complexity_score: self.calculate_complexity_score(code),
            suggestions: self.extract_suggestions(&response.text),
        })
    }

    pub async fn review_code(
        &self,
        code: &str,
        file_path: Option<&str>,
        workspace_context: &WorkspaceContext,
    ) -> Result<CodeReview> {
        info!("Reviewing code from: {:?}", file_path);

        let language = self.detect_language(code, file_path);
        let context_info = if let Some(path) = file_path {
            self.get_file_context(path, workspace_context).await?
        } else {
            String::new()
        };

        let prompt = format!(
            r#"Bu kodu gözden geçir ve analiz et:

```{}
{}
```

Proje bağlamı:
{}

İnceleme kriterleri:
1. Kod kalitesi ve okunabilirlik
2. Performans optimizasyonları
3. Güvenlik açıkları
4. Best practice'lere uygunluk
5. Potansiyel bug'lar
6. Test edilebilirlik
7. Maintainability

Her kategori için puan ver (1-10) ve öneriler sun.
Format:
KATEGORI: PUAN - AÇIKLAMA
ÖNERI: Spesifik iyileştirme önerisi"#,
            language,
            code,
            context_info
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(2000),
            temperature: Some(0.1),
            system_prompt: Some("Sen bir senior kod reviewer'sın. Kodları titizlikle inceleyip yapıcı geri bildirim veriyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let review = self.parse_code_review_response(&response.text)?;

        Ok(review)
    }

    pub async fn suggest_refactoring(
        &self,
        code: &str,
        file_path: Option<&str>,
        workspace_context: &WorkspaceContext,
    ) -> Result<RefactoringSuggestion> {
        info!("Suggesting refactoring for: {:?}", file_path);

        let language = self.detect_language(code, file_path);
        let context_info = if let Some(path) = file_path {
            self.get_file_context(path, workspace_context).await?
        } else {
            String::new()
        };

        let prompt = format!(
            r#"Bu kod için refactoring önerileri sun:

```{}
{}
```

Proje bağlamı:
{}

Refactoring hedefleri:
1. Kod okunabilirliğini artır
2. Performansı optimize et
3. Maintainability'yi iyileştir
4. Design pattern'leri uygula
5. Code smell'leri gider

Her öneri için:
- Mevcut problem
- Önerilen çözüm
- Refactor edilmiş kod
- Faydalar"#,
            language,
            code,
            context_info
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(2500),
            temperature: Some(0.2),
            system_prompt: Some("Sen bir refactoring uzmanısın. Kodları daha temiz, verimli ve maintainable hale getiriyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let suggestion = self.parse_refactoring_response(&response.text, code)?;

        Ok(suggestion)
    }

    pub async fn generate_tests(
        &self,
        code: &str,
        file_path: Option<&str>,
        workspace_context: &WorkspaceContext,
    ) -> Result<TestGeneration> {
        info!("Generating tests for: {:?}", file_path);

        let language = self.detect_language(code, file_path);
        let test_framework = self.detect_test_framework(workspace_context, &language);

        let prompt = format!(
            r#"Bu kod için kapsamlı testler yaz:

```{}
{}
```

Test framework: {}
Proje tipi: {:?}

Test gereksinimleri:
1. Unit testler - her fonksiyon için
2. Edge case'ler
3. Error handling testleri
4. Integration testleri (gerekirse)
5. Mock'lar (external dependency'ler için)

Test kodunu {} framework'ü kullanarak yaz.
Test coverage %90+ olmalı."#,
            language,
            code,
            test_framework,
            workspace_context.project_type,
            test_framework
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(3000),
            temperature: Some(0.2),
            system_prompt: Some("Sen bir test uzmanısın. Kapsamlı, güvenilir ve maintainable testler yazıyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let test_generation = self.parse_test_generation_response(&response.text, &test_framework)?;

        Ok(test_generation)
    }

    pub async fn fix_code(
        &self,
        code: &str,
        error_message: &str,
        file_path: Option<&str>,
        workspace_context: &WorkspaceContext,
    ) -> Result<CodeFix> {
        info!("Fixing code error: {}", error_message);

        let language = self.detect_language(code, file_path);
        let context_info = if let Some(path) = file_path {
            self.get_file_context(path, workspace_context).await?
        } else {
            String::new()
        };

        let prompt = format!(
            r#"Bu kodda hata var, düzelt:

Hatalı kod:
```{}
{}
```

Hata mesajı:
{}

Dosya bağlamı:
{}

Çözüm:
1. Hatanın nedenini açıkla
2. Düzeltilmiş kodu ver
3. Neden bu çözümün doğru olduğunu açıkla
4. Gelecekte bu hatayı önleme yolları"#,
            language,
            code,
            error_message,
            context_info
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(2000),
            temperature: Some(0.1),
            system_prompt: Some("Sen bir debugging uzmanısın. Hataları hızlı ve doğru şekilde tespit edip çözüyorsun.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let fix = self.parse_code_fix_response(&response.text, code)?;

        Ok(fix)
    }

    // Helper methods
    async fn build_code_context(
        &self,
        workspace_context: &WorkspaceContext,
        code_context: &CodeContext,
    ) -> Result<String> {
        let mut context_parts = Vec::new();

        // Proje bilgisi
        context_parts.push(format!("Proje tipi: {:?}", workspace_context.project_type));
        context_parts.push(format!("Build sistem: {:?}", workspace_context.build_system));

        // Mevcut dosya
        if let Some(current_file) = &code_context.current_file {
            context_parts.push(format!("Mevcut dosya: {}", current_file));
        }

        // Açık dosyalar
        if !code_context.open_files.is_empty() {
            let open_files: Vec<String> = code_context.open_files
                .iter()
                .map(|f| f.path.clone())
                .collect();
            context_parts.push(format!("Açık dosyalar: {:?}", open_files));
        }

        // Son fonksiyonlar
        if !code_context.recent_functions.is_empty() {
            let functions: Vec<String> = code_context.recent_functions
                .iter()
                .map(|f| f.name.clone())
                .collect();
            context_parts.push(format!("Son fonksiyonlar: {:?}", functions));
        }

        // Import'lar
        if !code_context.imports.is_empty() {
            let imports: Vec<String> = code_context.imports
                .iter()
                .map(|i| i.module.clone())
                .collect();
            context_parts.push(format!("Import'lar: {:?}", imports));
        }

        Ok(context_parts.join("\n"))
    }

    fn build_code_generation_prompt(&self, request: &CodeGenerationRequest, context: &str) -> String {
        format!(
            r#"Kod oluşturma isteği: {}

Bağlam:
{}

Gereksinimler:
- Temiz, okunabilir kod yaz
- Best practice'leri uygula
- Uygun error handling ekle
- Gerekirse comment'ler ekle
- Test edilebilir kod yaz

Eğer dosya yolu belirtilmişse, o dosyaya uygun format kullan.
Kod bloklarını ``` ile işaretle ve dili belirt."#,
            request.description,
            context
        )
    }

    fn get_code_generation_system_prompt(&self, workspace_context: &WorkspaceContext) -> String {
        let project_specific = match workspace_context.project_type.as_deref() {
            Some("rust") => "Rust best practice'lerini uygula. Memory safety'yi göz önünde bulundur.",
            Some("node") => "Modern JavaScript/TypeScript kullan. Async/await pattern'lerini tercih et.",
            Some("python") => "PEP 8 standartlarına uy. Type hint'leri kullan.",
            Some("java") => "Java conventions'larını takip et. SOLID principles'ları uygula.",
            _ => "Genel programming best practice'lerini uygula.",
        };

        format!(
            "Sen bir uzman yazılım geliştiricisisin. {}. Kod yazarken güvenlik, performans ve maintainability'yi öncelikle.",
            project_specific
        )
    }

    fn detect_language(&self, code: &str, file_path: Option<&str>) -> String {
        if let Some(path) = file_path {
            if let Some(extension) = Path::new(path).extension() {
                return match extension.to_str() {
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
                    _ => "text".to_string(),
                };
            }
        }

        // Kod içeriğinden tahmin et
        if code.contains("fn ") && code.contains("->") {
            "rust".to_string()
        } else if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            "javascript".to_string()
        } else if code.contains("def ") && code.contains(":") {
            "python".to_string()
        } else if code.contains("public class ") || code.contains("private ") {
            "java".to_string()
        } else {
            "text".to_string()
        }
    }

    fn detect_test_framework(&self, workspace_context: &WorkspaceContext, language: &str) -> String {
        match language {
            "rust" => "cargo test".to_string(),
            "javascript" | "typescript" => {
                if workspace_context.dependencies.iter().any(|d| d.name == "jest") {
                    "jest".to_string()
                } else if workspace_context.dependencies.iter().any(|d| d.name == "mocha") {
                    "mocha".to_string()
                } else {
                    "jest".to_string()
                }
            }
            "python" => {
                if workspace_context.dependencies.iter().any(|d| d.name == "pytest") {
                    "pytest".to_string()
                } else {
                    "unittest".to_string()
                }
            }
            "java" => "junit".to_string(),
            _ => "generic".to_string(),
        }
    }

    async fn get_file_context(&self, file_path: &str, workspace_context: &WorkspaceContext) -> Result<String> {
        let mut context_parts = Vec::new();

        // Dosya tipi
        let language = self.detect_language("", Some(file_path));
        context_parts.push(format!("Dosya tipi: {}", language));

        // Dosya boyutu (varsa)
        if let Ok(metadata) = fs::metadata(file_path).await {
            context_parts.push(format!("Dosya boyutu: {} bytes", metadata.len()));
        }

        // İlgili dosyalar (aynı dizindeki)
        if let Some(parent) = Path::new(file_path).parent() {
            if let Ok(mut entries) = fs::read_dir(parent).await {
                let mut related_files = Vec::new();
                while let Some(entry) = entries.next_entry().await? {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(&format!(".{}", language)) {
                            related_files.push(name.to_string());
                        }
                    }
                }
                if !related_files.is_empty() {
                    context_parts.push(format!("İlgili dosyalar: {:?}", related_files));
                }
            }
        }

        Ok(context_parts.join("\n"))
    }

    fn calculate_complexity_score(&self, code: &str) -> f32 {
        let lines = code.lines().count() as f32;
        let cyclomatic_complexity = self.calculate_cyclomatic_complexity(code);
        
        // Basit complexity score hesaplama
        let base_score = (lines / 10.0).min(5.0);
        let complexity_score = (cyclomatic_complexity as f32 / 5.0).min(5.0);
        
        (base_score + complexity_score) / 2.0
    }

    fn calculate_cyclomatic_complexity(&self, code: &str) -> usize {
        let mut complexity = 1; // Base complexity
        
        // Control flow keywords
        let keywords = ["if", "else", "while", "for", "match", "case", "catch", "&&", "||"];
        
        for keyword in keywords {
            complexity += code.matches(keyword).count();
        }
        
        complexity
    }

    // Parse response methods
    fn parse_code_generation_response(&self, response: &str, request: &CodeGenerationRequest) -> Result<CodeGenerationResult> {
        // AI response'undan kod bloklarını çıkar
        let code_blocks = self.extract_code_blocks(response);
        
        let mut code_changes = Vec::new();
        
        for (i, code_block) in code_blocks.iter().enumerate() {
            let file_path = request.target_file.clone()
                .unwrap_or_else(|| format!("generated_code_{}.txt", i));
                
            code_changes.push(CodeChange {
                file_path,
                change_type: ChangeType::Create,
                old_content: None,
                new_content: code_block.clone(),
                line_start: 0,
                line_end: code_block.lines().count(),
                description: format!("Generated code: {}", request.description),
            });
        }

        Ok(CodeGenerationResult {
            code: code_blocks.join("\n\n"),
            changes: code_changes,
            explanation: response.to_string(),
            suggestions: self.extract_suggestions(response),
        })
    }

    fn parse_code_review_response(&self, response: &str) -> Result<CodeReview> {
        let mut scores = std::collections::HashMap::new();
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        for line in response.lines() {
            if line.contains(": ") && line.contains(" - ") {
                let parts: Vec<&str> = line.split(" - ").collect();
                if parts.len() >= 2 {
                    let category_score: Vec<&str> = parts[0].split(": ").collect();
                    if category_score.len() >= 2 {
                        if let Ok(score) = category_score[1].parse::<u8>() {
                            scores.insert(category_score[0].to_string(), score);
                        }
                    }
                }
            } else if line.starts_with("ÖNERI:") {
                suggestions.push(line.strip_prefix("ÖNERI:").unwrap_or("").trim().to_string());
            }
        }

        Ok(CodeReview {
            overall_score: scores.values().sum::<u8>() as f32 / scores.len() as f32,
            category_scores: scores,
            issues,
            suggestions,
            security_issues: Vec::new(), // TODO: Parse security issues
            performance_issues: Vec::new(), // TODO: Parse performance issues
        })
    }

    fn parse_refactoring_response(&self, response: &str, original_code: &str) -> Result<RefactoringSuggestion> {
        let refactored_code = self.extract_code_blocks(response).join("\n");
        
        Ok(RefactoringSuggestion {
            original_code: original_code.to_string(),
            refactored_code,
            explanation: response.to_string(),
            benefits: self.extract_benefits(response),
            estimated_effort: "medium".to_string(), // TODO: Parse effort estimation
        })
    }

    fn parse_test_generation_response(&self, response: &str, framework: &str) -> Result<TestGeneration> {
        let test_code = self.extract_code_blocks(response).join("\n");
        
        Ok(TestGeneration {
            test_code,
            framework: framework.to_string(),
            test_cases: Vec::new(), // TODO: Parse individual test cases
            coverage_estimate: 85.0, // TODO: Calculate coverage estimate
            setup_code: None,
        })
    }

    fn parse_code_fix_response(&self, response: &str, original_code: &str) -> Result<CodeFix> {
        let fixed_code = self.extract_code_blocks(response).join("\n");
        
        Ok(CodeFix {
            original_code: original_code.to_string(),
            fixed_code,
            explanation: response.to_string(),
            error_type: "runtime".to_string(), // TODO: Parse error type
            prevention_tips: self.extract_prevention_tips(response),
        })
    }

    fn extract_code_blocks(&self, text: &str) -> Vec<String> {
        let mut blocks = Vec::new();
        let mut in_block = false;
        let mut current_block = String::new();

        for line in text.lines() {
            if line.starts_with("```") {
                if in_block {
                    blocks.push(current_block.trim().to_string());
                    current_block.clear();
                    in_block = false;
                } else {
                    in_block = true;
                }
            } else if in_block {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }

        blocks
    }

    fn extract_suggestions(&self, text: &str) -> Vec<String> {
        text.lines()
            .filter(|line| line.contains("öneri") || line.contains("suggestion") || line.contains("recommend"))
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn extract_benefits(&self, text: &str) -> Vec<String> {
        text.lines()
            .filter(|line| line.contains("fayda") || line.contains("benefit") || line.contains("advantage"))
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn extract_prevention_tips(&self, text: &str) -> Vec<String> {
        text.lines()
            .filter(|line| line.contains("önle") || line.contains("prevent") || line.contains("avoid"))
            .map(|line| line.trim().to_string())
            .collect()
    }
}

// Request/Response types
#[derive(Debug, Clone)]
pub struct CodeGenerationRequest {
    pub description: String,
    pub target_file: Option<String>,
    pub language: Option<String>,
    pub style_preferences: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodeGenerationResult {
    pub code: String,
    pub changes: Vec<CodeChange>,
    pub explanation: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CodeExplanation {
    pub explanation: String,
    pub language: String,
    pub complexity_score: f32,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CodeReview {
    pub overall_score: f32,
    pub category_scores: std::collections::HashMap<String, u8>,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
    pub security_issues: Vec<String>,
    pub performance_issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RefactoringSuggestion {
    pub original_code: String,
    pub refactored_code: String,
    pub explanation: String,
    pub benefits: Vec<String>,
    pub estimated_effort: String,
}

#[derive(Debug, Clone)]
pub struct TestGeneration {
    pub test_code: String,
    pub framework: String,
    pub test_cases: Vec<String>,
    pub coverage_estimate: f32,
    pub setup_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodeFix {
    pub original_code: String,
    pub fixed_code: String,
    pub explanation: String,
    pub error_type: String,
    pub prevention_tips: Vec<String>,
}