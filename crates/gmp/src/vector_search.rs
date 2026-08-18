//! GMP Vector Similarity Search
//!
//! Provides vector-based document search functionality.
//! Searches document embeddings using cosine similarity.

use crate::document::{Document, TABLE_DOCUMENTS};
use crate::embedding::{
    cosine_similarity, DocumentEmbedding, EmbeddingModel, HashEmbeddingModel, EMBEDDING_DIM,
    TABLE_EMBEDDINGS,
};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlError, SqlResult, Value};

/// Search result with document ID and similarity score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: i64,
    pub title: String,
    pub doc_type: String,
    pub similarity: f32,
}

/// Create the embeddings table if it doesn't exist.
/// Columns: chunk_id (PK), embedding (JSON), updated_at, model_name, dimension, vector_hash
pub fn create_embeddings_table(storage: &mut dyn StorageEngine) -> SqlResult<()> {
    if !storage.has_table(TABLE_EMBEDDINGS) {
        let columns = vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "chunk_id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "embedding".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "updated_at".to_string(),
                data_type: "INTEGER".to_string(),
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
                name: "vector_hash".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_EMBEDDINGS.to_string(),
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

/// Store or update an embedding for a chunk.
/// Stores model_name and vector_hash for auditability.
///
/// v3.13.0 §4.1.2 dimension drift fail-closed: rejects silently mismatched
/// dimensions instead of letting the index accumulate rows of incompatible
/// shape. `embedding.len() != EMBEDDING_DIM` returns `ConstraintViolation`
/// before any row is written.
///
/// v3.13.0 §4.1.4 model-name consistency: caller must pass the canonical
/// model name. The name is recorded in `gmp_embeddings.model_name` and
/// verified by `vector_search` to ensure all stored vectors come from the
/// same embedding model.
pub fn upsert_embedding(
    storage: &mut dyn StorageEngine,
    chunk_id: i64,
    embedding: &[f32],
    model_name: &str,
) -> SqlResult<()> {
    use crate::vector_index::vector_hash;

    create_embeddings_table(storage)?;

    // v3.13.0 §4.1.2 — fail-closed on dimension drift.
    let dimension = embedding.len();
    if dimension != EMBEDDING_DIM {
        return Err(SqlError::ConstraintViolation(format!(
            "embedding dimension drift for chunk_id={chunk_id}: got {dimension}, expected {EMBEDDING_DIM}"
        )));
    }

    // v3.13.0 §4.1.4 — model_name must be non-empty and free of NULs.
    if model_name.is_empty() {
        return Err(SqlError::ConstraintViolation(
            "embedding model_name must not be empty".to_string(),
        ));
    }
    if model_name.contains('\0') {
        return Err(SqlError::ConstraintViolation(format!(
            "embedding model_name contains NUL byte: {model_name:?}"
        )));
    }

    let json = DocumentEmbedding::embedding_to_json(embedding);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let hash_str = vector_hash(embedding);

    // Check if embedding already exists for this chunk
    let rows = storage.scan(TABLE_EMBEDDINGS)?;
    let existing = rows
        .iter()
        .any(|row| matches!(&row[0], Value::Integer(id) if *id == chunk_id));

    if existing {
        let _ = storage.delete(TABLE_EMBEDDINGS, &[Value::Integer(chunk_id)]);
    }

    let row = vec![
        Value::Integer(chunk_id),
        Value::Text(json),
        Value::Integer(now),
        Value::Text(model_name.to_string()),
        Value::Integer(dimension as i64),
        Value::Text(hash_str),
    ];
    storage.insert(TABLE_EMBEDDINGS, vec![row])?;
    Ok(())
}

/// Get all stored embeddings
pub fn get_all_embeddings(storage: &dyn StorageEngine) -> SqlResult<Vec<DocumentEmbedding>> {
    let rows = storage.scan(TABLE_EMBEDDINGS)?;
    let embeddings = rows
        .into_iter()
        .filter_map(|row| {
            let doc_id = match &row[0] {
                Value::Integer(n) => *n,
                _ => return None,
            };
            let embedding_str = match &row[1] {
                Value::Text(s) => s.clone(),
                _ => return None,
            };
            let updated_at = match &row[2] {
                Value::Integer(n) => *n,
                _ => return None,
            };
            let embedding = DocumentEmbedding::embedding_from_json(&embedding_str);
            Some(DocumentEmbedding {
                doc_id,
                embedding,
                updated_at,
            })
        })
        .collect();
    Ok(embeddings)
}

/// Perform vector similarity search
///
/// Searches all stored document embeddings and returns the top_k most similar
/// documents to the query text.
///
/// v3.13.0 §4.1.3 empty index fail-closed: when the embeddings table is
/// empty, return `ConstraintViolation` instead of silently returning an
/// empty result. Callers that previously observed `Ok(vec![])` for an
/// index-less storage now see an explicit error and can decide how to
/// recover (build the index first, or surface the failure to the user).
///
/// v3.13.0 §4.1.4 model-name consistency: every stored embedding must share
/// the same `model_name` as the query generator. Comparing vectors from
/// different models is meaningless, so a heterogeneous index triggers
/// `ConstraintViolation` rather than silently producing nonsense scores.
pub fn vector_search(
    storage: &dyn StorageEngine,
    query: &str,
    top_k: usize,
) -> SqlResult<Vec<SearchResult>> {
    let model = HashEmbeddingModel::default();
    let query_embedding = model.generate_embedding(query);
    let query_model_name = model.model_name();

    // Get all embeddings
    let embeddings = get_all_embeddings(storage)?;

    // v3.13.0 §4.1.3 — fail-closed on empty index.
    if embeddings.is_empty() {
        return Err(SqlError::ConstraintViolation(
            "vector index is empty; ingest documents and rebuild the index before searching"
                .to_string(),
        ));
    }

    // v3.13.0 §4.1.4 — verify model-name consistency.
    //
    // We scan the table directly (not the parsed `DocumentEmbedding` payload)
    // because the model_name column is not carried on `DocumentEmbedding`.
    let rows = storage.scan(TABLE_EMBEDDINGS)?;
    let mut distinct: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for row in &rows {
        if let Some(Value::Text(name)) = row.get(3) {
            distinct.insert(name.clone());
        }
    }
    if distinct.len() > 1 {
        let mut sorted: Vec<&String> = distinct.iter().collect();
        sorted.sort();
        return Err(SqlError::ConstraintViolation(format!(
            "vector index has inconsistent model_name set: {:?} (expected all equal to {:?})",
            sorted, query_model_name
        )));
    }
    if let Some(only) = distinct.iter().next() {
        if only != query_model_name {
            return Err(SqlError::ConstraintViolation(format!(
                "vector index model_name={only:?} does not match query generator={query_model_name:?}"
            )));
        }
    }

    // Compute similarities
    let mut results: Vec<(i64, f32)> = embeddings
        .into_iter()
        .map(|emb| {
            let sim = cosine_similarity(&query_embedding, &emb.embedding);
            (emb.doc_id, sim)
        })
        .collect();

    // Sort by similarity (descending)
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top_k
    let top_results = results.into_iter().take(top_k).collect::<Vec<_>>();

    // Get document metadata
    let mut search_results = Vec::new();
    let all_doc_rows = storage.scan(TABLE_DOCUMENTS)?;

    for (doc_id, similarity) in top_results {
        // Find document by id
        if let Some(row) = all_doc_rows
            .iter()
            .find(|row| matches!(&row[0], Value::Integer(id) if *id == doc_id))
        {
            let title = row
                .get(1)
                .and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let doc_type = row
                .get(2)
                .and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            search_results.push(SearchResult {
                doc_id,
                title,
                doc_type,
                similarity,
            });
        }
    }

    Ok(search_results)
}

/// Hybrid search combining vector and keyword search
pub fn hybrid_search(
    storage: &dyn StorageEngine,
    query: &str,
    top_k: usize,
    text_boost: f32,
) -> SqlResult<Vec<SearchResult>> {
    let vector_results = vector_search(storage, query, top_k * 2)?;

    let keyword_lower = query.to_lowercase();
    let mut scored: Vec<_> = vector_results
        .into_iter()
        .map(|mut r| {
            if r.title.to_lowercase().contains(&keyword_lower)
                || r.doc_type.to_lowercase().contains(&keyword_lower)
            {
                r.similarity = (r.similarity + text_boost).min(1.0);
            }
            r
        })
        .collect();

    scored.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);
    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_search_empty() {
        let storage = sqlrustgo_storage::MemoryStorage::new();
        // v3.13.0 §4.1.3: empty index now fails closed with ConstraintViolation.
        let err = vector_search(&storage, "test", 5).unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "empty index must fail closed, got {err:?}"
        );
        assert!(
            err.to_string().contains("vector index is empty"),
            "error message must explain the failure mode, got: {err}"
        );
    }

    #[test]
    fn test_hybrid_search_text_boost() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        // Seed at least one embedding so the underlying vector_search does
        // not trip the empty-index fail-closed path. Note that the
        // embedding's doc_id needs no matching document row for the
        // pass-through (hybrid_search returns `vec![]` if no document
        // matches); what we're checking is that the call does NOT error
        // out with `ConstraintViolation`.
        let seed = vec![0.1f32; EMBEDDING_DIM];
        upsert_embedding(&mut storage, 1, &seed, "hash").unwrap();
        // Should not error — the empty-index fail-closed path is not
        // triggered because we have a seeded embedding.
        let results = hybrid_search(&storage, "test query", 5, 0.5).unwrap();
        // No document exists for doc_id=1, so the search returns empty —
        // the test is concerned with the error path, not the result shape.
        assert!(results.is_empty());
    }

    // v3.13.0 §4.1.3 — empty index fail-closed acceptance test.
    //
    // Even hybrid_search propagates the fail-closed error from vector_search
    // when the index has nothing to score. This is what makes the regression
    // detectable: a fail-open implementation would silently return `vec![]`.
    #[test]
    fn test_hybrid_search_empty_index_fails_closed() {
        let storage = sqlrustgo_storage::MemoryStorage::new();
        let err = hybrid_search(&storage, "anything", 5, 0.1).unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "hybrid_search on empty index must surface the fail-closed error, got {err:?}"
        );
    }

    #[test]
    fn test_search_result_struct() {
        let r = SearchResult {
            doc_id: 1,
            title: "Test".to_string(),
            doc_type: "type".to_string(),
            similarity: 0.95,
        };
        assert_eq!(r.doc_id, 1);
        assert_eq!(r.similarity, 0.95);
    }

    #[test]
    fn test_upsert_embedding_update() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        let emb1 = vec![0.1f32; 256];
        let emb2 = vec![0.2f32; 256];

        upsert_embedding(&mut storage, 1, &emb1, "hash").unwrap();
        upsert_embedding(&mut storage, 1, &emb2, "hash").unwrap();

        let all = get_all_embeddings(&storage).unwrap();
        assert_eq!(all.len(), 1);
    }

    // v3.13.0 §4.1.2 — dimension drift fail-closed acceptance test.
    //
    // Upserts with mismatched dimension must be rejected with
    // `ConstraintViolation` and must NOT leave a partial row in the table.
    #[test]
    fn test_upsert_embedding_dimension_drift_fail_closed() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();

        // Wrong dimension (255) must be rejected.
        let wrong = vec![0.1f32; 255];
        let err = upsert_embedding(&mut storage, 1, &wrong, "hash").unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "wrong dimension must return ConstraintViolation, got {err:?}"
        );
        assert!(
            err.to_string().contains("dimension drift"),
            "error message must explain the failure mode, got: {err}"
        );

        // Empty embedding must also be rejected.
        let empty: Vec<f32> = vec![];
        let err = upsert_embedding(&mut storage, 2, &empty, "hash").unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "empty embedding must be rejected, got {err:?}"
        );

        // Failed inserts must not have written any rows.
        let all = get_all_embeddings(&storage).unwrap();
        assert_eq!(
            all.len(),
            0,
            "failed dimension-drift inserts must not leak rows"
        );

        // Correct dimension must succeed.
        let ok = vec![0.3f32; EMBEDDING_DIM];
        upsert_embedding(&mut storage, 3, &ok, "hash").unwrap();
        let all = get_all_embeddings(&storage).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_upsert_embedding_accepts_exact_embedding_dim() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        let emb = vec![0.42f32; EMBEDDING_DIM];
        // Should not panic, should not error.
        upsert_embedding(&mut storage, 99, &emb, "hash").unwrap();
        let all = get_all_embeddings(&storage).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].embedding.len(), EMBEDDING_DIM);
    }

    // v3.13.0 §4.1.4 — model-name consistency acceptance test.
    //
    // 1. Single homogeneous model_name → query succeeds.
    // 2. Mixed model_name (two different names stored) → fail-closed.
    // 3. Mismatched single model_name (rows = "minilm-l6", query = "hash")
    //    → fail-closed.
    // 4. Empty / NUL model_name rejected at insert time.
    #[test]
    fn test_vector_search_model_name_consistency_fail_closed() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();

        // 1. Homogeneous — must succeed.
        let emb_a = vec![0.1f32; EMBEDDING_DIM];
        upsert_embedding(&mut storage, 1, &emb_a, "hash").unwrap();
        let emb_b = vec![0.2f32; EMBEDDING_DIM];
        upsert_embedding(&mut storage, 2, &emb_b, "hash").unwrap();
        // Need embeddings for `vector_search` to not trip the empty-index
        // check; the model-name check is the assertion here.
        let rows = storage.scan(crate::embedding::TABLE_EMBEDDINGS).unwrap();
        let mut distinct: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for row in &rows {
            if let Some(Value::Text(name)) = row.get(3) {
                distinct.insert(name.clone());
            }
        }
        assert_eq!(
            distinct.len(),
            1,
            "homogeneous index must have one model_name"
        );
        assert!(distinct.contains("hash"));

        // 2. Mixed — write a row with a different model_name directly via
        //    upsert (validation accepts any non-empty non-NUL string).
        let emb_c = vec![0.3f32; EMBEDDING_DIM];
        upsert_embedding(&mut storage, 3, &emb_c, "minilm-l6").unwrap();
        let rows = storage.scan(crate::embedding::TABLE_EMBEDDINGS).unwrap();
        let mut distinct2: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for row in &rows {
            if let Some(Value::Text(name)) = row.get(3) {
                distinct2.insert(name.clone());
            }
        }
        assert_eq!(distinct2.len(), 2, "mixed index must have 2 model_names");
        let err = vector_search(&storage, "anything", 5).unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "mixed model_name set must fail closed, got {err:?}"
        );
        assert!(
            err.to_string().contains("inconsistent model_name set"),
            "error message must explain the failure mode, got: {err}"
        );

        // 3. Single non-matching model_name — must also fail.
        let mut storage2 = sqlrustgo_storage::MemoryStorage::new();
        let emb_d = vec![0.4f32; EMBEDDING_DIM];
        upsert_embedding(&mut storage2, 1, &emb_d, "minilm-l6").unwrap();
        let err = vector_search(&storage2, "anything", 5).unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "single but mismatched model_name must fail closed, got {err:?}"
        );
        assert!(
            err.to_string().contains("does not match query generator"),
            "error message must explain the mismatch, got: {err}"
        );

        // 4. Empty model_name rejected at insert.
        let mut storage3 = sqlrustgo_storage::MemoryStorage::new();
        let emb_e = vec![0.5f32; EMBEDDING_DIM];
        let err = upsert_embedding(&mut storage3, 1, &emb_e, "").unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "empty model_name must be rejected, got {err:?}"
        );

        // 5. NUL byte model_name rejected at insert.
        let err = upsert_embedding(&mut storage3, 2, &emb_e, "hash\0other").unwrap_err();
        assert!(
            matches!(err, SqlError::ConstraintViolation(_)),
            "NUL-containing model_name must be rejected, got {err:?}"
        );
    }
}
