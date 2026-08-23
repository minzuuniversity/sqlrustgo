//! File-based table storage
//! Persists table data to JSON files

use crate::bplus_tree::BPlusTree;
use crate::engine::{
    ColumnDefinition, ForeignKeyConstraint, Record, RowFilter, RowMutation, SharedSliceIter,
    StorageEngine, TableData, TableInfo, TriggerInfo, UniqueConstraint,
};
use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

/// File-based storage manager
pub struct FileStorage {
    /// Base directory for database files
    data_dir: PathBuf,
    /// In-memory cache of tables
    tables: HashMap<String, TableData>,
    /// B+ Tree indexes protected by RwLock for concurrent access
    indexes: RwLock<HashMap<(String, String), BPlusTree>>,
    /// Insert buffer for batching writes
    insert_buffer: HashMap<String, Vec<Record>>,
    /// Threshold to trigger buffer flush
    buffer_threshold: usize,
    /// Enable insert buffering
    enable_buffer: bool,
    /// PR-842: the active transaction id (0 == autocommit). Mirrored from
    /// the ExecutionEngine via `set_current_tx_id` so `in_transaction()`
    /// can answer correctly even on the bare FileStorage path.
    current_tx_id: u64,
    /// Trigger definitions keyed by trigger name, protected by RwLock for concurrent access
    triggers: RwLock<HashMap<String, TriggerInfo>>,
    /// Gap lock manager for REPEATABLE-READ isolation (F-16 Gap Locking)
    #[allow(dead_code)]
    gap_lock_manager: Option<std::sync::Arc<crate::lock::GapLockManager>>,
    /// V311-07: Dirty table tracker - marks tables modified since last flush
    dirty_tables: HashSet<String>,
}

impl FileStorage {
    /// Create a new FileStorage with the given data directory
    pub fn new(data_dir: PathBuf) -> std::io::Result<Self> {
        // Create directory if it doesn't exist
        fs::create_dir_all(&data_dir)?;

        let mut storage = Self {
            data_dir,
            tables: HashMap::new(),
            indexes: RwLock::new(HashMap::new()),
            insert_buffer: HashMap::new(),
            // v3.12.0 #4020 follow-up: raise default buffer flush threshold from
            // 100 to 10_000. Each flush goes through insert_direct, which clones
            // the full TableData and serializes it via serde_json::to_string_pretty
            // — an O(rows_loaded) operation. With threshold=100 the bulk-load
            // cost was O(N^2), which made TPC-H SF=10 supplier only achieve
            // ~111 rows/s. Micro-bench (bulk_load_quadraticity) shows
            // threshold=10000 is ~50.6x faster on identical final state.
            // Callers that need the old behaviour should use
            // new_with_buffer_config(dir, 100, true).
            buffer_threshold: 10_000,
            enable_buffer: true,
            current_tx_id: 0,
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: None,
            dirty_tables: HashSet::new(),
        };

        // Load existing tables
        storage.load_all_tables()?;

        // Load existing indexes
        storage.load_all_indexes()?;

        Ok(storage)
    }

    pub fn new_with_buffer_config(
        data_dir: PathBuf,
        buffer_threshold: usize,
        enable_buffer: bool,
    ) -> std::io::Result<Self> {
        fs::create_dir_all(&data_dir)?;

        let mut storage = Self {
            data_dir,
            tables: HashMap::new(),
            indexes: RwLock::new(HashMap::new()),
            insert_buffer: HashMap::new(),
            buffer_threshold,
            enable_buffer,
            current_tx_id: 0,
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: None,
            dirty_tables: HashSet::new(),
        };

        storage.load_all_tables()?;
        storage.load_all_indexes()?;

        Ok(storage)
    }

    /// Create a new FileStorage with WAL (Write-Ahead Log) enabled for crash recovery.
    /// The WAL file will be stored in the data directory as "sqlrustgo.wal".
    /// Returns Err if WAL cannot be initialized.
    pub fn new_with_wal(data_dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&data_dir)?;

        let _wal_path = data_dir.join("sqlrustgo.wal");

        let mut storage = Self {
            data_dir,
            tables: HashMap::new(),
            indexes: RwLock::new(HashMap::new()),
            insert_buffer: HashMap::new(),
            // v3.12.0 #4020 follow-up: see FileStorage::new — default raised to
            // 10_000 to amortise O(N) insert_direct over a much larger batch.
            buffer_threshold: 10_000,
            enable_buffer: true, // Transaction boundary handled by buffer flush on commit
            current_tx_id: 0,
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: None,
            dirty_tables: HashSet::new(),
        };

        // Load existing tables
        storage.load_all_tables()?;

        // Load existing indexes
        storage.load_all_indexes()?;

        // Load existing triggers
        storage.load_all_triggers()?;

        Ok(storage)
    }

    /// Create a new FileStorage with a shared GapLockManager for REPEATABLE-READ isolation.
    ///
    /// This enables gap locking on index operations for transactions with
    /// REPEATABLE-READ isolation level.
    ///
    /// # Arguments
    /// * `data_dir` - Directory for database files
    /// * `lock_manager` - Shared GapLockManager instance (typically Arc::new(GapLockManager::new()))
    ///
    /// # Returns
    /// * `Ok(Self)` - FileStorage with gap locking enabled
    pub fn new_with_lock_manager(
        data_dir: PathBuf,
        lock_manager: std::sync::Arc<crate::lock::GapLockManager>,
    ) -> std::io::Result<Self> {
        fs::create_dir_all(&data_dir)?;

        let mut storage = Self {
            data_dir,
            tables: HashMap::new(),
            indexes: RwLock::new(HashMap::new()),
            insert_buffer: HashMap::new(),
            // v3.12.0 #4020 follow-up: see FileStorage::new — default raised to
            // 10_000 to amortise O(N) insert_direct over a much larger batch.
            buffer_threshold: 10_000,
            enable_buffer: true,
            current_tx_id: 0,
            triggers: RwLock::new(HashMap::new()),
            gap_lock_manager: Some(lock_manager),
            dirty_tables: HashSet::new(),
        };

        storage.load_all_tables()?;
        storage.load_all_indexes()?;

        Ok(storage)
    }

    /// Get the path for a table file
    fn table_path(&self, table_name: &str) -> PathBuf {
        self.data_dir.join(format!("{}.json", table_name))
    }

    /// Get the path for an index file
    fn index_path(&self, table_name: &str, column_name: &str) -> PathBuf {
        self.data_dir
            .join(format!("{}_idx_{}.json", table_name, column_name))
    }

    /// Get the path for a trigger file (named after the trigger, not the table)
    fn trigger_path(&self, trigger_name: &str) -> PathBuf {
        self.data_dir.join(format!("trigger_{}.json", trigger_name))
    }

    /// Load a single trigger from disk
    fn load_trigger(&self, trigger_name: &str) -> std::io::Result<TriggerInfo> {
        let path = self.trigger_path(trigger_name);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let info: TriggerInfo = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(info)
    }

    /// Save a trigger to disk
    fn save_trigger(&self, info: &TriggerInfo) -> std::io::Result<()> {
        let path = self.trigger_path(&info.name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string_pretty(info)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(json.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// Remove a trigger file from disk (best-effort: missing file is OK)
    fn remove_trigger_file(&self, trigger_name: &str) -> std::io::Result<()> {
        let path = self.trigger_path(trigger_name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Load all triggers from the data directory
    fn load_all_triggers(&mut self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with("trigger_") && file_name.ends_with(".json") {
                    if let Some(name) = file_name
                        .strip_prefix("trigger_")
                        .and_then(|s| s.strip_suffix(".json"))
                    {
                        if let Ok(info) = self.load_trigger(name) {
                            if let Ok(mut triggers) = self.triggers.write() {
                                triggers.insert(name.to_string(), info);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load all tables from the data directory
    fn load_all_tables(&mut self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(table_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(table_data) = self.load_table(table_name) {
                        self.tables.insert(table_name.to_string(), table_data);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load all indexes from the data directory
    fn load_all_indexes(&mut self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Look for index files: table_idx_column.json
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.ends_with(".json") && file_name.contains("_idx_") {
                    // Parse table_idx_column.json
                    if let Some((table_name, column_name)) = file_name
                        .strip_suffix(".json")
                        .and_then(|s| s.split_once("_idx_"))
                    {
                        if let Ok(index) = self.load_index(table_name, column_name) {
                            if let Ok(mut indexes) = self.indexes.write() {
                                indexes.insert(
                                    (table_name.to_string(), column_name.to_string()),
                                    index,
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single index from disk
    fn load_index(&self, table_name: &str, column_name: &str) -> std::io::Result<BPlusTree> {
        let path = self.index_path(table_name, column_name);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let index: BPlusTree = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(index)
    }

    /// Save an index to disk
    fn save_index(
        &self,
        table_name: &str,
        column_name: &str,
        index: &BPlusTree,
    ) -> std::io::Result<()> {
        let path = self.index_path(table_name, column_name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        let json = serde_json::to_string_pretty(index)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(())
    }

    /// Load a single table from disk
    fn load_table(&self, table_name: &str) -> std::io::Result<TableData> {
        let path = self.table_path(table_name);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let stored: StoredTableData = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(TableData {
            info: TableInfo {
                name: stored.name,
                columns: stored.columns,
                foreign_keys: stored.foreign_keys,
                unique_constraints: stored.unique_constraints,
                check_constraints: vec![],
                compression: None,
                collations: HashMap::new(),
                partition_info: None,
            },
            rows: stored.rows,
        })
    }

    /// Save a table to disk
    fn save_table(&self, table_name: &str, table_data: &TableData) -> std::io::Result<()> {
        let path = self.table_path(table_name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        let stored = StoredTableData {
            name: table_data.info.name.clone(),
            columns: table_data.info.columns.clone(),
            foreign_keys: table_data.info.foreign_keys.clone(),
            unique_constraints: table_data.info.unique_constraints.clone(),
            rows: table_data.rows.clone(),
        };

        let json = serde_json::to_string_pretty(&stored)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        Ok(())
    }

    /// Get a table by name
    pub fn get_table(&self, name: &str) -> Option<&TableData> {
        self.tables.get(name)
    }

    /// Get a mutable table by name
    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut TableData> {
        self.tables.get_mut(name)
    }

    /// Insert a new table
    pub fn insert_table(&mut self, name: String, table_data: TableData) -> std::io::Result<()> {
        self.tables.insert(name.clone(), table_data.clone());
        self.save_table(&name, &table_data)
    }

    /// Drop (delete) a table
    pub fn drop_table(&mut self, name: &str) -> std::io::Result<()> {
        self.tables.remove(name);

        let path = self.table_path(name);
        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    /// Force save all dirty tables to disk
    /// V311-07: Only persist tables that have been modified since last flush
    pub fn flush(&mut self) -> std::io::Result<()> {
        // V311-07: Take dirty tables set, leaving empty set behind
        let dirty: Vec<String> = std::mem::take(&mut self.dirty_tables).into_iter().collect();
        for name in &dirty {
            if let Some(table_data) = self.tables.get(name) {
                self.save_table(name, table_data)?;
            }
        }
        Ok(())
    }

    /// Check if a table exists
    pub fn contains_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// Create a new database directory under data_dir.
    pub fn create_database(&mut self, db_name: &str) -> std::io::Result<()> {
        let db_path = self.data_dir.join(db_name);
        fs::create_dir_all(&db_path)?;
        Ok(())
    }

    /// Drop a database directory. Refuses to drop if the directory is not empty.
    pub fn drop_database(&mut self, db_name: &str) -> std::io::Result<()> {
        let db_path = self.data_dir.join(db_name);
        if db_path.exists() {
            for entry in fs::read_dir(&db_path)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                // Skip WAL files; reject everything else
                if !name.ends_with(".wal") {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::DirectoryNotEmpty,
                        format!("database '{}' is not empty", db_name),
                    ));
                }
            }
            fs::remove_dir(&db_path)?;
        }
        Ok(())
    }

    /// Save a table to disk (call after modifications)
    pub fn persist_table(&self, name: &str) -> std::io::Result<()> {
        if let Some(table_data) = self.tables.get(name) {
            self.save_table(name, table_data)
        } else {
            Ok(())
        }
    }

    // ==================== Index Methods ====================

    /// Check if an index exists for a table column
    pub fn has_index(&self, table_name: &str, column_name: &str) -> bool {
        self.indexes
            .read()
            .map(|indexes| indexes.contains_key(&(table_name.to_string(), column_name.to_string())))
            .unwrap_or(false)
    }

    /// Get an index for a table column (read-only)
    pub fn get_index(&self, table_name: &str, column_name: &str) -> Option<BPlusTree> {
        self.indexes.read().ok().and_then(|indexes| {
            indexes
                .get(&(table_name.to_string(), column_name.to_string()))
                .cloned()
        })
    }

    /// Create or update an index for a table column from existing data
    pub fn create_index(
        &mut self,
        table_name: &str,
        column_name: &str,
        column_index: usize,
    ) -> std::io::Result<()> {
        let table = self
            .tables
            .get(table_name)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Table not found"))?;

        // Build B+ Tree from existing rows
        let mut index = crate::bplus_tree::BPlusTree::new();
        for (row_id, row) in table.rows.iter().enumerate() {
            if let Some(value) = row.get(column_index) {
                if let Some(key) = value.to_index_key() {
                    index.insert(key, row_id as u32);
                }
            }
        }

        // Save to disk
        self.save_index(table_name, column_name, &index)?;

        // Store in memory
        if let Ok(mut indexes) = self.indexes.write() {
            indexes.insert((table_name.to_string(), column_name.to_string()), index);
        }

        Ok(())
    }

    /// Insert a row and update index
    pub fn insert_with_index(
        &mut self,
        table_name: &str,
        column_name: &str,
        key: i64,
        row_id: u32,
    ) -> std::io::Result<()> {
        let key_exists = (table_name.to_string(), column_name.to_string());

        // Clone the key for later use
        let has_index = self
            .indexes
            .read()
            .map(|indexes| indexes.contains_key(&key_exists))
            .unwrap_or(false);

        if has_index {
            // First get a clone of the index to save, then modify
            let index_clone = {
                let indexes = self.indexes.read().unwrap();
                indexes.get(&key_exists).cloned()
            };

            // Update the in-memory index
            if let Ok(mut indexes) = self.indexes.write() {
                if let Some(index) = indexes.get_mut(&key_exists) {
                    index.insert(key, row_id);
                }
            }

            // Save to disk (outside the write lock)
            if let Some(idx) = index_clone {
                let _ = self.save_index(table_name, column_name, &idx);
            }
        }

        Ok(())
    }

    /// Search using index - returns row IDs matching the key
    pub fn search_index(&self, table_name: &str, column_name: &str, key: i64) -> Option<u32> {
        self.indexes.read().ok().and_then(|indexes| {
            indexes
                .get(&(table_name.to_string(), column_name.to_string()))
                .and_then(|index| index.search(key))
        })
    }

    /// Range query using index
    pub fn range_index(
        &self,
        table_name: &str,
        column_name: &str,
        start: i64,
        end: i64,
    ) -> Vec<u32> {
        self.indexes
            .read()
            .ok()
            .and_then(|indexes| {
                indexes
                    .get(&(table_name.to_string(), column_name.to_string()))
                    .map(|index| index.range_query(start, end))
            })
            .unwrap_or_default()
    }

    /// Drop an index
    pub fn drop_index(&mut self, table_name: &str, column_name: &str) -> std::io::Result<()> {
        let key = (table_name.to_string(), column_name.to_string());

        if let Ok(mut indexes) = self.indexes.write() {
            indexes.remove(&key);
        }

        let path = self.index_path(table_name, column_name);
        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    /// Flush all indexes to disk
    pub fn flush_indexes(&self) -> std::io::Result<()> {
        if let Ok(indexes) = self.indexes.read() {
            for ((table_name, column_name), index) in indexes.iter() {
                self.save_index(table_name, column_name, index)?;
            }
        }
        Ok(())
    }
}

/// Stored table data (for serialization)
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredTableData {
    name: String,
    columns: Vec<ColumnDefinition>,
    foreign_keys: Vec<ForeignKeyConstraint>,
    unique_constraints: Vec<UniqueConstraint>,
    rows: Vec<Vec<Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ColumnDefinition;
    use std::fs::remove_dir_all;

    #[test]
    fn test_file_storage() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_file_storage");
        let _ = remove_dir_all(&temp_dir);

        {
            let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

            // Insert a table
            let table_data = TableData {
                info: TableInfo {
                    name: "users".to_string(),
                    columns: vec![
                        ColumnDefinition {
                            name: "id".to_string(),
                            data_type: "INTEGER".to_string(),
                            nullable: false,
                            primary_key: true,
                            char_max_length: None,
                            collation: None,
                            default_value: None,
                            auto_increment: false,
                        },
                        ColumnDefinition {
                            name: "name".to_string(),
                            data_type: "TEXT".to_string(),
                            nullable: true,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,
                            default_value: None,
                            auto_increment: false,
                        },
                    ],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                },
                rows: vec![vec![Value::Integer(1), Value::Text("Alice".to_string())]],
            };

            storage
                .insert_table("users".to_string(), table_data)
                .unwrap();
        }

        // Load from disk
        {
            let storage = FileStorage::new(temp_dir.clone()).unwrap();
            let table = storage.get_table("users").unwrap();
            assert_eq!(table.info.name, "users");
            assert_eq!(table.rows.len(), 1);
        }

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_contains_and_drop() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_contains");
        let _ = remove_dir_all(&temp_dir);

        let table_data = TableData {
            info: TableInfo {
                name: "test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();
        storage
            .insert_table("test".to_string(), table_data)
            .unwrap();

        // Test contains_table
        assert!(storage.contains_table("test"));
        assert!(!storage.contains_table("nonexistent"));

        // Test table_names
        let names = storage.table_names();
        assert!(names.contains(&"test".to_string()));

        // Test drop_table
        storage.drop_table("test").unwrap();
        assert!(!storage.contains_table("test"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_persist() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_persist");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Create table without saving
        let table_data = TableData {
            info: TableInfo {
                name: "persist_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("persist_test".to_string(), table_data)
            .unwrap();

        // Test persist_table
        storage.persist_table("persist_test").unwrap();

        // Test flush
        storage.flush().unwrap();

        // Verify table still exists after reload
        let storage2 = FileStorage::new(temp_dir.clone()).unwrap();
        assert!(storage2.contains_table("persist_test"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_get_mut() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_get_mut");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "mutable".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("mutable".to_string(), table_data)
            .unwrap();

        // Test get_table_mut
        {
            let table = storage.get_table_mut("mutable").unwrap();
            table.rows.push(vec![Value::Integer(1)]);
        }

        let table = storage.get_table("mutable").unwrap();
        assert_eq!(table.rows.len(), 1);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_index() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_index");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Insert table with data
        let table_data = TableData {
            info: TableInfo {
                name: "idx_test".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        ..Default::default()
                    },
                    ColumnDefinition {
                        name: "value".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        ..Default::default()
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![
                vec![Value::Integer(1), Value::Integer(100)],
                vec![Value::Integer(2), Value::Integer(200)],
            ],
        };
        storage
            .insert_table("idx_test".to_string(), table_data)
            .unwrap();

        // Create index on id column (column_index = 0)
        storage.create_index("idx_test", "id", 0).unwrap();

        // Test has_index
        assert!(storage.has_index("idx_test", "id"));
        assert!(!storage.has_index("idx_test", "nonexistent"));

        // Test search_index
        let row_id = storage.search_index("idx_test", "id", 1);
        assert!(row_id.is_some());

        // Test range_index
        let range_results = storage.range_index("idx_test", "id", 1, 3);
        assert!(!range_results.is_empty());

        // Test insert_with_index
        storage.insert_with_index("idx_test", "id", 3, 2).unwrap();

        // Test drop_index
        storage.drop_index("idx_test", "id").unwrap();
        assert!(!storage.has_index("idx_test", "id"));

        // Test flush_indexes
        storage.flush_indexes().unwrap();

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_index_search() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_idx_search");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Create table and index
        let table_data = TableData {
            info: TableInfo {
                name: "search_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("search_test".to_string(), table_data)
            .unwrap();
        storage.create_index("search_test", "id", 0).unwrap();

        // Insert with index
        storage
            .insert_with_index("search_test", "id", 10, 0)
            .unwrap();
        storage
            .insert_with_index("search_test", "id", 20, 1)
            .unwrap();

        // Search
        let result = storage.search_index("search_test", "id", 10);
        assert_eq!(result, Some(0));

        // Range query
        let range = storage.range_index("search_test", "id", 5, 15);
        assert!(!range.is_empty());

        let _ = remove_dir_all(&temp_dir);
    }

    // ==================== Additional Coverage Tests ====================

    #[test]
    fn test_file_storage_get_index() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_get_index");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "get_idx_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("get_idx_test".to_string(), table_data)
            .unwrap();
        storage.create_index("get_idx_test", "id", 0).unwrap();

        // Test get_index
        let index = storage.get_index("get_idx_test", "id");
        assert!(index.is_some());

        // Test get_index for non-existent
        let index_none = storage.get_index("get_idx_test", "nonexistent");
        assert!(index_none.is_none());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_index_no_matching_rows() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_no_match");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Create table with non-integer columns (will skip indexing)
        let table_data = TableData {
            info: TableInfo {
                name: "text_table".to_string(),
                columns: vec![ColumnDefinition {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![vec![Value::Text("Alice".to_string())]],
        };
        storage
            .insert_table("text_table".to_string(), table_data)
            .unwrap();

        // Create index - this will work but won't have any entries
        storage.create_index("text_table", "name", 0).unwrap();

        // search_index should return None for TEXT column (no Integer keys)
        let result = storage.search_index("text_table", "name", 1);
        assert_eq!(result, None);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_empty_tables() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_empty");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Test empty storage
        assert_eq!(storage.table_names().len(), 0);
        assert!(!storage.contains_table("anything"));
        assert!(storage.get_table("anything").is_none());

        // Test flush on empty storage
        storage.flush().unwrap();
        storage.flush_indexes().unwrap();

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_persist_nonexistent() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_persist_none");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // persist_table on non-existent table should return Ok
        let result = storage.persist_table("nonexistent");
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_range_index_no_results() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_range_empty");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "range_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("range_test".to_string(), table_data)
            .unwrap();
        storage.create_index("range_test", "id", 0).unwrap();

        // Add some data
        storage.insert_with_index("range_test", "id", 5, 0).unwrap();

        // Range with no matching results
        let range = storage.range_index("range_test", "id", 100, 200);
        assert!(range.is_empty());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_insert_with_index_no_index() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_no_idx");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "no_idx_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("no_idx_test".to_string(), table_data)
            .unwrap();

        // Insert with index when no index exists - should be ok (no-op)
        let result = storage.insert_with_index("no_idx_test", "id", 1, 0);
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_has_index_no_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_has_idx_no");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Check index on non-existent table - should return false
        let result = storage.has_index("nonexistent", "id");
        assert!(!result);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_drop_index_no_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_drop_no");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Try to drop index from non-existent table - should return Ok (no-op)
        let result = storage.drop_index("nonexistent", "id");
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_range_index_no_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_range_no");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Range query on non-existent table - should return empty
        let result = storage.range_index("nonexistent", "id", 0, 100);
        assert_eq!(result, Vec::<u32>::new());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_add_column() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_add_col");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "add_col_test".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: true,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("add_col_test".to_string(), table_data)
            .unwrap();

        // Test add_column
        let new_col = ColumnDefinition {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        };
        let result = storage.add_column("add_col_test", new_col);
        assert!(result.is_ok());

        // Verify column was added
        let table = storage.get_table("add_col_test").unwrap();
        assert_eq!(table.info.columns.len(), 2);
        assert_eq!(table.info.columns[1].name, "name");

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_rename_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_rename");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        let table_data = TableData {
            info: TableInfo {
                name: "old_name".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            },
            rows: vec![],
        };
        storage
            .insert_table("old_name".to_string(), table_data)
            .unwrap();

        // Test rename_table
        let result = storage.rename_table("old_name", "new_name");
        assert!(result.is_ok());

        // Verify table was renamed
        assert!(!storage.contains_table("old_name"));
        assert!(storage.contains_table("new_name"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_rename_nonexistent() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_rename_none");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Rename non-existent table should return Ok (no-op)
        let result = storage.rename_table("nonexistent", "new_name");
        assert!(result.is_ok());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_trigger_operations() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_triggers");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Test create_trigger (returns Ok but does nothing)
        let trigger_info = TriggerInfo {
            name: "test_trigger".to_string(),
            table_name: "test_table".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "BEGIN END".to_string(),
        };
        let result = storage.create_trigger(trigger_info);
        assert!(result.is_ok());

        // PR-2760: drop_trigger is now implemented (FileStorage trigger persistence).
        // Drop should succeed and return Ok.
        let result = storage.drop_trigger("test_trigger");
        assert!(result.is_ok(), "drop_trigger should succeed after PR-2760");

        // Test get_trigger returns None (dropped above)
        let result = storage.get_trigger("test_trigger");
        assert!(result.is_none());

        // Test list_triggers returns empty (dropped above)
        let result = storage.list_triggers("test_table");
        assert!(result.is_empty());

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_storage_view_operations() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_views");
        let _ = remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();

        // Test has_view returns false
        assert!(!storage.has_view("test_view"));

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_buffering() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_buffer");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 10, true).unwrap();

        let table_info = TableInfo {
            name: "test_table".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&table_info).unwrap();

        for i in 0..5 {
            let record = vec![Value::Integer(i as i64)];
            storage.insert("test_table", vec![record]).unwrap();
        }

        assert!(storage.insert_buffer.contains_key("test_table"));
        assert_eq!(storage.insert_buffer.get("test_table").unwrap().len(), 5);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_buffer_threshold() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_threshold");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 3, true).unwrap();

        let table_info = TableInfo {
            name: "test_table".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&table_info).unwrap();

        for i in 0..5 {
            let record = vec![Value::Integer(i as i64)];
            storage.insert("test_table", vec![record]).unwrap();
        }

        assert!(storage.insert_buffer.contains_key("test_table"));
        assert_eq!(storage.insert_buffer.get("test_table").unwrap().len(), 2);

        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_flush_all_buffers() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_flush");
        let _ = remove_dir_all(&temp_dir);

        let mut storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 100, true).unwrap();

        let table_info = TableInfo {
            name: "test_table".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&table_info).unwrap();

        for i in 0..5 {
            let record = vec![Value::Integer(i as i64)];
            storage.insert("test_table", vec![record]).unwrap();
        }

        assert!(storage.insert_buffer.contains_key("test_table"));

        storage.flush_all_buffers().unwrap();

        assert!(!storage.insert_buffer.contains_key("test_table"));

        let table = storage.get_table("test_table").unwrap();
        assert_eq!(table.rows.len(), 5);

        let _ = remove_dir_all(&temp_dir);
    }

    fn make_storage(dir: &str) -> FileStorage {
        let temp_dir = std::env::temp_dir().join(dir);
        let _ = remove_dir_all(&temp_dir);
        FileStorage::new_with_buffer_config(temp_dir, 100, false).unwrap()
    }

    #[test]
    fn test_new_with_buffer_config() {
        let temp_dir = std::env::temp_dir().join("file_storage_buf_cfg");
        let _ = remove_dir_all(&temp_dir);
        let storage = FileStorage::new_with_buffer_config(temp_dir.clone(), 50, false).unwrap();
        assert!(storage.buffer_threshold == 50);
        assert!(!storage.enable_buffer);
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_new_with_wal() {
        let temp_dir = std::env::temp_dir().join("file_storage_wal");
        let _ = remove_dir_all(&temp_dir);
        let storage = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        assert!(storage.data_dir.ends_with("file_storage_wal"));
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_table_mut() {
        let mut storage = make_storage("fs_get_table_mut");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        let t = storage.get_table_mut("t");
        assert!(t.is_some());
    }

    #[test]
    fn test_get_table_mut_nonexistent() {
        let mut storage = make_storage("fs_get_table_mut_ne");
        assert!(storage.get_table_mut("nonexistent").is_none());
    }

    #[test]
    fn test_insert_table() {
        let mut storage = make_storage("fs_insert_table");
        let info = TableInfo {
            name: "t1".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        let data = TableData { info, rows: vec![] };
        storage.insert_table("t1".into(), data).unwrap();
        assert!(storage.contains_table("t1"));
    }

    #[test]
    fn test_table_names() {
        let mut storage = make_storage("fs_table_names");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        let names = storage.table_names();
        assert!(names.contains(&"t".to_string()));
    }

    #[test]
    fn test_table_names_empty() {
        let storage = make_storage("fs_table_names_empty");
        assert!(storage.table_names().is_empty());
    }

    #[test]
    fn test_contains_table() {
        let mut storage = make_storage("fs_contains_table");
        let info = TableInfo {
            name: "x".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        assert!(storage.contains_table("x"));
        assert!(!storage.contains_table("y"));
    }

    #[test]
    fn test_drop_database_nonexistent() {
        let mut storage = make_storage("fs_drop_db_ne");
        let result = storage.drop_database("nonexistent_db");
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_database() {
        let mut storage = make_storage("fs_create_db");
        let result = storage.create_database("my_db");
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(storage.data_dir.join("my_db"));
    }

    #[test]
    fn test_clear_all_tables() {
        let mut storage = make_storage("fs_clear_all");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.clear_all_tables();
        let rows = storage.scan("t").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_flush_all_buffers_extra() {
        let mut storage = make_storage("fs_flush_all_ex");
        storage.flush_all_buffers().unwrap();
    }

    #[test]
    fn test_in_transaction() {
        let storage = make_storage("fs_in_tx");
        assert!(!storage.in_transaction());
        assert_eq!(storage.current_tx_id(), 0);
    }

    #[test]
    fn test_set_current_tx_id() {
        let mut storage = make_storage("fs_set_tx");
        storage.set_current_tx_id(42);
        assert_eq!(storage.current_tx_id(), 42);
        assert!(storage.in_transaction());
    }

    #[test]
    fn test_force_insert() {
        let mut storage = make_storage("fs_force_ins");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.force_insert("t", vec![Value::Integer(5)]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert_eq!(rows, vec![vec![Value::Integer(5)]]);
    }

    #[test]
    fn test_delete_if() {
        let mut storage = make_storage("fs_del_if");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]])
            .unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        let removed = storage.delete_if("t", &filter).unwrap();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_delete_if_nonexistent() {
        let mut storage = make_storage("fs_del_if_ne");
        let filter: RowFilter = Box::new(|_: &Record| true);
        let removed = storage.delete_if("nonexistent", &filter).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_update_nonexistent() {
        let mut storage = make_storage("fs_upd_ne");
        let count = storage
            .update(
                "nonexistent",
                &[Value::Integer(1)],
                &[(0, Value::Integer(99))],
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_update_if_nonexistent() {
        let mut storage = make_storage("fs_upd_if_ne");
        let filter: RowFilter = Box::new(|_: &Record| true);
        let mutation = RowMutation::new(vec![(0, Value::Integer(99))], 0);
        let count = storage
            .update_if("nonexistent", &filter, &mutation)
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_update_if_existing() {
        let mut storage = make_storage("fs_upd_if_ex");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![
                ColumnDefinition::new("x", "INTEGER"),
                ColumnDefinition::new("y", "INTEGER"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", vec![vec![Value::Integer(1), Value::Integer(10)]])
            .unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        let mutation = RowMutation::new(vec![(1, Value::Integer(99))], 0);
        let count = storage.update_if("t", &filter, &mutation).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_create_table_storage() {
        let mut storage = make_storage("fs_create_t");
        let info = TableInfo {
            name: "users".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        assert!(storage.has_table("users"));
    }

    #[test]
    fn test_get_table_info_not_found() {
        let storage = make_storage("fs_gti_ne");
        let result = storage.get_table_info("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_table_storage() {
        let mut storage = make_storage("fs_has_t");
        let info = TableInfo {
            name: "x".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        assert!(storage.has_table("x"));
        assert!(!storage.has_table("y"));
    }

    #[test]
    fn test_list_tables_storage() {
        let mut storage = make_storage("fs_list_t");
        let info = TableInfo {
            name: "t1".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        let tables = storage.list_tables();
        assert!(tables.contains(&"t1".to_string()));
    }

    #[test]
    fn test_create_index_storage() {
        let mut storage = make_storage("fs_create_idx");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1)],
                    vec![Value::Integer(2)],
                    vec![Value::Integer(3)],
                ],
            )
            .unwrap();
        storage.create_index("t", "id", 0).unwrap();
        assert!(storage.has_index("t", "id"));
    }

    #[test]
    fn test_create_index_table_not_found() {
        let mut storage = make_storage("fs_create_idx_ne");
        let result = storage.create_index("nonexistent", "c", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_index_storage() {
        let mut storage = make_storage("fs_drop_idx");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.create_index("t", "id", 0).unwrap();
        storage.drop_index("t", "id").unwrap();
        assert!(!storage.has_index("t", "id"));
    }

    #[test]
    fn test_has_index_no_table() {
        let storage = make_storage("fs_has_idx_no_t");
        assert!(!storage.has_index("nonexistent", "c"));
    }

    #[test]
    fn test_add_column_storage() {
        let mut storage = make_storage("fs_add_col");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .add_column("t", ColumnDefinition::new("b", "TEXT"))
            .unwrap();
        let info_after = storage.get_table_info("t").unwrap();
        assert_eq!(info_after.columns.len(), 2);
    }

    #[test]
    fn test_rename_table_storage() {
        let mut storage = make_storage("fs_rename_t");
        let info = TableInfo {
            name: "old".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("old", vec![vec![Value::Integer(1)]])
            .unwrap();
        storage.rename_table("old", "new").unwrap();
        assert!(!storage.has_table("old"));
        assert!(storage.has_table("new"));
    }

    #[test]
    fn test_rename_table_nonexistent() {
        let mut storage = make_storage("fs_rename_t_ne");
        storage.rename_table("nonexistent", "new_name").unwrap();
    }

    #[test]
    fn test_trigger_operations_storage() {
        let mut storage = make_storage("fs_trigger_ops");
        let trigger = TriggerInfo {
            name: "trig1".into(),
            table_name: "t".into(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "BEGIN UPDATE stats SET n = n + 1; END".into(),
        };
        storage.create_trigger(trigger).unwrap();
        let got = storage.get_trigger("trig1");
        assert!(got.is_some());
        let triggers = storage.list_triggers("t");
        assert_eq!(triggers.len(), 1);
        storage.drop_trigger("trig1").unwrap();
        assert!(storage.get_trigger("trig1").is_none());
    }

    #[test]
    fn test_get_trigger_none() {
        let storage = make_storage("fs_get_trig_none");
        assert!(storage.get_trigger("nonexistent").is_none());
    }

    #[test]
    fn test_list_triggers_by_table() {
        let mut storage = make_storage("fs_list_triggers");
        let trigger1 = TriggerInfo {
            name: "t1".into(),
            table_name: "users".into(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "".into(),
        };
        let trigger2 = TriggerInfo {
            name: "t2".into(),
            table_name: "orders".into(),
            timing: crate::engine::TriggerTiming::After,
            event: crate::engine::TriggerEvent::Update,
            body: "".into(),
        };
        storage.create_trigger(trigger1).unwrap();
        storage.create_trigger(trigger2).unwrap();
        let users_triggers = storage.list_triggers("users");
        let orders_triggers = storage.list_triggers("orders");
        assert_eq!(users_triggers.len(), 1);
        assert_eq!(orders_triggers.len(), 1);
    }

    #[test]
    fn test_has_view_storage() {
        let storage = make_storage("fs_has_view");
        assert!(!storage.has_view("v"));
    }

    #[test]
    fn test_list_indexes_storage() {
        let mut storage = make_storage("fs_list_idx");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.create_index("t", "id", 0).unwrap();
        let indexes = storage.list_indexes("t");
        assert_eq!(indexes.len(), 1);
    }

    #[test]
    fn test_list_indexes_empty() {
        let storage = make_storage("fs_list_idx_empty");
        let indexes = storage.list_indexes("nonexistent");
        assert!(indexes.is_empty());
    }

    #[test]
    fn test_create_database_storage() {
        let mut storage = make_storage("fs_create_db_s");
        storage.create_database("db1").unwrap();
        let path = storage.data_dir.join("db1");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn test_drop_database_nonempty() {
        let mut storage = make_storage("fs_drop_db_ne2");
        storage.create_database("db1").unwrap();
        let path = storage.data_dir.join("db1");
        std::fs::write(path.join("marker.txt"), "x").unwrap();
        let result = storage.drop_database("db1");
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(storage.data_dir.join("db1"));
    }

    #[test]
    fn test_drop_column_storage() {
        let mut storage = make_storage("fs_drop_col");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![
                ColumnDefinition::new("a", "INTEGER"),
                ColumnDefinition::new("b", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![vec![Value::Integer(1), Value::Text("hello".into())]],
            )
            .unwrap();
        storage.drop_column("t", "b").unwrap();
        let info_after = storage.get_table_info("t").unwrap();
        assert_eq!(info_after.columns.len(), 1);
    }

    #[test]
    fn test_drop_column_table_not_found() {
        let mut storage = make_storage("fs_drop_col_ne");
        let result = storage.drop_column("nonexistent", "a");
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_column_not_found() {
        let mut storage = make_storage("fs_drop_col_nf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        let result = storage.drop_column("t", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_modify_column_storage() {
        let mut storage = make_storage("fs_mod_col");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        let new_def = ColumnDefinition {
            name: "a".into(),
            data_type: "TEXT".into(),
            nullable: true,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        };
        storage.modify_column("t", "a", new_def).unwrap();
    }

    #[test]
    fn test_modify_column_table_not_found() {
        let mut storage = make_storage("fs_mod_col_ne");
        let new_def = ColumnDefinition::new("a", "TEXT");
        let result = storage.modify_column("nonexistent", "a", new_def);
        assert!(result.is_err());
    }

    #[test]
    fn test_modify_column_not_found() {
        let mut storage = make_storage("fs_mod_col_nf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        let new_def = ColumnDefinition::new("nonexistent", "TEXT");
        let result = storage.modify_column("t", "nonexistent", new_def);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_merges_buffer() {
        let mut storage = make_storage("fs_scan_buf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.set_current_tx_id(1);
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert_eq!(rows.len(), 1);
        storage.set_current_tx_id(0);
    }

    #[test]
    fn test_buffered_insert_flush() {
        let dir = std::env::temp_dir().join("fs_buf_flush");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 2, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.insert("t", vec![vec![Value::Integer(2)]]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert!(rows.len() >= 1);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_buffered_insert_in_tx() {
        let dir = std::env::temp_dir().join("fs_buf_tx");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 10, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("x", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.set_current_tx_id(1);
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.flush_all_buffers().unwrap();
        storage.set_current_tx_id(0);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_trigger_roundtrip_disk() {
        let temp_dir = std::env::temp_dir().join("fs_trigger_roundtrip");
        let _ = remove_dir_all(&temp_dir);
        let mut storage = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        let trigger = TriggerInfo {
            name: "trig_disk".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "BEGIN UPDATE s SET n = n + 1; END".to_string(),
        };
        storage.create_trigger(trigger).unwrap();
        drop(storage);

        let mut storage2 = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        let got = storage2.get_trigger("trig_disk");
        assert!(got.is_some());
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_trigger_load_trigger_duplicate() {
        let temp_dir = std::env::temp_dir().join("fs_trigger_dup");
        let _ = remove_dir_all(&temp_dir);
        let mut storage = FileStorage::new(temp_dir.clone()).unwrap();
        let trigger = TriggerInfo {
            name: "trig_dup".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::After,
            event: crate::engine::TriggerEvent::Update,
            body: "".to_string(),
        };
        storage.create_trigger(trigger.clone()).unwrap();
        storage.create_trigger(trigger).unwrap();
        let count = storage.list_triggers("t").len();
        assert!(count >= 1);
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_drop_trigger_disk_removal() {
        let temp_dir = std::env::temp_dir().join("fs_drop_trig_disk");
        let _ = remove_dir_all(&temp_dir);
        let mut storage = FileStorage::new_with_wal(temp_dir.clone()).unwrap();
        let trigger = TriggerInfo {
            name: "trig_x".to_string(),
            table_name: "t".to_string(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Delete,
            body: "".to_string(),
        };
        storage.create_trigger(trigger).unwrap();
        storage.drop_trigger("trig_x").unwrap();
        let path = temp_dir.join("trigger_trig_x.json");
        assert!(!path.exists());
    }

    #[test]
    fn test_file_with_buffer_disabled_flow() {
        let mut storage = make_storage("fs_no_buf");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.delete("t", &[Value::Integer(1)]).unwrap();
        storage
            .update("t", &[Value::Integer(1)], &[(0, Value::Integer(99))])
            .unwrap();
        let rows = storage.scan("t").unwrap();
        assert!(rows.is_empty() || rows[0][0] != Value::Integer(1));
    }

    #[test]
    fn test_flush_all_buffers_with_buffered_table() {
        let dir = std::env::temp_dir().join("fs_flush_buf_table");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 1000, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.flush_all_buffers().unwrap();
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_load_table_invalid_json() {
        let temp_dir = std::env::temp_dir().join("fs_invalid_json");
        let _ = remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("bad.json"), "not valid json{{{").unwrap();
        let storage = FileStorage::new(temp_dir.clone()).unwrap();
        let result = storage.get_table("bad");
        assert!(result.is_none());
        let _ = remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_buffered_threshold_flushing() {
        let dir = std::env::temp_dir().join("fs_thr_flush");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 2, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1)],
                    vec![Value::Integer(2)],
                    vec![Value::Integer(3)],
                ],
            )
            .unwrap();
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_partition_rows_below_threshold() {
        let mut storage = make_storage("fs_pr_below");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", (0..100_i64).map(|i| vec![Value::Integer(i)]).collect())
            .unwrap();
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 100);
    }

    #[test]
    fn test_partition_rows_above_threshold() {
        let mut storage = make_storage("fs_pr_above");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 600_000);
    }

    #[test]
    fn test_partition_rows_missing_table() {
        let storage = make_storage("fs_pr_missing");
        let parts = storage.partition_rows("nonexistent", 4);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].is_empty());
    }

    #[test]
    fn test_partition_rows_num_partitions_zero() {
        let mut storage = make_storage("fs_pr_zero");
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 0);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_partition_rows_with_buffer() {
        let dir = std::env::temp_dir().join("fs_pr_buf");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 1000, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage.set_current_tx_id(1);
        storage
            .insert(
                "t",
                (0..600_000_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let _ = storage.partition_rows("t", 4);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_partition_rows_uneven_remainder() {
        let mut storage = FileStorage::new_with_buffer_config(
            std::env::temp_dir().join("fs_pr_uneven"),
            100,
            true,
        )
        .unwrap();
        let dir = std::env::temp_dir().join("fs_pr_uneven");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 100, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..503_003_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 503_003);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_partition_rows_two_partitions_above_threshold() {
        let dir = std::env::temp_dir().join("fs_pr_2p");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 100, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                (0..500_100_i64).map(|i| vec![Value::Integer(i)]).collect(),
            )
            .unwrap();
        let parts = storage.partition_rows("t", 2);
        assert_eq!(parts.len(), 2);
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_insert_direct_table_not_in_map() {
        let mut storage = make_storage("fs_ins_direct_ne");
        storage
            .insert("nonexistent", vec![vec![Value::Integer(1)]])
            .unwrap();
    }

    #[test]
    fn test_flush_buffer_with_buffered_rows() {
        let dir = std::env::temp_dir().join("fs_flush_buf");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 5, true).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("t", (0..10_i64).map(|i| vec![Value::Integer(i)]).collect())
            .unwrap();
        storage.flush_all_buffers().unwrap();
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_storage_save_reload_roundtrip() {
        let dir = std::env::temp_dir().join("fs_save_reload");
        let _ = remove_dir_all(&dir);
        let mut storage = FileStorage::new_with_buffer_config(dir.clone(), 100, false).unwrap();
        let info = TableInfo {
            name: "users".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        storage.create_table(&info).unwrap();
        storage
            .insert("users", vec![vec![Value::Integer(42)]])
            .unwrap();
        drop(storage);

        let storage2 = FileStorage::new(dir.clone()).unwrap();
        let rows = storage2.scan("users").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], Value::Integer(42));
        let _ = remove_dir_all(&dir);
    }

    #[test]
    fn test_storage_engine_get_table_info_not_found() {
        let storage = make_storage("fs_eng_gti_ne");
        let result = storage.get_table_info("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_engine_table_operations_empty() {
        let mut storage = make_storage("fs_eng_empty");
        assert!(storage.list_tables().is_empty());
        assert!(!storage.has_table("anytable"));
    }

    #[test]
    fn test_storage_engine_drop_index_nonexistent_table() {
        let mut storage = make_storage("fs_drop_idx_ne");
        storage.drop_index("nonexistent", "col").unwrap();
    }
}

impl FileStorage {
    fn insert_direct(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        if let Some(ref mut data) = self.tables.get_mut(table) {
            data.rows.extend(records);
            let table_data = data.clone();
            self.save_table(table, &table_data)?;
        }
        Ok(())
    }

    fn insert_buffered(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let buffered = self.insert_buffer.entry(table.to_string()).or_default();
        buffered.extend(records);

        if buffered.len() >= self.buffer_threshold {
            self.flush_buffer(table)?;
        }
        Ok(())
    }

    fn flush_buffer(&mut self, table: &str) -> SqlResult<()> {
        if let Some(records) = self.insert_buffer.remove(table) {
            self.insert_direct(table, records)?;
        }
        Ok(())
    }

    pub fn flush_all_buffers(&mut self) -> SqlResult<()> {
        let tables: Vec<String> = self.insert_buffer.keys().cloned().collect();
        for table in tables {
            self.flush_buffer(&table)?;
        }
        Ok(())
    }

    /// Discard all buffered in-memory inserts without persisting them.
    /// Used by `WalStorage::rollback_transaction` so the rolled-back
    /// transaction's writes are not visible to subsequent reads or to
    /// the next `flush()`. Issue #3964: previously rollback called
    /// `inner.flush()`, which pushed the buffer to `data.rows` and then
    /// persisted the table to disk — making rolled-back rows visible.
    pub fn discard_all_buffers(&mut self) {
        self.insert_buffer.clear();
        // No dirty_tables entry to remove — the buffer was never
        // persisted, so the dirty marker for the rolled-back tx was
        // either not yet added or, if previously added by a prior
        // committed tx in the same session, the next flush() will
        // simply re-save the persisted state.
    }

    /// v3.10.0 Issue #3703: returns pre-partitioned chunks so the caller
    /// (typically `engine_select::filter_partitions_parallel`) can
    /// process each chunk on a separate rayon worker, fusing scan
    /// + filter into one parallel pipeline. Same semantics as
    ///   `MemoryStorage::partition_rows`.
    ///
    /// Merges `insert_buffer` rows (F-09 fix from `scan()`) so same-
    /// transaction SELECT/UPDATE sees rows that were just inserted.
    pub fn partition_rows(&self, table: &str, num_partitions: usize) -> Vec<Vec<Record>> {
        const PARALLEL_SCAN_MIN_ROWS: usize = 500_000;
        let n_partitions = num_partitions.max(1);
        let Some(table_data) = self.get_table(table) else {
            return vec![Vec::new()];
        };
        let total_rows =
            table_data.rows.len() + self.insert_buffer.get(table).map(|b| b.len()).unwrap_or(0);
        if total_rows < PARALLEL_SCAN_MIN_ROWS || n_partitions <= 1 {
            let mut all: Vec<Record> = table_data.rows.clone();
            if let Some(buffered) = self.insert_buffer.get(table) {
                all.extend(buffered.iter().cloned());
            }
            return vec![all];
        }
        let mut all: Vec<Record> = table_data.rows.clone();
        if let Some(buffered) = self.insert_buffer.get(table) {
            all.extend(buffered.iter().cloned());
        }
        let total = all.len();
        let base = total / n_partitions;
        let rem = total % n_partitions;
        let mut out: Vec<Vec<Record>> = Vec::with_capacity(n_partitions);
        let mut cur = 0usize;
        for i in 0..n_partitions {
            let size = base + if i < rem { 1 } else { 0 };
            out.push(all[cur..cur + size].to_vec());
            cur += size;
        }
        out
    }
}

impl FileStorage {
    /// Discard all row data in every in-memory table while preserving the
    /// schema. Used by `with_wal_recovery` to make the WAL the sole source
    /// of truth on startup, so we never end up with both persisted rows
    /// and replayed rows for the same entries.
    pub fn clear_all_tables(&mut self) {
        for data in self.tables.values_mut() {
            data.rows.clear();
        }
        self.insert_buffer.clear();
    }
}

impl StorageEngine for FileStorage {
    fn in_transaction(&self) -> bool {
        self.current_tx_id != 0
    }

    fn current_tx_id(&self) -> u64 {
        self.current_tx_id
    }

    fn set_current_tx_id(&mut self, id: u64) {
        self.current_tx_id = id;
    }

    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        let mut rows: Vec<Record> = self
            .get_table(table)
            .map(|data| data.rows.clone())
            .unwrap_or_default();
        // F-09 fix: merge insert_buffer so same-transaction SELECT/UPDATE sees
        // the rows that were just inserted (and not yet flushed to data.rows).
        if let Some(buffered) = self.insert_buffer.get(table) {
            rows.extend(buffered.iter().cloned());
        }
        Ok(rows)
    }
    fn parallel_scan(
        &self,
        table: &str,
        num_partitions: usize,
    ) -> SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>> {
        // FileStorage caches all rows in memory (self.tables), so the
        // parallel_scan implementation mirrors MemoryStorage: partition
        // the cached row set into N iterators.
        //
        // For true disk-level parallelism (each worker reading a different
        // file offset), the file format would need to expose row offsets
        // via get_partition_boundaries(). The current on-disk format stores
        // rows in a length-prefixed binary format, so row-level seek is
        // possible but requires iterating from the start to find partition
        // boundaries. A future optimization can add that.
        let mut rows: Vec<Record> = self
            .get_table(table)
            .map(|data| data.rows.clone())
            .unwrap_or_default();
        // F-09 fix: merge insert_buffer for same-tx visibility
        if let Some(buffered) = self.insert_buffer.get(table) {
            rows.extend(buffered.iter().cloned());
        }

        let total = rows.len();
        if total == 0 || num_partitions == 0 {
            return Ok(vec![]);
        }
        let num_partitions = num_partitions.min(total);
        let base = total / num_partitions;
        let rem = total % num_partitions;
        let mut partitions: Vec<Box<dyn Iterator<Item = Record> + Send>> =
            Vec::with_capacity(num_partitions);
        let mut cur = 0;
        // v3.10.0 Issue #3776 / F-36: Arc-shared, no per-partition Vec clone
        let shared: Arc<Vec<Record>> = Arc::new(rows);
        for i in 0..num_partitions {
            let size = if i < rem { base + 1 } else { base };
            if size > 0 {
                let part = Arc::clone(&shared);
                partitions.push(Box::new(SharedSliceIter::new(part, cur, cur + size)));
            }
            cur += size;
        }
        Ok(partitions)
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        // PR-842: route inserts through the buffer when we are inside a
        // transaction so that a crash before COMMIT does not leak partially
        // applied rows to disk. Outside a transaction (autocommit) the
        // insert is durable immediately. `enable_buffer: false` is
        // overridden for tx-scoped writes so WAL recovery sees a clean
        // apply-or-rollback boundary.
        //
        // v3.11.0 P1 fix: removed `records.len() >= self.buffer_threshold`
        // condition that triggered immediate `insert_direct` (full table save).
        // This was causing O(N * table_size) behavior during bulk loads where
        // each batch of 100+ rows triggered a full table serialization and write.
        // Now: always buffer inserts, caller explicitly calls flush() to persist.
        if self.in_transaction() {
            self.insert_buffered(table, records)?
        } else if !self.enable_buffer {
            self.insert_direct(table, records)?
        } else {
            self.insert_buffered(table, records)?
        };
        // V311-07: Mark table dirty for optimized flush
        self.dirty_tables.insert(table.to_string());
        Ok(())
    }

    /// F-09 fix: bypass insert_buffer so WAL recovery can replay entries
    /// deterministically. Subsequent scan/delete in the same recovery pass
    /// see the row in `data.rows` directly, avoiding the "3 rows expected 1"
    /// regression caused by buffered inserts piling up during replay.
    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        self.insert_direct(table, vec![record])
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        if let Some(ref mut data) = self.tables.get_mut(table) {
            let original_len = data.rows.len();
            if filters.is_empty() {
                data.rows.clear();
            } else {
                // Row-level delete: keep rows that do NOT match the filter
                // (filter values are compared positionally against each row's
                // values; a row is "matched" when every filter slot equals
                // the row's value at the same slot).
                data.rows.retain(|row| {
                    !filters
                        .iter()
                        .enumerate()
                        .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
                });
            }
            let new_len = data.rows.len();
            let removed = original_len - new_len;

            // V311-07: Mark dirty instead of immediate persist
            if removed > 0 || filters.is_empty() {
                self.dirty_tables.insert(table.to_string());
            }

            // After full table delete (filters.is_empty()), clear any buffered
            // inserts. The caller (UPDATE implementation) will re-insert the
            // correct rows. We do NOT re-insert the buffered rows since they
            // represent old state that should be replaced, not preserved.
            if filters.is_empty() {
                self.insert_buffer.remove(table);
            } else if let Some(buffered) = self.insert_buffer.get_mut(table) {
                // For non-empty filters, also remove matching rows from the
                // insert_buffer so that UPDATE with WHERE clause does not
                // leave stale buffered rows that shadow the updated value.
                buffered.retain(|row| {
                    !filters
                        .iter()
                        .enumerate()
                        .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
                });
            }
            Ok(removed)
        } else {
            Ok(0)
        }
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        if let Some(ref mut data) = self.tables.get_mut(table) {
            let original_len = data.rows.len();
            data.rows.retain(|r| !filter(r));
            let new_len = data.rows.len();
            // V311-07: Mark dirty instead of immediate persist
            if new_len < original_len {
                self.dirty_tables.insert(table.to_string());
            }
            Ok(original_len - new_len)
        } else {
            Ok(0)
        }
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let Some(ref mut data) = self.tables.get_mut(table) else {
            return Ok(0);
        };

        let mut count = 0;
        for record in data.rows.iter_mut() {
            if filters.is_empty()
                || filters
                    .iter()
                    .enumerate()
                    .all(|(i, f)| record.get(i).map(|v| v == f).unwrap_or(false))
            {
                for &(col_idx, ref new_val) in updates {
                    if col_idx < record.len() {
                        record[col_idx] = new_val.clone();
                    }
                }
                count += 1;
            }
        }
        // V311-07: Mark dirty instead of immediate persist
        if count > 0 {
            self.dirty_tables.insert(table.to_string());
        }
        Ok(count)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        let Some(data) = self.tables.get_mut(table) else {
            return Ok(0);
        };

        let mut count = 0;
        let assignments = mutation.assignments();

        for record in data.rows.iter_mut() {
            if filter(record) {
                for &(col_idx, ref new_val) in assignments {
                    if col_idx < record.len() {
                        record[col_idx] = new_val.clone();
                    }
                }
                count += 1;
            }
        }
        // V311-07: Mark dirty instead of immediate persist
        if count > 0 {
            self.dirty_tables.insert(table.to_string());
        }
        Ok(count)
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        let table_data = TableData {
            info: info.clone(),
            rows: Vec::new(),
        };
        self.insert_table(info.name.clone(), table_data)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.drop_table(table)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.get_table(table)
            .map(|t| t.info.clone())
            .ok_or_else(|| SqlError::TableNotFound(table.to_string()))
    }

    fn flush(&mut self) -> SqlResult<()> {
        self.flush_all_buffers()?;
        let dirty: Vec<String> = std::mem::take(&mut self.dirty_tables).into_iter().collect();
        for name in dirty {
            if let Some(table_data) = self.tables.get(&name).cloned() {
                self.save_table(&name, &table_data)?;
            }
        }
        Ok(())
    }

    fn discard_all_buffers(&mut self) {
        // StorageEngine::discard_all_buffers default is a no-op; for
        // FileStorage we actually drop the buffered inserts. Issue
        // #3964: rollback must NOT persist.
        self.insert_buffer.clear();
    }

    fn has_table(&self, table: &str) -> bool {
        self.tables.contains_key(table)
    }

    fn list_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()> {
        // Inline the implementation to avoid potential recursion issues
        // Get table from tables
        let table_data = self
            .tables
            .get(table)
            .cloned()
            .ok_or_else(|| SqlError::TableNotFound(table.to_string()))?;

        // Build B+ Tree from existing rows
        let mut index = crate::bplus_tree::BPlusTree::new();
        for (row_id, row) in table_data.rows.iter().enumerate() {
            if let Some(value) = row.get(column_index) {
                if let Some(key) = value.to_index_key() {
                    index.insert(key, row_id as u32);
                }
            }
        }

        // Save to disk
        self.save_index(table, column, &index)
            .map_err(SqlError::from)?;

        // Store in memory
        let mut indexes = self.indexes.write().unwrap();
        indexes.insert((table.to_string(), column.to_string()), index);

        Ok(())
    }

    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()> {
        let key = (table.to_string(), column.to_string());

        if let Ok(mut indexes) = self.indexes.write() {
            indexes.remove(&key);
        }

        let path = self.index_path(table, column);
        if path.exists() {
            std::fs::remove_file(path).map_err(SqlError::from)?;
        }

        Ok(())
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        if let Some(data) = self.tables.get_mut(table) {
            data.info.columns.push(column);
            let table_data = data.clone();
            self.save_table(table, &table_data)?;
        }
        Ok(())
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        if let Some(mut table_data) = self.tables.remove(table) {
            table_data.info.name = new_name.to_string();
            let old_path = self.table_path(table);
            let new_path = self.table_path(new_name);

            self.save_table(new_name, &table_data)?;

            if old_path.exists() {
                std::fs::rename(&old_path, &new_path).map_err(SqlError::from)?;
            }

            self.tables.insert(new_name.to_string(), table_data);

            if let Ok(mut indexes) = self.indexes.write() {
                let keys: Vec<_> = indexes.keys().cloned().collect();
                for key in keys {
                    if key.0 == table {
                        let new_key = (new_name.to_string(), key.1.clone());
                        if let Some(idx) = indexes.remove(&key) {
                            indexes.insert(new_key, idx);
                        }
                    }
                }
            }

            if let Ok(indexes) = self.indexes.read() {
                for key in indexes.keys() {
                    if key.0 == new_name {
                        let old_idx_path = self.index_path(table, &key.1);
                        let new_idx_path = self.index_path(new_name, &key.1);
                        if old_idx_path.exists() {
                            std::fs::rename(&old_idx_path, &new_idx_path).ok();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        // Persist to disk first (WAL-style: write-ahead, then mutate in-memory)
        self.save_trigger(&info)
            .map_err(|e| SqlError::ExecutionError(format!("save trigger: {}", e)))?;
        let mut triggers = self.triggers.write().unwrap();
        triggers.insert(info.name.clone(), info);
        Ok(())
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.remove_trigger_file(name)
            .map_err(|e| SqlError::ExecutionError(format!("remove trigger: {}", e)))?;
        let mut triggers = self.triggers.write().unwrap();
        triggers.remove(name);
        Ok(())
    }

    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        let triggers = self.triggers.read().unwrap();
        triggers.get(name).cloned()
    }

    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        let triggers = self.triggers.read().unwrap();
        triggers
            .values()
            .filter(|t| t.table_name == table)
            .cloned()
            .collect()
    }

    fn has_view(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        let indexes = self.indexes.read().unwrap();
        indexes
            .iter()
            .filter(|((t, _c), _idx)| t == table)
            .map(|((t, c), _idx)| (c.clone(), format!("{}_idx_{}", t, c)))
            .collect()
    }

    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        let db_path = self.data_dir.join(db_name);
        std::fs::create_dir_all(&db_path)
            .map_err(|e| SqlError::ExecutionError(format!("create_database: {}", e)))
    }

    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        let db_path = self.data_dir.join(db_name);
        if db_path.exists() {
            let is_empty = std::fs::read_dir(&db_path)
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);
            if !is_empty {
                return Err(SqlError::ExecutionError(
                    "database is not empty".to_string(),
                ));
            }
            std::fs::remove_dir(&db_path)
                .map_err(|e| SqlError::ExecutionError(format!("drop_database: {}", e)))?;
        }
        Ok(())
    }

    fn drop_column(&mut self, table: &str, column: &str) -> SqlResult<()> {
        let table_data = self
            .tables
            .get_mut(table)
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col_idx = table_data
            .info
            .columns
            .iter()
            .position(|c| c.name == column)
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", column)))?;
        table_data.info.columns.remove(col_idx);
        for record in table_data.rows.iter_mut() {
            if col_idx < record.len() {
                record.remove(col_idx);
            }
        }
        Ok(())
    }

    fn modify_column(
        &mut self,
        table: &str,
        column: &str,
        new_def: ColumnDefinition,
    ) -> SqlResult<()> {
        let table_data = self
            .tables
            .get_mut(table)
            .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let col_idx = table_data
            .info
            .columns
            .iter()
            .position(|c| c.name == column)
            .ok_or_else(|| SqlError::ExecutionError(format!("Column not found: {}", column)))?;
        table_data.info.columns[col_idx] = new_def;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod parallel_scan_tests {
    use super::*;

    #[test]
    fn test_parallel_scan_file_storage() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_parallel_scan_test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Create storage and insert test data
        {
            let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

            let table_data = TableData {
                info: TableInfo {
                    name: "numbers".to_string(),
                    columns: vec![ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        char_max_length: None,
                        collation: None,
                        default_value: None,
                        auto_increment: false,
                    }],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                },
                rows: (0..100i64).map(|i| vec![Value::Integer(i)]).collect(),
            };

            storage
                .insert_table("numbers".to_string(), table_data)
                .unwrap();
        }

        // Load storage and test parallel_scan
        {
            let storage = FileStorage::new(temp_dir.clone()).unwrap();

            // Test with 4 partitions
            let partitions = storage.parallel_scan("numbers", 4).unwrap();

            // Note: FileStorage saves in binary format, so parallel_scan may return
            // fewer partitions due to format. The key invariant is that ALL rows
            // are returned across all partitions.
            assert!(!partitions.is_empty(), "Should have at least 1 partition");

            // Collect all rows from all partitions
            let total_rows: usize = partitions.into_iter().map(|p| p.count()).sum();
            assert_eq!(total_rows, 100, "Should return all 100 rows");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parallel_scan_empty_table() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_parallel_scan_empty");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let storage = FileStorage::new(temp_dir.clone()).unwrap();
        // Should return empty vec for non-existent table
        let partitions = storage.parallel_scan("nonexistent", 4).unwrap();
        assert!(
            partitions.is_empty(),
            "Non-existent table should return empty partitions"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parallel_scan_single_partition() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_parallel_scan_single");
        let _ = std::fs::remove_dir_all(&temp_dir);

        {
            let mut storage = FileStorage::new(temp_dir.clone()).unwrap();

            let table_data = TableData {
                info: TableInfo {
                    name: "small".to_string(),
                    columns: vec![ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        char_max_length: None,
                        collation: None,
                        default_value: None,
                        auto_increment: false,
                    }],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                },
                rows: vec![vec![Value::Integer(1)], vec![Value::Integer(2)]],
            };

            storage
                .insert_table("small".to_string(), table_data)
                .unwrap();
        }

        {
            let storage = FileStorage::new(temp_dir.clone()).unwrap();
            let partitions = storage.parallel_scan("small", 1).unwrap();
            assert!(!partitions.is_empty());

            let total: usize = partitions.into_iter().map(|p| p.count()).sum();
            assert_eq!(total, 2, "Should return all 2 rows");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
impl FileStorage {
    /// Flush dirty tables in parallel using std::thread
    /// V311-09: Addresses global lock bottleneck - parallel table writes
    pub fn flush_parallel(&mut self) -> std::io::Result<()> {
        // Take dirty tables set, leaving empty set behind
        let dirty: Vec<String> = std::mem::take(&mut self.dirty_tables).into_iter().collect();

        if dirty.is_empty() {
            return Ok(());
        }

        // For 1-2 tables, sequential is faster (no thread overhead)
        if dirty.len() <= 2 {
            return self.flush();
        }

        // For 3+ tables, flush in parallel using thread pool
        let results = std::thread::scope(|s| {
            let handles: Vec<_> = dirty
                .iter()
                .map(|name| {
                    s.spawn(|| {
                        if let Some(table_data) = self.tables.get(name) {
                            self.save_table(name, table_data)
                        } else {
                            Ok(())
                        }
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });

        // Combine all results - return first error if any
        for result in results {
            result?;
        }

        Ok(())
    }
}
