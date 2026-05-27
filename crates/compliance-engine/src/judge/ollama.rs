//! Ollama HTTP client for LLM inference
//!
//! 实现真实 Ollama REST API 调用，支持本地 LLM 集成。

use super::agent::{OllamaClient, OllamaError, OllamaResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 基于 reqwest 的 Ollama HTTP 客户端
#[cfg(feature = "ollama")]
#[derive(Debug, Clone)]
pub struct OllamaHttpClient {
    /// Ollama 服务器基础 URL（如 http://localhost:11434）
    base_url: String,
    /// HTTP 客户端（带连接池）
    client: reqwest::Client,
    /// 请求超时时间（毫秒）
    timeout_ms: u64,
}

#[cfg(feature = "ollama")]
impl OllamaHttpClient {
    /// 创建新的 Ollama HTTP 客户端
    ///
    /// # Arguments
    /// * `base_url` - Ollama 服务器地址，如 "http://localhost:11434"
    /// * `timeout_ms` - 请求超时时间（毫秒），建议 < 500ms
    pub fn new(base_url: &str, timeout_ms: u64) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(timeout_ms))
                .build()
                .expect("Failed to create HTTP client"),
            timeout_ms,
        }
    }

    /// 使用默认超时（450ms，为 judgment 500ms 预算留出余量）
    pub fn new_default(base_url: &str) -> Self {
        Self::new(base_url, 450)
    }
}

#[cfg(feature = "ollama")]
#[async_trait]
impl OllamaClient for OllamaHttpClient {
    async fn chat(&self, prompt: &str, model: &str) -> Result<OllamaResponse, OllamaError> {
        #[derive(Serialize)]
        struct ChatRequest<'a> {
            model: &'a str,
            prompt: &'a str,
            stream: bool,
            options: ChatOptions,
        }

        #[derive(Serialize)]
        struct ChatOptions {
            temperature: f32,
            num_predict: usize,
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            response: String,
            #[serde(default)]
            model: String,
        }

        // 构造请求体
        let request = ChatRequest {
            model,
            prompt,
            stream: false,
            options: ChatOptions {
                temperature: 0.3,
                num_predict: 256,
            },
        };

        // 发送 POST 请求到 Ollama API
        let url = format!("{}/api/generate", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    OllamaError::Timeout
                } else if e.is_connect() {
                    OllamaError::Network(format!(
                        "Cannot connect to Ollama at {}: {}",
                        self.base_url, e
                    ))
                } else {
                    OllamaError::Network(e.to_string())
                }
            })?;

        // 解析响应
        let result: ChatResponse = resp.json().await.map_err(|e| {
            OllamaError::InvalidResponse(format!("Failed to parse Ollama response: {}", e))
        })?;

        // 检查响应是否有效
        if result.response.is_empty() {
            return Err(OllamaError::InvalidResponse(
                "Empty response from Ollama".to_string(),
            ));
        }

        Ok(OllamaResponse {
            content: result.response,
            logprob: None, // Ollama generate API 不直接返回 logprob
            model: result.model,
        })
    }
}

/// 基于 sqlrustgo-llm crate 的适配器客户端
///
/// 当启用 `remote_llm` feature 时，复用 crates/llm 的 LlmClient。
#[cfg(feature = "remote_llm")]
pub mod llm_adapter {
    use super::*;
    use crate::judge::agent::OllamaClient;

    /// 适配器：将 sqlrustgo-llm 的 LlmClient 适配为本地 OllamaClient trait
    pub struct LlmClientAdapter {
        inner: Box<dyn crate::llm::LlmClient>,
        default_model: String,
    }

    impl LlmClientAdapter {
        pub fn new(inner: Box<dyn crate::llm::LlmClient>, default_model: &str) -> Self {
            Self {
                inner,
                default_model: default_model.to_string(),
            }
        }
    }

    #[async_trait]
    impl OllamaClient for LlmClientAdapter {
        async fn chat(
            &self,
            prompt: &str,
            _model: &str,
        ) -> Result<OllamaResponse, OllamaError> {
            let messages = vec![crate::llm::ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }];

            let content = self
                .inner
                .chat(&messages, 0.3, 256)
                .await
                .map_err(|e| OllamaError::Network(e.to_string()))?;

            Ok(OllamaResponse {
                content,
                logprob: None,
                model: self.default_model.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 OllamaHttpClient 构造
    #[test]
    fn test_http_client_construction() {
        #[cfg(feature = "ollama")]
        {
            let client = OllamaHttpClient::new("http://localhost:11434", 450);
            assert!(client.base_url.contains("localhost"));
            assert_eq!(client.timeout_ms, 450);
        }
    }

    /// 测试 URL 尾部斜杠清理
    #[test]
    fn test_base_url_trailing_slash() {
        #[cfg(feature = "ollama")]
        {
            let client = OllamaHttpClient::new("http://localhost:11434/", 450);
            assert!(!client.base_url.ends_with('/'));
        }
    }

    /// 测试默认超时构造
    #[test]
    fn test_default_construction() {
        #[cfg(feature = "ollama")]
        {
            let client = OllamaHttpClient::new_default("http://localhost:11434");
            assert_eq!(client.timeout_ms, 450);
        }
    }

    /// 模拟 Ollama API 的 JSON 请求格式验证
    #[test]
    fn test_ollama_request_format() {
        // 验证序列化后的请求结构符合 Ollama API 规范
        #[derive(Serialize)]
        struct TestChatRequest<'a> {
            model: &'a str,
            prompt: &'a str,
            stream: bool,
            options: TestOptions,
        }

        #[derive(Serialize)]
        struct TestOptions {
            temperature: f32,
            num_predict: usize,
        }

        let request = TestChatRequest {
            model: "test-model",
            prompt: "Hello",
            stream: false,
            options: TestOptions {
                temperature: 0.3,
                num_predict: 256,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"test-model\""));
        assert!(json.contains("\"prompt\":\"Hello\""));
        assert!(json.contains("\"stream\":false"));
        assert!(json.contains("\"temperature\":0.3"));
        assert!(json.contains("\"num_predict\":256"));
    }
}
