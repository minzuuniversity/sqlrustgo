# Capability Contract Audit Report

**Version**: SQLRustGo v3.8.0 (branch develop/v3.8.0, commit f6cf8e16e)
**Audit Date**: 2026-05-31
**Auditor**: Hermes Agent (Invariant Auditor)

---

## Capability Contract Definition

The capability contract defines how `is_wal_enabled` is enforced. The contract states:
- If WAL is enabled, all durability-dependent operations must use WAL
- If WAL is disabled, the system must correctly reject WAL-dependent operations
- There should be a public method `is_wal_enabled()` to query WAL status

---

## ① is_wal_enabled Enforcement Points

### Point #1: WalStorage Structure

**File**: crates/storage/src/wal_storage.rs:6-11

```rust
pub struct WalStorage<S: StorageEngine> {
    inner: S,
    wal: WalManager,
    current_tx_id: u64,
    wal_enabled: bool,  // Private field - NO public getter
}
```

**Issue**: `wal_enabled` is a private field with no public getter method `is_wal_enabled()`.

---

### Point #2: set_wal_enabled Method

**File**: wal_storage.rs:32-34

```rust
pub fn set_wal_enabled(&mut self, enabled: bool) {
    self.wal_enabled = enabled;
}
```

**Analysis**: Only setter exists, no getter. Cannot query WAL status.

---

### Point #3: new_without_wal Constructor

**File**: wal_storage.rs:23-29

```rust
pub fn new_without_wal(inner: S) -> Self {
    Self {
        inner,
        wal: WalManager::new(PathBuf::from("/dev/null")),
        current_tx_id: 0,
        wal_enabled: false,  // WAL explicitly disabled
    }
}
```

**Analysis**: Creates storage with WAL disabled. No method to later query or enable WAL.

---

### Point #4: Conditional WAL Logging

**File**: wal_storage.rs:110-134

```rust
fn log_insert(&self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
    if !self.wal_enabled || self.current_tx_id == 0 {
        return Ok(());  // Silently skips if WAL disabled
    }
    // ... logging ...
}

fn log_update(&self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
    if !self.wal_enabled || self.current_tx_id == 0 {
        return Ok(());
    }
    // ... logging ...
}

fn log_delete(&self, table_id: u64, key: Vec<u8>) -> SqlResult<()> {
    if !self.wal_enabled || self.current_tx_id == 0 {
        return Ok(());
    }
    // ... logging ...
}
```

**Analysis**: When `wal_enabled = false`, logging silently skips without error. No enforcement to ensure durability.

---

## ② Completeness Assessment

### Missing: is_wal_enabled() Public Method

**Search Result**: `is_wal_enabled` NOT FOUND in entire codebase

No implementation of `is_wal_enabled()` method exists anywhere. This makes it impossible to:
1. Query whether WAL is enabled for a given storage
2. Make decisions based on WAL capability
3. Validate capability contract compliance

---

### Missing: WAL Enforcement for Operations

**File**: crates/storage/src/wal_storage.rs:251-492

The `StorageEngine` trait implementation for `WalStorage` does NOT enforce WAL usage:

```rust
impl<S: StorageEngine> StorageEngine for WalStorage<S> {
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let table_id = Self::table_name_to_id(table);
        for record in &records {
            let key = Self::record_key(record);
            let data = Self::record_to_bytes(record);
            self.log_insert(table_id, key, data)?;  // Optional, not enforced
        }
        self.inner.insert(table, records)  // Always succeeds
    }
    // ... other methods similar ...
}
```

**Issue**: Operations always proceed to `inner.insert()` regardless of WAL status. WAL logging is optional.

---

### Missing: Capability-Based Operation Rejection

**Scenario**: If WAL is disabled, durability-dependent operations should be rejected or warned.

**Current Behavior**: Operations proceed silently without WAL, potentially leading to data loss on crash.

**Expected Behavior**: 
- Insert/Update/Delete should return error or warning if WAL is disabled
- Or at minimum, `is_wal_enabled()` should be queryable to allow application-level decisions

---

## ③ Usage of wal_enabled Field

### Search Results for wal_enabled access

| File | Line | Usage |
|------|------|-------|
| wal_storage.rs:44 | `if self.wal_enabled` | In begin_transaction |
| wal_storage.rs:59 | `if self.wal_enabled` | In commit_transaction |
| wal_storage.rs:75 | `if self.wal_enabled` | In rollback_transaction |
| wal_storage.rs:92 | `if !self.wal_enabled` | In recover |
| wal_storage.rs:111 | `if !self.wal_enabled` | In log_insert |
| wal_storage.rs:120 | `if !self.wal_enabled` | In log_update |
| wal_storage.rs:129 | `if !self.wal_enabled` | In log_delete |

**Analysis**: `wal_enabled` is only checked internally within WalStorage. No external query capability.

---

## ④ Related Structures

### UnifiedFacade in local_executor.rs

**File**: crates/executor/src/local_executor.rs:44-98

```rust
struct UnifiedFacade {
    storage: Arc<RwLock<WalStorage<'a>>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}
```

No capability query method. Cannot determine if WAL is enabled through this facade.

---

### TransactionContext

**File**: crates/executor/src/execution/transaction_context.rs:8-16

```rust
pub struct TransactionContext {
    pub tx_id: u64,
    pub wal_segment_open: bool,
    pub is_active: bool,
}
```

Has `wal_segment_open` field indicating WAL status for transaction, but this is per-transaction, not per-storage.

---

## ⑤ Evidence of Incomplete Enforcement

### Evidence #1: local_executor_dml.rs is PLACEHOLDER

**File**: crates/executor/src/local_executor_dml.rs

Entire module is placeholder with no implementation. Cannot verify WAL capability enforcement.

### Evidence #2: Tests Use MemoryStorage Without WAL

**File**: tests/wal_tx_contract_test.rs:10-12

```rust
fn create_engine() -> MemoryExecutionEngine {
    ExecutionEngine::with_memory()
}
```

MemoryStorage doesn't support WAL at all, but no capability check prevents this.

### Evidence #3: new_without_wal Used in Tests

**Search**: `new_without_wal` found in:
- wal_storage.rs:23 (constructor)
- transactional_executor.rs:93 (alternative constructor)

These allow creating storage with WAL disabled, but no enforcement of capability requirements.

---

## ⑥ Capability Contract Gaps

### Gap #1: No Public WAL Status Query

**Problem**: Cannot query whether WAL is enabled for a storage instance.

**Impact**: Applications cannot make informed decisions about durability.

**Fix Needed**: Add `is_wal_enabled(&self) -> bool` method to WalStorage.

---

### Gap #2: Silent WAL Bypass

**Problem**: When `wal_enabled = false`, all WAL logging silently skips without warning or error.

**Impact**: Data may be written without durability guarantee, but no indication to caller.

**Fix Needed**: Either:
1. Return error when writing without WAL enabled, OR
2. Provide query method so caller can validate before write

---

### Gap #3: No WAL Enforcement in DML Operations

**Problem**: StorageEngine trait doesn't enforce WAL usage.

**Impact**: Even when WAL is enabled in config, operations may not use it correctly.

**Fix Needed**: Ensure insert/update/delete enforce WAL logging before proceeding to inner storage.

---

### Gap #4: Inconsistent WAL Constructors

**Problem**: `new()` enables WAL by default, `new_without_wal()` disables it, but no way to enable/disable dynamically.

**Fix Needed**: Either:
1. Provide `set_wal_enabled()` that actually changes behavior, OR
2. Document that WAL cannot be dynamically enabled after creation

---

## ⑦ Recommendations

### Priority 1: Add is_wal_enabled() Method

```rust
impl<S: StorageEngine> WalStorage<S> {
    pub fn is_wal_enabled(&self) -> bool {
        self.wal_enabled
    }
}
```

### Priority 2: Enforce WAL in Critical Operations

Consider making WAL mandatory for durability-critical operations:

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    if !self.wal_enabled {
        return Err(SqlError::ExecutionError(
            "WAL must be enabled for INSERT operations".to_string(),
        ));
    }
    // ... proceed with logging ...
}
```

### Priority 3: Document Capability Contract

Add documentation specifying:
- When WAL is required vs optional
- What happens when WAL is disabled
- How to query WAL status

### Priority 4: Update Tests

Update `tests/wal_tx_contract_test.rs` to use WalStorage instead of MemoryStorage for WAL contract tests.

---

## Verdict: FAIL

### Summary of Gaps

| Gap | Severity | Description |
|-----|----------|-------------|
| No is_wal_enabled() method | CRITICAL | Cannot query WAL status |
| Silent WAL bypass | CRITICAL | No error when WAL logging skipped |
| No WAL enforcement in DML | HIGH | Operations proceed regardless of WAL |
| Inconsistent constructors | MEDIUM | Cannot dynamically enable/disable WAL |
| Tests use non-WAL storage | HIGH | WAL capability not tested properly |

### Required Fixes

1. **Add is_wal_enabled()**: Public method to query WAL status
2. **Error on WAL-disabled write**: DML operations should return error if WAL disabled
3. **Consistent capability model**: Single way to query and set WAL capability
4. **Update test infrastructure**: Use WalStorage in tests, not MemoryStorage

---

## Files Requiring Changes

1. `crates/storage/src/wal_storage.rs` - Add is_wal_enabled(), enforce WAL for writes
2. `crates/executor/src/local_executor.rs` - Expose WAL capability through UnifiedFacade
3. `tests/wal_tx_contract_test.rs` - Use WalStorage instead of MemoryStorage
4. Documentation - Specify capability contract for WAL