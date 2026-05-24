//! SQLRustGo Storage Engine Adapter
//!
//! This module provides an adapter that wraps the SQLRustGo FileStorage engine
//! to work with GMP Storage types. It bridges between the GMP document-centric
//! storage model and SQLRustGo's table-based persistent storage.

use crate::storage::StorageConfig;
use anyhow::{Context, Result};
use sqlrustgo_storage::engine::{
    ColumnDefinition, Record, StorageEngine as SqlrustgoStorageEngine, TableInfo,
};
use sqlrustgo_storage::fts::{ChineseTokenizer, InvertedIndex};
use sqlrustgo_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Mutex;

/// Adapter that wraps SQLRustGo FileStorage to provide GMP-compatible persistent storage
///
/// This adapter translates between GMP's document-centric storage model and
/// SQLRustGo's relational table-based persistent storage model.
/// Uses FileStorage for persistence (JSON files on disk) and InvertedIndex
/// for full-text search on document content.
pub struct SqlrustgoAdapter {
    /// Internal SQLRustGo file-based storage engine (persistent)
    engine: Mutex<FileStorage>,
    /// FTS inverted index for document content search
    fts_index: Mutex<InvertedIndex>,
    /// Chinese tokenizer for FTS
    tokenizer: ChineseTokenizer,
    /// Configuration for reference
    config: StorageConfig,
}

impl SqlrustgoAdapter {
    /// Create a new adapter with FileStorage backend
    ///
    /// StorageConfig.data_dir is used as the storage directory.
    /// Previous data is automatically loaded from disk. Buffering is disabled
    /// to ensure each write is immediately persisted.
    pub fn new(config: StorageConfig) -> Result<Self> {
        let data_dir = PathBuf::from(&config.data_dir).join("sqlrustgo_storage");
        let engine = FileStorage::new_with_buffer_config(data_dir, 1, false)
            .map_err(|e| anyhow::anyhow!("Failed to create FileStorage: {}", e))?;

        Ok(Self {
            engine: Mutex::new(engine),
            fts_index: Mutex::new(InvertedIndex::new()),
            tokenizer: ChineseTokenizer::new(),
            config,
        })
    }

    /// Initialize the GMP schema in the SQLRustGo storage engine
    pub fn init_gmp_schema(&self) -> Result<()> {
        let mut engine = self.engine.lock().unwrap();

        // Create documents table
        let docs_table = TableInfo {
            name: "gmp_documents".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("doc_id", "TEXT"),
                ColumnDefinition::new("code", "TEXT"),
                ColumnDefinition::new("title", "TEXT"),
                ColumnDefinition::new("chapter", "TEXT"),
                ColumnDefinition::new("doc_type", "TEXT"),
                ColumnDefinition::new("department", "TEXT"),
                ColumnDefinition::new("status", "TEXT"),
                ColumnDefinition::new("source_file", "TEXT"),
                ColumnDefinition::new("file_mtime", "INTEGER"),
                ColumnDefinition::new("version", "TEXT"),
                ColumnDefinition::new("effective_date", "TEXT"),
                ColumnDefinition::new("created_at", "INTEGER"),
                ColumnDefinition::new("updated_at", "INTEGER"),
            ],
            ..Default::default()
        };
        engine
            .create_table(&docs_table)
            .context("Failed to create gmp_documents table")?;

        // Create document chunks table
        let chunks_table = TableInfo {
            name: "gmp_document_chunks".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("doc_id", "TEXT"),
                ColumnDefinition::new("chunk_index", "INTEGER"),
                ColumnDefinition::new("content", "TEXT"),
                ColumnDefinition::new("token_count", "INTEGER"),
            ],
            ..Default::default()
        };
        engine
            .create_table(&chunks_table)
            .context("Failed to create gmp_document_chunks table")?;

        let audit_table = TableInfo {
            name: "gmp_audit_chain".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("audit_id", "TEXT"),
                ColumnDefinition::new("work_order_id", "TEXT"),
                ColumnDefinition::new("timestamp", "INTEGER"),
                ColumnDefinition::new("action", "TEXT"),
                ColumnDefinition::new("actor", "TEXT"),
                ColumnDefinition::new("detail", "TEXT"),
                ColumnDefinition::new("signature", "TEXT"),
                ColumnDefinition::new("result", "TEXT"),
            ],
            ..Default::default()
        };
        engine
            .create_table(&audit_table)
            .context("Failed to create gmp_audit_chain table")?;

        self.insert_demo_audit_data()?;

        Ok(())
    }

    /// Insert a document record
    pub fn insert_document(&self, record: Record) -> Result<()> {
        let mut engine = self.engine.lock().unwrap();
        engine.insert("gmp_documents", vec![record])?;
        Ok(())
    }

    /// Insert a chunk record and index it in FTS
    pub fn insert_chunk(&self, chunk_id: i64, doc_id: &str, content: &str) -> Result<()> {
        let record = vec![
            Value::Integer(chunk_id),
            Value::Text(doc_id.to_string()),
            Value::Integer(0),
            Value::Text(content.to_string()),
            Value::Integer(content.len() as i64),
        ];

        let mut engine = self.engine.lock().unwrap();
        engine.insert("gmp_document_chunks", vec![record])?;

        // Also index in FTS
        let mut fts = self.fts_index.lock().unwrap();
        fts.add_document(chunk_id as u64, content);

        Ok(())
    }

    /// Scan all documents
    pub fn scan_documents(&self) -> Result<Vec<Record>> {
        let engine = self.engine.lock().unwrap();
        Ok(engine.scan("gmp_documents")?)
    }

    /// Scan all chunks
    pub fn scan_chunks(&self) -> Result<Vec<Record>> {
        let engine = self.engine.lock().unwrap();
        Ok(engine.scan("gmp_document_chunks")?)
    }

    /// Search chunks by FTS query
    pub fn search_chunks(&self, query: &str, top_k: usize) -> Result<Vec<u64>> {
        let fts = self.fts_index.lock().unwrap();

        // Tokenize the query
        let tokens = self.tokenizer.tokenize(query);
        if tokens.is_empty() {
            // Fall back to direct search on raw query
            let results = fts.search(query);
            return Ok(results.into_iter().take(top_k).collect());
        }

        // Search for each token and merge results
        let mut all_results = Vec::new();
        for token in &tokens {
            let results = fts.search(token);
            for doc_id in results {
                if !all_results.contains(&doc_id) {
                    all_results.push(doc_id);
                }
            }
        }

        all_results.truncate(top_k);
        Ok(all_results)
    }

    /// Get a document by doc_id
    pub fn get_document_by_id(&self, doc_id: &str) -> Result<Option<Record>> {
        let docs = self.scan_documents()?;
        for doc in docs {
            if doc.len() > 1 {
                if let Value::Text(ref id) = doc[1] {
                    if id == doc_id {
                        return Ok(Some(doc));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Get all chunks for a document
    pub fn get_chunks_by_doc_id(&self, doc_id: &str) -> Result<Vec<Record>> {
        let chunks = self.scan_chunks()?;
        Ok(chunks
            .into_iter()
            .filter(|chunk| {
                if chunk.len() > 1 {
                    if let Value::Text(ref id) = chunk[1] {
                        return id == doc_id;
                    }
                }
                false
            })
            .collect())
    }

    /// Check if the engine has the GMP schema initialized
    pub fn has_gmp_schema(&self) -> bool {
        let engine = self.engine.lock().unwrap();
        engine.has_table("gmp_documents") && engine.has_table("gmp_document_chunks")
    }

    /// Get the underlying FileStorage data directory path
    pub fn storage_path(&self) -> PathBuf {
        PathBuf::from(&self.config.data_dir).join("sqlrustgo_storage")
    }

    /// Flush any pending writes to disk (FileStorage persists automatically)
    pub fn flush(&self) -> Result<()> {
        Ok(())
    }

    pub fn insert_audit_record(
        &self,
        audit_id: &str,
        work_order_id: &str,
        timestamp: i64,
        action: &str,
        actor: &str,
        detail: &str,
        signature: Option<&str>,
        result: &str,
    ) -> Result<()> {
        let record = vec![
            Value::Integer(0),
            Value::Text(audit_id.to_string()),
            Value::Text(work_order_id.to_string()),
            Value::Integer(timestamp),
            Value::Text(action.to_string()),
            Value::Text(actor.to_string()),
            Value::Text(detail.to_string()),
            signature.map(|s| Value::Text(s.to_string())).unwrap_or(Value::Null),
            Value::Text(result.to_string()),
        ];
        let mut engine = self.engine.lock().unwrap();
        engine.insert("gmp_audit_chain", vec![record])?;
        Ok(())
    }

    pub fn scan_audit_chain(&self) -> Result<Vec<Record>> {
        let engine = self.engine.lock().unwrap();
        Ok(engine.scan("gmp_audit_chain")?)
    }

    pub fn insert_demo_audit_data(&self) -> Result<()> {
        let records = self.scan_audit_chain()?;
        if !records.is_empty() {
            return Ok(());
        }

        let demos = vec![
            ("AUDIT-001", "WE-20260522-001", 1747888338000i64, "工单创建", "operator_zhang", r#"{"task":"称量任务 #WE-001","operator":"张三"}"#, Some("ZhangSan#sha256:abc123"), "通过"),
            ("AUDIT-002", "WE-20260522-001", 1747888385000i64, "OCR识别", "system", r#"{"device":"BAL-001","result":"识别成功"}"#, None, "通过"),
            ("AUDIT-003", "WE-20260522-001", 1747888392000i64, "规则校验", "system", r#"{"dsl":"weigh_material","result":"校验通过"}"#, None, "通过"),
            ("AUDIT-004", "WE-20260522-001", 1747888425000i64, "主管审批", "supervisor_wang", r#"{"level":1,"result":"审批通过"}"#, Some("WangWu#sha256:def456"), "通过"),
            ("AUDIT-005", "WE-20260522-001", 1747888442000i64, "质量部审批", "quality_li", r#"{"level":2,"result":"第二级审批通过"}"#, Some("LiLiu#sha256:ghi789"), "通过"),
        ];

        for (audit_id, wo, ts, action, actor, detail, sig, result) in demos {
            self.insert_audit_record(audit_id, wo, ts, action, actor, detail, sig, result)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TestEnv {
        _dir: TempDir,
        config: StorageConfig,
    }

    fn create_test_env() -> TestEnv {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let config = StorageConfig {
            data_dir: dir.path().to_string_lossy().to_string(),
            db_path: dir.path().join("test.db").to_string_lossy().to_string(),
            embedding_dim: 1024,
            use_sqlrustgo_storage: false,
        };
        TestEnv { _dir: dir, config }
    }

    #[test]
    fn test_adapter_creation() {
        let env = create_test_env();
        let adapter = SqlrustgoAdapter::new(env.config.clone()).unwrap();
        assert!(adapter.storage_path().exists());
    }

    #[test]
    fn test_init_gmp_schema() {
        let env = create_test_env();
        let adapter = SqlrustgoAdapter::new(env.config).unwrap();
        adapter.init_gmp_schema().unwrap();
        assert!(adapter.has_gmp_schema());
    }

    #[test]
    fn test_insert_and_scan_document() {
        let env = create_test_env();
        let adapter = SqlrustgoAdapter::new(env.config).unwrap();
        adapter.init_gmp_schema().unwrap();

        let doc_record = vec![
            Value::Integer(1),
            Value::Text("DOC-001".to_string()),
            Value::Text("GMP-001".to_string()),
            Value::Text("Test Document".to_string()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Text("active".to_string()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Integer(1000),
            Value::Integer(1000),
        ];

        adapter.insert_document(doc_record).unwrap();
        let docs = adapter.scan_documents().unwrap();
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_insert_and_search_chunk() {
        let env = create_test_env();
        let adapter = SqlrustgoAdapter::new(env.config).unwrap();
        adapter.init_gmp_schema().unwrap();

        adapter
            .insert_chunk(1, "DOC-001", "设备验证管理规程 验证周期")
            .unwrap();

        // Search via FTS
        let results = adapter.search_chunks("设备验证", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0], 1);
    }

    #[test]
    fn test_get_document_by_id() {
        let env = create_test_env();
        let adapter = SqlrustgoAdapter::new(env.config).unwrap();
        adapter.init_gmp_schema().unwrap();

        let doc_record = vec![
            Value::Integer(1),
            Value::Text("DOC-001".to_string()),
            Value::Text("GMP-001".to_string()),
            Value::Text("Test Document".to_string()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Text("active".to_string()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Integer(1000),
            Value::Integer(1000),
        ];

        adapter.insert_document(doc_record).unwrap();
        let doc = adapter.get_document_by_id("DOC-001").unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap()[3], Value::Text("Test Document".to_string()));
    }

    #[test]
    fn test_persistence() {
        // Verify data survives adapter recreation
        let dir = TempDir::new().expect("Failed to create temp dir");
        let data_dir = dir.path().to_string_lossy().to_string();
        let config = StorageConfig {
            data_dir: data_dir.clone(),
            db_path: dir.path().join("test.db").to_string_lossy().to_string(),
            embedding_dim: 1024,
            use_sqlrustgo_storage: false,
        };

        // First instance
        {
            let adapter = SqlrustgoAdapter::new(config.clone()).unwrap();
            adapter.init_gmp_schema().unwrap();
            let doc_record = vec![
                Value::Integer(1),
                Value::Text("PERSIST-001".to_string()),
                Value::Text("PERSIST".to_string()),
                Value::Text("Persistent Document".to_string()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Text("active".to_string()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Integer(1000),
                Value::Integer(1000),
            ];
            adapter.insert_document(doc_record).unwrap();
        }

        // Second instance - should load from disk
        {
            let adapter = SqlrustgoAdapter::new(config).unwrap();
            assert!(adapter.has_gmp_schema());
            let doc = adapter.get_document_by_id("PERSIST-001").unwrap();
            assert!(
                doc.is_some(),
                "Data should persist across adapter instances"
            );
        }
    }
}
