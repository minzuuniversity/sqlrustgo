//! GMP Vector Similarity Search
//!
//! Provides vector-based document search functionality.
//! Searches document embeddings using cosine similarity.

use crate::document::{Document, TABLE_DOCUMENTS};
use crate::embedding::{
    cosine_similarity, DocumentEmbedding, EmbeddingModel, HashEmbeddingModel, TABLE_EMBEDDINGS,
};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};

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
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "embedding".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "updated_at".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "model_name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "dimension".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "vector_hash".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_EMBEDDINGS.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
        })?;
    }
    Ok(())
}

/// Store or update an embedding for a chunk.
/// Stores model_name and vector_hash for auditability.
pub fn upsert_embedding(
    storage: &mut dyn StorageEngine,
    chunk_id: i64,
    embedding: &[f32],
) -> SqlResult<()> {
    use crate::vector_index::vector_hash;

    create_embeddings_table(storage)?;

    let json = DocumentEmbedding::embedding_to_json(embedding);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let model_name = "hash".to_string();
    let dimension = embedding.len();
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
        Value::Text(model_name),
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
pub fn vector_search(
    storage: &dyn StorageEngine,
    query: &str,
    top_k: usize,
) -> SqlResult<Vec<SearchResult>> {
    let model = HashEmbeddingModel::default();
    let query_embedding = model.generate_embedding(query);

    // Get all embeddings
    let embeddings = get_all_embeddings(storage)?;

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
        let results = vector_search(&storage, "test", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_hybrid_search_text_boost() {
        let storage = sqlrustgo_storage::MemoryStorage::new();
        let results = hybrid_search(&storage, "test query", 5, 0.5).unwrap();
        assert!(results.is_empty());
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

        upsert_embedding(&mut storage, 1, &emb1).unwrap();
        upsert_embedding(&mut storage, 1, &emb2).unwrap();

        let all = get_all_embeddings(&storage).unwrap();
        assert_eq!(all.len(), 1);
    }
}
