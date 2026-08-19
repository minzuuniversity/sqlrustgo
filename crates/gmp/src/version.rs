//! GMP Document Version Management
//!
//! Provides version history tracking for GMP documents.
//! Each version records the source file hash, content hash, and metadata.

use crate::schema::TABLE_DOCUMENT_VERSIONS;
use sha2::{Digest, Sha256};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};

/// A version entry in the document version history.
#[derive(Debug, Clone)]
pub struct DocumentVersion {
    pub id: i64,
    pub doc_id: i64,
    pub version_number: i32,
    pub source_hash: String,
    pub content_hash: String,
    pub created_at: i64,
    pub change_desc: Option<String>,
}

impl DocumentVersion {
    /// Parse a DocumentVersion from a storage engine row.
    /// Expected column order: id, doc_id, version_number, source_hash, content_hash, created_at, change_desc
    pub fn from_row(row: &[Value]) -> Option<Self> {
        Some(DocumentVersion {
            id: match row.first()? {
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
            source_hash: match &row.get(3)? {
                Value::Text(s) => s.clone(),
                _ => return None,
            },
            content_hash: match &row.get(4)? {
                Value::Text(s) => s.clone(),
                _ => return None,
            },
            created_at: match &row.get(5)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
            change_desc: match &row.get(6)? {
                Value::Text(s) => Some(s.clone()),
                Value::Null => None,
                _ => return None,
            },
        })
    }

    /// Convert to a database row for insertion.
    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.id),
            Value::Integer(self.doc_id),
            Value::Integer(self.version_number as i64),
            Value::Text(self.source_hash.clone()),
            Value::Text(self.content_hash.clone()),
            Value::Integer(self.created_at),
            self.change_desc
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
        ]
    }
}

/// Compute SHA-256 hex of a byte slice.
pub fn sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Compute SHA-256 hex of a string.
pub fn sha256_str(s: &str) -> String {
    sha256(s.as_bytes())
}

/// Insert a new document version row.
/// Returns the new version id.
pub fn insert_version(
    storage: &mut dyn StorageEngine,
    doc_id: i64,
    version_number: i32,
    source_hash: &str,
    content_hash: &str,
    change_desc: Option<&str>,
) -> SqlResult<i64> {
    // Get next ID
    let rows = storage.scan(TABLE_DOCUMENT_VERSIONS)?;
    let next_id = rows
        .iter()
        .filter_map(|r| match r.first()? {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let row = vec![
        Value::Integer(next_id),
        Value::Integer(doc_id),
        Value::Integer(version_number as i64),
        Value::Text(source_hash.to_string()),
        Value::Text(content_hash.to_string()),
        Value::Integer(timestamp),
        change_desc
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
    ];

    storage.insert(TABLE_DOCUMENT_VERSIONS, vec![row])?;
    Ok(next_id)
}

/// Get all versions for a document ordered by version_number ascending.
pub fn get_document_versions(
    storage: &dyn StorageEngine,
    doc_id: i64,
) -> SqlResult<Vec<DocumentVersion>> {
    let rows = storage.scan(TABLE_DOCUMENT_VERSIONS)?;
    let mut versions: Vec<DocumentVersion> = rows
        .into_iter()
        .filter_map(|r| {
            let v = DocumentVersion::from_row(&r)?;
            if v.doc_id == doc_id {
                Some(v)
            } else {
                None
            }
        })
        .collect();
    versions.sort_by_key(|v| v.version_number);
    Ok(versions)
}

/// Get a specific document version by (doc_id, version_number).
pub fn get_document_version(
    storage: &dyn StorageEngine,
    doc_id: i64,
    version_number: i32,
) -> SqlResult<Option<DocumentVersion>> {
    let versions = get_document_versions(storage, doc_id)?;
    Ok(versions
        .into_iter()
        .find(|v| v.version_number == version_number))
}

/// Find a version by source hash (for idempotent re-import check).
/// Returns Some(version) if a version with the same source_hash already exists.
pub fn find_version_by_source_hash(
    storage: &dyn StorageEngine,
    source_hash: &str,
) -> SqlResult<Option<DocumentVersion>> {
    let rows = storage.scan(TABLE_DOCUMENT_VERSIONS)?;
    let version = rows
        .into_iter()
        .filter_map(|r| DocumentVersion::from_row(&r))
        .find(|v| v.source_hash == source_hash);
    Ok(version)
}

/// Get the highest version number for a document.
pub fn get_max_version_number(storage: &dyn StorageEngine, doc_id: i64) -> SqlResult<i32> {
    let versions = get_document_versions(storage, doc_id)?;
    Ok(versions.iter().map(|v| v.version_number).max().unwrap_or(0))
}

/// Compute content hash from a list of chunk content hashes.
pub fn compute_content_hash_from_chunks(chunk_hashes: &[String]) -> String {
    let combined = chunk_hashes.join("|");
    sha256_str(&combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_sha256() {
        let h = sha256(b"hello");
        assert_eq!(h.len(), 64);
        // Known SHA-256 of "hello"
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_insert_and_get_version() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_version(
            &mut storage,
            1,
            1,
            "abc123",
            "def456",
            Some("Initial import"),
        )
        .unwrap();

        let versions = get_document_versions(&storage, 1).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_number, 1);
        assert_eq!(versions[0].source_hash, "abc123");
        assert_eq!(versions[0].change_desc.as_deref(), Some("Initial import"));
    }

    #[test]
    fn test_find_version_by_source_hash() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_version(&mut storage, 1, 1, "abc123", "def456", None).unwrap();

        let found = find_version_by_source_hash(&storage, "abc123").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().source_hash, "abc123");

        let not_found = find_version_by_source_hash(&storage, "nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_idempotent_reimport() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        // First import
        let v1 = find_version_by_source_hash(&storage, "source_v1").unwrap();
        assert!(v1.is_none());

        insert_version(&mut storage, 1, 1, "source_v1", "hash1", None).unwrap();

        // Re-import same source — should find existing
        let v1again = find_version_by_source_hash(&storage, "source_v1").unwrap();
        assert!(v1again.is_some());

        // New version for changed content
        insert_version(&mut storage, 1, 2, "source_v2", "hash2", None).unwrap();
        let versions = get_document_versions(&storage, 1).unwrap();
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn test_content_hash_from_chunks() {
        let hashes = vec!["a".to_string(), "b".to_string()];
        let combined = compute_content_hash_from_chunks(&hashes);
        // Should be deterministic
        let combined2 = compute_content_hash_from_chunks(&hashes);
        assert_eq!(combined, combined2);
        assert_eq!(combined.len(), 64);
    }
}
