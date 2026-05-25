//! Streaming chat types for SSE

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Stream chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChatRequest {
    pub messages: Vec<crate::ChatMessage>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default)]
    pub contexts: Vec<crate::RetrievedDocument>,
    #[serde(default)]
    pub instructions: Option<String>,
}

fn default_model() -> String { "qwen2.5:14b".to_string() }
fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> usize { 2048 }

/// Stream response event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum StreamEvent {
    #[serde(rename = "content")]
    Content { content: String, done: bool },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "final")]
    Final { answer: String, citations: Vec<crate::Citation> },
}

/// Cancellation token for aborting streaming
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: std::sync::Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self { cancelled: std::sync::Arc::new(AtomicBool::new(false)) }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}