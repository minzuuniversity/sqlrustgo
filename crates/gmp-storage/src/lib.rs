//! GMP Storage library - Document storage with FTS, Vector, and Graph support

pub mod storage;

pub use hnsw_vector_index::HnswVectorIndex;
pub use sqlrustgo_adapter::SqlrustgoAdapter;
#[allow(unused_imports)]
pub use storage::{
    IndexStats, SearchResult, StorageConfig, StorageEngine, ValidationResult, VectorSearchResult,
    cosine_sim,
};
#[cfg(test)]
mod tests;
