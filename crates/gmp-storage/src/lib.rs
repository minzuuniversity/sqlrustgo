//! GMP Storage library - Document storage with FTS, Vector, and Graph support

pub mod cypher_engine;
pub mod hnsw_vector_index;
pub mod sqlrustgo_adapter;
pub mod storage;
pub mod traits;

pub use cypher_engine::{CypherEngine, CypherQueryResult};
pub use hnsw_vector_index::HnswVectorIndex;
pub use sqlrustgo_adapter::SqlrustgoAdapter;
pub use storage::{
    IndexStats, SearchResult, StorageConfig, StorageEngine, ValidationResult, VectorSearchResult,
};
pub use traits::GmpStorage;
mod tests;
