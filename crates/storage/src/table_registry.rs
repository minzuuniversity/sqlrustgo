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
    pub fn update(&self, name: &str, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize> {
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
