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
use std::any::Any;

/// VTU Violation Guard Wrapper
/// Wraps a concrete StorageEngine to detect direct DML calls that bypass VTU.
/// Only use this at external call sites — NEVER wrap VTU internal storage.
pub struct VtuGuard<S> {
    inner: S,
    location: &'static str,
}

impl<S: 'static> VtuGuard<S> {
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

    /// Execute a DML operation through the wrapped storage.
    /// INT-4: this is the single chokepoint for VTU-bypass prevention.
    /// The closure receives `&mut S` so it cannot escape the guard.
    pub fn execute_dml<F, R>(&mut self, op: F) -> SqlResult<R>
    where
        F: FnOnce(&mut S) -> SqlResult<R>,
    {
        op(&mut self.inner)
    }

    /// Assert that the inner storage is currently in an open transaction.
    /// Panics with VTU VIOLATION if the DML is not wrapped in a transaction.
    pub fn assert_dml_safe(&self, op: &'static str, table: &str)
    where
        S: StorageEngine,
    {
        if !self.inner.in_transaction() {
            panic!(
                "🚨 VTU VIOLATION DETECTED\n   Location: {}\n   Operation: {}\n   Table: {}\n\n   ❌ DML called without an open transaction.\n   ✅ FIX: Wrap the call in begin_transaction() / commit_transaction() (or use a unified facade).\n\n   VTU requires ALL DML to run inside an active transaction.\n",
                self.location, op, table,
            );
        }
    }

    /// Static assertion marker: ALL DML paths must explicitly call this method.
    ///
    /// Used by the ARCH-3 (#3169) gate
    /// `scripts/gate/check_arch3_no_bypass.sh` to verify that
    /// `ExecutionEngine::execute_insert/update/delete` are the ONLY entry
    /// points for DML. The function is intentionally side-effect-free at
    /// runtime; it documents intent and provides a stable grep target.
    ///
    /// Reference: docs/openspec/3169-arch3-vtu-main-path.md
    pub fn assert_path_for_dml(op: &'static str, table: &str) {
        let _ = (op, table);
    }
}

impl<S: StorageEngine + 'static> StorageEngine for VtuGuard<S> {
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

    fn create_index(&mut self, info: crate::engine::IndexInfo) -> SqlResult<()> {
        self.inner.create_index(info)
    }

    fn drop_index(&mut self, table: &str, index_name: &str) -> SqlResult<()> {
        self.inner.drop_index(table, index_name)
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColumnDefinition;

    fn make_info(name: &str) -> TableInfo {
        TableInfo {
            name: name.to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,

            ..Default::default()
        }
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_vtu_violation_insert() {
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "test_location");
        guarded.insert("test_table", vec![]).unwrap();
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_vtu_violation_delete() {
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "test_location");
        guarded.delete("test_table", &[]).unwrap();
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_vtu_violation_update() {
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "test_location");
        guarded.update("test_table", &[], &[]).unwrap();
    }

    #[test]
    fn test_vtu_guard_scan_allowed() {
        let storage = crate::MemoryStorage::new();
        let guarded = VtuGuard::new(storage, "test_location");
        let result = guarded.scan("test_table");
        assert!(result.is_ok());
    }

    #[test]
    fn test_vtu_guard_get_table_info_allowed() {
        // Fix: original test was missing `#[test]` attribute.
        let mut storage = crate::MemoryStorage::new();
        storage.create_table(&make_info("test_table")).unwrap();
        let guarded = VtuGuard::new(storage, "test_location");
        let result = guarded.get_table_info("test_table");
        assert!(result.is_ok());
    }

    #[test]
    fn test_int4_execute_dml_routes_closure_to_inner() {
        let mut guarded = VtuGuard::new(crate::MemoryStorage::new(), "int4_execute_dml");
        let result: SqlResult<usize> = guarded.execute_dml(|inner| inner.delete("t", &[]));
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    #[should_panic(expected = "VTU VIOLATION")]
    fn test_int4_assert_dml_safe_panics_outside_tx() {
        let guarded = VtuGuard::new(crate::MemoryStorage::new(), "int4_assert");
        guarded.assert_dml_safe("insert", "t");
    }

    #[test]
    fn test_int4_assert_dml_safe_passes_inside_tx() {
        let inner = crate::MemoryStorage::new();
        let wal = crate::wal::MemoryWalManager::new();
        let mut storage = crate::WalStorage::new(inner, wal).unwrap();
        storage.set_current_tx_id(42);
        let guarded = VtuGuard::new(storage, "int4_assert_in_tx");
        guarded.assert_dml_safe("insert", "t");
    }

    /// #3129: ARCH-3 obstruction 1 — verify MemoryStorage::in_transaction()
    /// now reflects the current_tx_id field (was always false before fix).
    #[test]
    fn test_3129_memory_storage_in_transaction_reflects_tx_id() {
        let mut storage = crate::MemoryStorage::new();
        assert!(!storage.in_transaction(), "fresh storage must not be in TX");
        assert_eq!(storage.current_tx_id(), 0);

        storage.set_current_tx_id(7);
        assert!(
            storage.in_transaction(),
            "TX id 7 must mark storage as in-tx"
        );
        assert_eq!(storage.current_tx_id(), 7);

        storage.set_current_tx_id(0);
        assert!(
            !storage.in_transaction(),
            "TX id 0 must mark storage as idle"
        );
        assert_eq!(storage.current_tx_id(), 0);
    }

    /// #3129: ARCH-3 obstruction 2 — verify VtuGuard::assert_dml_safe
    /// passes when MemoryStorage is in a transaction.
    #[test]
    fn test_3129_vtu_guard_passes_in_tx_with_memory_storage() {
        let mut storage = crate::MemoryStorage::new();
        storage.set_current_tx_id(123);
        let guarded = VtuGuard::new(storage, "arch3_in_tx_memory");
        guarded.assert_dml_safe("insert", "users");
    }

    /// #3129: ARCH-3 — verify DML via VtuGuard::execute_dml succeeds
    /// when MemoryStorage is in an implicit TX.
    #[test]
    fn test_3129_vtu_guard_execute_dml_with_memory_storage_in_tx() {
        let mut storage = crate::MemoryStorage::new();
        storage.create_table(&make_info("t")).unwrap();
        storage.set_current_tx_id(99);
        let mut guarded = VtuGuard::new(storage, "arch3_execute_dml_memory");
        let result: SqlResult<usize> = guarded.execute_dml(|inner| inner.delete("t", &[]));
        assert_eq!(result.unwrap(), 0);
    }

    // === V311-14 coverage work: new tests for previously-uncovered delegation paths ===

    #[test]
    fn test_vtu_guard_inner_accessors() {
        // `inner()` returns &S, `inner_mut()` returns &mut S,
        // `into_inner()` consumes and returns S unchanged.
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "accessors_test");
        let _inner_ref: &crate::MemoryStorage = guarded.inner();
        {
            let _inner_mut: &mut crate::MemoryStorage = guarded.inner_mut();
        }
        let _recovered: crate::MemoryStorage = guarded.into_inner();
    }

    #[test]
    fn test_vtu_guard_create_drop_table_passthrough() {
        // create_table / drop_table / has_table / list_tables delegate.
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "ddl_test");
        guarded.create_table(&make_info("t")).unwrap();
        assert!(guarded.has_table("t"));
        let mut names = guarded.list_tables();
        names.sort();
        assert_eq!(names, vec!["t".to_string()]);
        guarded.drop_table("t").unwrap();
        assert!(!guarded.has_table("t"));
    }

    #[test]
    fn test_vtu_guard_trigger_methods_passthrough() {
        // create_trigger / drop_trigger / get_trigger / list_triggers
        // all delegate to inner.
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "meta_test");
        let trig = crate::engine::TriggerInfo {
            name: "tr".into(),
            table_name: "t".into(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: String::new(),
            update_columns: None,
            original_sql: String::new(),
        };
        guarded.create_trigger(trig).unwrap();
        assert!(guarded.get_trigger("tr").is_some());
        assert_eq!(guarded.list_triggers("t").len(), 1);
        guarded.drop_trigger("tr").unwrap();
        assert!(guarded.get_trigger("tr").is_none());
    }

    #[test]
    fn test_vtu_guard_tx_state_via_set_current_tx_id() {
        // Use set_current_tx_id (the canonical way to mark storage as
        #[test]
        fn test_vtu_guard_tx_state_via_set_current_tx_id() {
            // VtuGuard does not expose set_current_tx_id directly (the
            // delegation is via inner storage). Exercise the accessor
            // pattern: get inner, set tx, then confirm guarded.in_transaction()
            // delegates correctly.
            let storage = crate::MemoryStorage::new();
            let mut guarded = VtuGuard::new(storage, "tx_test");
            assert!(!guarded.in_transaction());
            guarded.inner_mut().set_current_tx_id(7);
            assert!(guarded.in_transaction());
            assert_eq!(guarded.current_tx_id(), 7);
            guarded.inner_mut().set_current_tx_id(0);
            assert!(!guarded.in_transaction());
            guarded.flush().unwrap();
        }
    }

    #[test]
    fn test_vtu_guard_add_column_rename_passthrough() {
        // add_column and rename_table are pure DDL passthroughs.
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "rename_test");
        guarded
            .create_table(&TableInfo {
                name: "old".into(),
                columns: vec![ColumnDefinition {
                    name: "x".into(),
                    data_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: false,
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

                ..Default::default()
            })
            .unwrap();
        guarded
            .add_column(
                "old",
                ColumnDefinition {
                    name: "y".into(),
                    data_type: "TEXT".into(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                },
            )
            .unwrap();
        guarded.rename_table("old", "new").unwrap();
        assert!(guarded.has_table("new"));
        assert!(!guarded.has_table("old"));
    }

    #[test]
    fn test_vtu_guard_assert_path_for_dml_is_idempotent() {
        // Static marker; no runtime side effect. Call twice.
        VtuGuard::<crate::MemoryStorage>::assert_path_for_dml("insert", "t");
        VtuGuard::<crate::MemoryStorage>::assert_path_for_dml("update", "u");
    }

    #[test]
    fn test_vtu_guard_dml_violation_panic_message_contains_table() {
        // The panic message must include the VTU VIOLATION text and
        // the offending table name so operators can grep for it.
        let storage = crate::MemoryStorage::new();
        let mut guarded = VtuGuard::new(storage, "msg_check");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = guarded.insert("specific_offending_table", vec![]);
        }));
        assert!(result.is_err());
        let panic_payload = result.unwrap_err();
        let msg = if let Some(s) = panic_payload.downcast_ref::<&'static str>() {
            s.to_string()
        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
            s.clone()
        } else {
            String::from("unknown panic payload")
        };
        assert!(msg.contains("VTU VIOLATION"), "missing marker: {}", msg);
        assert!(
            msg.contains("specific_offending_table"),
            "missing table name: {}",
            msg
        );
    }
}
