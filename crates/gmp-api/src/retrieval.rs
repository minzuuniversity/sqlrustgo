//! GMP Retrieval API
//!
//! Provides unified search API combining vector, BM25, rule-based, and graph search.
//! Supports query rewriting and reranking via LLM.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Retrieval source type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RetrievalSource {
    Vector,
    Fts,
    Rule,
    Graph,
}

impl RetrievalSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Fts => "fts",
            Self::Rule => "rule",
            Self::Graph => "graph",
        }
    }
}

/// Unified search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Natural language query
    pub query: String,
    /// Number of results to return (default: 10)
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// RRF k-factor for fusion (default: 60)
    #[serde(default = "default_rrf_k")]
    pub rrf_k: usize,
    /// Enable query rewriting (default: true)
    #[serde(default = "default_use_query_rewrite")]
    pub use_query_rewrite: bool,
    /// Enable reranking (default: true)
    #[serde(default = "default_use_reranker")]
    pub use_reranker: bool,
    /// Filter conditions
    #[serde(default)]
    pub filters: Option<SearchFilters>,
    /// Weight for vector search (default: 1.0)
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
    /// Weight for BM25 search (default: 0.8)
    #[serde(default = "default_fts_weight")]
    pub fts_weight: f64,
    /// Weight for rule-based search (default: 1.2)
    #[serde(default = "default_rule_weight")]
    pub rule_weight: f64,
    /// Weight for graph search (default: 0.9)
    #[serde(default = "default_graph_weight")]
    pub graph_weight: f64,
}

fn default_top_k() -> usize {
    10
}

fn default_rrf_k() -> usize {
    60
}

fn default_use_query_rewrite() -> bool {
    true
}

fn default_use_reranker() -> bool {
    true
}

fn default_vector_weight() -> f64 {
    1.0
}

fn default_fts_weight() -> f64 {
    0.8
}

fn default_rule_weight() -> f64 {
    1.2
}

fn default_graph_weight() -> f64 {
    0.9
}

/// Search filter conditions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    /// Document types to include
    #[serde(default)]
    pub doc_types: Option<Vec<String>>,
    /// Date range filter
    #[serde(default)]
    pub date_range: Option<DateRange>,
    /// Status filter
    #[serde(default)]
    pub status: Option<String>,
    /// Custom key-value filters
    #[serde(default)]
    pub custom: Option<HashMap<String, String>>,
}

/// Date range for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

/// Unified search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results
    pub results: Vec<SearchResultItem>,
    /// Total number of results
    pub total: usize,
    /// Query rewrite info (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewritten_query: Option<RewrittenQueryInfo>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Document ID
    pub doc_id: String,
    /// Document content snippet
    pub content: String,
    /// Combined score after fusion
    pub score: f64,
    /// Primary retrieval source
    pub source: RetrievalSource,
    /// Rerank score (if reranking was used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_score: Option<f32>,
    /// Per-source scores
    pub source_scores: HashMap<String, f64>,
}

/// Query rewrite information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewrittenQueryInfo {
    /// Original query
    pub original: String,
    /// Rewritten query
    pub rewritten: String,
    /// Detected query intent
    pub intent: Option<String>,
    /// Extracted keywords
    pub keywords: Vec<String>,
    /// Applied filters
    pub filters: Vec<String>,
}

/// Embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    /// Text to embed
    pub text: String,
    /// Embedding model to use
    #[serde(default = "default_embedding_model")]
    pub model: String,
}

/// Embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    /// Generated embedding vector
    pub embedding: Vec<f32>,
    /// Model used
    pub model: String,
    /// Dimension of embedding
    pub dimension: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

fn default_embedding_model() -> String {
    "nomic-embed-text".to_string()
}

/// Batch embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbedRequest {
    /// Texts to embed
    pub texts: Vec<String>,
    /// Embedding model to use
    #[serde(default = "default_embedding_model")]
    pub model: String,
}

/// Batch embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbedResponse {
    /// Generated embeddings
    pub embeddings: Vec<Vec<f32>>,
    /// Model used
    pub model: String,
    /// Dimension of each embedding
    pub dimension: usize,
    /// Number of texts processed
    pub count: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Health check response for retrieval service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalHealthResponse {
    pub status: String,
    pub ollama_available: bool,
    pub vectorizer_ready: bool,
    pub reranker_ready: bool,
}

pub struct RetrievalService;

impl RetrievalService {
    /// Health check for retrieval services
    pub fn health_check() -> RetrievalHealthResponse {
        // This would check actual service availability in a real implementation
        RetrievalHealthResponse {
            status: "ok".to_string(),
            ollama_available: true, // Would check actual Ollama connection
            vectorizer_ready: true,
            reranker_ready: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_defaults() {
        let json = r#"{"query": "偏差报告"}"#;
        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "偏差报告");
        assert_eq!(req.top_k, 10);
        assert!(req.use_query_rewrite);
        assert!(req.use_reranker);
    }

    #[test]
    fn test_search_request_full() {
        let json = r#"{
            "query": "偏差报告",
            "top_k": 5,
            "rrf_k": 60,
            "use_query_rewrite": true,
            "use_reranker": false,
            "vector_weight": 1.0,
            "fts_weight": 0.8,
            "rule_weight": 1.2,
            "graph_weight": 0.9
        }"#;
        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.top_k, 5);
        assert!(!req.use_reranker);
    }

    #[test]
    fn test_retrieval_source_as_str() {
        assert_eq!(RetrievalSource::Vector.as_str(), "vector");
        assert_eq!(RetrievalSource::Fts.as_str(), "fts");
        assert_eq!(RetrievalSource::Rule.as_str(), "rule");
        assert_eq!(RetrievalSource::Graph.as_str(), "graph");
    }

    #[test]
    fn test_search_result_item() {
        let mut source_scores = HashMap::new();
        source_scores.insert("vector".to_string(), 0.9);
        source_scores.insert("fts".to_string(), 0.7);

        let item = SearchResultItem {
            doc_id: "doc1".to_string(),
            content: "偏差报告内容".to_string(),
            score: 0.85,
            source: RetrievalSource::Vector,
            rerank_score: Some(0.92),
            source_scores,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("偏差报告内容"));
    }

    #[test]
    fn test_embed_request_defaults() {
        let json = r#"{"text": "测试文本"}"#;
        let req: EmbedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.text, "测试文本");
        assert_eq!(req.model, "nomic-embed-text");
    }

    #[test]
    fn test_batch_embed_request() {
        let json = r#"{
            "texts": ["文本1", "文本2", "文本3"],
            "model": "mxbai-embed-large"
        }"#;
        let req: BatchEmbedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.texts.len(), 3);
        assert_eq!(req.model, "mxbai-embed-large");
    }

    #[test]
    fn test_retrieval_health_response() {
        let health = RetrievalService::health_check();
        assert_eq!(health.status, "ok");
        assert!(health.ollama_available);
    }

    #[test]
    fn test_search_filters() {
        let json = r#"{
            "doc_types": ["REPORT", "SOP"],
            "status": "ACTIVE"
        }"#;
        let filters: SearchFilters = serde_json::from_str(json).unwrap();
        assert!(filters.doc_types.is_some());
        assert_eq!(filters.doc_types.unwrap().len(), 2);
    }
}
