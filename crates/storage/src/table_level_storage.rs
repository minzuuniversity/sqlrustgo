//! TableLevelStorage - Storage wrapper with per-table locking
//!
//! V311-09: Replaces global lock with per-table locks
//!
//! Architecture:
//! - WAL remains serial (must be ordered)
//! - Table operations use per-table locks (concurrent on different tables)

use crate::engine::{Record, SqlResult, TableInfo, Value, SqlError, ColumnDefinition};
use crate::table_registry::TableRegistry;
use crate::wal::WalManager;
use std::sync::{Arc, RwLock};

/// Storage wrapper with per-table locking instead of global locking
pub struct TableLevelStorage {
    /// Per-table locks
    pub tables: Arc<TableRegistry>,
    /// WAL manager (serial access required)
    wal: Arc<RwLock<Box<dyn WalManager>>>,
}

impl TableLevelStorage {
    pub fn new(wal: Box<dyn WalManager>) -> Self {
        Self {
            tables: Arc::new(TableRegistry::new()),
            wal: Arc::new(RwLock::new(wal)),
        }
    }
    
    /// Write to WAL (serial operation)
    pub fn write_wal(&self, _sql: &str) -> SqlResult<()> {
        // WAL append is serial - simplified for now
        Ok(())
    }
    
    /// Flush WAL (batch sync)
    pub fn flush_wal(&self) -> SqlResult<()> {
        let mut wal = self.wal.write().unwrap();
        wal.flush()
            .map_err(|e| SqlError::ExecutionError(format!("WAL flush error: {}", e)))
    }
}

impl crate::engine::StorageEngine for TableLevelStorage {
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.tables.insert(table, records)
    }
    
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.tables.scan(table)
    }
    
    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        self.tables.delete(table, filters)
    }
    
    fn update(&mut self, table: &str, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize> {
        self.tables.update(table, filters, updates)
    }
    
    fn create_table(&mut self, _info: &TableInfo) -> SqlResult<()> {
        Err(SqlError::ExecutionError("create_table not yet integrated".to_string()))
    }
    
    fn drop_table(&mut self, _table: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError("drop_table not yet integrated".to_string()))
    }
    
    fn has_table(&self, table: &str) -> bool {
        self.tables.has_table(table)
    }
    
    fn list_tables(&self) -> Vec<String> {
        self.tables.list_tables()
    }
    
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.tables.get_table_info(table)
    }
    
    fn flush(&mut self) -> SqlResult<()> {
        for name in self.tables.list_tables() {
            self.tables.flush_table(&name)?;
        }
        Ok(())
    }
    
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        Ok(1)
    }
    
    fn commit_transaction(&mut self) -> SqlResult<()> {
        self.flush_wal()
    }
    
    fn rollback_transaction(&mut self) -> SqlResult<()> {
        Ok(())
    }
    
    fn in_transaction(&self) -> bool {
        false
    }
    
    fn set_current_tx_id(&mut self, _tx_id: u64) {}
    
    fn is_wal_enabled(&self) -> bool {
        true
    }
    
    fn delete_if(&mut self, table: &str, _filter: &crate::engine::RowFilter) -> SqlResult<usize> {
        self.tables.delete(table, &[])
    }
    
    fn update_if(&mut self, table: &str, _filter: &crate::engine::RowFilter, _mutation: &crate::engine::RowMutation) -> SqlResult<usize> {
        self.tables.update(table, &[], &[])
    }
    
    fn create_index(&mut self, _table: &str, _column: &str, _column_index: usize) -> SqlResult<()> { Ok(()) }
    fn drop_index(&mut self, _table: &str, _column: &str) -> SqlResult<()> { Ok(()) }
    fn add_column(&mut self, _table: &str, _column: ColumnDefinition) -> SqlResult<()> { Ok(()) }
    fn rename_table(&mut self, _table: &str, _new_name: &str) -> SqlResult<()> { Ok(()) }
    fn create_trigger(&mut self, _trigger: crate::engine::TriggerInfo) -> SqlResult<()> { Ok(()) }
    fn drop_trigger(&mut self, _name: &str) -> SqlResult<()> { Ok(()) }
    fn get_trigger(&self, _name: &str) -> Option<crate::engine::TriggerInfo> { None }
    fn list_triggers(&self, _table: &str) -> Vec<crate::engine::TriggerInfo> { vec![] }
    fn list_indexes(&self, _table: &str) -> Vec<(String, String)> { vec![] }
    fn has_view(&self, _name: &str) -> bool { false }
}
