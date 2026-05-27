//! 生产级 Ollama LLM 客户端 — 连接 192.168.0.252:11434
//!
//! 使用 qwen3:8b-gmp 模型

use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use sqlrustgo_compliance_engine::judge::agent::{
    OllamaError, OllamaResponse,
};
use crate::ai::report::translator::TranslateOllamaClient;

/// 连接 192.168.0.252 Ollama 的生产客户端
pub struct OllamaTranslateClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaTranslateClient {
    pub fn new(model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest Client");

        Self {
            client,
            base_url: "http://192.168.0.252:11434".to_string(),
            model: model.to_string(),
        }
    }

    pub fn with_url(base_url: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest Client");

        Self {
            client,
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl TranslateOllamaClient for OllamaTranslateClient {
    async fn chat(&self, prompt: &str, _model: &str) -> Result<OllamaResponse, OllamaError> {
        let request_body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.3,
                "num_predict": 2048
            }
        });

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OllamaError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OllamaError::Network(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OllamaError::Network(e.to_string()))?;

        let content = json["response"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(OllamaResponse {
            content,
            logprob: None,
            model: self.model.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 Ollama 运行在 192.168.0.252:11434
    async fn test_ollama_connection() {
        let client = OllamaTranslateClient::new("qwen3:8b-gmp");
        let result = client
            .chat("Say exactly: hello", "qwen3:8b-gmp")
            .await;

        match result {
            Ok(r) => println!("SUCCESS: {}", r.content),
            Err(e) => println!("ERROR: {}", e),
        }
    }
}