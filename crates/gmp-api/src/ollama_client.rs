//! Ollama client adapter for compliance engine
//!
//! Implements the OllamaClient trait from compliance-engine to bridge
//! with the HTTP-based Ollama API.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use sqlrustgo_compliance_engine::judge::agent::{
    OllamaClient as EngineOllamaClient, OllamaError as EngineOllamaError,
    OllamaResponse as EngineOllamaResponse,
};

/// Ollama API client errors
#[derive(Error, Debug)]
pub enum OllamaClientError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Ollama error: {0}")]
    Ollama(String),
    #[error("Timeout")]
    Timeout,
}

/// Chat message structure for Ollama API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat request for Ollama API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

/// Chat response from Ollama API
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ChatMessage,
    #[serde(default)]
    pub logprob: Option<f64>,
}

/// Ollama client implementation for gmp-api
#[derive(Clone)]
pub struct OllamaClientImpl {
    base_url: String,
    client: Client,
}

impl OllamaClientImpl {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl EngineOllamaClient for OllamaClientImpl {
    async fn chat(&self, prompt: &str, model: &str) -> Result<EngineOllamaResponse, EngineOllamaError> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
            .map_err(|e| EngineOllamaError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(EngineOllamaError::InvalidResponse(format!(
                "Status: {}",
                response.status()
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| EngineOllamaError::InvalidResponse(e.to_string()))?;

        Ok(EngineOllamaResponse {
            content: chat_response.message.content,
            logprob: chat_response.logprob.map(|l| l as f32),
            model: chat_response.model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClientImpl::new("http://localhost:11434");
        assert_eq!(client.base_url, "http://localhost:11434");
    }
}
