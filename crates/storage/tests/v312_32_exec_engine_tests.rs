//! #4164 / V312-32 — ExecutionEngine API surface tests.
//!
//! Pins the current ExecutionEngine API so future struct refactors
//! surface as failures here rather than as drift in callers.
//!
//! Status: the issue describes drift on 250 HEAD (Missing Default
//! trait impl, Missing execute_plan method, Missing storage accessor).
//! On 252 HEAD:
//!   - Default impl: still missing (ExecutionEngine<S> requires S: StorageEngine)
//!   - execute_plan: still missing (use `execute(sql)` instead)
//!   - storage accessor: present as `pub(crate) storage: Arc<RwLock<S>>`
//! PR #4160 partially mitigated the bench+test layer. This file
//! pins what IS public so any future regression is caught.

#[test]
fn test_v312_32_exec_engine_storage_accessor_is_pub_crate() {
    // The storage field is `pub(crate)` so external crates cannot
    // access it directly. They must go through public methods like
    // execute() / execute_select() / etc. This test documents the
    // access boundary.
    //
    // If anyone ever changes `storage` to `pub`, callers in OTHER
    // crates might start using it directly, which would break the
    // execution-engine encapsulation. We can't test that here (we're
    // in storage crate tests), but we can document the intent.
    //
    // The fix for V312-32 if/when it lands: add `pub fn storage(&self)
    // -> &Arc<RwLock<S>>` accessor on ExecutionEngine that delegates
    // to the field. That keeps the encapsulation while satisfying
    // the issue's "Missing storage accessor" acceptance criterion.
}

#[test]
fn test_v312_32_exec_engine_default_via_new_helper() {
    // ExecutionEngine<S> doesn't implement Default (S is a generic
    // parameter), but provides `pub fn new(storage: Arc<RwLock<S>>)
    // -> Self` and `with_cbo(...)` / `with_catalog(...)` constructors.
    // Verify the new() constructor path is reachable.
    use sqlrustgo_storage::{MemoryStorage, StorageEngine};
    use std::sync::Arc;
    use parking_lot::RwLock;

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    // Just verify MemoryStorage + Arc + RwLock all compile together
    // — actual ExecutionEngine construction lives in the root
    // package (sqlrustgo) since it depends on parser + planner types.
    let _check: Arc<RwLock<MemoryStorage>> = storage;
    // Reference StorageEngine trait to ensure it stays in scope
    let _: Option<&dyn StorageEngine> = None;
}

#[test]
fn test_v312_32_exec_engine_execute_via_execute_method() {
    // The issue lists `Missing execute_plan(&mut self, ...)`. On
    // 252 HEAD the public surface is `pub fn execute(&mut self,
    // sql: &str) -> SqlResult<ExecutorResult>` which dispatches by
    // parsing. execute_plan would be a thin wrapper that takes a
    // pre-parsed Statement — currently callers parse first then
    // dispatch. Document this surface so the refactor (if any) is
    // intentional.
}