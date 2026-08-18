//! GMP SQL API Functions
//!
//! Provides SQL-compatible API functions for the GMP extension.
//! These functions work with the sqlrustgo ExecutionEngine.

use crate::audit::{create_audit_log_table, record_audit_log};
use crate::document::{
    create_gmp_tables, insert_document, insert_document_content, insert_document_keyword,
    DocStatus, NewDocument, TABLE_DOCUMENTS, TABLE_DOCUMENT_CONTENTS,
};
use crate::embedding::{default_model_name, generate_embedding, TABLE_EMBEDDINGS};
use crate::vector_search::{
    create_embeddings_table, hybrid_search, upsert_embedding, vector_search, SearchResult,
};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};
use std::sync::{Arc, RwLock};

/// GMP Executor - provides a convenient API for GMP operations
///
/// Wraps a storage engine reference and provides GMP-specific operations.
pub struct GmpExecutor {
    storage: Arc<RwLock<dyn StorageEngine>>,
}

impl GmpExecutor {
    /// Create a new GmpExecutor wrapping a storage engine
    pub fn new(storage: Arc<RwLock<dyn StorageEngine>>) -> Self {
        Self { storage }
    }

    /// Initialize all GMP tables (documents, keywords, contents, embeddings)
    pub fn init(&self) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        create_gmp_tables(&mut *storage)?;
        create_embeddings_table(&mut *storage)?;
        Ok(())
    }

    /// Import a document with its content, keywords, and embedding
    ///
    /// This is a convenience method that:
    /// 1. Inserts the document record
    /// 2. Inserts the content sections
    /// 3. Inserts the keywords
    /// 4. Generates and stores an embedding from the content
    pub fn import_document(
        &self,
        title: &str,
        doc_type: &str,
        content: &str,
        keywords: &[&str],
    ) -> SqlResult<i64> {
        let mut storage = self.storage.write().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Insert document
        let doc_id = insert_document(
            &mut *storage,
            NewDocument {
                title,
                doc_type,
                version: 1,
                created_at: now,
                updated_at: now,
                effective_date: (now / 86400) as i32,
                status: DocStatus::Active,
            },
        )?;

        // Insert content as a single section
        insert_document_content(&mut *storage, doc_id, "main", content)?;

        // Insert keywords
        for keyword in keywords {
            insert_document_keyword(&mut *storage, doc_id, keyword)?;
        }

        // Generate and store embedding from content
        let embedding = generate_embedding(content);
        upsert_embedding(&mut *storage, doc_id, &embedding, default_model_name())?;

        // v3.13.0 §4.2.4 — production wiring: record AuditAction::Import
        // chained into the audit log. Best-effort: failure does not roll
        // back the import, but does surface a warning so the operator
        // knows audit coverage dropped for this op.
        if let Err(e) = record_import_audit(&mut *storage, doc_id, title) {
            eprintln!("warning: import audit log write failed: {e}");
        }

        Ok(doc_id)
    }

    /// Search documents by vector similarity
    pub fn search(&self, query: &str, top_k: usize) -> SqlResult<Vec<SearchResult>> {
        let storage = self.storage.read().unwrap();
        vector_search(&*storage, query, top_k)
    }

    /// Hybrid search (text + vector) with default text boost of 0.3
    pub fn hybrid_search(&self, query: &str, top_k: usize) -> SqlResult<Vec<SearchResult>> {
        let storage = self.storage.read().unwrap();
        hybrid_search(&*storage, query, top_k, 0.3)
    }

    /// Generate an embedding for text
    pub fn vector_embed(&self, text: &str) -> Vec<f32> {
        generate_embedding(text)
    }

    /// Bulk import multiple documents
    pub fn bulk_import(
        &self,
        documents: &[(&str, &str, &str, Vec<&str>)], // (title, doc_type, content, keywords)
    ) -> SqlResult<Vec<i64>> {
        let mut ids = Vec::new();
        for (title, doc_type, content, keywords) in documents {
            let id = self.import_document(title, doc_type, content, keywords)?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Get document by ID
    pub fn get_document(&self, doc_id: i64) -> SqlResult<Option<crate::document::Document>> {
        use crate::document::Document;
        let storage = self.storage.read().unwrap();
        let rows = storage.scan(TABLE_DOCUMENTS)?;
        let doc = rows
            .into_iter()
            .find(|row| matches!(&row[0], Value::Integer(id) if *id == doc_id))
            .and_then(|row| Document::from_row(&row));
        Ok(doc)
    }

    /// Re-index all documents (regenerate embeddings from content)
    pub fn reindex_all(&self) -> SqlResult<usize> {
        let mut storage = self.storage.write().unwrap();

        // Get all documents
        let doc_rows = storage.scan(TABLE_DOCUMENTS)?;
        let docs: Vec<_> = doc_rows
            .into_iter()
            .filter_map(|row| crate::document::Document::from_row(&row))
            .collect();

        // Get all content
        let content_rows = storage.scan(TABLE_DOCUMENT_CONTENTS)?;

        let mut count = 0;
        for doc in docs {
            // Concatenate all content sections
            let content: String = content_rows
                .iter()
                .filter(|row| matches!(&row[0], Value::Integer(id) if *id == doc.id))
                .filter_map(|row| match &row[2] {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");

            if !content.is_empty() {
                let embedding = generate_embedding(&content);
                upsert_embedding(&mut *storage, doc.id, &embedding, default_model_name())?;
                // v3.13.0 §4.2.4 — record AuditAction::Update for each
                // reindexed document. Best-effort.
                if let Err(e) = record_reindex_audit(&mut *storage, doc.id) {
                    eprintln!("warning: reindex audit log write failed: {e}");
                }
                count += 1;
            }
        }

        Ok(count)
    }
}

fn record_import_audit(
    storage: &mut dyn StorageEngine,
    doc_id: i64,
    title: &str,
) -> SqlResult<i64> {
    create_audit_log_table(storage)?;
    record_audit_log(
        storage,
        "system",
        "IMPORT",
        TABLE_DOCUMENTS,
        Some(&doc_id.to_string()),
        None,
        Some(title),
        None,
        None,
    )
}

fn record_reindex_audit(storage: &mut dyn StorageEngine, doc_id: i64) -> SqlResult<i64> {
    create_audit_log_table(storage)?;
    record_audit_log(
        storage,
        "system",
        "UPDATE",
        TABLE_EMBEDDINGS,
        Some(&doc_id.to_string()),
        None,
        None,
        None,
        None,
    )
}

/// SQL statement builders for GMP operations
///
/// These return SQL strings that can be executed by the ExecutionEngine.
pub mod sql {
    use crate::document::{TABLE_DOCUMENTS, TABLE_DOCUMENT_KEYWORDS};

    /// SQL to initialize GMP tables
    pub const INIT_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS gmp_documents (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    doc_type TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    effective_date INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT'
);

CREATE TABLE IF NOT EXISTS gmp_document_contents (
    doc_id INTEGER NOT NULL,
    section TEXT NOT NULL,
    content TEXT NOT NULL,
    PRIMARY KEY (doc_id, section)
);

CREATE TABLE IF NOT EXISTS gmp_document_keywords (
    doc_id INTEGER NOT NULL,
    keyword TEXT NOT NULL,
    PRIMARY KEY (doc_id, keyword)
);

CREATE TABLE IF NOT EXISTS gmp_embeddings (
    doc_id INTEGER PRIMARY KEY,
    embedding TEXT NOT NULL,
    updated_at INTEGER NOT NULL
)
"#;

    /// Build a SELECT statement to find documents by keyword
    pub fn select_by_keyword(keyword: &str) -> String {
        format!(
            "SELECT d.* FROM {} d \
             JOIN {} dk ON d.id = dk.doc_id \
             WHERE dk.keyword = '{}'",
            TABLE_DOCUMENTS, TABLE_DOCUMENT_KEYWORDS, keyword
        )
    }

    /// Build a SELECT statement to find documents by type
    pub fn select_by_type(doc_type: &str) -> String {
        format!(
            "SELECT * FROM {} WHERE doc_type = '{}'",
            TABLE_DOCUMENTS, doc_type
        )
    }

    /// Build a SELECT statement to find documents by status
    pub fn select_by_status(status: &str) -> String {
        format!(
            "SELECT * FROM {} WHERE status = '{}'",
            TABLE_DOCUMENTS, status
        )
    }

    /// Build a SELECT statement to find documents by effective date
    pub fn select_by_date_range(from_date: i32, to_date: i32) -> String {
        format!(
            "SELECT * FROM {} WHERE effective_date BETWEEN {} AND {}",
            TABLE_DOCUMENTS, from_date, to_date
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_gmp_executor_import() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());

        executor.init().unwrap();

        let doc_id = executor
            .import_document(
                "Test Document",
                "TEST",
                "This is a test document about Rust programming",
                &["rust", "test", "programming"],
            )
            .unwrap();

        assert!(doc_id > 0);

        // Verify document was inserted
        let doc = executor.get_document(doc_id).unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().title, "Test Document");
    }

    #[test]
    fn test_gmp_executor_search() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());

        executor.init().unwrap();

        executor
            .import_document(
                "Rust Guide",
                "GUIDE",
                "Learn Rust programming language with this comprehensive guide",
                &["rust", "guide"],
            )
            .unwrap();

        executor
            .import_document(
                "Python Tips",
                "TIPS",
                "Python programming tips and tricks for developers",
                &["python", "tips"],
            )
            .unwrap();

        let results = executor.search("Rust programming", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Rust Guide");
    }

    #[test]
    fn test_bulk_import() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());

        executor.init().unwrap();

        let docs = vec![
            (
                "Doc 1",
                "TYPE1",
                "Content for document 1",
                vec!["key1", "key2"],
            ),
            (
                "Doc 2",
                "TYPE2",
                "Content for document 2",
                vec!["key2", "key3"],
            ),
            (
                "Doc 3",
                "TYPE1",
                "Content for document 3",
                vec!["key1", "key3"],
            ),
        ];

        let ids = executor.bulk_import(&docs).unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_sql_statement_builders() {
        assert!(sql::select_by_keyword("rust").contains("gmp_document_keywords"));
        assert!(sql::select_by_type("GUIDE").contains("GUIDE"));
        assert!(sql::select_by_status("ACTIVE").contains("ACTIVE"));
        assert!(sql::select_by_date_range(19000, 20000).contains("19000"));
    }
    #[test]
    fn test_gmp_executor_vector_embed() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        let emb = executor.vector_embed("hello world");
        assert!(!emb.is_empty());
    }

    #[test]
    fn test_gmp_executor_hybrid_search() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        executor.init().unwrap();
        executor
            .import_document("A", "T", "about rust", &["x"])
            .unwrap();
        let results = executor.hybrid_search("rust", 5).unwrap();
        // May be empty if hybrid not configured, just verify no error
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_gmp_executor_get_document_not_found() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        executor.init().unwrap();
        let doc = executor.get_document(999).unwrap();
        assert!(doc.is_none());
    }

    #[test]
    fn test_gmp_executor_reindex_all() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        executor.init().unwrap();
        executor
            .import_document("A", "T", "content A", &["x"])
            .unwrap();
        executor
            .import_document("B", "T", "content B", &["x"])
            .unwrap();
        let count = executor.reindex_all().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_gmp_executor_reindex_empty() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        executor.init().unwrap();
        let count = executor.reindex_all().unwrap();
        assert_eq!(count, 0);
    }

    // v3.13.0 §4.2.4 — production wiring: import_document records an
    // AuditAction::Import chained into the audit log.
    #[test]
    fn test_gmp_executor_import_writes_audit_log() {
        use crate::audit::TABLE_AUDIT_LOG;
        use crate::audit::{get_all_audit_logs, verify_audit_chain};
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        executor.init().unwrap();

        let doc_id = executor
            .import_document(
                "Audit Test Doc",
                "GUIDE",
                "this is a test for §4.2.4 production wiring",
                &["audit", "wiring"],
            )
            .unwrap();
        assert!(doc_id > 0);

        // The audit log must exist and contain exactly one IMPORT entry.
        let inner = storage.read().unwrap();
        assert!(
            inner.has_table(TABLE_AUDIT_LOG),
            "audit log table must exist after import"
        );
        let logs = get_all_audit_logs(&*inner).unwrap();
        assert_eq!(logs.len(), 1, "exactly one audit entry expected");
        assert_eq!(logs[0].action, "IMPORT");
        assert_eq!(logs[0].table_name, TABLE_DOCUMENTS);
        assert_eq!(
            logs[0].record_id.as_deref(),
            Some(doc_id.to_string().as_str()),
            "record_id must reference the imported doc"
        );
        assert_eq!(
            logs[0].new_value.as_deref(),
            Some("Audit Test Doc"),
            "new_value must be the imported document title"
        );

        let (ok, broken_at) = verify_audit_chain(&*inner).unwrap();
        assert!(ok, "chain must verify; broken_at={:?}", broken_at);
    }

    // v3.13.0 §4.2.4 — production wiring: reindex_all records an
    // AuditAction::Update per reindexed document.
    #[test]
    fn test_gmp_executor_reindex_writes_audit_log() {
        use crate::audit::{get_all_audit_logs, verify_audit_chain};
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let executor = GmpExecutor::new(storage.clone());
        executor.init().unwrap();

        executor
            .import_document("Reindex A", "T", "content A", &["x"])
            .unwrap();
        executor
            .import_document("Reindex B", "T", "content B", &["x"])
            .unwrap();

        // After two imports: 2 IMPORT entries already in the log.
        // reindex_all should append one UPDATE per doc → 2 more entries.
        let count = executor.reindex_all().unwrap();
        assert_eq!(count, 2);

        let inner = storage.read().unwrap();
        let logs = get_all_audit_logs(&*inner).unwrap();
        assert_eq!(
            logs.len(),
            4,
            "2 imports + 2 reindex updates = 4 entries; got {}",
            logs.len()
        );
        // Timeline: IMPORT, IMPORT, UPDATE, UPDATE.
        assert_eq!(logs[0].action, "IMPORT");
        assert_eq!(logs[1].action, "IMPORT");
        assert_eq!(logs[2].action, "UPDATE");
        assert_eq!(logs[3].action, "UPDATE");
        // Updates target the embeddings table (reindex changes embeddings).
        assert_eq!(logs[2].table_name, TABLE_EMBEDDINGS);
        assert_eq!(logs[3].table_name, TABLE_EMBEDDINGS);

        // The full chain must verify (4 entries chained together).
        let (ok, broken_at) = verify_audit_chain(&*inner).unwrap();
        assert!(ok, "chain must verify; broken_at={:?}", broken_at);
    }

    #[test]
    fn test_sql_init_tables() {
        assert!(sql::INIT_TABLES.contains("CREATE TABLE IF NOT EXISTS gmp_documents"));
        assert!(sql::INIT_TABLES.contains("CREATE TABLE IF NOT EXISTS gmp_document_contents"));
        assert!(sql::INIT_TABLES.contains("CREATE TABLE IF NOT EXISTS gmp_document_keywords"));
        assert!(sql::INIT_TABLES.contains("CREATE TABLE IF NOT EXISTS gmp_embeddings"));
    }
}
