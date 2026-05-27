//! Reranker for GMP Retrieval
//!
//! LLM-based reranking to improve search result quality through relevance scoring.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;

/// Maximum batch size for reranking
const MAX_BATCH_SIZE: usize = 50;

/// Default model for reranking
const DEFAULT_RERANK_MODEL: &str = "bge-reranker-v2-m3";

/// Errors that can occur during reranking
#[derive(Error, Debug)]
pub enum RerankerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Reranker unavailable: {0}")]
    Unavailable(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Batch size exceeds maximum: {0} > {MAX_BATCH_SIZE}")]
    BatchSizeExceeded(usize),

    #[error("Timeout: reranking exceeded {0}ms")]
    Timeout(u64),
}

/// Result type for reranker operations
pub type RerankerResult<T> = Result<T, RerankerError>;

/// Rerank request
#[derive(Debug, Clone, Serialize)]
pub struct RerankRequest {
    /// The search query
    pub query: String,
    /// Documents to rerank (doc_id, doc_content)
    pub documents: Vec<(String, String)>,
    /// Number of results to return
    pub top_n: usize,
}

/// Rerank result for a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// Document ID
    pub doc_id: String,
    /// Relevance score [0.0, 1.0]
    pub score: f32,
}

/// Ollama reranker client trait
#[async_trait]
pub trait OllamaRerankClient: Send + Sync {
    /// Rerank documents given a query
    async fn rerank(
        &self,
        request: &RerankRequest,
        model: &str,
    ) -> RerankerResult<Vec<RerankResult>>;

    /// Check if the reranker is available
    fn is_available(&self) -> bool;
}

/// Ollama-hosted reranker using cross-encoder scoring
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
    /// * `model` - Model name for reranking (e.g., "bge-reranker-v2-m3")
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            client: Client::new(),
        }
    }

    /// Check if Ollama server is available
    fn check_availability(&self) -> bool {
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

    /// Compute reranking scores for query-document pairs
    async fn compute_scores(
        &self,
        query: &str,
        documents: &[(String, String)],
    ) -> RerankerResult<Vec<(String, f32)>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // Use Ollama's generate endpoint with a prompt-based scoring approach
        // since standard embedding models don't provide cross-encoder scores
        let mut results = Vec::new();

        for (doc_id, content) in documents {
            let score = self.score_pair(query, content).await?;
            results.push((doc_id.clone(), score));
        }

        Ok(results)
    }

    /// Score a single query-document pair
    async fn score_pair(&self, query: &str, document: &str) -> RerankerResult<f32> {
        // Use a simple prompt-based scoring approach via Ollama generate
        // In production, you would use a proper cross-encoder model
        #[derive(Serialize)]
        struct GenerateRequest<'a> {
            model: &'a str,
            prompt: &'a str,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct GenerateResponse {
            response: String,
        }

        // Create a scoring prompt
        let scoring_prompt = format!(
            "Query: {}\n\nDocument: {}\n\nDoes the document answer the query? Answer with a score from 0.0 to 1.0 where 1.0 means highly relevant. Only output the numeric score.",
            query, document
        );

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&GenerateRequest {
                model: &self.model,
                prompt: &scoring_prompt,
                stream: false,
            })
            .send()
            .await
            .map_err(|e| RerankerError::Unavailable(e.to_string()))?;

        if resp.status() == 404 {
            return Err(RerankerError::Unavailable("Model not found".to_string()));
        }

        let result: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| RerankerError::InvalidResponse(e.to_string()))?;

        // Parse the score from the response
        let score = self.parse_score(&result.response)?;
        Ok(score)
    }

    /// Parse score from LLM response
    fn parse_score(&self, response: &str) -> RerankerResult<f32> {
        // Try to extract a number from the response
        let text = response.trim();

        // Look for a number pattern
        for token in text.split_whitespace() {
            // Try direct parse
            if let Ok(score) = token.parse::<f32>() {
                if (0.0..=1.0).contains(&score) {
                    return Ok(score);
                }
            }

            // Try parsing with potential trailing punctuation
            let cleaned: String = token
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let cleaned = cleaned.trim_end_matches('.');
            if let Ok(score) = cleaned.parse::<f32>() {
                if (0.0..=1.0).contains(&score) {
                    return Ok(score);
                }
            }
        }

        // Default to 0.5 if no score found
        Ok(0.5)
    }
}

#[async_trait]
impl OllamaRerankClient for OllamaReranker {
    async fn rerank(
        &self,
        request: &RerankRequest,
        _model: &str,
    ) -> RerankerResult<Vec<RerankResult>> {
        if request.documents.is_empty() {
            return Ok(vec![]);
        }

        if request.documents.len() > MAX_BATCH_SIZE {
            return Err(RerankerError::BatchSizeExceeded(request.documents.len()));
        }

        // Compute scores for all documents
        let scores = self
            .compute_scores(&request.query, &request.documents)
            .await?;

        // Convert to RerankResult and sort by score descending
        let mut results: Vec<RerankResult> = scores
            .into_iter()
            .map(|(doc_id, score)| RerankResult { doc_id, score })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate to top_n
        results.truncate(request.top_n);

        Ok(results)
    }

    fn is_available(&self) -> bool {
        self.check_availability()
    }
}

/// No-op reranker that returns documents in original order with uniform scores
pub struct NoopReranker;

impl NoopReranker {
    /// Create a new NoopReranker
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OllamaRerankClient for NoopReranker {
    async fn rerank(
        &self,
        request: &RerankRequest,
        _model: &str,
    ) -> RerankerResult<Vec<RerankResult>> {
        // Return documents with uniform score in original order
        let results: Vec<RerankResult> = request
            .documents
            .iter()
            .take(request.top_n)
            .map(|(doc_id, _)| RerankResult {
                doc_id: doc_id.clone(),
                score: 1.0,
            })
            .collect();

        Ok(results)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for NoopReranker {
    fn default() -> Self {
        Self::new()
    }
}

/// Reranker with fallback support
pub struct RerankerWithFallback {
    primary: Box<dyn OllamaRerankClient>,
    fallback: Box<dyn OllamaRerankClient>,
}

impl RerankerWithFallback {
    /// Create a new RerankerWithFallback
    ///
    /// # Arguments
    /// * `primary` - Primary reranker (e.g., OllamaReranker)
    /// * `fallback` - Fallback reranker (e.g., NoopReranker)
    pub fn new(
        primary: Box<dyn OllamaRerankClient>,
        fallback: Box<dyn OllamaRerankClient>,
    ) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl OllamaRerankClient for RerankerWithFallback {
    async fn rerank(
        &self,
        request: &RerankRequest,
        model: &str,
    ) -> RerankerResult<Vec<RerankResult>> {
        // Try primary first
        if self.primary.is_available() {
            match timeout(
                Duration::from_millis(150), // 150ms timeout for reranking
                self.primary.rerank(request, model),
            )
            .await
            {
                Ok(Ok(results)) => return Ok(results),
                Ok(Err(e)) => {
                    eprintln!("Primary reranker error: {}, falling back", e);
                }
                Err(_) => {
                    eprintln!("Primary reranker timeout, falling back");
                }
            }
        }

        // Fall back to noop reranker
        self.fallback.rerank(request, model).await
    }

    fn is_available(&self) -> bool {
        self.primary.is_available()
    }
}

/// Mock reranker client for testing
#[cfg(test)]
pub struct MockRerankClient {
    pub results: Vec<RerankResult>,
    pub should_fail: bool,
}

#[cfg(test)]
impl MockRerankClient {
    pub fn new(results: Vec<RerankResult>) -> Self {
        Self {
            results,
            should_fail: false,
        }
    }

    pub fn with_failure(self) -> Self {
        Self {
            should_fail: true,
            ..self
        }
    }
}

#[cfg(test)]
#[async_trait]
impl OllamaRerankClient for MockRerankClient {
    async fn rerank(
        &self,
        _request: &RerankRequest,
        _model: &str,
    ) -> RerankerResult<Vec<RerankResult>> {
        if self.should_fail {
            return Err(RerankerError::Unavailable("Mock failure".to_string()));
        }
        Ok(self.results.clone())
    }

    fn is_available(&self) -> bool {
        !self.should_fail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rerank_result_serialization() {
        let result = RerankResult {
            doc_id: "doc1".to_string(),
            score: 0.95,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("0.95"));
    }

    #[test]
    fn test_rerank_request_serialization() {
        let request = RerankRequest {
            query: "test query".to_string(),
            documents: vec![
                ("doc1".to_string(), "content 1".to_string()),
                ("doc2".to_string(), "content 2".to_string()),
            ],
            top_n: 10,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("doc1"));
    }

    #[test]
    fn test_parse_score_direct() {
        let reranker = OllamaReranker::new("http://localhost:11434", DEFAULT_RERANK_MODEL);
        assert_eq!(reranker.parse_score("0.85").unwrap(), 0.85);
    }

    #[test]
    fn test_parse_score_with_text() {
        let reranker = OllamaReranker::new("http://localhost:11434", DEFAULT_RERANK_MODEL);
        assert_eq!(
            reranker
                .parse_score("The relevance score is 0.75.")
                .unwrap(),
            0.75
        );
    }

    #[test]
    fn test_parse_score_default() {
        let reranker = OllamaReranker::new("http://localhost:11434", DEFAULT_RERANK_MODEL);
        assert_eq!(reranker.parse_score("not a number").unwrap(), 0.5);
    }

    #[tokio::test]
    async fn test_noop_reranker_basic() {
        let reranker = NoopReranker;
        let request = RerankRequest {
            query: "test".to_string(),
            documents: vec![
                ("doc1".to_string(), "content 1".to_string()),
                ("doc2".to_string(), "content 2".to_string()),
            ],
            top_n: 10,
        };
        let results = reranker
            .rerank(&request, DEFAULT_RERANK_MODEL)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].doc_id, "doc1");
        assert_eq!(results[0].score, 1.0);
    }

    #[tokio::test]
    async fn test_noop_reranker_top_n() {
        let reranker = NoopReranker;
        let request = RerankRequest {
            query: "test".to_string(),
            documents: vec![
                ("doc1".to_string(), "content 1".to_string()),
                ("doc2".to_string(), "content 2".to_string()),
                ("doc3".to_string(), "content 3".to_string()),
            ],
            top_n: 2,
        };
        let results = reranker
            .rerank(&request, DEFAULT_RERANK_MODEL)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_rerank_client() {
        let mock = MockRerankClient::new(vec![
            RerankResult {
                doc_id: "doc1".to_string(),
                score: 0.9,
            },
            RerankResult {
                doc_id: "doc2".to_string(),
                score: 0.8,
            },
        ]);
        let request = RerankRequest {
            query: "test".to_string(),
            documents: vec![
                ("doc1".to_string(), "content 1".to_string()),
                ("doc2".to_string(), "content 2".to_string()),
            ],
            top_n: 10,
        };
        let results = mock.rerank(&request, DEFAULT_RERANK_MODEL).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].score, 0.9);
    }

    #[tokio::test]
    async fn test_mock_rerank_client_failure() {
        let mock = MockRerankClient::new(vec![]).with_failure();
        let request = RerankRequest {
            query: "test".to_string(),
            documents: vec![("doc1".to_string(), "content 1".to_string())],
            top_n: 10,
        };
        let results = mock.rerank(&request, DEFAULT_RERANK_MODEL).await;
        assert!(results.is_err());
    }

    #[tokio::test]
    async fn test_fallback_reranker_primary_available() {
        let mock_primary = MockRerankClient::new(vec![RerankResult {
            doc_id: "doc1".to_string(),
            score: 0.9,
        }]);
        let fallback = NoopReranker::new();
        let reranker = RerankerWithFallback::new(Box::new(mock_primary), Box::new(fallback));

        let request = RerankRequest {
            query: "test".to_string(),
            documents: vec![("doc1".to_string(), "content 1".to_string())],
            top_n: 10,
        };
        let results = reranker
            .rerank(&request, DEFAULT_RERANK_MODEL)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].score, 0.9); // Should use primary result
    }

    #[tokio::test]
    async fn test_fallback_reranker_primary_unavailable() {
        let mock_primary = MockRerankClient::new(vec![]).with_failure();
        let fallback = NoopReranker::new();
        let reranker = RerankerWithFallback::new(Box::new(mock_primary), Box::new(fallback));

        let request = RerankRequest {
            query: "test".to_string(),
            documents: vec![("doc1".to_string(), "content 1".to_string())],
            top_n: 10,
        };
        let results = reranker
            .rerank(&request, DEFAULT_RERANK_MODEL)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].score, 1.0); // Should use fallback result with uniform score
    }

    #[test]
    fn test_noop_reranker_availability() {
        let reranker = NoopReranker;
        assert!(reranker.is_available());
    }
}
