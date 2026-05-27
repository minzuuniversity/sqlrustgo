//! Unified Search API for GMP Retrieval v3
//!
//! Integrates vectorizer, query rewriter, retrieval engine, and reranker
//! into a single search pipeline.

use crate::query_rewriter::{QueryRewriter, RewrittenQuery};
use crate::reranker::{OllamaRerankClient, OllamaReranker, RerankRequest};
use crate::retrieval::{RetrievalEngine, RetrievalSource, SearchResult};
use crate::OllamaVectorizer;
use std::sync::Arc;

/// Configuration for unified search
#[derive(Debug, Clone)]
pub struct UnifiedSearchConfig {
    /// RRF k-factor for fusion
    pub rrf_k: usize,
    /// Number of results to return before reranking
    pub pre_limit: usize,
    /// Number of final results after reranking
    pub post_limit: usize,
    /// Whether to use reranker
    pub use_reranker: bool,
    /// Whether to use query rewriting
    pub use_query_rewrite: bool,
}

impl Default for UnifiedSearchConfig {
    fn default() -> Self {
        Self {
            rrf_k: 60,
            pre_limit: 50,
            post_limit: 10,
            use_reranker: true,
            use_query_rewrite: true,
        }
    }
}

/// Unified search result
#[derive(Debug, Clone)]
pub struct UnifiedSearchResult {
    pub doc_id: String,
    pub content: String,
    pub score: f64,
    pub source: RetrievalSource,
    pub rerank_score: Option<f32>,
}

/// Unified searcher that combines all retrieval components
pub struct UnifiedSearcher {
    config: UnifiedSearchConfig,
    retrieval: RetrievalEngine,
    #[allow(dead_code)]
    vectorizer: Option<Arc<OllamaVectorizer>>,
    query_rewriter: Option<Arc<QueryRewriter>>,
    reranker: Option<Arc<dyn OllamaRerankClient>>,
}

impl UnifiedSearcher {
    /// Create a new unified searcher with all components
    pub fn new(
        retrieval: RetrievalEngine,
        #[allow(dead_code)] vectorizer: Option<Arc<OllamaVectorizer>>,
        query_rewriter: Option<Arc<QueryRewriter>>,
        reranker: Option<Arc<dyn OllamaRerankClient>>,
        config: UnifiedSearchConfig,
    ) -> Self {
        Self {
            config,
            retrieval,
            vectorizer,
            query_rewriter,
            reranker,
        }
    }

    /// Create with Ollama-based services
    pub fn with_ollama(
        retrieval: RetrievalEngine,
        ollama_base_url: &str,
        embedding_model: &str,
        chat_model: &str,
        rerank_model: &str,
    ) -> Self {
        let vectorizer = OllamaVectorizer::new(ollama_base_url, embedding_model);
        let chat_client =
            crate::query_rewriter::OllamaChatClientImpl::new(ollama_base_url, chat_model);
        let query_rewriter = QueryRewriter::new(Box::new(chat_client), chat_model);
        let reranker = OllamaReranker::new(ollama_base_url, rerank_model);

        Self {
            config: UnifiedSearchConfig::default(),
            retrieval,
            vectorizer: Some(Arc::new(vectorizer)),
            query_rewriter: Some(Arc::new(query_rewriter)),
            reranker: Some(Arc::new(reranker)),
        }
    }

    /// Search with natural language query
    pub async fn search(&self, query: &str) -> Vec<UnifiedSearchResult> {
        // Step 1: Rewrite query if enabled
        let rewritten = if self.config.use_query_rewrite {
            if let Some(ref rewriter) = self.query_rewriter {
                rewriter.rewrite(query).await
            } else {
                RewrittenQuery::from_original(query)
            }
        } else {
            RewrittenQuery::from_original(query)
        };

        // Step 2: Execute searches in parallel
        let fts_results = self.retrieval.search_bm25(query, self.config.pre_limit);
        let rule_results = self
            .retrieval
            .search_with_filter(&rewritten, self.config.pre_limit);

        // Step 3: Fuse results
        let fused = self
            .retrieval
            .fuse_results(vec![fts_results, rule_results], self.config.rrf_k);

        // Step 4: Rerank if enabled
        let final_results = if self.config.use_reranker {
            if let Some(ref reranker) = self.reranker {
                if reranker.is_available() {
                    self.rerank_results(fused, &rewritten.original).await
                } else {
                    fused
                }
            } else {
                fused
            }
        } else {
            fused
        };

        // Step 5: Convert to unified results
        final_results
            .into_iter()
            .take(self.config.post_limit)
            .map(|r| {
                let content = self
                    .retrieval
                    .get_doc(r.doc_id.parse().unwrap_or(0))
                    .cloned()
                    .unwrap_or_default();
                UnifiedSearchResult {
                    doc_id: r.doc_id,
                    content,
                    score: r.score,
                    source: r.source,
                    rerank_score: None,
                }
            })
            .collect()
    }

    /// Rerank search results
    async fn rerank_results(&self, results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
        if results.is_empty() {
            return results;
        }

        let reranker = match &self.reranker {
            Some(r) => r,
            None => return results,
        };

        let docs: Vec<(String, String)> = results
            .iter()
            .map(|r| {
                let content = self
                    .retrieval
                    .get_doc(r.doc_id.parse().unwrap_or(0))
                    .cloned()
                    .unwrap_or_default();
                (r.doc_id.clone(), content)
            })
            .collect();

        let request = RerankRequest {
            query: query.to_string(),
            documents: docs,
            top_n: self.config.post_limit,
        };

        match reranker.rerank(&request, "").await {
            Ok(reranked) => reranked
                .into_iter()
                .map(|r| {
                    results
                        .iter()
                        .find(|sr| sr.doc_id == r.doc_id)
                        .cloned()
                        .unwrap_or(SearchResult::new(
                            r.doc_id,
                            r.score as f64,
                            RetrievalSource::Fts,
                        ))
                })
                .collect(),
            Err(_) => results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_rewriter::MockChatClient;
    use crate::reranker::MockRerankClient;
    use crate::reranker::RerankResult;

    #[tokio::test]
    async fn test_unified_searcher_basic() {
        let docs = vec![
            "GMP偏差报告A".to_string(),
            "GMP偏差报告B".to_string(),
            "其他文档".to_string(),
        ];
        let retrieval = RetrievalEngine::with_bm25(docs);

        // Use mock rewriter and reranker
        let mock_rewriter =
            MockChatClient::new(r#"{"keywords":["偏差","报告"],"filters":[],"intent":"search"}"#);
        let query_rewriter = Arc::new(QueryRewriter::new(Box::new(mock_rewriter), "qwen2.5:14b"));

        let mock_reranker = MockRerankClient::new(vec![
            RerankResult {
                doc_id: "0".to_string(),
                score: 0.9,
            },
            RerankResult {
                doc_id: "1".to_string(),
                score: 0.8,
            },
        ]);
        let reranker: Arc<dyn OllamaRerankClient> = Arc::new(mock_reranker);

        let searcher = UnifiedSearcher {
            config: UnifiedSearchConfig::default(),
            retrieval,
            vectorizer: None,
            query_rewriter: Some(query_rewriter),
            reranker: Some(reranker),
        };

        let results = searcher.search("偏差报告").await;
        assert!(!results.is_empty());
    }

    #[test]
    fn test_unified_search_config_default() {
        let config = UnifiedSearchConfig::default();
        assert_eq!(config.rrf_k, 60);
        assert_eq!(config.pre_limit, 50);
        assert_eq!(config.post_limit, 10);
        assert!(config.use_reranker);
        assert!(config.use_query_rewrite);
    }
}
