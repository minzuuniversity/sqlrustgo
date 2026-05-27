//! Ollama LLM integration

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors for Ollama operations
#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("Ollama service unavailable: {0}")]
    ConnectionError(String),
    #[error("Model not found: {0}. Available models: {1:?}")]
    ModelNotFoundError(String, Vec<String>),
    #[error("Request timeout")]
    Timeout,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Ollama chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Ollama chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

/// Ollama chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ChatMessage,
    pub logprob: Option<f64>,
}

/// SSE streaming chunk from Ollama
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub model: String,
    pub done: bool,
    pub message: Option<ChatMessage>,
}

/// Ollama client trait for abstraction and testing
#[async_trait]
pub trait OllamaClient: Send + Sync {
    /// Send a chat request and get a response
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, OllamaError>;

    /// Send a streaming chat request
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<StreamChunk, OllamaError>>, OllamaError>;
}

/// Ollama client implementation using reqwest
#[derive(Clone)]
pub struct OllamaClientImpl {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaClientImpl {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
            .map_err(|e| OllamaError::ConnectionError(e.to_string()))?;

        if response.status() == 404 {
            let available = vec![];
            return Err(OllamaError::ModelNotFoundError(request.model, available));
        }

        if !response.status().is_success() {
            return Err(OllamaError::InvalidResponse(format!(
                "Status: {}",
                response.status()
            )));
        }

        response
            .json::<ChatResponse>()
            .await
            .map_err(|e| OllamaError::InvalidResponse(e.to_string()))
    }

    pub async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<StreamChunk, OllamaError>>, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
            .map_err(|e| OllamaError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OllamaError::InvalidResponse(format!(
                "Status: {}",
                response.status()
            )));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let model = request.model.clone();

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = Vec::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);
                        let mut lines = String::from_utf8_lossy(&buffer).to_string();
                        while let Some(newline_pos) = lines.find('\n') {
                            let line = lines[..newline_pos].trim().to_string();
                            lines = lines[newline_pos + 1..].to_string();

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    let _ = tx
                                        .send(Ok(StreamChunk {
                                            model: model.clone(),
                                            done: true,
                                            message: None,
                                        }))
                                        .await;
                                    return;
                                } else if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data)
                                {
                                    let _ = tx.send(Ok(chunk)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(OllamaError::InvalidResponse(e.to_string())))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Is this operation compliant?".to_string(),
            }],
            stream: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("llama3"));
        assert!(json.contains("Is this operation compliant?"));
    }
}
