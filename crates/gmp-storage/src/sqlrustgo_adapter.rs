//! SQLRustGo Storage Engine Adapter
//!
//! This module provides an adapter that wraps the SQLRustGo StorageEngine trait
//! to work with the GMP Storage types. It bridges the gap between the custom
//! GMP storage implementation and the standardized SQLRustGo storage interface.

use crate::storage::StorageConfig;
use anyhow::{Context, Result};
use sqlrustgo_storage::engine::{
    ColumnDefinition, MemoryStorage, Record, StorageEngine as SqlrustgoStorageEngine, TableInfo,
};

/// Adapter that wraps SQLRustGo StorageEngine to provide GMP-compatible storage
///
/// This adapter translates between GMP's document-centric storage model and
/// SQLRustGo's relational table-based storage model.
pub struct SqlrustgoAdapter {
    /// Internal SQLRustGo storage engine
    engine: MemoryStorage,
    /// Configuration for reference
    config: StorageConfig,
}

impl SqlrustgoAdapter {
    /// Create a new adapter with a MemoryStorage backend
    pub fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self {
            engine: MemoryStorage::new(),
            config,
        })
    }

    /// Get a reference to the underlying SQLRustGo storage engine
    pub fn engine(&self) -> &MemoryStorage {
        &self.engine
    }

    /// Get a mutable reference to the underlying SQLRustGo storage engine
    pub fn engine_mut(&mut self) -> &mut MemoryStorage {
        &mut self.engine
    }

    /// Get the configuration
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Initialize the GMP schema in the SQLRustGo storage engine
    pub fn init_gmp_schema(&mut self) -> Result<()> {
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
        self.engine
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
        self.engine
            .create_table(&chunks_table)
            .context("Failed to create gmp_document_chunks table")?;

        // Create embeddings table
        let embeddings_table = TableInfo {
            name: "gmp_embeddings".to_string(),
            columns: vec![
                ColumnDefinition::new("chunk_id", "INTEGER"),
                ColumnDefinition::new("embedding", "BLOB"),
            ],
            ..Default::default()
        };
        self.engine
            .create_table(&embeddings_table)
            .context("Failed to create gmp_embeddings table")?;

        Ok(())
    }

    /// Insert a document record
    pub fn insert_document(&mut self, record: Record) -> Result<()> {
        self.engine.insert("gmp_documents", vec![record])?;
        Ok(())
    }

    /// Insert a chunk record
    pub fn insert_chunk(&mut self, record: Record) -> Result<()> {
        self.engine.insert("gmp_document_chunks", vec![record])?;
        Ok(())
    }

    /// Scan all documents
    pub fn scan_documents(&self) -> Result<Vec<Record>> {
        Ok(self.engine.scan("gmp_documents")?)
    }

    /// Scan all chunks
    pub fn scan_chunks(&self) -> Result<Vec<Record>> {
        Ok(self.engine.scan("gmp_document_chunks")?)
    }

    /// Check if the engine has the GMP schema initialized
    pub fn has_gmp_schema(&self) -> bool {
        self.engine.has_table("gmp_documents")
            && self.engine.has_table("gmp_document_chunks")
            && self.engine.has_table("gmp_embeddings")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    #[test]
    fn test_adapter_creation() {
        let config = StorageConfig::default();
        let adapter = SqlrustgoAdapter::new(config.clone()).unwrap();
        assert_eq!(adapter.config().data_dir, config.data_dir);
    }

    #[test]
    fn test_init_gmp_schema() {
        let config = StorageConfig::default();
        let mut adapter = SqlrustgoAdapter::new(config).unwrap();
        adapter.init_gmp_schema().unwrap();
        assert!(adapter.has_gmp_schema());
    }

    #[test]
    fn test_insert_and_scan_document() {
        let config = StorageConfig::default();
        let mut adapter = SqlrustgoAdapter::new(config).unwrap();
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
    fn test_insert_and_scan_chunk() {
        let config = StorageConfig::default();
        let mut adapter = SqlrustgoAdapter::new(config).unwrap();
        adapter.init_gmp_schema().unwrap();

        let chunk_record = vec![
            Value::Integer(1),
            Value::Text("DOC-001".to_string()),
            Value::Integer(0),
            Value::Text("Test content".to_string()),
            Value::Integer(10),
        ];

        adapter.insert_chunk(chunk_record).unwrap();
        let chunks = adapter.scan_chunks().unwrap();
        assert_eq!(chunks.len(), 1);
    }
}
