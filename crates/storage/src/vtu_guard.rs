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
    // 事务状态必须向被包裹的存储透传。此前这些方法全部落到 trait 默认实现
    // （in_transaction=false / current_tx_id=0 / set=no-op），于是
    // `guarded.in_transaction()` 在 inner 已开事务时仍报 false。
    // 由 vtu_guard 的 tx-state 测试激活后实测暴露（该测试此前被嵌套结构
    // 吞掉、从未运行）。VtuGuard 的契约是"不干扰被包裹存储"，故一律透传。
    fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
    }

    fn current_tx_id(&self) -> u64 {
        self.inner.current_tx_id()
    }

    fn set_current_tx_id(&mut self, id: u64) {
        self.inner.set_current_tx_id(id)
    }

    fn set_current_tx_id_shared(&self, id: u64) {
        self.inner.set_current_tx_id_shared(id)
    }

    fn is_transaction_active(&self, tx_id: u64) -> bool {
        self.inner.is_transaction_active(tx_id)
    }

    // 事务生命周期同样必须透传：trait 默认实现是「不支持事务」错误，
    // 若不转发，任何经 VtuGuard 包装的存储都会在执行 BEGIN/COMMIT/ROLLBACK
    // 时直接失败。VtuGuard 是透明包装，唯一有权拦截的是 DML（VTU 违规检测）。
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        self.inner.begin_transaction()
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        self.inner.commit_transaction()
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        self.inner.rollback_transaction()
    }

    fn commit_transaction_for(&mut self, tx_id: u64) -> SqlResult<()> {
        self.inner.commit_transaction_for(tx_id)
    }

    fn rollback_transaction_for(&mut self, tx_id: u64) -> SqlResult<()> {
        self.inner.rollback_transaction_for(tx_id)
    }

    fn begin_transaction_lockfree(&self, tx_id: u64) -> SqlResult<()> {
        self.inner.begin_transaction_lockfree(tx_id)
    }

    fn commit_transaction_lockfree(&self) -> SqlResult<()> {
        self.inner.commit_transaction_lockfree()
    }

    fn rollback_transaction_lockfree(&self) -> SqlResult<()> {
        self.inner.rollback_transaction_lockfree()
    }

    fn commit_transaction_lockfree_for(&self, tx_id: u64) -> SqlResult<()> {
        self.inner.commit_transaction_lockfree_for(tx_id)
    }

    fn rollback_transaction_lockfree_for(&self, tx_id: u64) -> SqlResult<()> {
        self.inner.rollback_transaction_lockfree_for(tx_id)
    }

    /// #5277: forward `(db, table, reader_tx)`. Without this the type
    /// inherited the trait default, which dropped `db` and resolved the table
    /// against the storage's current database. The guard must not disturb the wrapped storage.
    fn scan_in_tx_db(&self, db: &str, table: &str, reader_tx: u64) -> SqlResult<Vec<Record>> {
        self.inner.scan_in_tx_db(db, table, reader_tx)
    }
    /// #5266 S1: reads must be asked of the stated database. The old trait
    /// default resolved them against the current one, so a read of `VtuGuard`'s
    /// inner `d1.t` could return `default.t`'s rows or report it missing.
    fn get_table_info_in(&self, db: &str, table: &str) -> SqlResult<TableInfo> {
        self.inner.get_table_info_in(db, table)
    }

    /// #5266 S1: see `get_table_info_in` — the database must be carried through.
    fn scan_in_db(&self, db: &str, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan_in_db(db, table)
    }
    /// #5250: the check must be asked of the stated database. The old trait
    /// default resolved this question against the current database, so a
    /// storage could correctly refuse a write and still answer `true` here.
    fn has_table_in(&self, db: &str, table: &str) -> bool {
        self.inner.has_table_in(db, table)
    }

    // -------------------------------------------------------------------------
    // Transaction state and control must be FORWARDED, not dropped.
    //
    // The `StorageEngine` trait gives every one of these a default that
    // reports "this engine has no transactions":
    //
    //   in_transaction()                  -> false
    //   current_tx_id()                   -> 0
    //   set_current_tx_id(id)             -> a no-op
    //   set_current_tx_id_shared(id)      -> a no-op
    //   begin_transaction()               -> Err("Transactions not supported
    //                                         by this storage engine")
    //
    // A wrapper that does not override them therefore does not merely return
    // stale values — it *silently removes the capability*. Ask a VtuGuard
    // whether a transaction is open and it says no; set a transaction id
    // through it and the id is dropped on the floor, so the wrapped storage
    // never learns which transaction it is in, and its own DML guard then
    // sees "no transaction" and rejects legitimate in-transaction writes.
    //
    // `VtuGuard`'s whole purpose is to be a TRANSPARENT wrapper around an
    // engine — it forwards reads and writes (see the delegations above) — so
    // the transaction family has to travel the same road. `WalStorage`
    // implements all of these soundly (its tx id lives in an `AtomicU64`);
    // the wrapper was simply not carrying them.
    //
    // Found by the GA gate run: `vtu_guard::tests::
    // test_vtu_guard_tx_state_via_set_current_tx_id` failed with
    // `assertion failed: guarded.in_transaction()` — the tx id was set on
    // the inner storage and the wrapper still reported "not in a
    // transaction", because the wrapper was answering from the trait's
    // default rather than from the engine it wraps.
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        self.inner.begin_transaction()
    }
    fn commit_transaction(&mut self) -> SqlResult<()> {
        self.inner.commit_transaction()
    }
    fn rollback_transaction(&mut self) -> SqlResult<()> {
        self.inner.rollback_transaction()
    }
    fn commit_transaction_for(&mut self, tx_id: u64) -> SqlResult<()> {
        self.inner.commit_transaction_for(tx_id)
    }
    fn rollback_transaction_for(&mut self, tx_id: u64) -> SqlResult<()> {
        self.inner.rollback_transaction_for(tx_id)
    }
    fn begin_transaction_for(&mut self, tx_id: u64) -> SqlResult<()> {
        self.inner.begin_transaction_for(tx_id)
    }
    fn begin_transaction_lockfree(&self, tx_id: u64) -> SqlResult<()> {
        self.inner.begin_transaction_lockfree(tx_id)
    }
    fn commit_transaction_lockfree(&self) -> SqlResult<()> {
        self.inner.commit_transaction_lockfree()
    }
    fn rollback_transaction_lockfree(&self) -> SqlResult<()> {
        self.inner.rollback_transaction_lockfree()
    }
    fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
    }
    fn current_tx_id(&self) -> u64 {
        self.inner.current_tx_id()
    }
    fn set_current_tx_id(&mut self, id: u64) {
        self.inner.set_current_tx_id(id)
    }
    fn set_current_tx_id_shared(&self, id: u64) {
        self.inner.set_current_tx_id_shared(id)
    }
    /// `flush` was a no-op on the wrapper for the same reason: the trait's
    /// default is `Ok(())`, so a flush asked of a VtuGuard never reached the
    /// engine it wraps.
    fn flush(&mut self) -> SqlResult<()> {
        self.inner.flush()
    }
    /// #5250: delegate with the stated database. This wrapper holds a
    /// single inner `StorageEngine`, so the database has to be carried
    /// through — the previous trait default dropped it (`let _ = db;`) and
    /// operated on whatever database was current.
    /// #5250: delegate with the stated database — the guard must not disturb the wrapped storage
    /// The old trait default did `let _ = db;` and operated on whatever
    /// database happened to be current, which silently created or dropped
    /// the wrong table.
    fn create_table_in_db(&mut self, db: &str, info: &TableInfo) -> SqlResult<()> {
        self.inner_mut().create_table_in_db(db, info)
    }

    /// #5250: see `create_table_in_db` — same contract, the database must be used.
    fn drop_table_in_db(&mut self, db: &str, table: &str) -> SqlResult<()> {
        self.inner_mut().drop_table_in_db(db, table)
    }

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

    /// #5266: the database must be carried through to the guarded engine.
    fn list_tables_in_db(&self, db: &str) -> Vec<String> {
        self.inner.list_tables_in_db(db)
    }

    /// #5266: delegate with the stated database.
    fn scan_with_filter_in_db(
        &self,
        db: &str,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
    ) -> SqlResult<Vec<Record>> {
        self.inner.scan_with_filter_in_db(db, table, filter)
    }

    /// #5266: delegate with the stated database.
    fn scan_with_index_in_db(
        &self,
        db: &str,
        table: &str,
        index_name: &str,
        key: &Value,
    ) -> SqlResult<Vec<Record>> {
        self.inner.scan_with_index_in_db(db, table, index_name, key)
    }

    /// #5266: delegate with the stated database.
    fn parallel_scan_in_db(
        &self,
        db: &str,
        table: &str,
        num_partitions: usize,
    ) -> SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>> {
        self.inner.parallel_scan_in_db(db, table, num_partitions)
    }

    /// #5266: delegate with the stated database.
    fn insert_in_db(&mut self, db: &str, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.inner_mut().insert_in_db(db, table, records)
    }

    /// #5266: delegate with the stated database.
    fn delete_in_db(&mut self, db: &str, table: &str, filters: &[Value]) -> SqlResult<usize> {
        self.inner_mut().delete_in_db(db, table, filters)
    }

    /// #5266: delegate with the stated database.
    fn delete_collect_pks_in_db(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
    ) -> SqlResult<Vec<Value>> {
        self.inner_mut()
            .delete_collect_pks_in_db(db, table, filters)
    }

    /// #5266: delegate with the stated database.
    fn delete_if_in_db(&mut self, db: &str, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        self.inner_mut().delete_if_in_db(db, table, filter)
    }

    /// #5266: delegate with the stated database.
    fn update_in_db(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        self.inner_mut().update_in_db(db, table, filters, updates)
    }

    /// #5266: delegate with the stated database.
    fn update_if_in_db(
        &mut self,
        db: &str,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        self.inner_mut()
            .update_if_in_db(db, table, filter, mutation)
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

    fn has_view(&self, db: &str, name: &str) -> bool {
        self.inner.has_view(db, name)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn gc(&self, _gc_lag: u64) -> usize {
        0
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

    // 修复 tip 存量的嵌套损坏：此处曾是「外层 fn 半截（注释截断在 'as'）＋
    // 函数体中间又插入一个完整 #[test] fn」的双层结构 ⇒ clippy
    // `cannot test inner items` + dead_code。合并为单个完整 test。
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

    /// 事务生命周期必须与直接调用 inner 等价：trait 默认实现是
    /// “Transactions not supported” 错误，不透传会让透明包装变成功能削减。
    #[test]
    fn test_vtu_guard_transaction_lifecycle_matches_inner() {
        let mut guarded = VtuGuard::new(crate::MemoryStorage::new(), "tx_lifecycle");
        let tx = guarded.begin_transaction().unwrap();
        assert!(guarded.in_transaction());
        assert_eq!(guarded.current_tx_id(), tx);
        guarded.rollback_transaction().unwrap();
        assert!(!guarded.in_transaction());

        let tx = guarded.begin_transaction().unwrap();
        guarded.commit_transaction().unwrap();
        assert!(!guarded.in_transaction());
        assert!(tx > 0);
    }

    #[test]
    fn test_vtu_guard_lockfree_tx_control_delegates() {
        // MemoryStorage / MvccStorage 本身不实现 lockfree 变体（trait 默认
        // 返回 unsupported）；WalStorage 有真实的 lockfree 事务控制可透传。
        let wal_storage = crate::WalStorage::new(
            crate::MemoryStorage::new(),
            crate::wal::MemoryWalManager::new(),
        )
        .unwrap();
        let guarded = VtuGuard::new(wal_storage, "tx_lockfree");
        guarded.begin_transaction_lockfree(41).unwrap();
        assert_eq!(guarded.current_tx_id(), 41);
        assert!(guarded.is_transaction_active(41));
        guarded.commit_transaction_lockfree_for(41).unwrap();
        assert!(!guarded.in_transaction());

        guarded.begin_transaction_lockfree(42).unwrap();
        guarded.rollback_transaction_lockfree_for(42).unwrap();
        assert!(!guarded.in_transaction());
    }

    /// 嵌套包装也必须逐层透传，不能在内层 Guard 处退化成默认实现。
    #[test]
    fn test_nested_vtu_guard_propagates_tx_state() {
        let inner = VtuGuard::new(crate::MemoryStorage::new(), "nested_inner");
        let mut outer = VtuGuard::new(inner, "nested_outer");
        outer.inner_mut().set_current_tx_id(77);
        assert!(outer.in_transaction());
        assert_eq!(outer.current_tx_id(), 77);
        outer.set_current_tx_id_shared(0);
        assert!(!outer.in_transaction());
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
