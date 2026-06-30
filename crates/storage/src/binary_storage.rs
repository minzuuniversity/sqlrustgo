//! Binary Table Storage
//!
//! Fast binary format for table storage, replacing JSON.

use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlError, SqlResult, StorageEngine,
    TableData, TableInfo, TriggerInfo, Value,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Binary storage for tables
pub struct BinaryTableStorage {
    data_dir: PathBuf,
    /// In-memory cache of loaded tables
    tables: HashMap<String, TableData>,
    /// Track which tables have been loaded from .bin (lazy load)
    loaded: HashMap<String, bool>,
}

impl BinaryTableStorage {
    pub fn new(data_dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self {
            data_dir,
            tables: HashMap::new(),
            loaded: HashMap::new(),
        })
    }

    /// Create a new BinaryTableStorage and immediately load all .bin files from data_dir.
    /// Use this when the data_dir already contains .bin files (e.g. TPC-H SF1 data).
    pub fn new_with_data(data_dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let mut storage = Self {
            data_dir,
            tables: HashMap::new(),
            loaded: HashMap::new(),
        };
        storage.load_all_tables()?;
        Ok(storage)
    }

    fn load_all_tables(&mut self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                if let Some(table_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(table_data) = self.load(table_name) {
                        self.tables.insert(table_name.to_string(), table_data);
                        self.loaded.insert(table_name.to_string(), true);
                    }
                }
            }
        }
        Ok(())
    }

    fn table_path(&self, table: &str) -> PathBuf {
        self.data_dir.join(format!("{}.bin", table))
    }

    /// Save table in binary format (BINT v2).
    ///
    /// Header (BINT v2):
    ///   4 bytes: magic "BINT"
    ///   4 bytes: version u32 LE (= 2)
    ///   4 bytes: column_count u32 LE
    ///   For each column:
    ///     1 byte: type_code (1=INT, 2=FLOAT, 3=TEXT)
    ///     2 bytes: name_len u16 LE
    ///     N bytes: column name (utf-8)
    ///   8 bytes: row_count u64 LE
    /// Per value:
    ///   1 byte: type_code (0=Null, 1=Integer, 2=Float, 3=Text)
    ///   Integer: 8 bytes LE i64
    ///   Float:   8 bytes LE f64
    ///   Text:    4 bytes LE u32 (len) + len bytes
    pub fn save(&self, table: &str, data: &TableData) -> std::io::Result<()> {
        let path = self.table_path(table);
        let file = File::create(&path)?;
        let mut w = BufWriter::new(file);

        // Header: magic + version 2
        w.write_all(b"BINT")?;
        w.write_all(&2u32.to_le_bytes())?;

        // Column count
        w.write_all(&(data.info.columns.len() as u32).to_le_bytes())?;

        // Per-column type + name
        for col in &data.info.columns {
            let code = match col.data_type.to_uppercase().as_str() {
                t if t.contains("INT") => 1u8,
                t if t.contains("FLOAT") || t.contains("REAL") => 2u8,
                _ => 3u8, // TEXT
            };
            w.write_all(&[code])?;
            let name_bytes = col.name.as_bytes();
            w.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            w.write_all(name_bytes)?;
        }

        // Row count
        w.write_all(&(data.rows.len() as u64).to_le_bytes())?;

        // Write rows
        for row in &data.rows {
            for val in row {
                match val {
                    sqlrustgo_types::Value::Integer(i) => {
                        w.write_all(&[1])?;
                        w.write_all(&i.to_le_bytes())?;
                    }
                    sqlrustgo_types::Value::Float(f) => {
                        w.write_all(&[2])?;
                        w.write_all(&f.to_le_bytes())?;
                    }
                    sqlrustgo_types::Value::Text(s) => {
                        w.write_all(&[3])?;
                        let bytes = s.as_bytes();
                        w.write_all(&(bytes.len() as u32).to_le_bytes())?;
                        w.write_all(bytes)?;
                    }
                    sqlrustgo_types::Value::Null => {
                        w.write_all(&[0])?;
                    }
                    _ => {
                        w.write_all(&[0])?;
                    }
                }
            }
        }

        w.flush()
    }

    /// Load table from binary format (supports BINT v1 and v2).
    /// v1: header has col_count + col_types only; columns named col_0, col_1, ...
    /// v2: header has col_count + (type_code + name_len + name) per column.
    pub fn load(&self, table: &str) -> std::io::Result<TableData> {
        use std::io::{BufReader, Read};
        let path = self.table_path(table);
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file);

        // Header: magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"BINT" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a binary table file",
            ));
        }

        // Version
        let mut ver = [0u8; 4];
        reader.read_exact(&mut ver)?;
        let version = u32::from_le_bytes(ver);

        // Column count
        let mut col_count_buf = [0u8; 4];
        reader.read_exact(&mut col_count_buf)?;
        let col_count = u32::from_le_bytes(col_count_buf) as usize;

        // Column types and names
        let mut col_types = Vec::with_capacity(col_count);
        let mut col_names: Vec<String> = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let mut tc = [0u8; 1];
            reader.read_exact(&mut tc)?;
            col_types.push(tc[0]);
            if version >= 2 {
                let mut name_len_buf = [0u8; 2];
                reader.read_exact(&mut name_len_buf)?;
                let name_len = u16::from_le_bytes(name_len_buf) as usize;
                let mut name_bytes = vec![0u8; name_len];
                reader.read_exact(&mut name_bytes)?;
                col_names.push(String::from_utf8_lossy(&name_bytes).to_string());
            } else {
                col_names.push(format!("col_{}", i));
            }
        }

        // Row count
        let mut row_count_buf = [0u8; 8];
        reader.read_exact(&mut row_count_buf)?;
        let row_count = u64::from_le_bytes(row_count_buf) as usize;

        // Build columns
        let columns: Vec<ColumnDefinition> = (0..col_count)
            .map(|i| ColumnDefinition {
                name: col_names[i].clone(),
                data_type: match col_types[i] {
                    1 => "INTEGER".to_string(),
                    2 => "REAL".to_string(),
                    _ => "TEXT".to_string(),
                },
                nullable: true,
                primary_key: false,
                char_max_length: None,
            })
            .collect();

        let info = TableInfo {
            name: table.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
        };

        // Read rows
        let mut rows = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            let mut row = Vec::with_capacity(col_count);
            for _ in 0..col_types.len() {
                // Read value type code
                let mut vtc = [0u8; 1];
                reader.read_exact(&mut vtc)?;
                match vtc[0] {
                    0 => row.push(sqlrustgo_types::Value::Null),
                    1 => {
                        let mut buf = [0u8; 8];
                        reader.read_exact(&mut buf)?;
                        row.push(sqlrustgo_types::Value::Integer(i64::from_le_bytes(buf)));
                    }
                    2 => {
                        let mut buf = [0u8; 8];
                        reader.read_exact(&mut buf)?;
                        row.push(sqlrustgo_types::Value::Float(f64::from_le_bytes(buf)));
                    }
                    3 => {
                        let mut len = [0u8; 4];
                        reader.read_exact(&mut len)?;
                        let len = u32::from_le_bytes(len) as usize;
                        let mut s = vec![0u8; len];
                        reader.read_exact(&mut s)?;
                        row.push(sqlrustgo_types::Value::Text(
                            String::from_utf8_lossy(&s).to_string(),
                        ));
                    }
                    _ => row.push(sqlrustgo_types::Value::Null),
                }
            }
            rows.push(row);
        }

        Ok(TableData { info, rows })
    }

    pub fn exists(&self, table: &str) -> bool {
        self.table_path(table).exists()
    }

    /// Persist a table to disk in binary format
    pub fn persist_table(&self, name: &str) -> SqlResult<()> {
        if let Some(table_data) = self.tables.get(name) {
            self.save(name, table_data)
                .map_err(|e| SqlError::ExecutionError(format!("persist_table: {}", e)))?;
        }
        Ok(())
    }
}

impl StorageEngine for BinaryTableStorage {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        Ok(self
            .tables
            .get(table)
            .map(|data| data.rows.clone())
            .unwrap_or_default())
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let table_data = self
            .tables
            .entry(table.to_string())
            .or_insert_with(|| TableData {
                info: TableInfo {
                    name: table.to_string(),
                    columns: vec![],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    partition_info: None,
                },
                rows: Vec::new(),
            });
        table_data.rows.extend(records);
        self.persist_table(table)?;
        Ok(())
    }

    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        let table_data = self
            .tables
            .entry(table.to_string())
            .or_insert_with(|| TableData {
                info: TableInfo {
                    name: table.to_string(),
                    columns: vec![],
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    partition_info: None,
                },
                rows: Vec::new(),
            });
        table_data.rows.push(record);
        self.persist_table(table)?;
        Ok(())
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        let Some(data) = self.tables.get_mut(table) else {
            return Ok(0);
        };
        let original_len = data.rows.len();
        if filters.is_empty() {
            data.rows.clear();
        } else {
            data.rows.retain(|row| {
                !filters
                    .iter()
                    .enumerate()
                    .all(|(i, f)| row.get(i).map(|v| v == f).unwrap_or(false))
            });
        }
        let removed = original_len - data.rows.len();
        if removed > 0 || filters.is_empty() {
            self.persist_table(table)?;
        }
        Ok(removed)
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        let Some(data) = self.tables.get_mut(table) else {
            return Ok(0);
        };
        let original_len = data.rows.len();
        data.rows.retain(|r| !filter(r));
        let removed = original_len - data.rows.len();
        if removed > 0 {
            self.persist_table(table)?;
        }
        Ok(removed)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let Some(data) = self.tables.get_mut(table) else {
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
        if count > 0 {
            self.persist_table(table)?;
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
        if count > 0 {
            self.persist_table(table)?;
        }
        Ok(count)
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        let table_data = TableData {
            info: info.clone(),
            rows: Vec::new(),
        };
        self.tables.insert(info.name.clone(), table_data);
        self.loaded.insert(info.name.clone(), true);
        self.persist_table(&info.name)?;
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.tables.remove(table);
        self.loaded.remove(table);
        let path = self.table_path(table);
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| SqlError::ExecutionError(format!("drop_table: {}", e)))?;
        }
        Ok(())
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.tables
            .get(table)
            .map(|t| t.info.clone())
            .ok_or_else(|| SqlError::TableNotFound(table.to_string()))
    }

    fn has_table(&self, table: &str) -> bool {
        self.tables.contains_key(table)
    }

    fn list_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    fn create_index(&mut self, _table: &str, _column: &str, _column_index: usize) -> SqlResult<()> {
        // BinaryTableStorage doesn't support indexes yet
        Ok(())
    }

    fn drop_index(&mut self, _table: &str, _column: &str) -> SqlResult<()> {
        Ok(())
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        if let Some(data) = self.tables.get_mut(table) {
            data.info.columns.push(column);
            self.persist_table(table)?;
        }
        Ok(())
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        if let Some(mut table_data) = self.tables.remove(table) {
            table_data.info.name = new_name.to_string();
            self.tables.insert(new_name.to_string(), table_data);
            self.loaded.remove(table);
            self.loaded.insert(new_name.to_string(), true);

            let old_path = self.table_path(table);
            let new_path = self.table_path(new_name);

            if old_path.exists() {
                std::fs::rename(&old_path, &new_path)
                    .map_err(|e| SqlError::ExecutionError(format!("rename: {}", e)))?;
            }
        }
        Ok(())
    }

    fn create_trigger(&mut self, _info: crate::engine::TriggerInfo) -> SqlResult<()> {
        // BinaryTableStorage doesn't support triggers
        Ok(())
    }

    fn drop_trigger(&mut self, _name: &str) -> SqlResult<()> {
        Ok(())
    }

    fn get_trigger(&self, _name: &str) -> Option<crate::engine::TriggerInfo> {
        None
    }

    fn list_triggers(&self, _table: &str) -> Vec<crate::engine::TriggerInfo> {
        vec![]
    }

    fn list_indexes(&self, _table: &str) -> Vec<(String, String)> {
        vec![]
    }

    fn has_view(&self, _name: &str) -> bool {
        false
    }

    fn create_database(&mut self, _db_name: &str) -> SqlResult<()> {
        // BinaryTableStorage is directory-based; no-op since data_dir already exists
        Ok(())
    }

    fn drop_database(&mut self, _db_name: &str) -> SqlResult<()> {
        // BinaryTableStorage is directory-based; no-op
        Ok(())
    }

    fn flush(&mut self) -> SqlResult<()> {
        for name in self.tables.keys() {
            self.persist_table(name)?;
        }
        Ok(())
    }

    fn is_wal_enabled(&self) -> bool {
        false
    }
}

/// A `StorageEngine` wrapper that forwards all calls through a `Box<dyn StorageEngine>`.
/// This is the simplest way to create a type-erased `StorageEngine` — use `Box::new(s)` to wrap
/// any concrete `S: StorageEngine`, then `Arc::new(RwLock::new(BoxStorageEngine(box)))`.
pub struct BoxStorageEngine {
    inner: Box<dyn StorageEngine>,
}

impl BoxStorageEngine {
    /// Wrap any `S: StorageEngine` in a `BoxStorageEngine`.
    pub fn new<S: StorageEngine + 'static>(inner: S) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl std::ops::Deref for BoxStorageEngine {
    type Target = dyn StorageEngine;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl std::ops::DerefMut for BoxStorageEngine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl StorageEngine for BoxStorageEngine {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        (**self).scan(table)
    }
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        (**self).insert(table, records)
    }
    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        (**self).force_insert(table, record)
    }
    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        (**self).delete(table, filters)
    }
    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        (**self).delete_if(table, filter)
    }
    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        (**self).update(table, filters, updates)
    }
    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        (**self).update_if(table, filter, mutation)
    }
    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        (**self).create_database(db_name)
    }
    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        (**self).drop_database(db_name)
    }
    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        (**self).create_table(info)
    }
    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        (**self).drop_table(table)
    }
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        (**self).get_table_info(table)
    }
    fn has_table(&self, table: &str) -> bool {
        (**self).has_table(table)
    }
    fn list_tables(&self) -> Vec<String> {
        (**self).list_tables()
    }
    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()> {
        (**self).create_index(table, column, column_index)
    }
    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()> {
        (**self).drop_index(table, column)
    }
    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        (**self).add_column(table, column)
    }
    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        (**self).rename_table(table, new_name)
    }
    fn drop_column(&mut self, table: &str, column: &str) -> SqlResult<()> {
        (**self).drop_column(table, column)
    }
    fn modify_column(
        &mut self,
        table: &str,
        column: &str,
        new_def: ColumnDefinition,
    ) -> SqlResult<()> {
        (**self).modify_column(table, column, new_def)
    }
    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        (**self).create_trigger(info)
    }
    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        (**self).drop_trigger(name)
    }
    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        (**self).get_trigger(name)
    }
    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        (**self).list_triggers(table)
    }
    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        (**self).list_indexes(table)
    }
    fn has_view(&self, name: &str) -> bool {
        (**self).has_view(name)
    }
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        (**self).begin_transaction()
    }
    fn commit_transaction(&mut self) -> SqlResult<()> {
        (**self).commit_transaction()
    }
    fn rollback_transaction(&mut self) -> SqlResult<()> {
        (**self).rollback_transaction()
    }
    fn in_transaction(&self) -> bool {
        (**self).in_transaction()
    }
    fn current_tx_id(&self) -> u64 {
        (**self).current_tx_id()
    }
    fn set_current_tx_id(&mut self, id: u64) {
        (**self).set_current_tx_id(id)
    }
    fn flush(&mut self) -> SqlResult<()> {
        (**self).flush()
    }
    fn is_wal_enabled(&self) -> bool {
        (**self).is_wal_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_save_load() {
        let tmp = std::env::temp_dir().join("bin_test");
        let storage = BinaryTableStorage::new(tmp.clone()).unwrap();

        let cols = vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        }];
        let rows = vec![vec![sqlrustgo_types::Value::Integer(42)]];

        let data = TableData {
            info: TableInfo {
                name: "test".to_string(),
                columns: cols,
                ..Default::default()
            },
            rows,
        };

        storage.save("test", &data).unwrap();
        assert!(storage.exists("test"));

        let loaded = storage.load("test").unwrap();
        assert_eq!(loaded.rows.len(), 1);

        if let sqlrustgo_types::Value::Integer(i) = loaded.rows[0][0] {
            assert_eq!(i, 42);
        } else {
            panic!("Expected Integer");
        }

        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn test_binary_storage_new() {
        let tmp = std::env::temp_dir().join("bin_test_new");
        let result = BinaryTableStorage::new(tmp.clone());
        assert!(result.is_ok());
        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn test_binary_storage_exists() {
        let tmp = std::env::temp_dir().join("bin_test_exists");
        let storage = BinaryTableStorage::new(tmp.clone()).unwrap();
        assert!(!storage.exists("nonexistent"));
        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn test_binary_storage_multiple_tables() {
        let tmp = std::env::temp_dir().join("bin_test_multi");
        let storage = BinaryTableStorage::new(tmp.clone()).unwrap();

        for i in 1..=3 {
            let cols = vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            }];
            let rows = vec![vec![sqlrustgo_types::Value::Integer(i)]];
            let data = TableData {
                info: TableInfo {
                    name: format!("table_{}", i),
                    columns: cols,
                    ..Default::default()
                },
                rows,
            };
            storage.save(&format!("table_{}", i), &data).unwrap();
        }

        assert!(storage.exists("table_1"));
        assert!(storage.exists("table_2"));
        assert!(storage.exists("table_3"));
        assert!(!storage.exists("table_4"));

        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn test_binary_storage_load_float() {
        let tmp = std::env::temp_dir().join("bin_test_float");
        let storage = BinaryTableStorage::new(tmp.clone()).unwrap();

        let cols = vec![ColumnDefinition {
            name: "value".to_string(),
            data_type: "REAL".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        }];
        let rows = vec![vec![sqlrustgo_types::Value::Float(3.14159)]];

        let data = TableData {
            info: TableInfo {
                name: "floats".to_string(),
                columns: cols,
                ..Default::default()
            },
            rows,
        };

        storage.save("floats", &data).unwrap();
        let loaded = storage.load("floats").unwrap();
        assert_eq!(loaded.rows.len(), 1);

        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn test_binary_storage_load_text() {
        let tmp = std::env::temp_dir().join("bin_test_text");
        let storage = BinaryTableStorage::new(tmp.clone()).unwrap();

        let cols = vec![ColumnDefinition {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        }];
        let rows = vec![vec![sqlrustgo_types::Value::Text("hello".to_string())]];

        let data = TableData {
            info: TableInfo {
                name: "texts".to_string(),
                columns: cols,
                ..Default::default()
            },
            rows,
        };

        storage.save("texts", &data).unwrap();
        let loaded = storage.load("texts").unwrap();
        assert_eq!(loaded.rows.len(), 1);

        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn test_binary_storage_multiple_rows() {
        let tmp = std::env::temp_dir().join("bin_test_rows");
        let storage = BinaryTableStorage::new(tmp.clone()).unwrap();

        let cols = vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        }];
        let rows: Vec<Vec<sqlrustgo_types::Value>> = (1..=100)
            .map(|i| vec![sqlrustgo_types::Value::Integer(i)])
            .collect();

        let data = TableData {
            info: TableInfo {
                name: "many_rows".to_string(),
                columns: cols,
                ..Default::default()
            },
            rows,
        };

        storage.save("many_rows", &data).unwrap();
        let loaded = storage.load("many_rows").unwrap();
        assert_eq!(loaded.rows.len(), 100);

        std::fs::remove_dir_all(tmp).ok();
    }
}
