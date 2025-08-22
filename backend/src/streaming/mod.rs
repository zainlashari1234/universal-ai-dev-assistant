use anyhow::Result;
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        Response,
    },
    Json,
};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::{auth::AuthContext, providers::traits::CompletionRequest, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub language: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub stream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "start")]
    Start {
        stream_id: String,
        provider: String,
        model: String,
        estimated_tokens: Option<u32>,
    },
    #[serde(rename = "chunk")]
    Chunk {
        stream_id: String,
        content: String,
        tokens_used: Option<u32>,
        finish_reason: Option<String>,
    },
    #[serde(rename = "progress")]
    Progress {
        stream_id: String,
        percentage: f32,
        tokens_generated: u32,
        estimated_total: Option<u32>,
    },
    #[serde(rename = "metadata")]
    Metadata {
        stream_id: String,
        provider_latency: Option<u64>,
        cost_estimate: Option<f64>,
        quality_score: Option<f32>,
    },
    #[serde(rename = "complete")]
    Complete {
        stream_id: String,
        total_tokens: u32,
        total_cost: f64,
        completion_time: u64,
        quality_metrics: QualityMetrics,
    },
    #[serde(rename = "error")]
    Error {
        stream_id: String,
        error: String,
        retry_after: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub coherence_score: f32,
    pub relevance_score: f32,
    pub code_quality_score: Option<f32>,
    pub security_score: Option<f32>,
}

pub struct StreamingManager {
    active_streams: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, StreamInfo>>>,
}

#[derive(Debug, Clone)]
struct StreamInfo {
    user_id: Uuid,
    provider: String,
    model: String,
    start_time: std::time::Instant,
    tokens_generated: u32,
    estimated_cost: f64,
}

impl StreamingManager {
    pub fn new() -> Self {
        Self {
            active_streams: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn create_stream(
        &self,
        state: &AppState,
        auth_context: &AuthContext,
        request: StreamingRequest,
    ) -> Result<impl Stream<Item = Result<Event, Infallible>>> {
        let stream_id = request.stream_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let provider = request.provider.as_deref().unwrap_or("openrouter");
        let model = request.model.as_deref().unwrap_or("gpt-4o-mini");

        // Validate API key
        let api_key = state
            .api_key_manager
            .get_api_key(auth_context.user.id, provider)
            .await?;

        if api_key.is_none() {
            return Err(anyhow::anyhow!("No API key found for provider: {}", provider));
        }

        // Create channel for streaming
        let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);

        // Store stream info
        {
            let mut streams = self.active_streams.lock().unwrap();
            streams.insert(
                stream_id.clone(),
                StreamInfo {
                    user_id: auth_context.user.id,
                    provider: provider.to_string(),
                    model: model.to_string(),
                    start_time: std::time::Instant::now(),
                    tokens_generated: 0,
                    estimated_cost: 0.0,
                },
            );
        }

        // Start streaming task
        let stream_id_clone = stream_id.clone();
        let state_clone = state.clone();
        let auth_context_clone = auth_context.clone();
        let request_clone = request.clone();
        let streams_clone = self.active_streams.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::handle_streaming(
                stream_id_clone,
                state_clone,
                auth_context_clone,
                request_clone,
                tx,
                streams_clone,
            )
            .await
            {
                tracing::error!("Streaming error: {}", e);
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    async fn handle_streaming(
        stream_id: String,
        state: AppState,
        auth_context: AuthContext,
        request: StreamingRequest,
        tx: mpsc::Sender<Result<Event, Infallible>>,
        streams: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, StreamInfo>>>,
    ) -> Result<()> {
        let provider = request.provider.as_deref().unwrap_or("openrouter");
        let model = request.model.as_deref().unwrap_or("gpt-4o-mini");

        // Send start event
        let start_event = StreamEvent::Start {
            stream_id: stream_id.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            estimated_tokens: Some(request.max_tokens.unwrap_or(1000)),
        };

        Self::send_event(&tx, &stream_id, start_event).await?;

        // Create completion request
        let completion_request = CompletionRequest {
            prompt: request.prompt,
            model: request.model,
            provider: request.provider,
            language: request.language,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            system_prompt: request.system_prompt,
            stream: Some(true),
        };

        // Start streaming from provider
        match Self::stream_from_provider(&state, &completion_request, &stream_id, &tx, &streams).await {
            Ok(_) => {
                // Send completion event
                let completion_event = StreamEvent::Complete {
                    stream_id: stream_id.clone(),
                    total_tokens: Self::calculate_total_tokens(&streams, &stream_id),
                    total_cost: Self::calculate_total_cost(&streams, &stream_id),
                    completion_time: Self::calculate_completion_time(&streams, &stream_id)
                    quality_metrics: QualityMetrics {
                        coherence_score: 0.9,
                        relevance_score: 0.85,
                        code_quality_score: Some(0.8),
                        security_score: Some(0.95),
                    },
                };
                Self::send_event(&tx, &stream_id, completion_event).await?;
            }
            Err(e) => {
                // Send error event
                let error_event = StreamEvent::Error {
                    stream_id: stream_id.clone(),
                    error: e.to_string(),
                    retry_after: Some(5),
                };
                Self::send_event(&tx, &stream_id, error_event).await?;
            }
        }

        // Clean up stream info
        {
            let mut streams_guard = streams.lock().unwrap();
            streams_guard.remove(&stream_id);
        }

        Ok(())
    }

    async fn stream_from_provider(
        state: &AppState,
        request: &CompletionRequest,
        stream_id: &str,
        tx: &mpsc::Sender<Result<Event, Infallible>>,
        streams: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, StreamInfo>>>,
    ) -> Result<()> {
        // Simulate streaming for now - in real implementation, this would call the actual provider
        let chunks = vec![
            "Here's",
            " a",
            " streaming",
            " response",
            " that",
            " demonstrates",
            " real-time",
            " AI",
            " completion",
            " with",
            " progress",
            " tracking",
            " and",
            " quality",
            " metrics.",
        ];

        let total_chunks = chunks.len();
        
        for (i, chunk) in chunks.iter().enumerate() {
            // Send chunk event
            let chunk_event = StreamEvent::Chunk {
                stream_id: stream_id.to_string(),
                content: chunk.to_string(),
                tokens_used: Some(1),
                finish_reason: if i == total_chunks - 1 { Some("stop".to_string()) } else { None },
            };
            Self::send_event(tx, stream_id, chunk_event).await?;

            // Send progress event
            let progress = (i + 1) as f32 / total_chunks as f32 * 100.0;
            let progress_event = StreamEvent::Progress {
                stream_id: stream_id.to_string(),
                percentage: progress,
                tokens_generated: (i + 1) as u32,
                estimated_total: Some(total_chunks as u32),
            };
            Self::send_event(tx, stream_id, progress_event).await?;

            // Send metadata periodically
            if i % 5 == 0 {
                let metadata_event = StreamEvent::Metadata {
                    stream_id: stream_id.to_string(),
                    provider_latency: Some(50 + i as u64 * 10),
                    cost_estimate: Some(0.0001 * (i + 1) as f64),
                    quality_score: Some(0.85 + (i as f32 * 0.01)),
                };
                Self::send_event(tx, stream_id, metadata_event).await?;
            }

            // Simulate realistic delay
            tokio::time::sleep(Duration::from_millis(100 + i as u64 * 50)).await;
        }

        Ok(())
    }

    async fn send_event(
        tx: &mpsc::Sender<Result<Event, Infallible>>,
        stream_id: &str,
        event: StreamEvent,
    ) -> Result<()> {
        let event_data = serde_json::to_string(&event)?;
        let sse_event = Event::default()
            .id(stream_id)
            .event(match &event {
                StreamEvent::Start { .. } => "start",
                StreamEvent::Chunk { .. } => "chunk",
                StreamEvent::Progress { .. } => "progress",
                StreamEvent::Metadata { .. } => "metadata",
                StreamEvent::Complete { .. } => "complete",
                StreamEvent::Error { .. } => "error",
            })
            .data(event_data);

        tx.send(Ok(sse_event)).await.map_err(|_| anyhow::anyhow!("Failed to send event"))?;
        Ok(())
    }

    pub fn get_active_streams(&self) -> Vec<String> {
        let streams = self.active_streams.lock().unwrap();
        streams.keys().cloned().collect()
    }

    pub fn get_stream_info(&self, stream_id: &str) -> Option<StreamInfo> {
        let streams = self.active_streams.lock().unwrap();
        streams.get(stream_id).cloned()
    }
}

// Handler for streaming endpoint
pub async fn streaming_completion_handler(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Json(request): Json<StreamingRequest>,
) -> Response {
    let streaming_manager = StreamingManager::new();
    
    match streaming_manager.create_stream(&state, &auth_context, request).await {
        Ok(stream) => {
            let sse = Sse::new(stream).keep_alive(
                KeepAlive::new()
                    .interval(Duration::from_secs(1))
                    .text("keep-alive-text"),
            );
            sse.into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create stream: {}", e);
            axum::response::Response::builder()
                .status(500)
                .body(format!("Streaming error: {}", e).into())
                .unwrap()
        }
    }
}