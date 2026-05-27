//! GMP Retrieval Endpoints
//!
//! Provides REST API endpoints for unified retrieval search.
//! Endpoints:
//! - POST /api/v1/retrieval/search - Unified search with query rewriting and reranking
//! - POST /api/v1/retrieval/embed - Generate embeddings for text
//! - GET  /api/v1/retrieval/health - Health check for retrieval services

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlrustgo_gmp::retrieval::{
    EmbeddingRequest, OllamaConfig, OllamaEmbeddingClient, OllamaVectorizer,
    UnifiedSearchConfig, UnifiedSearcher, MXBAI_EMBED_LARGE, NOMIC_EMBED_TEXT,
};
use std::sync::{Arc, RwLock};
use std::time::Instant;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Unified search request
#[derive(Debug, Deserialize)]
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
    #[serde(default = "default_true")]
    pub use_query_rewrite: bool,
    /// Enable reranking (default: true)
    #[serde(default = "default_true")]
    pub use_reranker: bool,
    /// Filter conditions
    #[serde(default)]
    pub filters: Option<SearchFilters>,
}

fn default_top_k() -> usize {
    10
}

fn default_rrf_k() -> usize {
    60
}

fn default_true() -> bool {
    true
}

/// Search filter conditions
#[derive(Debug, Deserialize, Default)]
pub struct SearchFilters {
    /// Document types to include
    #[serde(default)]
    pub doc_types: Option<Vec<String>>,
    /// Status filter
    #[serde(default)]
    pub status: Option<String>,
}

/// Unified search response
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewritten_query: Option<RewrittenQueryInfo>,
    pub execution_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single search result item
#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub doc_id: String,
    pub content: String,
    pub score: f64,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_score: Option<f32>,
}

/// Query rewrite information
#[derive(Debug, Serialize)]
pub struct RewrittenQueryInfo {
    pub original: String,
    pub rewritten: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

/// Embedding request
#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    /// Text to embed
    pub text: String,
    /// Embedding model to use
    #[serde(default = "default_embedding_model")]
    pub model: String,
}

fn default_embedding_model() -> String {
    NOMIC_EMBED_TEXT.to_string()
}

/// Embedding response
#[derive(Debug, Serialize)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimension: usize,
    pub execution_time_ms: u64,
}

/// Batch embedding request
#[derive(Debug, Deserialize)]
pub struct BatchEmbedRequest {
    pub texts: Vec<String>,
    #[serde(default = "default_embedding_model")]
    pub model: String,
}

/// Batch embedding response
#[derive(Debug, Serialize)]
pub struct BatchEmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub dimension: usize,
    pub count: usize,
    pub execution_time_ms: u64,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct RetrievalHealthResponse {
    pub status: String,
    pub ollama_available: bool,
    pub vectorizer_ready: bool,
    pub reranker_ready: bool,
}

// ============================================================================
// App State
// ============================================================================

/// Shared application state for retrieval services
pub struct RetrievalAppState {
    pub searcher: Option<UnifiedSearcher>,
    pub vectorizer: Option<Arc<OllamaVectorizer>>,
    pub ollama_config: OllamaConfig,
}

impl RetrievalAppState {
    pub fn new() -> Self {
        Self {
            searcher: None,
            vectorizer: None,
            ollama_config: OllamaConfig::default(),
        }
    }

    pub fn with_ollama_config(mut self, config: OllamaConfig) -> Self {
        self.ollama_config = config;
        self
    }

    pub fn init_searcher(&mut self) {
        // Create vectorizer if not already created
        if self.vectorizer.is_none() {
            let vectorizer = OllamaVectorizer::new(
                &self.ollama_config.base_url,
                &self.ollama_config.embedding_model,
            );
            self.vectorizer = Some(Arc::new(vectorizer));
        }

        // Create a basic retrieval engine for now
        // In a full implementation, this would be initialized with actual document store
        let retrieval = sqlrustgo_gmp::retrieval::RetrievalEngine::with_bm25(vec![]);

        // Create searcher
        if let Some(ref vectorizer) = self.vectorizer {
            let config = UnifiedSearchConfig {
                rrf_k: 60,
                pre_limit: 50,
                post_limit: 10,
                use_reranker: true,
                use_query_rewrite: true,
            };

            self.searcher = Some(UnifiedSearcher::with_ollama(
                retrieval,
                &self.ollama_config.base_url,
                &self.ollama_config.embedding_model,
                &self.ollama_config.chat_model,
                &self.ollama_config.rerank_model,
            ));
        }
    }
}

impl Default for RetrievalAppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// POST /api/v1/retrieval/search - Unified search endpoint
pub async fn search_handler(
    data: web::Data<Arc<RwLock<RetrievalAppState>>>,
    req: web::Json<SearchRequest>,
) -> impl Responder {
    let start = Instant::now();

    let state = match data.read() {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(SearchResponse {
                results: vec![],
                total: 0,
                rewritten_query: None,
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Failed to acquire state: {}", e)),
            });
        }
    };

    let searcher = match &state.searcher {
        Some(s) => s,
        None => {
            return HttpResponse::ServiceUnavailable().json(SearchResponse {
                results: vec![],
                total: 0,
                rewritten_query: None,
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: Some("Searcher not initialized".to_string()),
            });
        }
    };

    // Execute search
    let results = searcher.search(&req.query).await;

    let items: Vec<SearchResultItem> = results
        .into_iter()
        .take(req.top_k)
        .map(|r| SearchResultItem {
            doc_id: r.doc_id,
            content: r.content,
            score: r.score,
            source: format!("{:?}", r.source).to_lowercase(),
            rerank_score: r.rerank_score,
        })
        .collect();

    let total = items.len();

    HttpResponse::Ok().json(SearchResponse {
        results: items,
        total,
        rewritten_query: None, // Could extract from searcher if needed
        execution_time_ms: start.elapsed().as_millis() as u64,
        error: None,
    })
}

/// POST /api/v1/retrieval/embed - Embedding generation endpoint
pub async fn embed_handler(
    data: web::Data<Arc<RwLock<RetrievalAppState>>>,
    req: web::Json<EmbedRequest>,
) -> impl Responder {
    let start = Instant::now();

    let state = match data.read() {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(EmbedResponse {
                embedding: vec![],
                model: req.model.clone(),
                dimension: 0,
                execution_time_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    let vectorizer = match &state.vectorizer {
        Some(v) => v,
        None => {
            return HttpResponse::ServiceUnavailable().json(EmbedResponse {
                embedding: vec![],
                model: req.model.clone(),
                dimension: 0,
                execution_time_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    // Generate embedding
    let embed_req = EmbeddingRequest {
        texts: vec![req.text.clone()],
        model: req.model.clone(),
    };
    match vectorizer.embed(&embed_req).await {
        Ok(result) => {
            let embedding = result.embeddings.into_iter().next().unwrap_or_default();
            let dimension = embedding.len();
            HttpResponse::Ok().json(EmbedResponse {
                embedding,
                model: result.model,
                dimension,
                execution_time_ms: start.elapsed().as_millis() as u64,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(EmbedResponse {
            embedding: vec![],
            model: req.model.clone(),
            dimension: 0,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }),
    }
}

/// GET /api/v1/retrieval/health - Health check endpoint
pub async fn health_handler(
    data: web::Data<Arc<RwLock<RetrievalAppState>>>,
) -> impl Responder {
    let state = match data.read() {
        Ok(s) => s,
        Err(_) => {
            return HttpResponse::ServiceUnavailable().json(RetrievalHealthResponse {
                status: "error".to_string(),
                ollama_available: false,
                vectorizer_ready: false,
                reranker_ready: false,
            });
        }
    };

    HttpResponse::Ok().json(RetrievalHealthResponse {
        status: "ok".to_string(),
        ollama_available: state.vectorizer.is_some(),
        vectorizer_ready: state.vectorizer.is_some(),
        reranker_ready: state.searcher.is_some(),
    })
}

// ============================================================================
// Route Configuration
// ============================================================================

/// Configure retrieval routes
pub fn configure_retrieval_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/retrieval")
            .route("/search", web::post().to(search_handler))
            .route("/embed", web::post().to(embed_handler))
            .route("/health", web::get().to(health_handler)),
    );
}

// ============================================================================
// Tests
// ============================================================================

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
    fn test_embed_request_defaults() {
        let json = r#"{"text": "测试文本"}"#;
        let req: EmbedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.text, "测试文本");
        assert_eq!(req.model, NOMIC_EMBED_TEXT);
    }

    #[test]
    fn test_retrieval_app_state_default() {
        let state = RetrievalAppState::default();
        assert!(state.searcher.is_none());
        assert!(state.vectorizer.is_none());
        assert_eq!(state.ollama_config.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_retrieval_app_state_with_config() {
        let config = OllamaConfig::new(
            "http://custom:11434",
            MXBAI_EMBED_LARGE,
            "llama3:8b",
            "bge-reranker-v2-m3",
        );
        let state = RetrievalAppState::new().with_ollama_config(config);
        assert_eq!(state.ollama_config.base_url, "http://custom:11434");
        assert_eq!(state.ollama_config.embedding_model, MXBAI_EMBED_LARGE);
    }
}
