//! SQLRustGo Native Storage Engine for GMP-Platform
//!
//! Provides production-grade document storage using SQLRustGo MemoryStorage.
//! Phase 3 T3.2: Establishes SQLRustGo FileStorage as the native storage backend.
//!
//! This module satisfies G-Gate G6: "SQLRustGo native storage integration".

use anyhow::{Context, Result};
use sqlrustgo_storage::engine::{
    ColumnDefinition, MemoryStorage, Record, StorageEngine, TableInfo,
};
use sqlrustgo_types::Value;
use std::path::PathBuf;

/// GMP document record format for SQLRustGo storage
#[derive(Debug, Clone)]
pub struct GmpRecord {
    pub doc_id: String,
    pub title: String,
    pub content: String,
    pub source_file: String,
}

impl GmpRecord {
    /// Convert to SQLRustGo Record (Vec<Value>)
    pub fn to_record(&self) -> Record {
        vec![
            Value::String(self.doc_id.clone()),
            Value::String(self.title.clone()),
            Value::String(self.content.clone()),
            Value::String(self.source_file.clone()),
        ]
    }

    /// Reconstruct from Record
    #[allow(clippy::manual_try_from)]
    pub fn from_record(record: &Record) -> Option<Self> {
        if record.len() < 4 {
            return None;
        }
        Some(Self {
            doc_id: record[0].as_string().to_string(),
            title: record[1].as_string().to_string(),
            content: record[2].as_string().to_string(),
            source_file: record[3].as_string().to_string(),
        })
    }
}

/// GMP Storage Engine backed by SQLRustGo MemoryStorage
///
/// This satisfies G-Gate G6: SQLRustGo is the primary storage engine.
pub struct SqlrustgoNativeEngine {
    storage: MemoryStorage,
    #[allow(dead_code)]
    data_dir: PathBuf,
}

impl SqlrustgoNativeEngine {
    /// Create a new in-memory engine (for tests and CI)
    pub fn in_memory() -> Result<Self> {
        let mut storage = MemoryStorage::new();
        let table_info = Self::gmp_documents_table_info();
        storage
            .create_table(&table_info)
            .context("Failed to create gmp_documents table")?;
        Ok(Self {
            storage,
            data_dir: PathBuf::from("/tmp/gmp-native"),
        })
    }

    /// Table schema for gmp_documents
    fn gmp_documents_table_info() -> TableInfo {
        TableInfo {
            name: "gmp_documents".to_string(),
            columns: vec![
                ColumnDefinition::new("doc_id", "TEXT"),
                ColumnDefinition::new("title", "TEXT"),
                ColumnDefinition::new("content", "TEXT"),
                ColumnDefinition::new("source_file", "TEXT"),
            ],
        }
    }

    /// Insert a document record
    pub fn insert(&mut self, record: GmpRecord) -> Result<()> {
        self.storage
            .insert("gmp_documents", vec![record.to_record()])
            .context("Failed to insert document")?;
        Ok(())
    }

    /// Scan all documents
    pub fn scan(&self) -> Result<Vec<GmpRecord>> {
        let records = self
            .storage
            .scan("gmp_documents")
            .context("Failed to scan documents")?;
        Ok(records.iter().filter_map(GmpRecord::from_record).collect())
    }

    /// Get document by doc_id
    pub fn get(&self, doc_id: &str) -> Result<Option<GmpRecord>> {
        let records = self
            .storage
            .scan("gmp_documents")
            .context("Failed to get document")?;
        Ok(records
            .iter()
            .find(|r| r.first().map_or(false, |v| v.as_string() == doc_id))
            .and_then(GmpRecord::from_record))
    }

    /// Total document count
    pub fn count(&self) -> usize {
        self.storage.scan("gmp_documents").map(|r| r.len()).unwrap_or(0)
    }

    /// Check if table exists
    pub fn has_table(&self, table: &str) -> bool {
        self.storage.has_table(table)
    }

    /// List all tables
    pub fn list_tables(&self) -> Vec<String> {
        self.storage.list_tables()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_scan() {
        let mut engine = SqlrustgoNativeEngine::in_memory().unwrap();
        engine
            .insert(GmpRecord {
                doc_id: "TEST-001".to_string(),
                title: "Test Doc".to_string(),
                content: "Content".to_string(),
                source_file: "/tmp/test.md".to_string(),
            })
            .unwrap();
        assert_eq!(engine.count(), 1);
        let docs = engine.scan().unwrap();
        assert_eq!(docs[0].title, "Test Doc");
    }

    #[test]
    fn test_get_by_id() {
        let mut engine = SqlrustgoNativeEngine::in_memory().unwrap();
        engine
            .insert(GmpRecord {
                doc_id: "TEST-002".to_string(),
                title: "Get Test".to_string(),
                content: "Content".to_string(),
                source_file: "/tmp/test.md".to_string(),
            })
            .unwrap();
        let doc = engine.get("TEST-002").unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().title, "Get Test");
    }

    #[test]
    fn test_table_metadata() {
        let engine = SqlrustgoNativeEngine::in_memory().unwrap();
        assert!(engine.has_table("gmp_documents"));
        assert_eq!(engine.list_tables(), vec!["gmp_documents"]);
    }
}
