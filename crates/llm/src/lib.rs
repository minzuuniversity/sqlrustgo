//! LLM Chat module for SQLRustGo
//!
//! Provides Ollama Chat Completion interface for RAG-based answer generation.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during LLM operations
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// LLM server unavailable
    #[error("LLM unavailable: {0}")]
    Unavailable(String),

    /// Invalid response from LLM
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Timeout
    #[error("Request timeout")]
    Timeout,
}

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role: "system", "user", or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
}

/// Retrieved document for RAG context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    /// Document ID
    pub doc_id: String,
    /// Document title
    pub title: String,
    /// Chunk content
    pub content: String,
    /// Relevance score
    pub score: f32,
    /// Source channel: "vector", "fts", "graph", "rule"
    pub source: String,
}

/// Citation in generated answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Referenced document ID
    pub doc_id: String,
    /// Referenced document title
    pub title: String,
    /// Quote from the document
    pub snippet: String,
}

/// Generated answer from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAnswer {
    /// The generated answer text
    pub answer: String,
    /// Citations used in the answer
    pub citations: Vec<Citation>,
    /// Confidence score (0.0-1.0), if available
    pub confidence: Option<f32>,
    /// Whether the answer needs human review
    pub needs_human_review: bool,
}

/// LLM client trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
        max_tokens: usize,
    ) -> LlmResult<String>;

    /// Generate answer based on retrieved context (RAG)
    async fn generate_answer(
        &self,
        question: &str,
        contexts: &[RetrievedDocument],
        instructions: Option<&str>,
    ) -> LlmResult<GeneratedAnswer>;

    /// Name of the LLM
    fn name(&self) -> &str;

    /// Check if the LLM is available
    fn is_available(&self) -> bool;
}

/// Ollama-hosted LLM client
pub struct OllamaLlmClient {
    base_url: String,
    model: String,
    client: Client,
    prompt_template: PromptTemplate,
}

/// RAG prompt template
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// System prompt
    pub system: String,
    /// User prompt template with {contexts} and {question} placeholders
    pub user_template: String,
}

impl Default for PromptTemplate {
    fn default() -> Self {
        Self {
            system: r#"你是 GMP 合规审核 AI 助手。请严格基于提供的材料回答问题。
如果材料不足以回答，请明确说明"当前材料无法回答此问题"。
不要编造任何信息。"#
                .to_string(),
            user_template: r#"## 相关文档
{contexts}

## 问题
{question}

## 回答要求
1. 给出明确答案
2. 在每句话后标注来源文档编号，如 [doc-001]
3. 如有不确定性，在句末标注 [需人工确认]"#
                .to_string(),
        }
    }
}

impl OllamaLlmClient {
    /// Create a new OllamaLLM client
    ///
    /// # Arguments
    /// * `base_url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (e.g., "qwen2.5:14b")
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            client: Client::new(),
            prompt_template: PromptTemplate::default(),
        }
    }

    /// Set custom prompt template
    pub fn with_template(mut self, template: PromptTemplate) -> Self {
        self.prompt_template = template;
        self
    }

    /// Format contexts into a string
    fn format_contexts(&self, contexts: &[RetrievedDocument]) -> String {
        contexts
            .iter()
            .enumerate()
            .map(|(i, ctx)| {
                format!(
                    "[doc-{:03}] {} ({})\n{}\n",
                    i + 1,
                    ctx.title,
                    ctx.source,
                    ctx.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n")
    }

    /// Parse citations from LLM response
    fn parse_citations(&self, answer: &str, contexts: &[RetrievedDocument]) -> Vec<Citation> {
        let mut citations = Vec::new();
        // Look for [doc-NNN] patterns in answer
        for ctx in contexts {
            if answer.contains(&format!("[{}]", ctx.doc_id))
                || answer.contains(&format!("[doc-{:03}]", 0))
            {
                citations.push(Citation {
                    doc_id: ctx.doc_id.clone(),
                    title: ctx.title.clone(),
                    snippet: ctx.content.chars().take(100).collect(),
                });
            }
        }
        citations
    }
}

#[async_trait]
impl LlmClient for OllamaLlmClient {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
        max_tokens: usize,
    ) -> LlmResult<String> {
        #[derive(Serialize)]
        struct ChatRequest<'a> {
            model: &'a str,
            messages: &'a [ChatMessage],
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
            message: ChatMessage,
        }

        let request = ChatRequest {
            model: &self.model,
            messages,
            stream: false,
            options: ChatOptions {
                temperature,
                num_predict: max_tokens,
            },
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: ChatResponse = resp.json().await?;
        Ok(result.message.content)
    }

    async fn generate_answer(
        &self,
        question: &str,
        contexts: &[RetrievedDocument],
        instructions: Option<&str>,
    ) -> LlmResult<GeneratedAnswer> {
        // Build messages
        let system = if let Some(inst) = instructions {
            format!("{}\n\n## 额外指令\n{}", self.prompt_template.system, inst)
        } else {
            self.prompt_template.system.clone()
        };

        let contexts_text = if contexts.is_empty() {
            "(无相关文档)".to_string()
        } else {
            self.format_contexts(contexts)
        };

        let user_prompt = self
            .prompt_template
            .user_template
            .replace("{contexts}", &contexts_text)
            .replace("{question}", question);

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let answer = self.chat(&messages, 0.3, 1024).await?;

        // Check if answer indicates no context
        let needs_human_review =
            contexts.is_empty() || answer.contains("无法回答") || answer.contains("材料不足");

        // Extract citations
        let citations = self.parse_citations(&answer, contexts);

        Ok(GeneratedAnswer {
            answer,
            citations,
            confidence: None,
            needs_human_review,
        })
    }

    fn name(&self) -> &str {
        "ollama-llm"
    }

    fn is_available(&self) -> bool {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();
        if let Some(rt) = rt {
            rt.block_on(async {
                reqwest::get(&format!("{}/api/tags", self.base_url))
                    .await
                    .is_ok()
            })
        } else {
            false
        }
    }
}

/// No-op LLM client that returns a fixed response
pub struct NoopLlmClient;

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn chat(
        &self,
        _messages: &[ChatMessage],
        _temperature: f32,
        _max_tokens: usize,
    ) -> LlmResult<String> {
        Ok("LLM unavailable.".to_string())
    }

    async fn generate_answer(
        &self,
        _question: &str,
        _contexts: &[RetrievedDocument],
        _instructions: Option<&str>,
    ) -> LlmResult<GeneratedAnswer> {
        Ok(GeneratedAnswer {
            answer: "LLM unavailable. Please try again later.".to_string(),
            citations: vec![],
            confidence: None,
            needs_human_review: true,
        })
    }

    fn name(&self) -> &str {
        "noop-llm"
    }

    fn is_available(&self) -> bool {
        true
    }
}

/// Reciprocal Rank Fusion
///
/// Fuses multiple result sets using the RRF formula:
/// score = Σ w_c / (k + rank_c)
///
/// # Arguments
/// * `result_sets` - Each channel's results as (doc_id, original_score) sorted by score descending
/// * `k` - RRF smoothing constant (default 60)
/// * `top_k` - Maximum number of fused results
///
/// # Returns
/// Fused (doc_id, rrf_score) sorted by score descending
pub fn reciprocal_rank_fusion(
    result_sets: &[Vec<(String, f32)>],
    k: f32,
    top_k: usize,
) -> Vec<(String, f32)> {
    if result_sets.is_empty() {
        return Vec::new();
    }

    // Collect all doc_ids and their RRF scores
    let mut doc_scores: HashMap<String, f32> = HashMap::new();

    for (rank_offset, result_set) in result_sets.iter().enumerate() {
        for (rank, (_doc_id, _original_score)) in result_set.iter().enumerate() {
            let zero_based_rank = rank;
            // Weight is 1.0 for all channels by default
            // Could extend to per-channel weights
            let rrf_score = 1.0 / (k + zero_based_rank as f32);
            let doc_id = &_doc_id;
            *doc_scores.entry(doc_id.to_string()).or_insert(0.0) += rrf_score;
        }
    }

    // Sort by RRF score descending
    let mut sorted: Vec<_> = doc_scores.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(top_k);
    sorted
}

/// Weighted Reciprocal Rank Fusion
///
/// RRF with per-channel weights for boosting certain channels (e.g., rules).
pub fn weighted_reciprocal_rank_fusion(
    result_sets: &[(Vec<(String, f32)>, f32)], // (results, weight)
    k: f32,
    top_k: usize,
) -> Vec<(String, f32)> {
    if result_sets.is_empty() {
        return Vec::new();
    }

    let mut doc_scores: HashMap<String, f32> = HashMap::new();

    for (result_set, weight) in result_sets {
        for (rank, (doc_id, _original_score)) in result_set.iter().enumerate() {
            let zero_based_rank = rank;
            let rrf_score = weight / (k + zero_based_rank as f32);
            *doc_scores.entry(doc_id.to_string()).or_insert(0.0) += rrf_score;
        }
    }

    let mut sorted: Vec<_> = doc_scores.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(top_k);
    sorted
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_basic() {
        // Channel 1: doc_a (score 1.0), doc_b (score 0.8)
        // Channel 2: doc_b (score 0.9), doc_a (score 0.7)
        let ch1 = vec![
            ("doc_a".to_string(), 1.0),
            ("doc_b".to_string(), 0.8),
        ];
        let ch2 = vec![
            ("doc_b".to_string(), 0.9),
            ("doc_a".to_string(), 0.7),
        ];

        let result = reciprocal_rank_fusion(&[ch1, ch2], 60.0, 10);

        // doc_a: 1/(60+0) + 1/(60+1) = 0.01639 + 0.01613 = 0.03252
        // doc_b: 1/(60+1) + 1/(60+0) = 0.01613 + 0.01639 = 0.03252
        // They should tie or be very close
        assert!(result.len() <= 2);
    }

    #[test]
    fn test_rrf_single_channel() {
        let ch1 = vec![
            ("doc_a".to_string(), 1.0),
            ("doc_b".to_string(), 0.8),
            ("doc_c".to_string(), 0.6),
        ];

        let result = reciprocal_rank_fusion(&[ch1], 60.0, 2);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "doc_a");
        assert_eq!(result[1].0, "doc_b");
    }

    #[test]
    fn test_rrf_empty_channel() {
        let ch1 = vec![
            ("doc_a".to_string(), 1.0),
            ("doc_b".to_string(), 0.8),
        ];
        let ch2: Vec<(String, f32)> = vec![];

        let result = reciprocal_rank_fusion(&[ch1, ch2], 60.0, 10);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "doc_a");
    }

    #[test]
    fn test_rrf_weighted() {
        let ch1 = vec![("doc_a".to_string(), 1.0)];
        let ch2 = vec![("doc_a".to_string(), 1.0)];

        let result = weighted_reciprocal_rank_fusion(
            &[(ch1, 1.0), (ch2, 2.0)],
            60.0,
            10,
        );

        // doc_a: 1.0/(60+0) + 2.0/(60+0) = 0.01667 + 0.03333 = 0.05
        assert!((result[0].1 - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_noop_llm() {
        let llm = NoopLlmClient;
        assert_eq!(llm.name(), "noop-llm");
        assert!(llm.is_available());
    }
}
