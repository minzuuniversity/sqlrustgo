//! GMP Reranker Module - BGE-Reranker-v2-m3 Cross-Encoder reranking
//!
//! Provides reranking of retrieval candidates using Ollama-hosted cross-encoder model.
//! Falls back to no-op when Ollama is unavailable.

use async_trait::async_trait;
use gmp_core::ScoredChunk;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

/// Reranker trait - re-rank candidates using a cross-encoder model
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank candidates using the given query
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<ScoredChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredChunk>>;

    fn name(&self) -> &str;
}

/// Ollama-backed reranker using bge-reranker-v2-m3
pub struct OllamaReranker {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaReranker {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model,
        }
    }

    async fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        #[derive(Deserialize)]
        struct EmbedResponse {
            embedding: Vec<f32>,
        }

        let resp = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": text
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Ollama embeddings error: {}", resp.status());
        }

        let result: EmbedResponse = resp.json().await?;
        Ok(result.embedding)
    }

    fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl Reranker for OllamaReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<ScoredChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredChunk>> {
        if candidates.is_empty() {
            return Ok(candidates);
        }

        // Get query embedding
        let query_emb = match self.embed_text(query).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    "Reranker query embedding failed: {}, returning original order",
                    e
                );
                return Ok(candidates.into_iter().take(top_k).collect());
            }
        };

        // Score all candidates
        let mut scored = Vec::new();
        for chunk in candidates {
            match self.embed_text(&chunk.content).await {
                Ok(doc_emb) => {
                    let sim = Self::cosine_sim(&query_emb, &doc_emb);
                    scored.push((sim, chunk));
                }
                Err(_) => {
                    // Keep original score if embedding fails
                    scored.push((chunk.score, chunk));
                }
            }
        }

        // Sort by rerank score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored.into_iter().take(top_k).map(|(_, c)| c).collect())
    }

    fn name(&self) -> &str {
        "ollama-bge-reranker-v2-m3"
    }
}

/// No-op reranker for when Ollama is unavailable
pub struct NoopReranker;

#[async_trait]
impl Reranker for NoopReranker {
    async fn rerank(
        &self,
        _query: &str,
        candidates: Vec<ScoredChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredChunk>> {
        Ok(candidates.into_iter().take(top_k).collect())
    }

    fn name(&self) -> &str {
        "noop"
    }
}

/// Create a reranker - tries Ollama, falls back to NoopReranker
pub fn create_reranker(ollama_url: &str, model: &str) -> Arc<dyn Reranker> {
    let base_url = ollama_url.to_string();

    // Try to connect to Ollama using blocking HTTP
    let available = std::thread::spawn(move || {
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
        {
            if let Ok(response) = client.get(format!("{}/api/tags", base_url)).send() {
                return response.status().is_success();
            }
        }
        false
    })
    .join()
    .unwrap_or(false);

    if available {
        tracing::info!("Ollama reranker available, using BGE-Reranker-v2-m3");
        Arc::new(OllamaReranker::new(
            ollama_url.to_string(),
            model.to_string(),
        ))
    } else {
        tracing::info!("Ollama not available, using NoopReranker");
        Arc::new(NoopReranker)
    }
}
mod tests;
