//! GMP Vector Index Management
//!
//! Provides vector index rebuild functionality for GMP embeddings.
//! Supports Flat (brute-force) index. HNSW is stubbed for future.

use crate::embedding::{cosine_similarity, DocumentEmbedding, EMBEDDING_DIM};
use crate::version::sha256;
use serde::{Deserialize, Serialize};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};
use std::collections::HashMap;

/// Vector index type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorIndexType {
    Flat,
    Hnsw,
}

impl VectorIndexType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VectorIndexType::Flat => "FLAT",
            VectorIndexType::Hnsw => "HNSW",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "FLAT" => Some(VectorIndexType::Flat),
            "HNSW" => Some(VectorIndexType::Hnsw),
            _ => None,
        }
    }
}

/// Metadata for a vector index.
#[derive(Debug, Clone)]
pub struct VectorIndexMeta {
    pub id: i64,
    pub index_type: VectorIndexType,
    pub model_name: String,
    pub dimension: usize,
    pub embedding_count: usize,
    pub built_at: i64,
    pub index_path: Option<String>,
}

impl VectorIndexMeta {
    pub fn from_row(row: &[Value]) -> Option<Self> {
        Some(VectorIndexMeta {
            id: match &row.get(0)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
            index_type: match &row.get(1)? {
                Value::Text(s) => VectorIndexType::from_str(s)?,
                _ => return None,
            },
            model_name: match &row.get(2)? {
                Value::Text(s) => s.clone(),
                _ => return None,
            },
            dimension: match &row.get(3)? {
                Value::Integer(n) => *n as usize,
                _ => return None,
            },
            embedding_count: match &row.get(4)? {
                Value::Integer(n) => *n as usize,
                _ => return None,
            },
            built_at: match &row.get(5)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
            index_path: match &row.get(6)? {
                Value::Text(s) => Some(s.clone()),
                Value::Null => None,
                _ => return None,
            },
        })
    }
}

/// Table name for vector index metadata.
pub const TABLE_VECTOR_INDEX: &str = "gmp_vector_index";

/// Create the vector index metadata table.
pub fn create_vector_index_table(storage: &mut dyn StorageEngine) -> SqlResult<()> {
    if !storage.has_table(TABLE_VECTOR_INDEX) {
        let columns = vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "index_type".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "model_name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "dimension".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "embedding_count".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "built_at".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "index_path".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_VECTOR_INDEX.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }
    Ok(())
}

/// A chunk embedding record with full metadata.
#[derive(Debug, Clone)]
pub struct ChunkEmbedding {
    pub chunk_id: i64,
    pub doc_id: i64,
    pub vector: Vec<f32>,
    pub model_name: String,
    pub dimension: usize,
    pub vector_hash: String,
    pub updated_at: i64,
}

/// Compute SHA-256 hash of a vector.
pub fn vector_hash(embedding: &[f32]) -> String {
    let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    sha256(&bytes)
}

/// Flat (brute-force) vector index.
/// Stores all embeddings in memory for fast cosine similarity search.
#[derive(Debug, Clone)]
pub struct FlatIndex {
    pub vectors: Vec<(i64, Vec<f32>)>, // (chunk_id, vector)
    pub dimension: usize,
}

impl FlatIndex {
    /// Build a flat index from a list of chunk embeddings.
    pub fn build(embeddings: &[ChunkEmbedding]) -> Self {
        let dimension = if embeddings.is_empty() {
            EMBEDDING_DIM
        } else {
            embeddings[0].dimension
        };
        let vectors: Vec<_> = embeddings
            .iter()
            .map(|e| (e.chunk_id, e.vector.clone()))
            .collect();
        FlatIndex { vectors, dimension }
    }

    /// Search the index for the top_k most similar vectors to the query.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(i64, f32)> {
        let mut results: Vec<_> = self
            .vectors
            .iter()
            .map(|(chunk_id, vector)| {
                let score = cosine_similarity(query, vector);
                (*chunk_id, score)
            })
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }
}

/// Index rebuild report.
#[derive(Debug, Clone, Default)]
pub struct IndexBuildReport {
    pub index_type: String,
    pub model_name: String,
    pub dimension: usize,
    pub embedding_count: usize,
    pub built_at: i64,
}

/// Rebuild the vector index from all stored embeddings.
pub fn rebuild_flat_index(
    storage: &mut dyn StorageEngine,
    model_name: &str,
) -> SqlResult<IndexBuildReport> {
    use crate::vector_search::get_all_embeddings;

    create_vector_index_table(storage)?;

    // Load all embeddings
    let embeddings = get_all_embeddings(storage)?;

    let dimension = embeddings
        .first()
        .map(|e| e.embedding.len())
        .unwrap_or(EMBEDDING_DIM);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Build flat index
    let _index = FlatIndex::build(
        &embeddings
            .iter()
            .map(|e| ChunkEmbedding {
                chunk_id: e.doc_id,
                doc_id: e.doc_id,
                vector: e.embedding.clone(),
                model_name: model_name.to_string(),
                dimension,
                vector_hash: vector_hash(&e.embedding),
                updated_at: e.updated_at,
            })
            .collect::<Vec<_>>(),
    );

    // Store index metadata
    let rows = storage.scan(TABLE_VECTOR_INDEX)?;
    let next_id = rows
        .iter()
        .filter_map(|r| {
            r.get(0).and_then(|v| match v {
                Value::Integer(n) => Some(*n),
                _ => None,
            })
        })
        .max()
        .unwrap_or(0)
        + 1;

    let row = vec![
        Value::Integer(next_id),
        Value::Text("FLAT".to_string()),
        Value::Text(model_name.to_string()),
        Value::Integer(dimension as i64),
        Value::Integer(embeddings.len() as i64),
        Value::Integer(now),
        Value::Null,
    ];

    storage.insert(TABLE_VECTOR_INDEX, vec![row])?;

    Ok(IndexBuildReport {
        index_type: "FLAT".to_string(),
        model_name: model_name.to_string(),
        dimension,
        embedding_count: embeddings.len(),
        built_at: now,
    })
}

/// Get the latest vector index metadata.
pub fn get_latest_index(storage: &dyn StorageEngine) -> SqlResult<Option<VectorIndexMeta>> {
    let rows = storage.scan(TABLE_VECTOR_INDEX)?;
    let mut indexes: Vec<_> = rows
        .into_iter()
        .filter_map(|r| VectorIndexMeta::from_row(&r))
        .collect();
    indexes.sort_by_key(|i| i.built_at);
    Ok(indexes.into_iter().last())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_index_build_and_search() {
        let embeddings = vec![
            ChunkEmbedding {
                chunk_id: 1,
                doc_id: 1,
                vector: vec![1.0, 0.0, 0.0],
                model_name: "hash".to_string(),
                dimension: 3,
                vector_hash: "".to_string(),
                updated_at: 0,
            },
            ChunkEmbedding {
                chunk_id: 2,
                doc_id: 1,
                vector: vec![0.0, 1.0, 0.0],
                model_name: "hash".to_string(),
                dimension: 3,
                vector_hash: "".to_string(),
                updated_at: 0,
            },
        ];

        let index = FlatIndex::build(&embeddings);
        assert_eq!(index.dimension, 3);
        assert_eq!(index.vectors.len(), 2);

        let results = index.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results[0].0, 1); // chunk_id 1 is most similar to itself
    }

    #[test]
    fn test_vector_hash() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![1.0, 0.0];
        let v3 = vec![0.0, 1.0];
        assert_eq!(vector_hash(&v1), vector_hash(&v2));
        assert_ne!(vector_hash(&v1), vector_hash(&v3));
        assert_eq!(vector_hash(&v1).len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_index_type_conversion() {
        assert_eq!(VectorIndexType::Flat.as_str(), "FLAT");
        assert_eq!(VectorIndexType::Hnsw.as_str(), "HNSW");
        assert_eq!(
            VectorIndexType::from_str("FLAT"),
            Some(VectorIndexType::Flat)
        );
        assert_eq!(
            VectorIndexType::from_str("HNSW"),
            Some(VectorIndexType::Hnsw)
        );
        assert_eq!(VectorIndexType::from_str("invalid"), None);
    }
}
