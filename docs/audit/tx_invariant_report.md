# TX Invariant Audit Report

**Version**: SQLRustGo v3.8.0 (branch develop/v3.8.0, commit f6cf8e16e)
**Audit Date**: 2026-05-31
**Auditor**: Hermes Agent (Invariant Auditor)

---

## TX Invariant Definition

```
require_active_transaction MUST cover ALL DML operations
```

This means all Data Manipulation Language operations (INSERT, UPDATE, DELETE) must check for an active transaction context before proceeding. Silent writes (writes without transaction context) are not allowed.

---

## ① DML Coverage Analysis

### INSERT Operation

**File**: src/execution_engine.rs:813-932

```rust
fn execute_insert(&self, insert: &InsertStatement) -> SqlResult<ExecutorResult> {
    // IMPL-001 & IMPL-004: TX lifecycle enforcement
    // IDLE/Active with no current_tx_id = implicit autocommit TX (allowed)
    // Committed/Aborted state = no new implicit TX (error)
    match self.tx_status {
        TxStatus::Committed => {
            return Err(SqlError::ExecutionError(
                "transaction already committed".to_string(),
            ));
        }
        TxStatus::Aborted => {
            return Err(SqlError::ExecutionError(
                "transaction already aborted".to_string(),
            ));
        }
        TxStatus::Idle | TxStatus::Active => {
            // Autocommit: allow DML without explicit BEGIN
            // New implicit TX started implicitly when current_tx_id is None
        }
    }
    // ... proceed with insert ...
}
```

**Analysis**: 
- TX Status check: ✅ YES (lines 817-832)
- However: `Idle | TxStatus::Active` allows implicit autocommit
- **NOT** requiring active transaction for INSERT

---

### UPDATE Operation

**File**: src/execution_engine.rs:957-1130

```rust
fn execute_update(&self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
    // IMPL-001 & IMPL-004: TX lifecycle enforcement
    match self.tx_status {
        TxStatus::Committed => {
            return Err(SqlError::ExecutionError(
                "transaction already committed".to_string(),
            ));
        }
        TxStatus::Aborted => {
            return Err(SqlError::ExecutionError(
                "transaction already aborted".to_string(),
            ));
        }
        TxStatus::Idle | TxStatus::Active => {
            // Autocommit: allow DML without explicit BEGIN
        }
    }
    // ... proceed with update ...
}
```

**Analysis**:
- TX Status check: ✅ YES (lines 959-972)
- However: `Idle | TxStatus::Active` allows implicit autocommit
- **NOT** requiring active transaction for UPDATE

---

### DELETE Operation

**File**: src/execution_engine.rs:1132-1229

```rust
fn execute_delete(&self, delete: &DeleteStatement) -> SqlResult<ExecutorResult> {
    // IMPL-001 & IMPL-004: TX lifecycle enforcement
    match self.tx_status {
        TxStatus::Committed => {
            return Err(SqlError::ExecutionError(
                "transaction already committed".to_string(),
            ));
        }
        TxStatus::Aborted => {
            return Err(SqlError::ExecutionError(
                "transaction already aborted".to_string(),
            ));
        }
        TxStatus::Idle | TxStatus::Active => {
            // Autocommit: allow DML without explicit BEGIN
        }
    }
    // ... proceed with delete ...
}
```

**Analysis**:
- TX Status check: ✅ YES (lines 1134-1148)
- However: `Idle | TxStatus::Active` allows implicit autocommit
- **NOT** requiring active transaction for DELETE

---

## ② Silent Write Findings

### Finding #1: Autocommit Mode Creates Implicit Transactions

**Evidence**: src/execution_engine.rs:828-831

```rust
TxStatus::Idle | TxStatus::Active => {
    // Autocommit: allow DML without explicit BEGIN
    // New implicit TX started implicitly when current_tx_id is None
}
```

When `tx_status` is `Idle` or `Active`, DML is allowed without explicit BEGIN. This is **implicit autocommit** behavior, not strict TX enforcement.

### Finding #2: execute_insert() Uses storage.insert() Without TX Context

**Evidence**: src/execution_engine.rs:891-916

```rust
{
    let mut storage = self.storage.write().unwrap();
    // ... constraint validation ...
    storage.insert(&table_name, processed_records)?;  // NO TX CONTEXT CHECK
}
```

The storage.insert() is called without checking if there's an active transaction in the ExecutionEngine. The tx_status check at line 817-832 only checks the ExecutionEngine state, not whether a proper transaction context exists in WalStorage.

### Finding #3: execute_update() DELETE/INSERT Pattern Bypasses TX Context

**Evidence**: src/execution_engine.rs:1082-1112

```rust
{
    let mut storage = self.storage.write().unwrap();
    storage.delete(&table_name, &[])?;  // No TX context required
    if !rows_to_keep.is_empty() {
        storage.insert(&table_name, rows_to_keep)?;  // No TX context required
    }
    if !trigger_modified_rows.is_empty() {
        storage.insert(&table_name, trigger_modified_rows)?;  // No TX context required
    }
}
```

These operations call storage methods directly without explicit transaction context verification. The WalStorage will log if `current_tx_id != 0`, but this is not enforced.

### Finding #4: execute_delete() DELETE/INSERT Pattern

**Evidence**: src/execution_engine.rs:1207-1212

```rust
{
    let mut storage = self.storage.write().unwrap();
    storage.delete(&table_name, &[])?;  // No TX context required
    if !rows_to_keep.is_empty() {
        storage.insert(&table_name, rows_to_keep)?;  // No TX context required
    }
}
```

Same pattern as UPDATE - bypasses strict TX requirement.

---

## ③ Transaction Lifecycle Analysis

### ExecutionEngine Transaction State Machine

**File**: src/execution_engine.rs:48-55

```rust
pub enum TxStatus {
    Idle,      // No transaction started
    Active,    // Transaction in progress
    Committed, // Transaction committed (terminal)
    Aborted,   // Transaction rolled back (terminal)
}
```

### Lifecycle Rules Observed

| Current State | Allowed Operations | Disallowed |
|---------------|-------------------|------------|
| Idle | BEGIN, DML (implicit TX) | COMMIT, ROLLBACK |
| Active | DML, COMMIT, ROLLBACK | BEGIN (nested) |
| Committed | BEGIN only | Any DML |
| Aborted | BEGIN only | Any DML |

### Issue: Committed/Aborted States After DML

**Evidence**: tests/wal_tx_contract_test.rs:64-87

```rust
/// TX-004: INSERT after COMMIT should fail (no active transaction)
#[test]
fn test_insert_after_commit_panics() {
    // ...
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'test')").unwrap();
    let commit_result = engine.execute("COMMIT");
    // ...
    let result = engine.execute("INSERT INTO t VALUES (2, 'after_commit')");
    if result.is_err() {
        // Expected after IMPL-004
    }
}
```

The test shows that INSERT after COMMIT currently succeeds (autocommit behavior), but the comment says "After IMPL-004 (strict TX lifecycle), this should return Err".

---

## ④ WalStorage TX Context Analysis

**File**: crates/storage/src/wal_storage.rs

### Transaction Methods in WalStorage

```rust
impl<S: StorageEngine> WalStorage<S> {
    pub fn begin_transaction(&mut self) -> SqlResult<u64> {
        if self.current_tx_id != 0 {
            return Err(/* nested TX error */);
        }
        let tx_id = self.generate_tx_id();
        if self.wal_enabled {
            self.wal.log_begin(tx_id)?;
        }
        self.current_tx_id = tx_id;
        Ok(tx_id)
    }

    pub fn commit_transaction(&mut self) -> SqlResult<()> {
        if self.current_tx_id == 0 {
            return Err(/* no TX error */);
        }
        // ...
        self.current_tx_id = 0;
        Ok(())
    }

    pub fn rollback_transaction(&mut self) -> SqlResult<()> {
        // ... similar to commit ...
    }

    pub fn in_transaction(&self) -> bool {
        self.current_tx_id != 0
    }

    pub fn current_tx_id(&self) -> u64 {
        self.current_tx_id
    }
}
```

### Key Finding: DML Operations Don't Check Transaction Context

**Evidence**: wal_storage.rs:289-296

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    let table_id = Self::table_name_to_id(table);
    for record in &records {
        let key = Self::record_key(record);
        let data = Self::record_to_bytes(record);
        self.log_insert(table_id, key, data)?;  // Only logs if tx_id != 0
    }
    self.inner.insert(table, records)  // Always writes to storage
}
```

The `insert()` method doesn't check if there's an active transaction before writing. It only logs to WAL if `current_tx_id != 0`. This means:
- Silent writes occur when no transaction is active
- WAL logging is optional (only if tx_id != 0)

---

## ⑤ AUTOCOMMIT Behavior Analysis

### Current Implementation

**File**: src/execution_engine.rs:817-832

The code explicitly implements AUTOCOMMIT behavior:
- DML in `Idle` or `Active` state succeeds without explicit BEGIN
- This means every DML implicitly starts a transaction if `current_tx_id` is None

### WalStorage Logging Behavior

**File**: wal_storage.rs:110-117

```rust
fn log_insert(&self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
    if !self.wal_enabled || self.current_tx_id == 0 {
        return Ok(());  // SILENTLY SKIPS - no error returned
    }
    self.wal.log_insert(self.current_tx_id, table_id, key, data)?;
    Ok(())
}
```

**Critical Issue**: When `current_tx_id == 0`, the logging is silently skipped without returning an error. This means:
1. DML without transaction context succeeds (autocommit)
2. WAL logging is skipped silently
3. No error indicates the silent write occurred

---

## ⑥ Test File Analysis

**File**: tests/wal_tx_contract_test.rs

### Tests Showing Current Behavior

| Test | Description | Current Result | Expected After IMPL-004 |
|------|-------------|-----------------|--------------------------|
| TX-001 | INSERT without TX | Succeeds (autocommit) | Should return Err |
| TX-002 | UPDATE without TX | Succeeds (autocommit) | Should return Err |
| TX-003 | DELETE without TX | Succeeds (autocommit) | Should return Err |
| TX-004 | INSERT after COMMIT | Succeeds (autocommit) | Should return Err |
| TX-005 | INSERT after ROLLBACK | Succeeds (autocommit) | Should return Err |
| TX-006 | Double COMMIT | Returns Err | Already correct |

---

## Verdict: FAIL

### Summary of Violations

| Violation | Severity | Description |
|-----------|----------|-------------|
| Autocommit implicit TX | CRITICAL | DML allowed without explicit BEGIN |
| Silent writes when no TX | CRITICAL | DML succeeds even when current_tx_id == 0 |
| WalStorage doesn't enforce TX | HIGH | insert/update/delete don't require active transaction |
| log_* methods silently skip | HIGH | No error when WAL logging is skipped |
| No require_active_transaction | CRITICAL | No such method found in codebase |

### Comparison to Invariant

**Invariant**: `require_active_transaction MUST cover ALL DML operations`

**Current Reality**: 
- No `require_active_transaction` method exists
- DML operations check `tx_status` but allow `Idle` state (autocommit)
- WalStorage DML methods don't require active transaction

### Required Fixes

1. **Add require_active_transaction method**: Create method in ExecutionEngine that returns Err if no active transaction
2. **Enforce for all DML**: Call `require_active_transaction()` at start of execute_insert/update/delete
3. **Remove autocommit behavior**: Change `Idle | Active` check to require explicit `Active` state
4. **Error on silent write**: WalStorage should return error when attempting write without TX context
5. **Document TX contract**: Clearly specify when transaction is required vs optional

---

## Files Requiring Changes

1. `src/execution_engine.rs` - Add require_active_transaction(), call in DML methods, remove autocommit
2. `crates/storage/src/wal_storage.rs` - Enforce TX context in insert/update/delete, error on silent write
3. `tests/wal_tx_contract_test.rs` - Update tests to expect Err for DML without TX
4. Documentation - Specify TX lifecycle contract clearly