//! VtuGuard: VTU Violation Detection Wrapper
//!
//! Design: Explicit wrapper (NOT blanket impl), only wraps external call sites.
//! Does NOT pollute the trait system. Does NOT affect VTU internal storage delegation.

use crate::{
    ColumnDefinition, Record, RowFilter, SqlResult, StorageEngine, TableInfo, TriggerInfo, Value,
};

/// VTU Violation Guard Wrapper
/// Wraps a concrete StorageEngine to detect direct DML calls that bypass VTU.
/// Only use this at external call sites - NEVER wrap VTU internal storage.
pub struct VtuGuard<S> {
    inner: S,
    location: &'static str,
}

impl<S> VtuGuard<S> {
    pub fn new(inner: S, location: &'static str) -> Self {
        Self { inner, location }
    }
    pub fn into_inner(self) -> S {
        self.inner
    }
    pub fn inner(&self) -> &S {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}

impl<S: StorageEngine> StorageEngine for VtuGuard<S> {
    fn insert(&mut self, table: &str, _records: Vec<Record>) -> SqlResult<()> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {} \n   Operation: insert\n   Table: {}\n\n   ❌ Direct storage.insert() call bypasses VTU execution path.\n   ✅ FIX: Replace with DmlGuard::execute()\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {} \n   Operation: delete\n   Table: {}\n\n   ❌ Direct storage.delete() call bypasses VTU execution path.\n   ✅ FIX: Replace with DmlGuard::execute()\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    fn delete_if(&mut self, table: &str, _filter: &RowFilter) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {} \n   Operation: delete_if\n   Table: {}\n\n   ❌ Direct storage.delete_if() call bypasses VTU execution path.\n   ✅ FIX: Replace with DmlGuard::execute()\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    fn update(
        &mut self,
        table: &str,
        _filters: &[Value],
        _updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {} \n   Operation: update\n   Table: {}\n\n   ❌ Direct storage.update() call bypasses VTU execution path.\n   ✅ FIX: Replace with DmlGuard::execute()\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    fn update_if(
        &mut self,
        table: &str,
        _filter: &RowFilter,
        _updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {} \n   Operation: update_if\n   Table: {}\n\n   ❌ Direct storage.update_if() call bypasses VTU execution path.\n   ✅ FIX: Replace with DmlGuard::execute()\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    // Read operations pass through safely
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan(table)
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.inner.get_table_info(table)
    }

    fn has_table(&self, table: &str) -> bool {
        self.inner.has_table(table)
    }

    fn list_tables(&self) -> Vec<String> {
        self.inner.list_tables()
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.inner.create_table(info)
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.inner.drop_table(table)
    }

    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()> {
        self.inner.create_index(table, column, column_index)
    }

    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()> {
        self.inner.drop_index(table, column)
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        self.inner.add_column(table, column)
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        self.inner.rename_table(table, new_name)
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        self.inner.create_trigger(info)
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.inner.drop_trigger(name)
    }

    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        self.inner.get_trigger(name)
    }

    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        self.inner.list_triggers(table)
    }

    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        self.inner.list_indexes(table)
    }

    fn has_view(&self, name: &str) -> bool {
        self.inner.has_view(name)
    }
}

#[cfg(test)]
mod tests {
    use crate::{MemoryStorage, Record, StorageEngine, VtuGuard};
    use sqlrustgo_types::Value;

    #[test]
    #[should_panic(expected = "VTU VIOLATION DETECTED")]
    fn vtu_violation_test_insert() {
        let inner = MemoryStorage::new();
        let mut guarded = VtuGuard::new(inner, "test_violation.rs:99");
        guarded
            .insert("users", vec![vec![Value::Integer(1)]])
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION DETECTED")]
    fn vtu_violation_test_delete() {
        let inner = MemoryStorage::new();
        let mut guarded = VtuGuard::new(inner, "test_violation.rs:110");
        guarded.delete("users", &[]).unwrap();
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION DETECTED")]
    fn vtu_violation_test_update() {
        let inner = MemoryStorage::new();
        let mut guarded = VtuGuard::new(inner, "test_violation.rs:121");
        guarded.update("users", &[], &[]).unwrap();
    }

    #[test]
    fn vtu_guard_read_passthrough() {
        let inner = MemoryStorage::new();
        let _guarded = VtuGuard::new(inner, "test_violation.rs:132");
    }
}
