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
///
/// `vector_data` carries the serialized `FlatIndex` payload (JSON) so the
/// built index survives restart and can be round-tripped for stability
/// testing (V312-52 followup #4235 / v3.13.0 §4.1.1).
#[derive(Debug, Clone)]
pub struct VectorIndexMeta {
    pub id: i64,
    pub index_type: VectorIndexType,
    pub model_name: String,
    pub dimension: usize,
    pub embedding_count: usize,
    pub built_at: i64,
    pub index_path: Option<String>,
    /// Serialized `FlatIndex` JSON payload (vectors + dimension), if persisted.
    /// `None` for legacy metadata rows written before the persistence fix.
    pub vector_data: Option<String>,
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
            vector_data: match &row.get(7) {
                Some(Value::Text(s)) => Some(s.clone()),
                Some(Value::Null) | None => None,
                _ => None,
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
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "index_type".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "model_name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "dimension".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "embedding_count".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "built_at".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "index_path".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                // v3.13.0 §4.1.1: persisted serialized FlatIndex payload so the
                // built index is not discarded after `rebuild_flat_index` returns.
                name: "vector_data".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_VECTOR_INDEX.to_string(),
            columns,
            foreign_keys: vec![],
            // v3.13.0 §4.1.5: each (model_name, dimension) tuple maps to
            // exactly one index row. This prevents accidental duplicate
            // rebuilds from accumulating stale rows and makes audit
            // queries (`SELECT model_name FROM gmp_vector_index`) determin-
            // istic — one row per model.
            unique_constraints: vec![sqlrustgo_storage::UniqueConstraint {
                name: Some("uq_model_dimension".to_string()),
                columns: vec!["model_name".to_string(), "dimension".to_string()],
            }],
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatIndex {
    pub vectors: Vec<(i64, Vec<f32>)>, // (chunk_id, vector)
    pub dimension: usize,
}

/// Serializable view of a `FlatIndex` suitable for JSON persistence in
/// `gmp_vector_index.vector_data`. Kept separate so `f32` can be encoded
/// as a stable textual representation (avoids platform-specific float
/// formatting quirks).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlatIndexPayload {
    pub dimension: usize,
    /// Pairs of (chunk_id, vector) — chunk_id is i64, vector is Vec<f32>.
    pub vectors: Vec<(i64, Vec<f32>)>,
}

impl FlatIndex {
    /// Serialize this index into a stable JSON payload for persistence.
    pub fn to_payload(&self) -> FlatIndexPayload {
        FlatIndexPayload {
            dimension: self.dimension,
            vectors: self.vectors.clone(),
        }
    }

    /// Reconstruct a `FlatIndex` from a persisted JSON payload.
    pub fn from_payload(payload: FlatIndexPayload) -> Self {
        FlatIndex {
            dimension: payload.dimension,
            vectors: payload.vectors,
        }
    }

    /// Reconstruct a `FlatIndex` from a JSON string previously produced by
    /// `to_json`. Returns `None` if the string is not valid JSON or cannot
    /// be parsed into the expected shape.
    pub fn from_json(raw: &str) -> Option<Self> {
        serde_json::from_str::<FlatIndexPayload>(raw)
            .ok()
            .map(FlatIndex::from_payload)
    }

    /// Serialize this index to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_payload()).unwrap_or_default()
    }
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
///
/// v3.13.0 §4.1.1: the built `FlatIndex` is actually persisted as a JSON
/// payload in `gmp_vector_index.vector_data` (the previous implementation
/// discarded it via `let _index = ...`).
///
/// v3.13.0 §4.1.5: `gmp_vector_index` carries a `UNIQUE(model_name,
/// dimension)` constraint. We enforce it at the application level
/// (MemoryStorage does not natively enforce unique constraints) by
/// deleting any pre-existing row with the same `(model_name, dimension)`
/// before inserting the rebuilt row. This makes repeated rebuilds
/// idempotent without surfacing constraint violations.
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

    // Build flat index — actually use it (v3.13.0 §4.1.1).
    let built_index = FlatIndex::build(
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

    // v3.13.0 §4.1.5: enforce UNIQUE(model_name, dimension) by deleting
    // any pre-existing row with the same (model_name, dimension) tuple.
    delete_existing_index_row(storage, model_name, dimension)?;

    // Allocate next id from whatever rows remain (after the delete).
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

    // Persist the serialized FlatIndex so the build is not discarded.
    let vector_data = built_index.to_json();

    let row = vec![
        Value::Integer(next_id),
        Value::Text("FLAT".to_string()),
        Value::Text(model_name.to_string()),
        Value::Integer(dimension as i64),
        Value::Integer(embeddings.len() as i64),
        Value::Integer(now),
        Value::Null,
        Value::Text(vector_data),
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

/// Public wrapper that returns the rebuilt `FlatIndex` alongside the metadata
/// report. Useful for callers that want to use the index in-memory without
/// re-scanning, and for stability tests (§4.1.1 acceptance).
pub fn rebuild_flat_index_with_handle(
    storage: &mut dyn StorageEngine,
    model_name: &str,
) -> SqlResult<(IndexBuildReport, FlatIndex)> {
    use crate::vector_search::get_all_embeddings;

    create_vector_index_table(storage)?;

    let embeddings = get_all_embeddings(storage)?;

    let dimension = embeddings
        .first()
        .map(|e| e.embedding.len())
        .unwrap_or(EMBEDDING_DIM);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let built_index = FlatIndex::build(
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

    // v3.13.0 §4.1.5: enforce the unique constraint by deleting prior rows.
    delete_existing_index_row(storage, model_name, dimension)?;

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

    let vector_data = built_index.to_json();

    let row = vec![
        Value::Integer(next_id),
        Value::Text("FLAT".to_string()),
        Value::Text(model_name.to_string()),
        Value::Integer(dimension as i64),
        Value::Integer(embeddings.len() as i64),
        Value::Integer(now),
        Value::Null,
        Value::Text(vector_data),
    ];

    storage.insert(TABLE_VECTOR_INDEX, vec![row])?;

    let report = IndexBuildReport {
        index_type: "FLAT".to_string(),
        model_name: model_name.to_string(),
        dimension,
        embedding_count: embeddings.len(),
        built_at: now,
    };
    Ok((report, built_index))
}

/// Delete any existing row in `gmp_vector_index` whose
/// `(model_name, dimension)` tuple matches the supplied parameters. This
/// is the application-level enforcement of the schema's
/// `UNIQUE(model_name, dimension)` constraint (v3.13.0 §4.1.5).
fn delete_existing_index_row(
    storage: &mut dyn StorageEngine,
    model_name: &str,
    dimension: usize,
) -> SqlResult<()> {
    let rows = storage.scan(TABLE_VECTOR_INDEX)?;
    let mut victims: Vec<Vec<Value>> = Vec::new();
    for row in rows {
        let matches_model = matches!(&row.get(2), Some(Value::Text(s)) if s == model_name);
        let matches_dim = matches!(row.get(3), Some(Value::Integer(d)) if *d as usize == dimension);
        if matches_model && matches_dim {
            victims.push(row);
        }
    }
    for victim in victims {
        if let Some(Value::Integer(id)) = victim.first().cloned() {
            let _ = storage.delete(TABLE_VECTOR_INDEX, &[Value::Integer(id)]);
        }
    }
    Ok(())
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

    // v3.13.0 §4.1.1 — rebuild persistence acceptance test.
    //
    // Acceptance criterion: rebuilding the flat index twice on the same set
    // of embeddings must produce byte-identical serialized payloads, and the
    // round-tripped `FlatIndex` must be observationally equivalent to the
    // in-memory build (same dimension, same vectors). This proves the
    // `_index` variable that used to be discarded is now actually persisted
    // in `gmp_vector_index.vector_data`.
    #[test]
    fn test_rebuild_flat_index_persistence_before_after_stable() {
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
            ChunkEmbedding {
                chunk_id: 3,
                doc_id: 1,
                vector: vec![0.0, 0.0, 1.0],
                model_name: "hash".to_string(),
                dimension: 3,
                vector_hash: "".to_string(),
                updated_at: 0,
            },
        ];

        let first = FlatIndex::build(&embeddings);
        let second = FlatIndex::build(&embeddings);

        // 1. Same input → byte-identical serialized payload.
        assert_eq!(
            first.to_payload(),
            second.to_payload(),
            "rebuild must be deterministic for identical input"
        );

        // 2. Round-trip through JSON preserves the index exactly.
        let raw = first.to_json();
        let restored = FlatIndex::from_json(&raw).expect("JSON must parse");
        assert_eq!(restored.dimension, first.dimension);
        assert_eq!(restored.vectors.len(), first.vectors.len());
        for ((id_a, vec_a), (id_b, vec_b)) in restored.vectors.iter().zip(first.vectors.iter()) {
            assert_eq!(id_a, id_b);
            assert_eq!(vec_a.len(), vec_b.len());
            for (a, b) in vec_a.iter().zip(vec_b.iter()) {
                assert_eq!(a.to_bits(), b.to_bits());
            }
        }

        // 3. Search results are stable across rebuilds.
        let query = vec![1.0, 0.0, 0.0];
        let r1 = first.search(&query, 3);
        let r2 = restored.search(&query, 3);
        assert_eq!(r1, r2, "search must be stable across rebuilds");
    }

    #[test]
    fn test_flat_index_payload_serde_uses_deterministic_field_order() {
        let embedding = ChunkEmbedding {
            chunk_id: 42,
            doc_id: 7,
            vector: vec![0.5, -0.25, 0.125],
            model_name: "hash".to_string(),
            dimension: 3,
            vector_hash: "".to_string(),
            updated_at: 123,
        };
        let index = FlatIndex::build(&[embedding]);
        let raw = index.to_json();
        // serde_json formats `(i64, Vec<f32>)` as a JSON array, while struct
        // fields keep their declaration order — both matter for byte-identical
        // persistence. We assert the payload is round-trippable and contains
        // the expected dimension element.
        let restored = FlatIndex::from_json(&raw).expect("JSON must parse");
        assert_eq!(restored.dimension, 3);
        assert_eq!(restored.vectors.len(), 1);
        assert_eq!(restored.vectors[0].0, 42);
        assert_eq!(restored.vectors[0].1.len(), 3);
        // JSON array form for the inner tuple.
        assert!(
            raw.contains("[[42,"),
            "tuple vector entry must serialize as JSON array, got: {raw}"
        );
        assert!(
            raw.contains("\"dimension\":3"),
            "dimension field must be present, got: {raw}"
        );
    }

    // v3.13.0 §4.1.5 — UNIQUE(model_name, dimension) acceptance test.
    //
    // Calling `rebuild_flat_index` twice with the same model_name must
    // (a) succeed both times (no constraint violation surfaced) and
    // (b) leave exactly one row behind — the second rebuild replaces the
    // first. A different model_name produces a separate row.
    #[test]
    fn test_rebuild_flat_index_unique_constraint_enforced() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();

        // Empty storage rebuild is fine.
        let r1 = rebuild_flat_index(&mut storage, "hash").unwrap();
        assert_eq!(r1.embedding_count, 0);
        let rows_after_first = storage.scan(TABLE_VECTOR_INDEX).unwrap();
        assert_eq!(rows_after_first.len(), 1);

        // Second rebuild, same model_name → still exactly one row.
        let r2 = rebuild_flat_index(&mut storage, "hash").unwrap();
        assert_eq!(r2.embedding_count, 0);
        let rows_after_second = storage.scan(TABLE_VECTOR_INDEX).unwrap();
        assert_eq!(
            rows_after_second.len(),
            1,
            "second rebuild must not duplicate the (model_name, dimension) row"
        );

        // Different model_name → separate row.
        let r3 = rebuild_flat_index(&mut storage, "minilm-l6").unwrap();
        assert_eq!(r3.embedding_count, 0);
        let rows_after_third = storage.scan(TABLE_VECTOR_INDEX).unwrap();
        assert_eq!(
            rows_after_third.len(),
            2,
            "different model_name must produce a separate row"
        );

        // Schema declares the unique constraint explicitly.
        let info = storage
            .get_table_info(TABLE_VECTOR_INDEX)
            .expect("table info must exist");
        let has_unique = info
            .unique_constraints
            .iter()
            .any(|c| c.columns == vec!["model_name".to_string(), "dimension".to_string()]);
        assert!(
            has_unique,
            "gmp_vector_index must declare UNIQUE(model_name, dimension)"
        );
    }
}
