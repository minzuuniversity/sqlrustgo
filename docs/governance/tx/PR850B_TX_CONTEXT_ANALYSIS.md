# PR-850B-RFC: Tx Context Analysis & Route Selection

> **PR**: PR-850B  
> **Type**: Architecture Decision (RFC)  
> **Status**: Draft  
> **Author**: Hermes Agent  
> **Date**: 2026-05-31

---

## 1. Context

PR-850A removed transaction state ownership from `WalStorage`:
- Deleted `current_tx_id` field
- Deleted internal tx_id counter logic
- Delegated state queries to `inner` (StorageEngine)

After PR-850A, `tx_id = 0` is used as placeholder in WAL entries. PR-850B must resolve this.

---

## 2. Current Architecture (Post-850A)

```
TransactionManager (tx_id SOLE SOURCE)
       ↓ set_current_tx_id(tx_id)
ExecutionEngine
       ↓
StorageEngine (Context Carrier)
       ↓ current_tx_id() / in_transaction()
WalStorage (Observer - writes WAL only)
```

### 2.1 Key Observations

1. **TransactionManager is tx_id sole source**
   - `ExecutionEngine.begin_transaction()` calls `TM.begin()`
   - TM returns `tx_id` to ExecutionEngine

2. **StorageEngine is context carrier, not owner**
   - `set_current_tx_id()` is called by ExecutionEngine AFTER TM generates tx_id
   - StorageEngine stores it for propagation to WalStorage
   - Default implementation in trait is no-op

3. **WalStorage is pure observer**
   - No longer has internal tx state
   - Delegates state queries to inner
   - Only writes WAL entries

---

## 3. Call Statistics

### 3.1 current_tx_id() Read Points (10 locations)

| Location | Source | Purpose |
|----------|--------|---------|
| local_executor.rs:78,97 | tx_manager | Direct read, not storage |
| wal_storage.rs:219,409 | inner | Delegated to StorageEngine |
| wal_transactional_facade.rs:95 | tx_manager | Direct read |
| transactional_executor.rs | tx_manager | Direct read |
| manager.rs:315 | internal | Test assertions |

### 3.2 set_current_tx_id() Write Points (3 locations)

| Location | Purpose |
|----------|---------|
| engine.rs:550 | Trait default (no-op) |
| wal_storage.rs:412-413 | Delegated to inner |
| execution_engine.rs:1488 | **ONLY call site** - TM begin sync |

### 3.3 in_transaction() Call Points (42 locations)

| Owner | Count |
|-------|-------|
| TransactionManager / MVCC | Majority |
| local_executor.rs | Delegates to tx_manager |
| wal_storage.rs | Delegates to inner |

---

## 4. Route Analysis

### Route A: Keep StorageEngine as Context Carrier (Recommended)

**Description**: Maintain current architecture where StorageEngine carries tx context for propagation.

**Pros**:
- Minimal changes (only eliminate `tx_id = 0` placeholder)
- All 275+ tests pass
- tx_id flows: TM → EE → SE → WS
- RecoveryEngine can still use current_tx_id() from storage

**Cons**:
- tx_id propagation is indirect

**Implementation**:
```rust
// In WalStorage, instead of:
tx_id: 0  // placeholder

// Use:
tx_id: self.inner.current_tx_id()
```

### Route B: Remove StorageEngine Context Role

**Description**: Remove all `current_tx_id()` / `set_current_tx_id()` from StorageEngine. Pass tx_id explicitly in DML operations.

**Pros**:
- Cleaner separation of concerns
- StorageEngine truly dumb

**Cons**:
- Massive changes across 4+ layers
- Many StorageEngine implementations to update
- RecoveryEngine needs redesign
- High risk of regressions

**Implementation**:
```rust
// Change all DML signatures:
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>
// To:
fn insert(&mut self, tx_id: u64, table: &str, records: Vec<Record>) -> SqlResult<()>
```

---

## 5. Recommendation

### Route A is RECOMMENDED

**Rationale**:
1. StorageEngine.current_tx_id() is a legitimate context carrier pattern
2. Minimal risk to existing stable code
3. tx_id propagation chain is correct after PR-850A
4. RecoveryEngine can continue using storage.current_tx_id()

### Route A Changes for PR-850B

1. **WalStorage.log_insert/update/delete**: Use `self.inner.current_tx_id()` instead of `0`
2. **WalStorage.begin/commit/rollback**: Use proper tx_id from context
3. **Add assertion**: Verify tx_id != 0 before WAL operations

---

## 6. PR-850B Scope (Final)

### Must Do
- [ ] Replace `tx_id: 0` placeholder with context-aware tx_id
- [ ] Add invariant checks in WAL logging methods
- [ ] Update tests if assertions change

### Must NOT Do
- [ ] DO NOT remove StorageEngine.current_tx_id() from trait
- [ ] DO NOT change DML signatures
- [ ] DO NOT modify TransactionManager
- [ ] DO NOT touch RecoveryEngine (unless TX-001 tests fail)

---

## 7. Verification Gate

After PR-850B:
```
cargo test -p sqlrustgo-storage   → 275+ passed
cargo test -p sqlrustgo-executor  → all passed
cargo test --test wal_tx_contract → 21+ passed
cargo clippy -p sqlrustgo-storage -- -D warnings → 0 warnings
```

---

## 8. Next Steps

After PR-850B, the tx_id propagation will be:
```
TM.begin() → tx_id
    ↓
ExecutionEngine.set_current_tx_id(tx_id)
    ↓
StorageEngine.current_tx_id() ← carries tx_id
    ↓
WalStorage uses tx_id from context
    ↓
WAL entry { tx_id: <real value> }
```

This enables reliable RecoveryEngine replay with proper tx_id filtering.