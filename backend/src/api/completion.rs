use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::AppState;
use crate::providers::traits::{CompletionRequest, ProviderError};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionApiRequest {
    pub prompt: String,
    pub language: Option<String>,
    pub context: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub provider_preference: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionApiResponse {
    pub completion: String,
    pub provider_used: String,
    pub cost: f64,
    pub response_time_ms: u64,
    pub confidence: f32,
    pub suggestions: Vec<String>,
    pub tokens_used: u64,
}

#[derive(Debug, Deserialize)]
pub struct CompletionQuery {
    pub provider: Option<String>,
    pub model: Option<String>,
}

pub async fn complete_code(
    State(state): State<AppState>,
    Query(query): Query<CompletionQuery>,
    Json(request): Json<CompletionApiRequest>,
) -> Result<Json<CompletionApiResponse>, StatusCode> {
    let start_time = Instant::now();
    
    // Create completion request
    let completion_request = CompletionRequest {
        prompt: request.prompt.clone(),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        model: request.model.or(query.model),
        system_prompt: None,
        context: request.context,
    };
    
    // Select provider
    let provider = if let Some(pref) = request.provider_preference.or(query.provider) {
        pref
    } else {
        // Use router to select optimal provider
        match state.provider_router.select_provider(&completion_request, crate::providers::router::RoutingStrategy::CostOptimized).await {
            Ok(p) => p,
            Err(_) => "ollama".to_string(), // Fallback to local provider
        }
    };
    
    // Calculate estimated cost
    let estimated_cost = state.provider_router
        .calculate_request_cost(&completion_request, &provider)
        .await;
    
    // Make completion request
    match state.provider_router.complete(&provider, completion_request).await {
        Ok(response) => {
            let response_time = start_time.elapsed().as_millis() as u64;
            let tokens_used = estimate_tokens(&request.prompt, &response.choices.first().map(|c| &c.text).unwrap_or(&String::new()));
            
            // Update metrics
            state.provider_router.update_metrics(
                &provider,
                true,
                response_time,
                tokens_used,
                estimated_cost,
            ).await;
            
            Ok(Json(CompletionApiResponse {
                completion: response.choices.first().map(|c| c.text.clone()).unwrap_or_default(),
                provider_used: provider,
                cost: estimated_cost,
                response_time_ms: response_time,
                confidence: 0.8, // Default confidence
                suggestions: response.choices.iter().skip(1).map(|c| c.text.clone()).collect(),
                tokens_used,
            }))
        }
        Err(e) => {
            let response_time = start_time.elapsed().as_millis() as u64;
            
            // Update metrics for failed request
            state.provider_router.update_metrics(
                &provider,
                false,
                response_time,
                0,
                0.0,
            ).await;
            
            eprintln!("Completion error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn estimate_tokens(prompt: &str, completion: &str) -> u64 {
    // Rough estimation: 4 characters per token
    ((prompt.len() + completion.len()) / 4) as u64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisApiRequest {
    pub code: String,
    pub language: String,
    pub analysis_type: String,
    pub provider_preference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisApiResponse {
    pub analysis_type: String,
    pub findings: Vec<String>,
    pub summary: String,
    pub confidence_score: f32,
    pub suggestions: Vec<String>,
    pub provider_used: String,
    pub response_time_ms: u64,
}

pub async fn analyze_code(
    State(state): State<AppState>,
    Json(request): Json<AnalysisApiRequest>,
) -> Result<Json<AnalysisApiResponse>, StatusCode> {
    let start_time = Instant::now();
    
    // Parse analysis type
    let analysis_type = match request.analysis_type.to_lowercase().as_str() {
        "security" => crate::providers::traits::AnalysisType::Security,
        "performance" => crate::providers::traits::AnalysisType::Performance,
        "quality" => crate::providers::traits::AnalysisType::Quality,
        "bugs" => crate::providers::traits::AnalysisType::Bugs,
        "suggestions" => crate::providers::traits::AnalysisType::Suggestions,
        "documentation" => crate::providers::traits::AnalysisType::Documentation,
        "testing" => crate::providers::traits::AnalysisType::Testing,
        _ => crate::providers::traits::AnalysisType::Quality,
    };
    
    let analysis_request = crate::providers::traits::AnalysisRequest {
        code: request.code,
        language: request.language,
        analysis_type: analysis_type.clone(),
        context: None,
    };
    
    // Select provider
    let provider = request.provider_preference.unwrap_or_else(|| "openrouter".to_string());
    
    match state.provider_router.analyze(&provider, analysis_request).await {
        Ok(response) => {
            let response_time = start_time.elapsed().as_millis() as u64;
            
            Ok(Json(AnalysisApiResponse {
                analysis_type: format!("{:?}", analysis_type),
                findings: response.findings,
                summary: response.summary,
                confidence_score: response.confidence_score,
                suggestions: response.suggestions,
                provider_used: provider,
                response_time_ms: response_time,
            }))
        }
        Err(e) => {
            eprintln!("Analysis error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}