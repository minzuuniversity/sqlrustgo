//! TableRegistry - Manages per-table locks for concurrent table access
//!
//! V311-09: Table-level lock architecture

use crate::engine::{Record, SqlResult, TableInfo, Value};
use crate::table_engine::TableEngine;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// TableRegistry manages all tables with per-table locking
/// Each table has its own RwLock, allowing concurrent access to different tables
pub struct TableRegistry {
    tables: HashMap<String, Arc<RwLock<Box<dyn TableEngine>>>>,
}

impl TableRegistry {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    /// Register a new table
    pub fn register(&mut self, name: String, engine: Box<dyn TableEngine>) {
        self.tables.insert(name, Arc::new(RwLock::new(engine)));
    }

    /// Get a table by name
    pub fn get(&self, name: &str) -> Option<Arc<RwLock<Box<dyn TableEngine>>>> {
        self.tables.get(name).cloned()
    }

    /// Get table info
    pub fn get_table_info(&self, name: &str) -> SqlResult<TableInfo> {
        let table = self.get(name).ok_or_else(|| {
            crate::engine::SqlError::ExecutionError(format!("Table not found: {}", name))
        })?;
        let guard = table.read().unwrap();
        Ok(guard.get_table_info().clone())
    }

    /// Insert records into a table
    pub fn insert(&self, name: &str, records: Vec<Record>) -> SqlResult<()> {
        let table = self.get(name).ok_or_else(|| {
            crate::engine::SqlError::ExecutionError(format!("Table not found: {}", name))
        })?;
        let mut guard = table.write().unwrap();
        guard.insert(records)
    }

    /// Scan records from a table
    pub fn scan(&self, name: &str) -> SqlResult<Vec<Record>> {
        let table = self.get(name).ok_or_else(|| {
            crate::engine::SqlError::ExecutionError(format!("Table not found: {}", name))
        })?;
        let guard = table.read().unwrap();
        guard.scan()
    }

    /// Delete records from a table
    pub fn delete(&self, name: &str, filters: &[Value]) -> SqlResult<usize> {
        let table = self.get(name).ok_or_else(|| {
            crate::engine::SqlError::ExecutionError(format!("Table not found: {}", name))
        })?;
        let mut guard = table.write().unwrap();
        guard.delete(filters)
    }

    /// Update records in a table
    pub fn update(
        &self,
        name: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let table = self.get(name).ok_or_else(|| {
            crate::engine::SqlError::ExecutionError(format!("Table not found: {}", name))
        })?;
        let mut guard = table.write().unwrap();
        guard.update(filters, updates)
    }

    /// Flush a specific table
    pub fn flush_table(&self, name: &str) -> SqlResult<()> {
        let table = self.get(name).ok_or_else(|| {
            crate::engine::SqlError::ExecutionError(format!("Table not found: {}", name))
        })?;
        let mut guard = table.write().unwrap();
        guard.flush()
    }

    /// List all table names
    pub fn list_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    /// Check if table exists
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }
}

impl Default for TableRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Minimal in-memory TableEngine for testing the registry plumbing only.
    struct MemTable {
        info: TableInfo,
        rows: Vec<Record>,
        dirty: HashSet<String>,
    }
    impl TableEngine for MemTable {
        fn get_table_info(&self) -> &TableInfo {
            &self.info
        }
        fn insert(&mut self, records: Vec<Record>) -> SqlResult<()> {
            self.rows.extend(records);
            self.dirty.insert(self.info.name.clone());
            Ok(())
        }
        fn scan(&self) -> SqlResult<Vec<Record>> {
            Ok(self.rows.clone())
        }
        fn delete(&mut self, _filters: &[Value]) -> SqlResult<usize> {
            Ok(0)
        }
        fn update(&mut self, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> SqlResult<()> {
            Ok(())
        }
        fn dirty_tables(&self) -> &HashSet<String> {
            &self.dirty
        }
        fn mark_dirty(&mut self, table: &str) {
            self.dirty.insert(table.to_string());
        }
        fn table_name(&self) -> &str {
            &self.info.name
        }
    }

    fn sample_info(name: &str) -> TableInfo {
        TableInfo {
            name: name.to_string(),
            columns: vec![crate::engine::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),

            partition_info: None,
            compression: None,
        }
    }

    fn mem_table(name: &str) -> Box<MemTable> {
        Box::new(MemTable {
            info: sample_info(name),
            rows: vec![],
            dirty: HashSet::new(),
        })
    }

    #[test]
    fn default_and_register_get_list() {
        let mut reg: TableRegistry = Default::default();
        assert!(!reg.has_table("x"));
        reg.register("a".into(), mem_table("a"));
        reg.register("b".into(), mem_table("b"));
        assert!(reg.has_table("a"));
        let mut names = reg.list_tables();
        names.sort();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
        assert!(reg.get("a").is_some());
    }

    #[test]
    fn get_table_info_insert_scan() {
        let mut reg = TableRegistry::new();
        reg.register("t".into(), mem_table("t"));
        let info = reg.get_table_info("t").unwrap();
        assert_eq!(info.name, "t");
        reg.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let rows = reg.scan("t").unwrap();
        assert_eq!(rows, vec![vec![Value::Integer(1)]]);
    }

    #[test]
    fn missing_table_errors() {
        let reg = TableRegistry::new();
        assert!(reg.get_table_info("nope").is_err());
        assert!(reg.insert("nope", vec![]).is_err());
        assert!(reg.scan("nope").is_err());
        assert!(reg.delete("nope", &[]).is_err());
        assert!(reg.update("nope", &[], &[]).is_err());
        assert!(reg.flush_table("nope").is_err());
    }

    #[test]
    fn flush_table_calls_engine_flush() {
        let mut reg = TableRegistry::new();
        reg.register("t".into(), mem_table("t"));
        reg.flush_table("t").unwrap();
    }
}
