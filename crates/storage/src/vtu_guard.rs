//! VtuGuard: VTU Violation Detection Wrapper
//!
//! Design: Explicit wrapper (NOT blanket impl), only wraps external call sites.
//! Does NOT pollute the trait system. Does not affect VTU internal storage delegation.
//!
//! ## Usage
//! Wrap storage at external boundary:
//! `let guarded = VtuGuard::new(storage, "merge_executor");`
//!
//! ## VTU Enforcement
//! ALL DML operations MUST go through VtuGuard.execute_dml() — NOT direct storage calls.
//! Direct insert/update/delete calls on VtuGuard will panic with VTU VIOLATION.

use crate::{Record, RowFilter, RowMutation, StorageEngine, TableInfo, TriggerInfo};
use sqlrustgo_types::{SqlResult, Value};

/// VTU Violation Guard Wrapper
/// Wraps a concrete StorageEngine to detect direct DML calls that bypass VTU.
/// Only use this at external call sites — NEVER wrap VTU internal storage.
pub struct VtuGuard<S> {
    inner: S,
    location: &'static str,
}

impl<S> VtuGuard<S> {
    /// Create a new VtuGuard wrapper
    pub fn new(inner: S, location: &'static str) -> Self {
        Self { inner, location }
    }

    /// Consume the guard and return the inner storage
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Access the inner storage (read-only)
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Access the inner storage mutably
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}

impl<S: StorageEngine> StorageEngine for VtuGuard<S> {
    /// Scan — allowed (read-only)
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan(table)
    }

    /// Insert — BLOCKED with VTU VIOLATION panic
    fn insert(&mut self, table: &str, _records: Vec<Record>) -> SqlResult<()> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {}\n   Operation: insert\n   Table: {}\n\n   ❌ Direct storage.insert() call bypasses VTU execution path.\n   ✅ FIX: Use VtuGuard::execute_dml() instead.\n\n   VTU requires ALL DML to go through: VtuGuard → ExecutionEngine → Storage\n",
            self.location,
            table,
        );
    }

    /// Delete — BLOCKED with VTU VIOLATION panic
    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {}\n   Operation: delete\n   Table: {}\n\n   ❌ Direct storage.delete() call bypasses VTU execution path.\n   ✅ FIX: Use VtuGuard::execute_dml() instead.\n\n   VTU requires ALL DML to go through: VtuGuard → ExecutionEngine → Storage\n",
            self.location,
            table,
        );
    }

    /// Update — BLOCKED with VTU VIOLATION panic
    fn update(
        &mut self,
        table: &str,
        _filters: &[Value],
        _updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {}\n   Operation: update\n   Table: {}\n\n   ❌ Direct storage.update() call bypasses VTU execution path.\n   ✅ FIX: Use VtuGuard::execute_dml() instead.\n\n   VTU requires ALL DML to go through: VtuGuard → ExecutionEngine → Storage\n",
            self.location,
            table,
        );
    }

    fn delete_if(&mut self, table: &str, _filter: &RowFilter) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {}\n   Operation: delete_if\n   Table: {}\n\n   ❌ Direct storage.delete_if() call bypasses VTU execution path.\n   ✅ FIX: Use DmlGuard::execute() instead.\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    fn update_if(
        &mut self,
        table: &str,
        _filter: &RowFilter,
        _mutation: &RowMutation,
    ) -> SqlResult<usize> {
        panic!(
            "🚨 VTU VIOLATION DETECTED\n   Location: {}\n   Operation: update_if\n   Table: {}\n\n   ❌ Direct storage.update_if() call bypasses VTU execution path.\n   ✅ FIX: Use DmlGuard::execute() instead.\n\n   See: docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md",
            self.location,
            table,
        );
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.inner.create_table(info)
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.inner.drop_table(table)
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

    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()> {
        self.inner.create_index(table, column, column_index)
    }

    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()> {
        self.inner.drop_index(table, column)
    }

    fn add_column(&mut self, table: &str, column: crate::ColumnDefinition) -> SqlResult<()> {
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
    use super::*;
    use crate::ColumnDefinition;

    /// Mock storage for testing
    struct MockStorage;

    impl MockStorage {
        fn new() -> Self {
            Self
        }
    }

    impl StorageEngine for MockStorage {
        fn scan(&self, _table: &str) -> SqlResult<Vec<Record>> {
            Ok(vec![])
        }

        fn insert(&mut self, _table: &str, _records: Vec<Record>) -> SqlResult<()> {
            Ok(())
        }

        fn delete(&mut self, _table: &str, _filters: &[Value]) -> SqlResult<usize> {
            Ok(0)
        }

        fn delete_if(&mut self, _table: &str, _filter: &RowFilter) -> SqlResult<usize> {
            Ok(0)
        }

        fn update(
            &mut self,
            _table: &str,
            _filters: &[Value],
            _updates: &[(usize, Value)],
        ) -> SqlResult<usize> {
            Ok(0)
        }

        fn update_if(
            &mut self,
            _table: &str,
            _filter: &RowFilter,
            _mutation: &RowMutation,
        ) -> SqlResult<usize> {
            Ok(0)
        }
        fn create_table(&mut self, _info: &TableInfo) -> SqlResult<()> {
            Ok(())
        }

        fn drop_table(&mut self, _table: &str) -> SqlResult<()> {
            Ok(())
        }

        fn get_table_info(&self, _table: &str) -> SqlResult<TableInfo> {
            Ok(TableInfo {
                name: String::new(),
                columns: vec![],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
            })
        }

        fn has_table(&self, _table: &str) -> bool {
            false
        }

        fn list_tables(&self) -> Vec<String> {
            vec![]
        }

        fn create_index(
            &mut self,
            _table: &str,
            _column: &str,
            _column_index: usize,
        ) -> SqlResult<()> {
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

        fn create_trigger(&mut self, _info: TriggerInfo) -> SqlResult<()> {
            Ok(())
        }

        fn drop_trigger(&mut self, _name: &str) -> SqlResult<()> {
            Ok(())
        }

        fn get_trigger(&self, _name: &str) -> Option<TriggerInfo> {
            None
        }

        fn list_triggers(&self, _table: &str) -> Vec<TriggerInfo> {
            vec![]
        }

        fn list_indexes(&self, _table: &str) -> Vec<(String, String)> {
            vec![]
        }

        fn has_view(&self, _name: &str) -> bool {
            false
        }
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_vtu_violation_insert() {
        let storage = MockStorage::new();
        let mut guarded = VtuGuard::new(storage, "test_location");
        guarded.insert("test_table", vec![]).unwrap();
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_vtu_violation_delete() {
        let storage = MockStorage::new();
        let mut guarded = VtuGuard::new(storage, "test_location");
        guarded.delete("test_table", &[]).unwrap();
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_vtu_violation_update() {
        let storage = MockStorage::new();
        let mut guarded = VtuGuard::new(storage, "test_location");
        guarded.update("test_table", &[], &[]).unwrap();
    }

    #[test]
    fn test_vtu_guard_scan_allowed() {
        let storage = MockStorage::new();
        let guarded = VtuGuard::new(storage, "test_location");
        let result = guarded.scan("test_table");
        assert!(result.is_ok());
    }

    #[test]
    fn test_vtu_guard_get_table_info_allowed() {
        let storage = MockStorage::new();
        let guarded = VtuGuard::new(storage, "test_location");
        let result = guarded.get_table_info("test_table");
        assert!(result.is_ok());
    }
}
