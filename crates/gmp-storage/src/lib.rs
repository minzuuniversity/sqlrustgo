//! GMP Storage library - Document storage with SQLRustGo backend

pub mod hnsw_vector_index;
pub mod sqlrustgo_adapter;
pub mod storage;

pub use hnsw_vector_index::HnswVectorIndex;
pub use sqlrustgo_adapter::SqlrustgoAdapter;
#[allow(unused_imports)]
pub use storage::{
    cosine_sim, IndexStats, SearchResult, StorageConfig, StorageEngine, ValidationResult,
    VectorSearchResult,
};
mod tests;
