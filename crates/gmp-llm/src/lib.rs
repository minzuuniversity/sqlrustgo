//! GMP LLM Module - Chat Completion + Intent Classification
//! 
//! Provides Ollama Chat Completion interface for RAG-based Q&A and
//! intent classification to route queries to appropriate channels.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: Client,
    base_url: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model,
        }
    }

    pub fn default_with_model(model: String) -> Self {
        Self::new(DEFAULT_OLLAMA_URL.to_string(), model)
    }

    /// Send a chat completion request to Ollama
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: Default::default(),
        };

        let url = format!("{}/api/chat", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama returned error: {}", response.status());
        }

        let chat_resp: ChatResponse = response.json().await?;
        Ok(chat_resp)
    }

    /// Classify user intent into channel categories
    pub async fn classify_intent(&self, query: &str) -> Result<Intent> {
        let system_prompt = r#"You are an intent classifier for a GMP compliance retrieval system.
Classify the user query into ONE of these categories:
- "rule": Questions about GMP regulations, rules, compliance requirements
- "factual": Questions about specific facts, numbers, dates in GMP documentation  
- "procedural": Questions about procedures, processes, how-to steps
- "clarification": Requests for explanation or clarification of concepts
- "comparison": Questions comparing different GMP requirements or versions
- "general": General questions that don't fit above categories

Respond with ONLY the category name, nothing else."#;

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: query.to_string() },
        ];

        let response = self.chat(messages).await?;
        let intent_str = response.message.content.trim().to_lowercase();
        
        let intent = match intent_str.as_str() {
            "rule" => Intent::Rule,
            "factual" => Intent::Factual,
            "procedural" => Intent::Procedural,
            "clarification" => Intent::Clarification,
            "comparison" => Intent::Comparison,
            _ => Intent::General,
        };
        
        Ok(intent)
    }

    /// Route query to best channel based on intent
    pub async fn route_query(&self, query: &str) -> Result<ChannelRoute> {
        let intent = self.classify_intent(query).await?;
        
        let channels = match intent {
            Intent::Rule => vec!["rule".to_string(), "vector".to_string(), "fts".to_string()],
            Intent::Factual => vec!["vector".to_string(), "fts".to_string(), "rule".to_string()],
            Intent::Procedural => vec!["rule".to_string(), "fts".to_string()],
            Intent::Clarification => vec!["vector".to_string(), "rule".to_string()],
            Intent::Comparison => vec!["rule".to_string(), "vector".to_string()],
            Intent::General => vec!["all".to_string()],
        };

        Ok(ChannelRoute { intent, channels })
    }

    /// Generate answer from retrieved context using RAG
    pub async fn generate_answer(
        &self,
        query: &str,
        context_chunks: &[String],
        max_context_tokens: usize,
    ) -> Result<String> {
        // Build context from chunks (truncate to token limit)
        let mut context = String::new();
        let mut total_len = 0;
        for chunk in context_chunks.iter().take(10) {
            if total_len + chunk.len() > max_context_tokens * 4 {
                break;
            }
            context.push_str(chunk);
            context.push_str("\n\n---\n\n");
            total_len += chunk.len();
        }

        let system_prompt = r#"You are a GMP compliance assistant. Answer questions based ONLY on the provided context.
If the context doesn't contain enough information to answer, say so.
Be precise and cite specific GMP requirements when relevant.
Always prefer official GMP terminology."#;

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { 
                role: "user".to_string(), 
                content: format!("Context:\n{}\n\nQuestion: {}", context, query),
            },
        ];

        let response = self.chat(messages).await?;
        Ok(response.message.content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(default)]
    pub options: RequestOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestOptions {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ResponseMessage,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: String,
}

/// User intent categories for channel routing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Rule,
    Factual,
    Procedural,
    Clarification,
    Comparison,
    General,
}

/// Channel routing decision based on intent
#[derive(Debug, Clone)]
pub struct ChannelRoute {
    pub intent: Intent,
    pub channels: Vec<String>,
}

/// Shared LLM client with connection pooling
pub type SharedLlmClient = Arc<RwLock<LlmClient>>;

pub fn new_shared(client: LlmClient) -> SharedLlmClient {
    Arc::new(RwLock::new(client))
}
