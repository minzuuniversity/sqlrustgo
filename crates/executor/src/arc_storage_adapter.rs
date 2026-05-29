//! ArcStorageAdapter - bridges Arc<RwLock<S>> to &dyn StorageEngine
//!
//! This adapter allows LocalExecutor (which uses &dyn StorageEngine)
//! to work with ExecutionEngine's Arc<RwLock<S>> storage.

use sqlrustgo_storage::{
    ColumnDefinition, Record, StorageEngine, TableInfo, TriggerInfo, UniqueConstraint,
};
use sqlrustgo_types::{SqlResult, Value};
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::RwLock;

/// Adapter that implements StorageEngine for Arc<RwLock<S>>
/// This allows passing Arc<RwLock<S>> as &dyn StorageEngine
pub struct ArcStorageAdapter<S: StorageEngine> {
    storage: Arc<RwLock<S>>,
}

impl<S: StorageEngine> ArcStorageAdapter<S> {
    pub fn new(storage: Arc<RwLock<S>>) -> Self {
        Self { storage }
    }

    /// Get the inner Arc<RwLock<S>>
    pub fn inner(&self) -> &Arc<RwLock<S>> {
        &self.storage
    }
}

impl<S: StorageEngine> StorageEngine for ArcStorageAdapter<S> {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        let storage = self.storage.read().unwrap();
        storage.scan(table)
    }

    fn get_records(&self, table: &str) -> SqlResult<Cow<'_, [Record]>> {
        let storage = self.storage.read().unwrap();
        let result = storage.get_records(table)?;
        // Convert to owned to avoid lifetime issues with the guard
        Ok(Cow::Owned(result.into_owned()))
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.insert(table, records)
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        let mut storage = self.storage.write().unwrap();
        storage.delete(table, filters)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let mut storage = self.storage.write().unwrap();
        storage.update(table, filters, updates)
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.create_table(info)
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.drop_table(table)
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        let storage = self.storage.read().unwrap();
        storage.get_table_info(table)
    }

    fn has_table(&self, table: &str) -> bool {
        let storage = self.storage.read().unwrap();
        storage.has_table(table)
    }

    fn list_tables(&self) -> Vec<String> {
        let storage = self.storage.read().unwrap();
        storage.list_tables()
    }

    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.create_index(table, column, column_index)
    }

    fn add_unique_constraint(
        &mut self,
        table: &str,
        constraint: UniqueConstraint,
    ) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.add_unique_constraint(table, constraint)
    }

    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.drop_index(table, column)
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.add_column(table, column)
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.rename_table(table, new_name)
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.create_trigger(info)
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.drop_trigger(name)
    }

    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        let storage = self.storage.read().unwrap();
        storage.get_trigger(name)
    }

    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        let storage = self.storage.read().unwrap();
        storage.list_triggers(table)
    }

    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        let storage = self.storage.read().unwrap();
        storage.list_indexes(table)
    }

    fn has_view(&self, name: &str) -> bool {
        let storage = self.storage.read().unwrap();
        storage.has_view(name)
    }

    fn delete_by_predicate(&mut self, table: &str, predicate: &str) -> SqlResult<usize> {
        let mut storage = self.storage.write().unwrap();
        storage.delete_by_predicate(table, predicate)
    }

    fn update_by_predicate(
        &mut self,
        table: &str,
        predicate: &str,
        assignments: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let mut storage = self.storage.write().unwrap();
        storage.update_by_predicate(table, predicate, assignments)
    }

    fn get_table_records_mut(&mut self, table: &str) -> SqlResult<&mut Vec<Record>> {
        let mut storage = self.storage.write().unwrap();
        // Get raw pointer to the records within the storage
        let records_ptr = {
            let records = storage.get_table_records_mut(table)? as *mut Vec<Record>;
            records
        };
        // Safety: The records are owned by S which is behind Arc<RwLock<S>>.
        // As long as &mut self is valid, the Arc and its data are valid.
        // The lock is held until storage goes out of scope at end of function.
        // We extend the lifetime to match &mut self.
        unsafe { Ok(&mut *records_ptr) }
    }

    fn delete_by_indices(&mut self, table: &str, indices: &[usize]) -> SqlResult<usize> {
        let mut storage = self.storage.write().unwrap();
        storage.delete_by_indices(table, indices)
    }

    fn find_by_index(&self, table: &str, column_index: usize, key: i64) -> Option<Vec<usize>> {
        let storage = self.storage.read().unwrap();
        storage.find_by_index(table, column_index, key)
    }

    fn get_row(&self, table: &str, index: usize) -> SqlResult<Option<Record>> {
        let storage = self.storage.read().unwrap();
        storage.get_row(table, index)
    }

    fn get_row_count(&self, table: &str) -> SqlResult<u64> {
        let storage = self.storage.read().unwrap();
        storage.get_row_count(table)
    }

    fn get_page_count(&self, table: &str) -> SqlResult<u64> {
        let storage = self.storage.read().unwrap();
        storage.get_page_count(table)
    }

    fn get_index_page_count(&self, table: &str, column_index: usize) -> SqlResult<u64> {
        let storage = self.storage.read().unwrap();
        storage.get_index_page_count(table, column_index)
    }

    fn get_next_auto_increment(&mut self, table: &str) -> SqlResult<i64> {
        let mut storage = self.storage.write().unwrap();
        storage.get_next_auto_increment(table)
    }

    fn get_and_increment_next_rowid(&mut self, table: &str) -> SqlResult<Option<u64>> {
        let mut storage = self.storage.write().unwrap();
        storage.get_and_increment_next_rowid(table)
    }
}
