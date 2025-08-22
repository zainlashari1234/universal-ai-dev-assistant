use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use regex::Regex;
use tracing::{info, debug};

use crate::providers::{ProviderRouter, CompletionRequest};
use super::{
    SearchRequest, SearchQueryType, ProcessedQuery, Entity, EntityType, QueryIntent,
    QueryFilter, FilterOperator, BoostTerm, EmbeddingRequest, EmbeddingType
};
use super::embedding_manager::EmbeddingManager;

pub struct QueryProcessor {
    provider_router: Arc<ProviderRouter>,
    embedding_manager: Arc<EmbeddingManager>,
    entity_patterns: HashMap<EntityType, Vec<Regex>>,
    intent_patterns: HashMap<QueryIntent, Vec<Regex>>,
    stop_words: Vec<String>,
}

impl QueryProcessor {
    pub fn new(
        provider_router: Arc<ProviderRouter>,
        embedding_manager: Arc<EmbeddingManager>,
    ) -> Self {
        let mut processor = Self {
            provider_router,
            embedding_manager,
            entity_patterns: HashMap::new(),
            intent_patterns: HashMap::new(),
            stop_words: Self::create_stop_words(),
        };
        
        processor.initialize_patterns();
        processor
    }

    pub async fn process_query(&self, request: &SearchRequest) -> Result<ProcessedQuery> {
        info!("Processing search query: {}", request.query);
        
        let normalized_query = self.normalize_query(&request.query);
        let keywords = self.extract_keywords(&normalized_query);
        let entities = self.extract_entities(&request.query).await?;
        let intent = self.determine_intent(&request.query, &entities).await?;
        let filters = self.extract_filters(&request.query, &entities);
        let boost_terms = self.generate_boost_terms(&request.query, &intent, &entities).await?;
        
        Ok(ProcessedQuery {
            original_query: request.query.clone(),
            normalized_query,
            keywords,
            entities,
            intent,
            filters,
            boost_terms,
        })
    }

    fn normalize_query(&self, query: &str) -> String {
        let mut normalized = query.to_lowercase();
        
        // Özel karakterleri temizle
        normalized = regex::Regex::new(r"[^\w\s]").unwrap()
            .replace_all(&normalized, " ").to_string();
        
        // Fazla boşlukları temizle
        normalized = regex::Regex::new(r"\s+").unwrap()
            .replace_all(&normalized, " ").to_string();
        
        // Stop words'leri kaldır
        let words: Vec<&str> = normalized.split_whitespace().collect();
        let filtered_words: Vec<&str> = words.into_iter()
            .filter(|word| !self.stop_words.contains(&word.to_string()))
            .collect();
        
        filtered_words.join(" ").trim().to_string()
    }

    fn extract_keywords(&self, normalized_query: &str) -> Vec<String> {
        normalized_query
            .split_whitespace()
            .filter(|word| word.len() > 2) // En az 3 karakter
            .map(|word| word.to_string())
            .collect()
    }

    async fn extract_entities(&self, query: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        
        // Pattern-based entity extraction
        for (entity_type, patterns) in &self.entity_patterns {
            for pattern in patterns {
                for mat in pattern.find_iter(query) {
                    entities.push(Entity {
                        text: mat.as_str().to_string(),
                        entity_type: entity_type.clone(),
                        confidence: 0.8,
                        start_pos: mat.start(),
                        end_pos: mat.end(),
                    });
                }
            }
        }
        
        // AI-based entity extraction for complex cases
        if entities.is_empty() || query.len() > 50 {
            let ai_entities = self.extract_entities_with_ai(query).await?;
            entities.extend(ai_entities);
        }
        
        // Overlapping entities'leri temizle
        entities.sort_by_key(|e| e.start_pos);
        self.remove_overlapping_entities(entities)
    }

    async fn extract_entities_with_ai(&self, query: &str) -> Result<Vec<Entity>> {
        let prompt = format!(
            r#"Extract programming-related entities from this search query: "{}"

Identify and classify these entity types:
- FunctionName: function or method names
- ClassName: class, struct, or type names  
- VariableName: variable or field names
- FileName: file or module names
- Language: programming languages
- Framework: frameworks or libraries
- Concept: programming concepts or patterns
- ErrorType: error types or exceptions

Return JSON format:
[{{"text": "entity", "type": "EntityType", "confidence": 0.9}}]

Query: {}"#,
            query, query
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(500),
            temperature: Some(0.1),
            system_prompt: Some("You are an expert at extracting programming entities from search queries. Return only valid JSON.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        self.parse_ai_entities(&response.text)
    }

    fn parse_ai_entities(&self, response: &str) -> Result<Vec<Entity>> {
        // JSON parsing with error handling
        match serde_json::from_str::<Vec<serde_json::Value>>(response) {
            Ok(json_entities) => {
                let mut entities = Vec::new();
                for json_entity in json_entities {
                    if let (Some(text), Some(entity_type), Some(confidence)) = (
                        json_entity["text"].as_str(),
                        json_entity["type"].as_str(),
                        json_entity["confidence"].as_f64(),
                    ) {
                        let entity_type = match entity_type {
                            "FunctionName" => EntityType::FunctionName,
                            "ClassName" => EntityType::ClassName,
                            "VariableName" => EntityType::VariableName,
                            "FileName" => EntityType::FileName,
                            "Language" => EntityType::Language,
                            "Framework" => EntityType::Framework,
                            "Concept" => EntityType::Concept,
                            "ErrorType" => EntityType::ErrorType,
                            _ => continue,
                        };
                        
                        entities.push(Entity {
                            text: text.to_string(),
                            entity_type,
                            confidence: confidence as f32,
                            start_pos: 0, // AI extraction doesn't provide positions
                            end_pos: text.len(),
                        });
                    }
                }
                Ok(entities)
            }
            Err(_) => {
                // Fallback: simple keyword extraction
                Ok(Vec::new())
            }
        }
    }

    fn remove_overlapping_entities(&self, mut entities: Vec<Entity>) -> Result<Vec<Entity>> {
        entities.sort_by_key(|e| (e.start_pos, e.end_pos));
        let mut result = Vec::new();
        
        for entity in entities {
            let overlaps = result.iter().any(|existing: &Entity| {
                entity.start_pos < existing.end_pos && entity.end_pos > existing.start_pos
            });
            
            if !overlaps {
                result.push(entity);
            }
        }
        
        Ok(result)
    }

    async fn determine_intent(&self, query: &str, entities: &[Entity]) -> Result<QueryIntent> {
        // Pattern-based intent detection
        for (intent, patterns) in &self.intent_patterns {
            for pattern in patterns {
                if pattern.is_match(query) {
                    debug!("Intent detected via pattern: {:?}", intent);
                    return Ok(intent.clone());
                }
            }
        }
        
        // Entity-based intent detection
        let intent = self.infer_intent_from_entities(entities);
        if intent != QueryIntent::ExploreCode {
            return Ok(intent);
        }
        
        // AI-based intent detection for complex queries
        self.determine_intent_with_ai(query).await
    }

    fn infer_intent_from_entities(&self, entities: &[Entity]) -> QueryIntent {
        let has_function = entities.iter().any(|e| matches!(e.entity_type, EntityType::FunctionName));
        let has_class = entities.iter().any(|e| matches!(e.entity_type, EntityType::ClassName));
        let has_error = entities.iter().any(|e| matches!(e.entity_type, EntityType::ErrorType));
        
        if has_error {
            QueryIntent::FindBugs
        } else if has_function {
            QueryIntent::FindFunction
        } else if has_class {
            QueryIntent::FindClass
        } else {
            QueryIntent::ExploreCode
        }
    }

    async fn determine_intent_with_ai(&self, query: &str) -> Result<QueryIntent> {
        let prompt = format!(
            r#"Determine the search intent for this code search query: "{}"

Possible intents:
- FindFunction: looking for specific functions or methods
- FindClass: looking for classes, structs, or types
- FindUsage: looking for where something is used
- FindDefinition: looking for where something is defined
- FindSimilar: looking for similar code patterns
- FindExamples: looking for code examples
- FindDocumentation: looking for documentation
- FindBugs: looking for bugs or errors
- FindPattern: looking for specific code patterns
- ExploreCode: general code exploration

Return only the intent name.

Query: {}"#,
            query
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(20),
            temperature: Some(0.0),
            system_prompt: Some("You are an expert at understanding code search intents. Return only the intent name.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        let intent_str = response.text.trim();
        
        match intent_str {
            "FindFunction" => Ok(QueryIntent::FindFunction),
            "FindClass" => Ok(QueryIntent::FindClass),
            "FindUsage" => Ok(QueryIntent::FindUsage),
            "FindDefinition" => Ok(QueryIntent::FindDefinition),
            "FindSimilar" => Ok(QueryIntent::FindSimilar),
            "FindExamples" => Ok(QueryIntent::FindExamples),
            "FindDocumentation" => Ok(QueryIntent::FindDocumentation),
            "FindBugs" => Ok(QueryIntent::FindBugs),
            "FindPattern" => Ok(QueryIntent::FindPattern),
            _ => Ok(QueryIntent::ExploreCode),
        }
    }

    fn extract_filters(&self, query: &str, entities: &[Entity]) -> Vec<QueryFilter> {
        let mut filters = Vec::new();
        
        // Language filters
        let language_entities: Vec<&Entity> = entities.iter()
            .filter(|e| matches!(e.entity_type, EntityType::Language))
            .collect();
        
        for entity in language_entities {
            filters.push(QueryFilter {
                field: "language".to_string(),
                operator: FilterOperator::Equals,
                value: entity.text.clone(),
                boost: 2.0,
            });
        }
        
        // Framework filters
        let framework_entities: Vec<&Entity> = entities.iter()
            .filter(|e| matches!(e.entity_type, EntityType::Framework))
            .collect();
        
        for entity in framework_entities {
            filters.push(QueryFilter {
                field: "tags".to_string(),
                operator: FilterOperator::Contains,
                value: entity.text.clone(),
                boost: 1.5,
            });
        }
        
        // File type filters from query
        if query.contains("test") || query.contains("spec") {
            filters.push(QueryFilter {
                field: "categories".to_string(),
                operator: FilterOperator::Contains,
                value: "test".to_string(),
                boost: 1.0,
            });
        }
        
        filters
    }

    async fn generate_boost_terms(
        &self,
        query: &str,
        intent: &QueryIntent,
        entities: &[Entity],
    ) -> Result<Vec<BoostTerm>> {
        let mut boost_terms = Vec::new();
        
        // Intent-based boosts
        match intent {
            QueryIntent::FindFunction => {
                boost_terms.push(BoostTerm {
                    term: "function".to_string(),
                    boost_factor: 2.0,
                    reason: "Function search intent".to_string(),
                });
                boost_terms.push(BoostTerm {
                    term: "method".to_string(),
                    boost_factor: 1.8,
                    reason: "Function search intent".to_string(),
                });
            }
            QueryIntent::FindClass => {
                boost_terms.push(BoostTerm {
                    term: "class".to_string(),
                    boost_factor: 2.0,
                    reason: "Class search intent".to_string(),
                });
                boost_terms.push(BoostTerm {
                    term: "struct".to_string(),
                    boost_factor: 1.8,
                    reason: "Class search intent".to_string(),
                });
            }
            QueryIntent::FindBugs => {
                boost_terms.push(BoostTerm {
                    term: "error".to_string(),
                    boost_factor: 2.0,
                    reason: "Bug search intent".to_string(),
                });
                boost_terms.push(BoostTerm {
                    term: "exception".to_string(),
                    boost_factor: 1.8,
                    reason: "Bug search intent".to_string(),
                });
            }
            _ => {}
        }
        
        // Entity-based boosts
        for entity in entities {
            let boost_factor = match entity.entity_type {
                EntityType::FunctionName => 2.5,
                EntityType::ClassName => 2.3,
                EntityType::VariableName => 1.8,
                EntityType::Language => 2.0,
                EntityType::Framework => 1.8,
                EntityType::Concept => 1.5,
                EntityType::ErrorType => 2.2,
                EntityType::FileName => 1.6,
            };
            
            boost_terms.push(BoostTerm {
                term: entity.text.clone(),
                boost_factor,
                reason: format!("Entity: {:?}", entity.entity_type),
            });
        }
        
        // Semantic expansion
        let expanded_terms = self.expand_query_semantically(query).await?;
        for term in expanded_terms {
            boost_terms.push(BoostTerm {
                term,
                boost_factor: 1.2,
                reason: "Semantic expansion".to_string(),
            });
        }
        
        Ok(boost_terms)
    }

    async fn expand_query_semantically(&self, query: &str) -> Result<Vec<String>> {
        let prompt = format!(
            r#"Generate semantically related programming terms for this search query: "{}"

Return 5-10 related terms that would help find relevant code. Focus on:
- Synonyms and alternative names
- Related programming concepts
- Common patterns or implementations
- Framework-specific terms

Return as a simple comma-separated list.

Query: {}"#,
            query
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(200),
            temperature: Some(0.3),
            system_prompt: Some("You are an expert programmer who understands code semantics and related concepts.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        
        // Parse comma-separated terms
        let terms: Vec<String> = response.text
            .split(',')
            .map(|term| term.trim().to_lowercase())
            .filter(|term| !term.is_empty() && term.len() > 2)
            .take(10)
            .map(|term| term.to_string())
            .collect();
        
        Ok(terms)
    }

    fn initialize_patterns(&mut self) {
        // Entity patterns
        self.entity_patterns.insert(EntityType::FunctionName, vec![
            Regex::new(r"\b(\w+)\s*\(").unwrap(),
            Regex::new(r"function\s+(\w+)").unwrap(),
            Regex::new(r"def\s+(\w+)").unwrap(),
            Regex::new(r"fn\s+(\w+)").unwrap(),
        ]);
        
        self.entity_patterns.insert(EntityType::ClassName, vec![
            Regex::new(r"class\s+(\w+)").unwrap(),
            Regex::new(r"struct\s+(\w+)").unwrap(),
            Regex::new(r"interface\s+(\w+)").unwrap(),
            Regex::new(r"enum\s+(\w+)").unwrap(),
        ]);
        
        self.entity_patterns.insert(EntityType::Language, vec![
            Regex::new(r"\b(rust|javascript|python|java|go|cpp|typescript|php|ruby|swift|kotlin)\b").unwrap(),
        ]);
        
        self.entity_patterns.insert(EntityType::Framework, vec![
            Regex::new(r"\b(react|vue|angular|django|flask|spring|express|rails|laravel)\b").unwrap(),
        ]);
        
        self.entity_patterns.insert(EntityType::ErrorType, vec![
            Regex::new(r"\b(\w*Error|\w*Exception)\b").unwrap(),
            Regex::new(r"error\s*:\s*(\w+)").unwrap(),
        ]);
        
        // Intent patterns
        self.intent_patterns.insert(QueryIntent::FindFunction, vec![
            Regex::new(r"\b(function|method|procedure)\b").unwrap(),
            Regex::new(r"\bhow\s+to\s+\w+").unwrap(),
            Regex::new(r"\bfind\s+function").unwrap(),
        ]);
        
        self.intent_patterns.insert(QueryIntent::FindClass, vec![
            Regex::new(r"\b(class|struct|type|interface)\b").unwrap(),
            Regex::new(r"\bfind\s+class").unwrap(),
        ]);
        
        self.intent_patterns.insert(QueryIntent::FindUsage, vec![
            Regex::new(r"\b(usage|used|where|references)\b").unwrap(),
            Regex::new(r"\bwhere\s+is\s+\w+\s+used").unwrap(),
        ]);
        
        self.intent_patterns.insert(QueryIntent::FindDefinition, vec![
            Regex::new(r"\b(definition|defined|declare|implementation)\b").unwrap(),
            Regex::new(r"\bwhere\s+is\s+\w+\s+defined").unwrap(),
        ]);
        
        self.intent_patterns.insert(QueryIntent::FindSimilar, vec![
            Regex::new(r"\b(similar|like|equivalent|alternative)\b").unwrap(),
            Regex::new(r"\bcode\s+like").unwrap(),
        ]);
        
        self.intent_patterns.insert(QueryIntent::FindExamples, vec![
            Regex::new(r"\b(example|sample|demo|tutorial)\b").unwrap(),
            Regex::new(r"\bhow\s+to\s+use").unwrap(),
        ]);
        
        self.intent_patterns.insert(QueryIntent::FindBugs, vec![
            Regex::new(r"\b(bug|error|exception|crash|fail)\b").unwrap(),
            Regex::new(r"\bwhy\s+\w+\s+not\s+work").unwrap(),
        ]);
    }

    fn create_stop_words() -> Vec<String> {
        vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", 
            "of", "with", "by", "is", "are", "was", "were", "be", "been", "have", 
            "has", "had", "do", "does", "did", "will", "would", "could", "should",
            "this", "that", "these", "those", "i", "you", "he", "she", "it", "we", "they"
        ].iter().map(|s| s.to_string()).collect()
    }

    pub async fn suggest_query_improvements(&self, query: &str) -> Result<Vec<String>> {
        let prompt = format!(
            r#"Suggest improvements for this code search query: "{}"

Provide 3-5 alternative or improved search queries that would:
- Be more specific and targeted
- Use better programming terminology
- Include relevant context or constraints
- Fix any unclear or ambiguous terms

Return each suggestion on a new line.

Original query: {}"#,
            query
        );

        let completion_request = CompletionRequest {
            prompt,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            max_tokens: Some(300),
            temperature: Some(0.4),
            system_prompt: Some("You are an expert at improving code search queries for better results.".to_string()),
            ..Default::default()
        };

        let response = self.provider_router.complete(completion_request).await?;
        
        let suggestions: Vec<String> = response.text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();
        
        Ok(suggestions)
    }

    pub async fn generate_query_embedding(&self, processed_query: &ProcessedQuery) -> Result<Vec<f32>> {
        // Combine different parts of the query for embedding
        let mut embedding_text = processed_query.normalized_query.clone();
        
        // Add entity information
        for entity in &processed_query.entities {
            embedding_text.push_str(&format!(" {}", entity.text));
        }
        
        // Add boost terms
        for boost_term in &processed_query.boost_terms {
            embedding_text.push_str(&format!(" {}", boost_term.term));
        }
        
        let embedding_request = EmbeddingRequest {
            text: embedding_text,
            context: Some(format!("Search query with intent: {:?}", processed_query.intent)),
            embedding_type: EmbeddingType::Query,
        };
        
        let embedding_response = self.embedding_manager.generate_embedding(embedding_request).await?;
        Ok(embedding_response.embedding)
    }
}