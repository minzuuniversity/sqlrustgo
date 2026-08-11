//! GMP Chunk Management
//!
//! Provides chunk-level storage for GMP documents.
//! Documents are split into chunks at ingestion time; each chunk
//! is stored with a content hash for deduplication and citation.

use crate::schema::TABLE_CHUNKS;
use crate::version::sha256_str;
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};

/// A chunk of document content.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: i64,
    pub doc_id: i64,
    pub version_number: i32,
    pub chunk_index: i32,
    pub section_name: Option<String>,
    pub content_hash: String,
    pub content_text: String,
    pub created_at: i64,
}

impl Chunk {
    /// Parse a Chunk from a storage engine row.
    /// Expected column order: id, doc_id, version_number, chunk_index, section_name, content_hash, content_text, created_at
    pub fn from_row(row: &[Value]) -> Option<Self> {
        Some(Chunk {
            id: match &row.get(0)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
            doc_id: match &row.get(1)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
            version_number: match &row.get(2)? {
                Value::Integer(n) => *n as i32,
                _ => return None,
            },
            chunk_index: match &row.get(3)? {
                Value::Integer(n) => *n as i32,
                _ => return None,
            },
            section_name: match &row.get(4)? {
                Value::Text(s) => Some(s.clone()),
                Value::Null => None,
                _ => return None,
            },
            content_hash: match &row.get(5)? {
                Value::Text(s) => s.clone(),
                _ => return None,
            },
            content_text: match &row.get(6)? {
                Value::Text(s) => s.clone(),
                _ => return None,
            },
            created_at: match &row.get(7)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
        })
    }

    /// Convert to a database row.
    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.id),
            Value::Integer(self.doc_id),
            Value::Integer(self.version_number as i64),
            Value::Integer(self.chunk_index as i64),
            self.section_name
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            Value::Text(self.content_hash.clone()),
            Value::Text(self.content_text.clone()),
            Value::Integer(self.created_at),
        ]
    }

    /// Compute the content hash for a given text.
    pub fn compute_hash(text: &str) -> String {
        sha256_str(text)
    }
}

/// Insert a chunk. Uses upsert logic: if a chunk with the same (doc_id, version_number, chunk_index)
/// already exists, it is replaced.
pub fn insert_chunk(
    storage: &mut dyn StorageEngine,
    doc_id: i64,
    version_number: i32,
    chunk_index: i32,
    section_name: Option<&str>,
    content_text: &str,
) -> SqlResult<i64> {
    let content_hash = Chunk::compute_hash(content_text);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Check if chunk already exists
    let existing = get_chunk_by_index(storage, doc_id, version_number, chunk_index)?;

    if let Some(existing_chunk) = existing {
        // Upsert: update existing
        let rows = storage.scan(TABLE_CHUNKS)?;
        for row in rows {
            if let Some(chunk) = Chunk::from_row(&row) {
                if chunk.id == existing_chunk.id {
                    let updated = vec![
                        Value::Integer(chunk.id),
                        Value::Integer(doc_id),
                        Value::Integer(version_number as i64),
                        Value::Integer(chunk_index as i64),
                        section_name
                            .map(|s| Value::Text(s.to_string()))
                            .unwrap_or(Value::Null),
                        Value::Text(content_hash),
                        Value::Text(content_text.to_string()),
                        Value::Integer(timestamp),
                    ];
                    // Delete old, insert new (simple upsert)
                    storage.delete(TABLE_CHUNKS, &[Value::Integer(chunk.id)])?;
                    storage.insert(TABLE_CHUNKS, vec![updated])?;
                    return Ok(chunk.id);
                }
            }
        }
    }

    // Get next ID
    let rows = storage.scan(TABLE_CHUNKS)?;
    let next_id = rows
        .iter()
        .filter_map(|r| match r.get(0)? {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;

    let row = vec![
        Value::Integer(next_id),
        Value::Integer(doc_id),
        Value::Integer(version_number as i64),
        Value::Integer(chunk_index as i64),
        section_name
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
        Value::Text(content_hash),
        Value::Text(content_text.to_string()),
        Value::Integer(timestamp),
    ];

    storage.insert(TABLE_CHUNKS, vec![row])?;
    Ok(next_id)
}

/// Get all chunks for a specific (doc_id, version_number) ordered by chunk_index.
pub fn get_chunks_for_version(
    storage: &dyn StorageEngine,
    doc_id: i64,
    version_number: i32,
) -> SqlResult<Vec<Chunk>> {
    let rows = storage.scan(TABLE_CHUNKS)?;
    let mut chunks: Vec<Chunk> = rows
        .into_iter()
        .filter_map(|r| {
            let c = Chunk::from_row(&r)?;
            if c.doc_id == doc_id && c.version_number == version_number {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    chunks.sort_by_key(|c| c.chunk_index);
    Ok(chunks)
}

/// Get a specific chunk by (doc_id, version_number, chunk_index).
pub fn get_chunk_by_index(
    storage: &dyn StorageEngine,
    doc_id: i64,
    version_number: i32,
    chunk_index: i32,
) -> SqlResult<Option<Chunk>> {
    let chunks = get_chunks_for_version(storage, doc_id, version_number)?;
    Ok(chunks.into_iter().find(|c| c.chunk_index == chunk_index))
}

/// Get a chunk by its primary key id.
pub fn get_chunk_by_id(storage: &dyn StorageEngine, id: i64) -> SqlResult<Option<Chunk>> {
    let rows = storage.scan(TABLE_CHUNKS)?;
    let chunk = rows
        .into_iter()
        .filter_map(|r| Chunk::from_row(&r))
        .find(|c| c.id == id);
    Ok(chunk)
}

/// Delete all chunks for a (doc_id, version_number) — used before re-importing.
pub fn delete_chunks_for_version(
    storage: &mut dyn StorageEngine,
    doc_id: i64,
    version_number: i32,
) -> SqlResult<usize> {
    let chunks = get_chunks_for_version(storage, doc_id, version_number)?;
    let count = chunks.len();
    for chunk in chunks {
        storage.delete(TABLE_CHUNKS, &[Value::Integer(chunk.id)])?;
    }
    Ok(count)
}

/// Chunking configuration.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum characters per chunk.
    pub chunk_size: usize,
    /// Optional overlap between chunks (in characters).
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 500,
            overlap: 50,
        }
    }
}

/// Simple text chunker: splits text by newlines into sections.
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<(String, Option<String>)> {
    // Split on double newlines (paragraphs) first
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;

    for para in paragraphs {
        let para_len = para.trim().len();
        if current_len + para_len + 2 > config.chunk_size && !current.is_empty() {
            // Flush current chunk
            let section_name = extract_heading(&current);
            chunks.push((current.trim().to_string(), section_name));
            current = String::new();
            current_len = 0;
        }
        if !current.is_empty() {
            current.push_str("\n\n");
            current_len += 2;
        }
        current.push_str(para);
        current_len += para_len;
    }

    if !current.trim().is_empty() {
        let section_name = extract_heading(&current);
        chunks.push((current.trim().to_string(), section_name));
    }

    chunks
}

/// Extract a section/heading name from chunk text (first line if it looks like a heading).
fn extract_heading(text: &str) -> Option<String> {
    let first_line = text.lines().next()?;
    let trimmed = first_line.trim();
    if trimmed.starts_with('#') {
        Some(trimmed.trim_start_matches('#').trim().to_string())
    } else if trimmed.len() < 80 {
        Some(trimmed.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_chunk_compute_hash() {
        let h1 = Chunk::compute_hash("hello");
        let h2 = Chunk::compute_hash("hello");
        let h3 = Chunk::compute_hash("world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_insert_and_get_chunks() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_chunk(&mut storage, 1, 1, 0, Some("Introduction"), "Hello world.").unwrap();
        insert_chunk(
            &mut storage,
            1,
            1,
            1,
            Some("Section 1"),
            "More content here.",
        )
        .unwrap();

        let chunks = get_chunks_for_version(&storage, 1, 1).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].chunk_index, 1);
    }

    #[test]
    fn test_chunk_upsert() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_chunk(&mut storage, 1, 1, 0, None, "Original").unwrap();
        insert_chunk(&mut storage, 1, 1, 0, None, "Updated").unwrap();

        let chunks = get_chunks_for_version(&storage, 1, 1).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_text, "Updated");
    }

    #[test]
    fn test_delete_chunks_for_version() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_chunk(&mut storage, 1, 1, 0, None, "Content A").unwrap();
        insert_chunk(&mut storage, 1, 1, 1, None, "Content B").unwrap();

        let deleted = delete_chunks_for_version(&mut storage, 1, 1).unwrap();
        assert_eq!(deleted, 2);

        let chunks = get_chunks_for_version(&storage, 1, 1).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_text() {
        let config = ChunkConfig::default();
        let text = "Introduction\n\nThis is a long paragraph that should be split.\nIt has multiple lines.\n\n## Section One\n\nSection content here.\n\n## Section Two\n\nMore content.";
        let chunks = chunk_text(text, &config);
        assert!(!chunks.is_empty());
        // All chunks should have non-empty content
        for (text, section) in &chunks {
            assert!(!text.is_empty());
        }
    }

    #[test]
    fn test_extract_heading() {
        assert_eq!(
            extract_heading("# Introduction"),
            Some("Introduction".to_string())
        );
        assert_eq!(
            extract_heading("## Section 1.2"),
            Some("Section 1.2".to_string())
        );
        assert_eq!(
            extract_heading("This is a very long first line that does not look like a heading and exceeds 80 chars"),
            None
        );
    }
}
