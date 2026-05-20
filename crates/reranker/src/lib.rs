//! Reranker module for SQLRustGo
//!
//! Provides cross-encoder reranking capabilities using BGE-Reranker-v2-m3 via Ollama.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during reranking
#[derive(Error, Debug)]
pub enum RerankerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Reranker unavailable (e.g., Ollama server not running)
    #[error("Reranker unavailable: {0}")]
    Unavailable(String),

    /// Invalid response from reranker
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type for reranker operations
pub type RerankerResult<T> = Result<T, RerankerError>;

/// Reranker trait for cross-encoder reranking
///
/// Implementors provide reranking of document candidates given a query.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank documents given a query
    ///
    /// # Arguments
    /// * `query` - The search query
    /// * `documents` - Vector of (doc_id, doc_content) pairs
    /// * `top_k` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of (doc_id, relevance_score) sorted by score descending
    async fn rerank(
        &self,
        query: &str,
        documents: &[(String, String)],
        top_k: usize,
    ) -> RerankerResult<Vec<(String, f32)>>;

    /// Name of the reranker
    fn name(&self) -> &str;

    /// Check if the reranker is available (e.g., Ollama is running)
    fn is_available(&self) -> bool;
}

/// Ollama-hosted cross-encoder reranker using embeddings + cosine similarity
///
/// Uses Ollama's embedding API to compute query and document embeddings,
/// then calculates cosine similarity for reranking.
pub struct OllamaReranker {
    base_url: String,
    model: String,
    client: Client,
}

impl OllamaReranker {
    /// Create a new OllamaReranker
    ///
    /// # Arguments
    /// * `base_url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name for embeddings (e.g., "bge-reranker-v2-m3" or "nomic-embed-text")
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }

    /// Compute embedding for a single text using Ollama API
    async fn get_embedding(&self, text: &str) -> RerankerResult<Vec<f32>> {
        #[derive(Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            prompt: &'a str,
        }

        let resp = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&EmbedRequest {
                model: &self.model,
                prompt: text,
            })
            .send()
            .await?;

        #[derive(Deserialize)]
        struct EmbedResponse {
            embedding: Vec<f32>,
        }

        let result: EmbedResponse = resp.json().await?;
        Ok(result.embedding)
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

#[async_trait]
impl Reranker for OllamaReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: &[(String, String)],
        top_k: usize,
    ) -> RerankerResult<Vec<(String, f32)>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        // Get query embedding
        let query_emb = self.get_embedding(query).await?;

        // Get document embeddings and compute scores
        let mut scored: Vec<(String, f32)> = Vec::new();

        for (doc_id, content) in documents {
            match self.get_embedding(content).await {
                Ok(doc_emb) => {
                    let score = Self::cosine_similarity(&query_emb, &doc_emb);
                    scored.push((doc_id.clone(), score));
                }
                Err(e) => {
                    // Log error but continue with other documents
                    eprintln!("Warning: failed to get embedding for doc {}: {}", doc_id, e);
                }
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored)
    }

    fn name(&self) -> &str {
        "ollama-reranker"
    }

    fn is_available(&self) -> bool {
        // Synchronous check using blocking reqwest
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

/// No-op reranker that returns documents in original order
///
/// Use this as a fallback when no Ollama server is available.
pub struct NoopReranker;

#[async_trait]
impl Reranker for NoopReranker {
    async fn rerank(
        &self,
        _query: &str,
        documents: &[(String, String)],
        top_k: usize,
    ) -> RerankerResult<Vec<(String, f32)>> {
        // Return documents with uniform score in original order
        let result: Vec<(String, f32)> = documents
            .iter()
            .take(top_k)
            .map(|(id, _)| (id.clone(), 1.0))
            .collect();
        Ok(result)
    }

    fn name(&self) -> &str {
        "noop-reranker"
    }

    fn is_available(&self) -> bool {
        true // Always available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_reranker_basic() {
        let reranker = NoopReranker;
        let docs = vec![
            ("doc1".to_string(), "content 1".to_string()),
            ("doc2".to_string(), "content 2".to_string()),
        ];
        let result = reranker.rerank("query", &docs, 10).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "doc1");
        assert_eq!(result[1].0, "doc2");
    }

    #[tokio::test]
    async fn test_noop_reranker_empty() {
        let reranker = NoopReranker;
        let result = reranker.rerank("query", &[], 10).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_noop_reranker_top_k() {
        let reranker = NoopReranker;
        let docs: Vec<(String, String)> = (0..20).map(|i| (format!("doc{}", i), format!("content {}", i))).collect();
        let result = reranker.rerank("query", &docs, 5).await.unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((OllamaReranker::cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((OllamaReranker::cosine_similarity(&a, &c) - 0.0).abs() < 0.0001);

        let d = vec![0.707, 0.707, 0.0];
        let e = vec![0.707, 0.707, 0.0];
        let sim = OllamaReranker::cosine_similarity(&d, &e);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let empty: Vec<f32> = vec![];
        assert_eq!(OllamaReranker::cosine_similarity(&empty, &empty), 0.0);
    }

    #[test]
    fn test_noop_reranker_name() {
        let reranker = NoopReranker;
        assert_eq!(reranker.name(), "noop-reranker");
        assert!(reranker.is_available());
    }
}
