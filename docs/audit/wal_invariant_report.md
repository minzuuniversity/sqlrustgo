# WAL Invariant Audit Report

**Version**: SQLRustGo v3.8.0 (branch develop/v3.8.0, commit f6cf8e16e)
**Audit Date**: 2026-05-31
**Auditor**: Hermes Agent (Invariant Auditor)

---

## WAL Invariant Definition

```
ALL write paths MUST satisfy: ExecutionEngine → WAL → Storage
```

This means every write operation (INSERT/UPDATE/DELETE/CREATE/DROP/ALTER) must flow through:
1. ExecutionEngine (top-level API)
2. WAL (Write-Ahead Log for durability)
3. Storage (actual data persistence)

---

## ① Write Path Inventory

### Primary Write Paths from ExecutionEngine (src/execution_engine.rs)

| Operation | Function | WAL Path | File:Line |
|-----------|----------|----------|-----------|
| INSERT | `execute_insert()` | Through `storage.insert()` which goes through WalStorage | src/execution_engine.rs:915 |
| UPDATE | `execute_update()` | Through `storage.update()` which goes through WalStorage | src/execution_engine.rs:978-1106 |
| DELETE | `execute_delete()` | Through `storage.delete()` which goes through WalStorage | src/execution_engine.rs:1154-1212 |
| CREATE TABLE | `execute_create_table()` | Direct `storage.create_table()` - NO WAL | src/execution_engine.rs:1231-1252 |
| DROP TABLE | `execute_drop_table()` | Direct `storage.drop_table()` - NO WAL | src/execution_engine.rs:1255-1258 |
| TRUNCATE | `execute_truncate()` | Direct `storage.delete()` - NO WAL | src/execution_engine.rs:1261-1273 |
| CREATE INDEX | `execute_create_index()` | Direct `storage.create_index()` - NO WAL | src/execution_engine.rs:1276-1290 |

### WalStorage Write Path (crates/storage/src/wal_storage.rs)

| Operation | Function | WAL Check | File:Line |
|-----------|----------|-----------|-----------|
| INSERT | `insert()` | `log_insert()` called before `inner.insert()` | wal_storage.rs:289-296 |
| UPDATE | `update()` | `log_update()` called before `inner.update()` | wal_storage.rs:317-328 |
| DELETE | `delete()` | `log_delete()` called before `inner.delete()` | wal_storage.rs:303-308 |
| DELETE_IF | `delete_if()` | `log_delete()` called before `inner.delete_if()` | wal_storage.rs:310-314 |
| UPDATE_IF | `update_if()` | `log_update()` called before `inner.update_if()` | wal_storage.rs:330-341 |

### Logging Functions in WalStorage

- `log_insert()` (line 110-117): Logs if `wal_enabled && current_tx_id != 0`
- `log_update()` (line 119-126): Logs if `wal_enabled && current_tx_id != 0`
- `log_delete()` (line 128-134): Logs if `wal_enabled && current_tx_id != 0`

---

## ② Bypass Findings

### CRITICAL BYPASS #1: DDL Operations Bypass WAL

**Evidence**: `execute_create_table()` at src/execution_engine.rs:1231 calls `storage.create_table()` directly without any WAL logging.

```rust
fn execute_create_table(&self, create: &CreateTableStatement) -> SqlResult<ExecutorResult> {
    let mut storage = self.storage.write().unwrap();
    // ... column building ...
    storage.create_table(&info)?;  // NO WAL LOGGING
    Ok(ExecutorResult::empty())
}
```

**Same for**:
- `execute_drop_table()` (line 1255) - NO WAL
- `execute_truncate()` (line 1261) - NO WAL (calls `storage.delete()` without logging)
- `execute_create_index()` (line 1276) - NO WAL

### CRITICAL BYPASS #2: execute_update() DELETE/REINSERT Pattern

**Evidence**: src/execution_engine.rs:1082-1112

The `execute_update()` function does NOT use `storage.update()`. Instead it:
1. Calls `storage.delete(&table_name, &[])?;` - NO WAL logged
2. Calls `storage.insert(&table_name, rows_to_keep)?;` - NO WAL logged
3. Calls `storage.insert(&table_name, trigger_modified_rows)?;` - NO WAL logged

This bypasses the WAL logging in `WalStorage::update()` because it uses delete+insert instead.

### CRITICAL BYPASS #3: execute_delete() DELETE/REINSERT Pattern

**Evidence**: src/execution_engine.rs:1207-1212

Similarly, `execute_delete()` uses:
1. `storage.delete(&table_name, &[])?;` - NO WAL logged
2. `storage.insert(&table_name, rows_to_keep)?;` - NO WAL logged

### BYPASS #4: MemoryStorage Does Not Support WAL

**Evidence**: crates/storage/src/engine.rs:519-700

`MemoryStorage` is a simple HashMap-based storage that does NOT implement WAL logging. Any write to MemoryStorage is non-durable.

**Type alias at line 80**: `pub type MemoryExecutionEngine = ExecutionEngine<MemoryStorage>;`

---

## ③ MemoryStorage Findings

### Finding #1: MemoryExecutionEngine Used in Tests

**Evidence**: tests/wal_tx_contract_test.rs:10-12

```rust
fn create_engine() -> MemoryExecutionEngine {
    ExecutionEngine::with_memory()
}
```

Uses `MemoryStorage` which has NO WAL.

### Finding #2: local_executor_dml.rs is a PLACEHOLDER

**Evidence**: crates/executor/src/local_executor_dml.rs (entire file)

```rust
/// Placeholder LocalExecutorDml
pub struct LocalExecutorDml;

/// Placeholder LocalExecutorDmlArc
pub struct LocalExecutorDmlArc;
```

Only has `new()` and `default()` - no actual implementation.

### Finding #3: No is_wal_enabled() Method Found

**Search Result**: `is_wal_enabled` NOT FOUND in codebase

There is no `is_wal_enabled()` method on `WalStorage` or any storage type. Only `wal_enabled` field exists and is set via `set_wal_enabled()`.

---

## ④ New Entry Points

### Entry Point #1: local_executor.rs UnifiedFacade

**File**: crates/executor/src/local_executor.rs:44-98

```rust
struct UnifiedFacade {
    storage: Arc<RwLock<WalStorage<'a>>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}
```

Has methods `begin()`, `commit()`, `rollback()` but these are internal.

### Entry Point #2: execution/facade.rs

**File**: crates/executor/src/execution/facade.rs

Simple facade pattern - no WAL integration visible.

### Entry Point #3: execution/engine.rs

**File**: crates/executor/src/execution/engine.rs:5-11

```rust
pub trait ExecutionEngine {
    fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError>;
    fn begin(&mut self) -> Result<u64, SqlError>;
    fn commit(&mut self, txn: u64) -> Result<(), SqlError>;
    fn rollback(&mut self, txn: u64) -> Result<(), SqlError>;
}
```

Trait-based execution engine - WAL integration depends on implementation.

---

## ⑤ WalStorage Analysis

### Critical Issue: Conditional WAL Logging

**Evidence**: wal_storage.rs:110-134

```rust
fn log_insert(&self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
    if !self.wal_enabled || self.current_tx_id == 0 {
        return Ok(());  // SILENTLY SKIPS LOGGING
    }
    // ... logging code ...
}
```

**Problem**: If `wal_enabled = false` OR `current_tx_id == 0`, the WAL log entry is silently skipped. This means:
- When WAL is disabled, writes bypass WAL
- When no transaction is active, writes bypass WAL

### Critical Issue: new_without_wal() Constructor

**Evidence**: wal_storage.rs:23-29

```rust
pub fn new_without_wal(inner: S) -> Self {
    Self {
        inner,
        wal: WalManager::new(PathBuf::from("/dev/null")),
        current_tx_id: 0,
        wal_enabled: false,  // WAL DISABLED
    }
}
```

Creates WalStorage with `wal_enabled = false`, meaning ALL writes skip WAL logging.

---

## Verdict: FAIL

### Summary of Violations

| Violation | Severity | Description |
|-----------|----------|-------------|
| DDL bypass WAL | CRITICAL | CREATE/DROP/ALTER/TRUNCATE don't log to WAL |
| UPDATE bypass WAL | CRITICAL | execute_update() uses delete+insert pattern, bypassing WAL |
| DELETE bypass WAL | CRITICAL | execute_delete() uses delete+insert pattern, bypassing WAL |
| MemoryStorage non-durable | HIGH | MemoryExecutionEngine has no WAL persistence |
| Conditional WAL skip | HIGH | log_insert/update/delete silently skip when wal_enabled=false |
| new_without_wal bypass | HIGH | WalStorage::new_without_wal() disables WAL completely |

### Required Fixes

1. **DDL WAL Logging**: Add WAL logging for CREATE/DROP/ALTER/TRUNCATE operations
2. **Update/Delete Pattern**: Change execute_update() and execute_delete() to use storage.update()/storage.delete() directly, not delete+insert
3. **MemoryStorage Warning**: Document that MemoryExecutionEngine is non-durable
4. **Conditional WAL Enforcement**: Consider enforcing WAL requirement for durability-critical operations
5. **is_wal_enabled Method**: Add public method to query WAL status for capability contract enforcement

---

## Files Requiring Changes

1. `src/execution_engine.rs` - Fix DDL WAL logging, fix UPDATE/DELETE pattern
2. `crates/storage/src/wal_storage.rs` - Add is_wal_enabled() method, enforce WAL for writes
3. `crates/storage/src/engine.rs` - Document MemoryStorage limitations
4. `tests/wal_tx_contract_test.rs` - Use WalStorage instead of MemoryStorage for WAL tests