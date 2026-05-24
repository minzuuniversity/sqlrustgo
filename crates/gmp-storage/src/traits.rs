//! GMP Storage trait abstraction
//!
//! Abstracts over storage backends (FileStorage + InvertedIndex + HnswVectorIndex)
//! to allow future替换 (e.g., PostgreSQL, Tantivy).
//!
//! 所有外部模块应通过此 trait 与 storage 交互，不直接依赖 StorageEngine 具体类型。

use crate::{IndexStats, SearchResult, ValidationResult, VectorSearchResult};
use anyhow::Result;

/// Primary storage interface for GMP document retrieval
pub trait GmpStorage: Send + Sync {
    // ─── Full-Text Search ───────────────────────────────────────

    /// FTS search returning basic results (token-matched doc IDs, no scoring)
    fn search_fts(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>>;

    /// FTS search with scoring — results sorted by relevance
    /// Fallback: if scoring is unavailable, returns results sorted by internal order
    fn search_fts_with_scores(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>>;

    /// Keyword fallback using in-memory LIKE filtering
    fn search_like(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>>;

    /// Keyword fallback with extra query terms for expansion
    fn search_like_with_extra(
        &self,
        query: &str,
        top_k: usize,
        extra_keywords: &[String],
    ) -> Result<Vec<SearchResult>>;

    // ─── Vector Search ───────────────────────────────────────────

    /// Vector similarity search using HNSW index
    fn search_vectors(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>>;

    // ─── Document Indexing ───────────────────────────────────────

    /// Index all markdown documents from a directory
    fn index_documents(&self, source_dir: &str) -> Result<IndexStats>;

    /// Generate and store embeddings for all chunks (calls Ollama)
    fn index_embeddings(&self, ollama_url: &str, model: &str) -> Result<usize>;

    /// Rebuild HNSW index from stored embeddings
    fn rebuild_hnsw_from_db(&self) -> Result<usize>;

    // ─── Validation & Stats ─────────────────────────────────────

    /// Validate storage integrity
    fn validate(&self) -> Result<ValidationResult>;

    /// Total document count
    fn get_document_count(&self) -> Result<i64>;

    // ─── Chunk Access (for debugging/admin) ─────────────────────

    /// Get chunk IDs up to limit
    fn get_chunk_ids(&self, limit: usize) -> Vec<i64>;
}
