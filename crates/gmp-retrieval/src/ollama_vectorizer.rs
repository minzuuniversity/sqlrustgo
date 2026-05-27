//! Ollama Vectorizer for GMP Retrieval
//!
//! Provides embedding generation using local Ollama models.

use async_trait::async_trait;
use lru::LruCache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::RwLock;
use thiserror::Error;

/// Maximum number of entries in the embedding cache
const EMBEDDING_CACHE_SIZE: usize = 10_000;

/// Maximum batch size for embedding requests
const MAX_BATCH_SIZE: usize = 100;

/// Supported embedding models and their dimensions
pub const NOMIC_EMBED_TEXT: &str = "nomic-embed-text";
pub const MXBAI_EMBED_LARGE: &str = "mxbai-embed-large";

/// Model dimension mapping
pub fn get_model_dimension(model: &str) -> Option<usize> {
    match model {
        NOMIC_EMBED_TEXT => Some(768),
        MXBAI_EMBED_LARGE => Some(1024),
        _ => None,
    }
}

/// Errors that can occur during vectorization
#[derive(Error, Debug)]
pub enum VectorizerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Model not found: {0}. Available models: nomic-embed-text, mxbai-embed-large")]
    ModelNotFound(String),

    #[error("Ollama unavailable: {0}")]
    Unavailable(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Batch size exceeds maximum: {0} > {MAX_BATCH_SIZE}")]
    BatchSizeExceeded(usize),
}

/// Result type for vectorizer operations
pub type VectorizerResult<T> = Result<T, VectorizerError>;

/// Embedding request
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    /// Texts to embed
    pub texts: Vec<String>,
    /// Model name
    pub model: String,
}

/// Embedding result
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResult {
    /// Generated embeddings
    pub embeddings: Vec<Vec<f32>>,
    /// Model used
    pub model: String,
}

/// Ollama embedding client trait
#[async_trait]
pub trait OllamaEmbeddingClient: Send + Sync {
    /// Generate embeddings for texts
    async fn embed(&self, request: &EmbeddingRequest) -> VectorizerResult<EmbeddingResult>;

    /// Check if the client is available
    fn is_available(&self) -> bool;

    /// Get the base URL
    fn base_url(&self) -> &str;

    /// Get the default model
    fn default_model(&self) -> &str;
}

/// Ollama-hosted embedding client
pub struct OllamaVectorizer {
    base_url: String,
    model: String,
    client: Client,
    cache: RwLock<LruCache<String, Vec<f32>>>,
}

impl OllamaVectorizer {
    /// Create a new OllamaVectorizer
    ///
    /// # Arguments
    /// * `base_url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name for embeddings
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            client: Client::new(),
            cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(EMBEDDING_CACHE_SIZE).unwrap(),
            )),
        }
    }

    /// Get a cache key for a text
    fn cache_key(text: &str, model: &str) -> String {
        format!("{}:{}", model, text)
    }

    /// Generate embedding for a single text
    async fn embed_single(&self, text: &str) -> VectorizerResult<Vec<f32>> {
        let cache_key = Self::cache_key(text, &self.model);

        // Check cache first (need write lock because get() moves entry to front)
        let cached_embedding = {
            let mut cache = self.cache.write().unwrap();
            cache.get(&cache_key).cloned()
        };

        if let Some(embedding) = cached_embedding {
            return Ok(embedding);
        }

        #[derive(Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            prompt: &'a str,
        }

        #[derive(Deserialize)]
        struct SingleEmbedResponse {
            embedding: Vec<f32>,
        }

        let resp = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&EmbedRequest {
                model: &self.model,
                prompt: text,
            })
            .send()
            .await
            .map_err(|e| VectorizerError::Unavailable(e.to_string()))?;

        if resp.status() == 404 {
            return Err(VectorizerError::ModelNotFound(self.model.clone()));
        }

        let result: SingleEmbedResponse = resp
            .json()
            .await
            .map_err(|e| VectorizerError::InvalidInput(e.to_string()))?;

        // Cache the result
        let embedding = result.embedding;
        self.cache.write().unwrap().put(cache_key, embedding.clone());

        Ok(embedding)
    }

    /// Check if Ollama server is available
    pub fn check_availability(&self) -> bool {
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

    /// Clear the embedding cache
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Get current cache size
    pub fn cache_size(&self) -> usize {
        self.cache.read().unwrap().len()
    }
}

#[async_trait]
impl OllamaEmbeddingClient for OllamaVectorizer {
    async fn embed(&self, request: &EmbeddingRequest) -> VectorizerResult<EmbeddingResult> {
        if request.texts.is_empty() {
            return Ok(EmbeddingResult {
                embeddings: vec![],
                model: self.model.clone(),
            });
        }

        if request.texts.len() > MAX_BATCH_SIZE {
            return Err(VectorizerError::BatchSizeExceeded(request.texts.len()));
        }

        // Validate model
        if get_model_dimension(&request.model).is_none() {
            return Err(VectorizerError::ModelNotFound(request.model.clone()));
        }

        // For batch requests, we process sequentially with caching
        // Ollama's /api/embeddings only supports single prompt per request
        let mut embeddings = Vec::with_capacity(request.texts.len());

        for text in &request.texts {
            let embedding = self.embed_single(text).await?;
            embeddings.push(embedding);
        }

        Ok(EmbeddingResult {
            embeddings,
            model: self.model.clone(),
        })
    }

    fn is_available(&self) -> bool {
        self.check_availability()
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

/// Mock embedding client for testing
#[cfg(test)]
pub struct MockEmbeddingClient {
    pub embeddings: Vec<Vec<f32>>,
    pub should_fail: bool,
}

#[cfg(test)]
impl MockEmbeddingClient {
    pub fn new(embeddings: Vec<Vec<f32>>) -> Self {
        Self {
            embeddings,
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
impl OllamaEmbeddingClient for MockEmbeddingClient {
    async fn embed(&self, _request: &EmbeddingRequest) -> VectorizerResult<EmbeddingResult> {
        if self.should_fail {
            return Err(VectorizerError::Unavailable("Mock failure".to_string()));
        }
        Ok(EmbeddingResult {
            embeddings: self.embeddings.clone(),
            model: NOMIC_EMBED_TEXT.to_string(),
        })
    }

    fn is_available(&self) -> bool {
        !self.should_fail
    }

    fn base_url(&self) -> &str {
        "http://localhost:11434"
    }

    fn default_model(&self) -> &str {
        NOMIC_EMBED_TEXT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_dimension() {
        assert_eq!(get_model_dimension(NOMIC_EMBED_TEXT), Some(768));
        assert_eq!(get_model_dimension(MXBAI_EMBED_LARGE), Some(1024));
        assert_eq!(get_model_dimension("unknown"), None);
    }

    #[test]
    fn test_cache_key() {
        let key = OllamaVectorizer::cache_key("hello world", NOMIC_EMBED_TEXT);
        assert_eq!(key, "nomic-embed-text:hello world");
    }

    #[tokio::test]
    async fn test_embed_empty_texts() {
        let vectorizer = OllamaVectorizer::new("http://localhost:11434", NOMIC_EMBED_TEXT);
        let request = EmbeddingRequest {
            texts: vec![],
            model: NOMIC_EMBED_TEXT.to_string(),
        };
        let result = vectorizer.embed(&request).await.unwrap();
        assert!(result.embeddings.is_empty());
    }

    #[tokio::test]
    async fn test_embed_invalid_model() {
        let vectorizer = OllamaVectorizer::new("http://localhost:11434", NOMIC_EMBED_TEXT);
        let request = EmbeddingRequest {
            texts: vec!["hello".to_string()],
            model: "unknown-model".to_string(),
        };
        let result = vectorizer.embed(&request).await;
        assert!(matches!(result, Err(VectorizerError::ModelNotFound(_))));
    }

    #[tokio::test]
    async fn test_mock_client_basic() {
        let embeddings = vec![vec![0.1, 0.2, 0.3], vec![0.4, 0.5, 0.6]];
        let client = MockEmbeddingClient::new(embeddings);
        let request = EmbeddingRequest {
            texts: vec!["text1".to_string(), "text2".to_string()],
            model: NOMIC_EMBED_TEXT.to_string(),
        };
        let result = client.embed(&request).await.unwrap();
        assert_eq!(result.embeddings.len(), 2);
        assert_eq!(result.model, NOMIC_EMBED_TEXT);
    }

    #[tokio::test]
    async fn test_mock_client_failure() {
        let embeddings = vec![vec![0.1, 0.2, 0.3]];
        let client = MockEmbeddingClient::new(embeddings).with_failure();
        let request = EmbeddingRequest {
            texts: vec!["text1".to_string()],
            model: NOMIC_EMBED_TEXT.to_string(),
        };
        let result = client.embed(&request).await;
        assert!(matches!(result, Err(VectorizerError::Unavailable(_))));
    }

    #[test]
    fn test_vectorizer_creation() {
        let vectorizer = OllamaVectorizer::new("http://localhost:11434", NOMIC_EMBED_TEXT);
        assert_eq!(vectorizer.base_url(), "http://localhost:11434");
        assert_eq!(vectorizer.default_model(), NOMIC_EMBED_TEXT);
        assert_eq!(vectorizer.cache_size(), 0);
    }
}
