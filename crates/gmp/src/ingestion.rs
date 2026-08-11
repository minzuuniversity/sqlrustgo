//! GMP Corpus Ingestion
//!
//! Ingests documents from a file-system corpus (e.g. `~/gmp-platform/gmp-md`)
//! into SQLRustGo-managed GMP tables.

use crate::chunk::{chunk_text, insert_chunk, ChunkConfig};
use crate::embedding::generate_embedding;
use crate::vector_search::upsert_embedding;
use crate::version::{
    compute_content_hash_from_chunks, find_version_by_source_hash, get_document_versions,
    insert_version, sha256_str,
};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};
use std::fs;

/// Ingestion report with statistics.
#[derive(Debug, Clone, Default)]
pub struct IngestionReport {
    pub documents_ingested: usize,
    pub documents_skipped: usize,
    pub documents_failed: usize,
    pub chunks_created: usize,
    pub embeddings_created: usize,
    pub relations_created: usize,
    pub skipped_files: Vec<String>,
    pub failed_files: Vec<String>,
    pub errors: Vec<String>,
}

impl IngestionReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn summary(&self) -> String {
        format!(
            "IngestionReport {{ ingested: {}, skipped: {}, failed: {}, chunks: {}, embeddings: {} }}",
            self.documents_ingested,
            self.documents_skipped,
            self.documents_failed,
            self.chunks_created,
            self.embeddings_created,
        )
    }
}

/// Ingest a single file into GMP tables.
/// Returns `(doc_id, version_number)` on success.
/// Returns `Err("SKIP")` if idempotent re-import detected.
/// Returns `Err("FAIL: <msg>")` on error.
pub fn ingest_file(
    storage: &mut dyn StorageEngine,
    file_path: &std::path::Path,
    source_path: &str,
) -> Result<(i64, i32), &'static str> {
    let content = fs::read_to_string(file_path).map_err(|_| "FAIL: read error")?;
    let source_hash = sha256_str(&content);

    let title = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    // Idempotency check: skip if same source_hash already ingested
    if find_version_by_source_hash(storage, &source_hash)
        .map_err(|_| "FAIL: db error")?
        .is_some()
    {
        return Err("SKIP");
    }

    let doc_id =
        ensure_document(storage, &title, source_path).map_err(|_| "FAIL: document error")?;

    let versions =
        get_document_versions(storage, doc_id).map_err(|_| "FAIL: version query error")?;
    let version_number = versions.iter().map(|v| v.version_number).max().unwrap_or(0) + 1;

    let config = ChunkConfig::default();
    let chunk_data = chunk_text(&content, &config);
    let mut chunk_hashes = Vec::new();

    for (idx, (chunk_text_content, section_name)) in chunk_data.iter().enumerate() {
        let chunk_hash = crate::chunk::Chunk::compute_hash(chunk_text_content);
        insert_chunk(
            storage,
            doc_id,
            version_number,
            idx as i32,
            section_name.as_deref(),
            chunk_text_content,
        )
        .map_err(|_| "FAIL: insert chunk error")?;
        chunk_hashes.push(chunk_hash);

        let embedding = generate_embedding(chunk_text_content);
        upsert_embedding(storage, idx as i64, &embedding)
            .map_err(|_| "FAIL: upsert embedding error")?;
    }

    let content_hash = compute_content_hash_from_chunks(&chunk_hashes);
    insert_version(
        storage,
        doc_id,
        version_number,
        &source_hash,
        &content_hash,
        None,
    )
    .map_err(|_| "FAIL: insert version error")?;

    Ok((doc_id, version_number))
}

fn ensure_document(
    storage: &mut dyn StorageEngine,
    title: &str,
    source_path: &str,
) -> SqlResult<i64> {
    use crate::document::TABLE_DOCUMENTS;

    let rows = storage.scan(TABLE_DOCUMENTS)?;
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

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let row = vec![
        Value::Integer(next_id),
        Value::Text(title.to_string()),
        Value::Text(source_path.to_string()),
        Value::Integer(1),
        Value::Integer(now),
        Value::Integer(now),
        Value::Integer(now),
        Value::Text("ACTIVE".to_string()),
    ];

    storage.insert(TABLE_DOCUMENTS, vec![row])?;
    Ok(next_id)
}

/// Ingest all files from a corpus directory recursively.
pub fn ingest_corpus(
    storage: &mut dyn StorageEngine,
    corpus_root: &std::path::Path,
    report: &mut IngestionReport,
) {
    for entry in walkdir(corpus_root) {
        if !entry.is_file {
            continue;
        }
        let ext = entry.path.extension().and_then(|s| s.to_str());
        if ext != Some("md") && ext != Some("txt") {
            continue;
        }

        let source_path = entry
            .path
            .strip_prefix(corpus_root)
            .unwrap_or(&entry.path)
            .to_string_lossy()
            .replace('\\', "/");

        match ingest_file(storage, &entry.path, &source_path) {
            Ok(_) => {
                report.documents_ingested += 1;
            }
            Err("SKIP") => {
                report.documents_skipped += 1;
                report.skipped_files.push(source_path);
            }
            Err(e) if e.starts_with("FAIL:") => {
                report.documents_failed += 1;
                let msg = e.strip_prefix("FAIL:").unwrap_or(e);
                report.failed_files.push(source_path.clone());
                report.errors.push(format!("{}: {}", source_path, msg));
            }
            Err(_) => {
                report.documents_failed += 1;
            }
        }
    }
}

struct WalkEntry {
    path: std::path::PathBuf,
    is_file: bool,
}

fn walkdir(root: &std::path::Path) -> Vec<WalkEntry> {
    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    results.push(WalkEntry {
                        path,
                        is_file: true,
                    });
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walkdir_finds_markdown_files() {
        let temp_dir = std::env::temp_dir().join("gmp_ingestion_test");
        let _ = fs::create_dir_all(&temp_dir);
        let _ = fs::write(temp_dir.join("doc1.md"), "# Doc 1");
        let _ = fs::write(temp_dir.join("doc2.txt"), "Doc 2 content");
        let _ = fs::write(temp_dir.join("ignored.pdf"), "PDF content");

        let entries: Vec<_> = walkdir(&temp_dir)
            .into_iter()
            .filter(|e| {
                e.path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext == "md" || ext == "txt")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(entries.len(), 2);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ingestion_report_default() {
        let report = IngestionReport::default();
        assert_eq!(report.documents_ingested, 0);
        assert_eq!(report.documents_skipped, 0);
    }

    #[test]
    fn test_ingestion_report_summary() {
        let mut report = IngestionReport::new();
        report.documents_ingested = 10;
        report.documents_skipped = 2;
        report.documents_failed = 1;
        report.chunks_created = 50;
        report.embeddings_created = 50;
        let s = report.summary();
        assert!(s.contains("ingested: 10"));
        assert!(s.contains("skipped: 2"));
        assert!(s.contains("chunks: 50"));
    }
}
