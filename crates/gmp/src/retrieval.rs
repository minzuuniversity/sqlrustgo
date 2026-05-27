//! GMP Retrieval v3 Integration
//!
//! This module integrates the four-way fusion retrieval system into the GMP platform.
//! It combines vector search, BM25 full-text search, rule-based search, and graph traversal
//! with LLM-powered query rewriting and reranking.

pub use sqlrustgo_gmp_retrieval::{
    UnifiedSearcher, UnifiedSearchConfig, UnifiedSearchResult,
    RetrievalEngine, RetrievalSource, SearchResult,
    Bm25Search, RrfFusion,
    OllamaVectorizer, OllamaEmbeddingClient,
    EmbeddingRequest, EmbeddingResult, VectorizerError,
    QueryRewriter, RewrittenQuery, QueryIntent, FilterCondition, RewriterError,
    OllamaReranker, OllamaRerankClient, RerankRequest, RerankResult, RerankerError,
    NoopReranker, RerankerWithFallback,
    MXBAI_EMBED_LARGE, NOMIC_EMBED_TEXT,
};

/// Configuration for Ollama-based retrieval services
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Ollama server base URL
    pub base_url: String,
    /// Embedding model (e.g., "nomic-embed-text", "mxbai-embed-large")
    pub embedding_model: String,
    /// Chat model for query rewriting (e.g., "qwen2.5:14b")
    pub chat_model: String,
    /// Rerank model (e.g., "bge-reranker-v2-m3")
    pub rerank_model: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            embedding_model: NOMIC_EMBED_TEXT.to_string(),
            chat_model: "qwen2.5:14b".to_string(),
            rerank_model: "bge-reranker-v2-m3".to_string(),
        }
    }
}

impl OllamaConfig {
    /// Create a new OllamaConfig with custom settings
    pub fn new(
        base_url: impl Into<String>,
        embedding_model: impl Into<String>,
        chat_model: impl Into<String>,
        rerank_model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            embedding_model: embedding_model.into(),
            chat_model: chat_model.into(),
            rerank_model: rerank_model.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.embedding_model, NOMIC_EMBED_TEXT);
    }

    #[test]
    fn test_ollama_config_custom() {
        let config = OllamaConfig::new(
            "http://custom:11434",
            "mxbai-embed-large",
            "llama3:8b",
            "bge-reranker-v2-m3",
        );
        assert_eq!(config.base_url, "http://custom:11434");
        assert_eq!(config.embedding_model, "mxbai-embed-large");
        assert_eq!(config.chat_model, "llama3:8b");
        assert_eq!(config.rerank_model, "bge-reranker-v2-m3");
    }
}
