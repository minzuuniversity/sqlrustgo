# SPEC: INT-1 DML WAL + TransactionManager Integration

> **Issue**: #2966
> **Version**: v3.8.0
> **Branch**: `fix/int1-dml-wal-txid`
> **Date**: 2026-06-04
> **Author**: Hermes Agent (claimed #2966)
> **Status**: IN PROGRESS

## Problem Statement

INT-1 (DML Bypass WAL/TransactionManager) has TWO sub-problems:

### Sub-problem A: INSERT/UPDATE via SQL text bypass WAL

`LocalExecutor::execute_dml()` dispatches SQL text by prefix:
- DELETE → `execute_delete_sql()` → unified_facade ✅
- INSERT → returns error "not yet implemented" ❌
- UPDATE → only has DELETE code path ❌

Even if implemented, the `set_current_tx_id` must be called for WAL entries to carry real tx_id.

### Sub-problem B: `set_current_tx_id` never called

`WalStorage::current_tx_id` starts at 0 and is never updated.
Every WAL entry carries tx_id=0, making crash recovery unable to distinguish:
- Committed DML from unstarted DML
- Transaction boundaries

## Root Cause

`LocalExecutor::begin()` is not implemented (returns TODO error).
Therefore `set_current_tx_id` is never called.

## Fix Specification

### Fix 1: Implement `LocalExecutor::begin()` (tx_id + set_current_tx_id)

**File**: `crates/executor/src/local_executor.rs`

```rust
impl<'a> ExecutionEngine for LocalExecutor<'a> {
    fn begin(&mut self) -> Result<u64, SqlError> {
        let tx_id = self.unified_facade
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("WAL facade required".into()))?
            .begin()?;
        // Wire tx_id into WalStorage so every WAL entry carries real tx_id
        if let Some(ref mut facade) = self.unified_facade {
            facade.set_tx_id(tx_id);
        }
        Ok(tx_id)
    }
    fn commit(&mut self, txn: u64) -> Result<(), SqlError> {
        self.unified_facade.as_ref()
            .ok_or_else(|| SqlError::ExecutionError("WAL facade required".into()))?
            .commit()
    }
    fn rollback(&mut self, txn: u64) -> Result<(), SqlError> {
        self.unified_facade.as_ref()
            .ok_or_else(|| SqlError::ExecutionError("WAL facade required".into()))?
            .rollback()
    }
}
```

**File**: `crates/executor/src/execution/facade.rs` (or wherever UnifiedFacade lives)

Add to `UnifiedFacade`:
```rust
pub fn set_tx_id(&mut self, tx_id: u64) {
    if let Ok(mut storage) = self.storage.try_write() {
        storage.set_current_tx_id(tx_id);
    }
}
```

### Fix 2: Implement `execute_insert_sql`

**File**: `crates/executor/src/local_executor.rs`

```rust
if sql_upper.starts_with("INSERT") {
    return self.execute_insert_sql(ctx);
}
```

Add new method:
```rust
fn execute_insert_sql(&self, ctx: &crate::execution::QueryContext) -> Result<crate::execution::ExecutionResult, SqlError> {
    // Parse INSERT INTO t VALUES (...)
    // Extract table name and values
    // unified_facade.execute_dml(|storage| storage.insert(table, records))
    // Returns ExecutionResult with affected_rows
}
```

### Fix 3: Fix `execute_update_sql` — complete the UPDATE branch

Currently the UPDATE branch falls through to "UPDATE not yet implemented". Fix to properly:
```rust
if sql_upper.starts_with("UPDATE") {
    return self.execute_update_sql(ctx);
}
```

Add new method (reuse the existing `execute_update` PhysicalPlan logic via SQL parsing):
```rust
fn execute_update_sql(&self, ctx: &crate::execution::QueryContext) -> Result<...> {
    // Parse UPDATE t SET col=val WHERE ...
    // unified_facade.execute_dml(|storage| storage.update_if(...))
}
```

## Acceptance Criteria

1. **INSERT via SQL text** → calls unified_facade.insert() → WAL entry written with real tx_id
2. **UPDATE via SQL text** → calls unified_facade.update() → WAL entry written with real tx_id
3. **`set_current_tx_id`** → called in `begin()`, verified by checking WAL entries carry non-zero tx_id
4. **All three DML** → work via `ExecutionEngine.execute()` (the VTU path)
5. **`unified_facade = None`** → returns clear error (fail-fast, not silent bypass)
6. **Tests pass**: existing tests + new tests for INSERT/UPDATE SQL text paths

## Key Invariants

- NO direct `storage.insert/update/delete` calls outside unified_facade
- NO INSERT/UPDATE bypassing unified_facade
- WAL entries always carry tx_id > 0 for DML within transactions
- `begin()` returns a valid tx_id (not TODO)

## Files to Change

1. `crates/executor/src/local_executor.rs` — begin/commit/rollback, execute_insert_sql, execute_update_sql
2. `crates/executor/src/execution/facade.rs` — add set_tx_id method (or wherever UnifiedFacade lives)
3. `crates/executor/src/execution/engine.rs` — ExecutionEngine trait already has begin/commit/rollback
