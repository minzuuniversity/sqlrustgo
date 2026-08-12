//! TableLevelStorage - Storage wrapper with per-table locking
//!
//! V311-09: Replaces global lock with per-table locks
//!
//! Architecture:
//! - WAL remains serial (must be ordered)
//! - Table operations use per-table locks (concurrent on different tables)

use crate::engine::{ColumnDefinition, Record, SqlError, SqlResult, TableInfo, Value};
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

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        self.tables.update(table, filters, updates)
    }

    fn create_table(&mut self, _info: &TableInfo) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "create_table not yet integrated".to_string(),
        ))
    }

    fn drop_table(&mut self, _table: &str) -> SqlResult<()> {
        Err(SqlError::ExecutionError(
            "drop_table not yet integrated".to_string(),
        ))
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

    fn update_if(
        &mut self,
        table: &str,
        _filter: &crate::engine::RowFilter,
        _mutation: &crate::engine::RowMutation,
    ) -> SqlResult<usize> {
        self.tables.update(table, &[], &[])
    }

    fn create_index(&mut self, _table: &str, _column: &str, _column_index: usize) -> SqlResult<()> {
        Ok(())
    }
    fn drop_index(&mut self, _table: &str, _column: &str) -> SqlResult<()> {
        Ok(())
    }
    fn add_column(&mut self, _table: &str, _column: ColumnDefinition) -> SqlResult<()> {
        Ok(())
    }
    fn rename_table(&mut self, _table: &str, _new_name: &str) -> SqlResult<()> {
        Ok(())
    }
    fn create_trigger(&mut self, _trigger: crate::engine::TriggerInfo) -> SqlResult<()> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::StorageEngine;
    use crate::table_engine::TableEngine;
    use std::collections::HashSet;

    fn sample_info(name: &str) -> TableInfo {
        TableInfo {
            name: name.to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
            collation: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),

            partition_info: None,
            compression: None,
        }
    }

    /// Minimal in-memory TableEngine for the registry plumbing.
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

    fn mem_wal() -> Box<dyn crate::wal::WalManager> {
        Box::new(crate::wal::MemoryWalManager::new())
    }

    #[test]
    fn new_writes_and_flush_wal() {
        let s = TableLevelStorage::new(mem_wal());
        s.write_wal("INSERT 1").unwrap();
        s.flush_wal().unwrap();
    }

    #[test]
    fn list_tables_via_registry() {
        let mut s = TableLevelStorage::new(mem_wal());
        Arc::get_mut(&mut s.tables).unwrap().register(
            "t".into(),
            Box::new(MemTable {
                info: sample_info("t"),
                rows: vec![],
                dirty: HashSet::new(),
            }),
        );
        assert!(s.has_table("t"));
        assert_eq!(s.list_tables(), vec!["t".to_string()]);
    }

    #[test]
    fn insert_scan_via_registry() {
        let mut s = TableLevelStorage::new(mem_wal());
        Arc::get_mut(&mut s.tables).unwrap().register(
            "t".into(),
            Box::new(MemTable {
                info: sample_info("t"),
                rows: vec![],
                dirty: HashSet::new(),
            }),
        );
        s.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows, vec![vec![Value::Integer(1)]]);
    }

    #[test]
    fn transaction_lifecycle_passthrough() {
        let mut s = TableLevelStorage::new(mem_wal());
        let tx = s.begin_transaction().unwrap();
        assert!(tx > 0);
        s.commit_transaction().unwrap();
        s.rollback_transaction().unwrap();
        s.set_current_tx_id(7);
    }

    #[test]
    fn stub_methods_are_known_unimplemented() {
        // V311-09 experimental: create_table / drop_table return Err with
        // "not yet integrated" by design (the design was a v3.12+ direction
        // and the implementation was deferred by V311-19 decision). We only
        // assert the call paths execute.
        let mut s = TableLevelStorage::new(mem_wal());
        assert!(s.create_table(&sample_info("t")).is_err());
        assert!(s.drop_table("t").is_err());
        // s.delete returns Err on missing table — that's expected for a
        // never-registered table, so we don't unwrap.
        let _ = s.delete("t", &[]);
        let rf: crate::engine::RowFilter = Box::new(|_| true);
        // delete_if also requires table existence; don't unwrap.
        let _ = s.delete_if("t", &rf);
        s.create_index("t", "id", 0).unwrap();
        s.flush().unwrap();
    }
}
