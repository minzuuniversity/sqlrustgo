//! AI Chat SSE Endpoints for GMP Platform
//!
//! Provides Server-Sent Events (SSE) streaming endpoint for LLM chat responses.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlrustgo_llm::{CancellationToken, ChatMessage, LlmClient, RetrievedDocument};
use std::sync::Arc;

/// AI Chat SSE endpoint configuration
pub struct AiChatConfig {
    pub llm_client: Arc<dyn LlmClient>,
}

impl AiChatConfig {
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self { llm_client }
    }
}

/// Stream chat request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default)]
    pub contexts: Vec<RetrievedDocument>,
    #[serde(default)]
    pub instructions: Option<String>,
}

fn default_model() -> String { "qwen2.5:14b".to_string() }
fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> usize { 2048 }

/// SSE event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum SseEvent {
    #[serde(rename = "content")]
    Content { content: String, done: bool },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "final")]
    Final { answer: String, citations: Vec<sqlrustgo_llm::Citation> },
}

/// POST /api/v1/ai/chat/stream - SSE streaming chat endpoint
pub async fn chat_stream(
    config: web::Data<AiChatConfig>,
    body: web::Json<StreamChatRequest>,
) -> HttpResponse {
    let cancellation = CancellationToken::new();

    let result = config
        .llm_client
        .chat_stream(
            &body.messages,
            body.temperature,
            body.max_tokens,
            cancellation,
        )
        .await;

    match result {
        Ok(answer) => {
            HttpResponse::Ok().json(serde_json::json!({
                "t": "final",
                "answer": answer.answer,
                "citations": answer.citations,
                "confidence": answer.confidence,
                "needs_human_review": answer.needs_human_review,
            }))
        }
        Err(e) => {
            HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "t": "error",
                "code": "LLM_ERROR",
                "message": e.to_string(),
            }))
        }
    }
}

/// Register AI chat routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/ai")
            .route("/chat/stream", web::post().to(chat_stream)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_llm::NoopLlmClient;

    #[actix_web::test]
    async fn test_chat_stream_noop() {
        let client = Arc::new(NoopLlmClient {}) as Arc<dyn LlmClient>;
        let config = web::Data::new(AiChatConfig::new(client));

        let request = StreamChatRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            model: "qwen2.5:14b".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            contexts: vec![],
            instructions: None,
        };

        let response = chat_stream(config, web::Json(request)).await;
        assert!(response.status().is_success());
    }
}