# v3.8.0 Development Plan

## Status: DRAFT

> Baseline: `origin/develop/v3.7.0` (commit 72223a80)
> Architecture: see `ARCHITECTURE.md`

---

## Phase Overview

| Phase | Name | Goal | Gate |
|-------|------|------|------|
| Phase 1 | **TransactionManager Integration** | Connect TransactionManager to dispatch layer | G1: BEGIN/COMMIT/ROLLBACK → txn_manager |
| Phase 2 | **WriteBuffer for DML** | Stage INSERT/UPDATE/DELETE in write_buffer | G2: DML → write_buffer staging |
| Phase 3 | **Commit Engine** | Flush write_buffer → StorageEngine on commit | G3: commit → storage flush |
| Phase 4 | **Rollback Engine** | Discard write_buffer on rollback | G4: rollback → discard |
| Phase 5 | **Read Consistency Prep** | Transaction snapshot + read-your-writes | G5: snapshot isolation ready |

**Phase 1 is prerequisite for all other phases.**

---

## Phase 1: TransactionManager Integration

### Goal

Connect `TransactionManager` to the SQL statement dispatch path in `mysql-server`. LocalExecutor remains unchanged.

### Current State (v3.7.0)

```
COM_QUERY
  └── eng.execute(&q)   ← LocalExecutor, no transaction awareness
```

### Target State (Phase 1)

```
COM_QUERY
  │
  ├── BEGIN  ──→ txn_manager.begin()
  ├── COMMIT ──→ txn_manager.commit()
  ├── ROLLBACK ──→ txn_manager.rollback()
  │
  └── Other ──→ LocalExecutor.execute()
```

### Implementation Steps

**Step 1**: Add `TransactionManager` to `mysql-server` session state

```rust
// crates/mysql-server/src/server/session.rs (new or existing)
pub struct Session {
    txn_manager: TransactionManager,
    executor: LocalExecutor<'a>,
}
```

**Step 2**: Parse `BEGIN`/`COMMIT`/`ROLLBACK` statements in dispatch

```rust
// crates/mysql-server/src/lib.rs around line 1081
match statement_type {
    StatementType::Begin => session.txn_manager.begin(),
    StatementType::Commit => session.txn_manager.commit(),
    StatementType::Rollback => session.txn_manager.rollback(),
    _ => session.executor.execute(&sql),
}
```

**Step 3**: Propagate session through `execute()` calls

```rust
// Change signature: execute(sql, session: &Session)
// Instead of: execute(sql)
```

### Files to Modify

| File | Change |
|------|--------|
| `crates/mysql-server/src/lib.rs` | Add BEGIN/COMMIT/ROLLBACK dispatch |
| `crates/mysql-server/src/server/session.rs` | Add `txn_manager: TransactionManager` field |
| `crates/transaction/src/manager.rs` | Ensure `begin()`/`commit()`/`rollback()` are stable |
| `crates/executor/src/lib.rs` | May need `execute(sql, &Session)` signature update |

### Gate G1 Criteria

```
✅ BEGIN → txn_manager.begin() called
✅ COMMIT → txn_manager.commit() called
✅ ROLLBACK → txn_manager.rollback() called
✅ LocalExecutor struct unchanged (no txn_manager field)
✅ 250 tests PASS (no regression)
```

### Risks

- **Risk**: `mysql-server/src/lib.rs` is large (2000+ lines)
  - **Mitigation**: Isolate changes to a single dispatch block (~line 1081)
- **Risk**: `execute()` signature change cascades to all callers
  - **Mitigation**: Add `&Session` param only where needed; default to None for backward compat

---

## Phase 2: WriteBuffer for DML

### Goal

Stage `INSERT`/`UPDATE`/`DELETE` operations in `TransactionManager.write_buffer` before commit. StorageEngine is not modified.

### Design

```rust
// Inside TransactionManager
pub struct TransactionManager {
    active_txn: Option<Transaction>,
    write_buffer: WriteBuffer,   // ← new
    snapshot: Option<SnapshotMeta>,
}

pub struct WriteBuffer {
    staging: Vec<WriteOp>,
    max_size: usize,
}

pub enum WriteOp {
    Insert { table_id: u32, row: Row },
    Update { table_id: u32, key: Vec<Value>, new_row: Row },
    Delete { table_id: u32, key: Vec<Value> },
}
```

### Implementation Steps

**Step 1**: Define `WriteOp` enum and `WriteBuffer` struct in `crates/transaction/src/`

**Step 2**: Add `write_buffer: WriteBuffer` to `TransactionManager`

**Step 3**: Add `stage_write(&mut self, op: WriteOp)` to `TransactionManager`

**Step 4**: Route DML through `txn_manager.stage_write()` at dispatch layer

```rust
match stmt {
    Statement::Insert { .. } => session.txn_manager.stage_write(op),
    Statement::Update { .. } => session.txn_manager.stage_write(op),
    Statement::Delete { .. } => session.txn_manager.stage_write(op),
    _ => session.executor.execute(&sql),
}
```

**Step 5**: Do NOT flush to storage in this phase — only stage

### Gate G2 Criteria

```
✅ INSERT stages to write_buffer (not yet to storage)
✅ UPDATE stages to write_buffer (not yet to storage)
✅ DELETE stages to write_buffer (not yet to storage)
✅ StorageEngine NOT called during DML staging
✅ write_buffer.visible() returns staged ops
```

---

## Phase 3: Commit Engine

### Goal

Flush `write_buffer` to `StorageEngine` when `COMMIT` is issued. This connects the buffered writes to the physical storage layer.

### Implementation Steps

**Step 1**: Add `flush_to_storage(&mut self, storage: &dyn StorageEngine)` to `TransactionManager`

**Step 2**: Call `flush_to_storage()` at the start of `commit()`

```rust
pub fn commit(&mut self, storage: &dyn StorageEngine) -> Result<(), TxError> {
    // 1. Validate transaction (SSI check)
    // 2. Flush write_buffer to storage
    self.write_buffer.flush_to(storage);
    // 3. Transition to COMMITTED
    self.active_txn = None;
    Ok(())
}
```

**Step 3**: StorageEngine receives physical writes (no transaction awareness)

```rust
// StorageEngine trait
fn write_pages(&self, pages: Vec<Page>) -> Result<()>;
fn delete_pages(&self, keys: Vec<Vec<Value>>) -> Result<()>;
```

### Gate G3 Criteria

```
✅ COMMIT → write_buffer flushed → storage
✅ storage receives physical writes (page-level)
✅ COMMITTED state transitions correctly
✅ concurrent COMMIT while ACTIVE → SSI abort or succeed
```

---

## Phase 4: Rollback Engine

### Goal

Discard `write_buffer` when `ROLLBACK` is issued. No physical storage changes.

### Implementation Steps

**Step 1**: Add `discard(&mut self)` to `WriteBuffer`

```rust
impl WriteBuffer {
    pub fn discard(&mut self) {
        self.staging.clear();
    }
}
```

**Step 2**: Call `discard()` in `rollback()`

```rust
pub fn rollback(&mut self) {
    self.write_buffer.discard();
    self.active_txn = None;
}
```

### Gate G4 Criteria

```
✅ ROLLBACK → write_buffer discarded
✅ storage NOT modified on rollback
✅ ABORTED state transitions correctly
```

---

## Phase 5: Read Consistency Prep

### Goal

Provide transaction snapshot and enforce read-your-writes semantics. This is a prerequisite for v3.9 MVCC.

### Design

```rust
pub struct SnapshotMeta {
    tx_id: TransactionId,
    committed_ts: u64,
    read_view: Vec<TransactionId>,
}

pub struct TransactionManager {
    // ... existing fields ...
    snapshot: Option<SnapshotMeta>,
}

impl TransactionManager {
    pub fn begin(&mut self) -> SnapshotMeta {
        let snapshot = SnapshotMeta::new(self.next_tx_id());
        self.snapshot = Some(snapshot.clone());
        snapshot
    }

    // Read-your-writes: return only changes from this transaction + committed before snapshot
    pub fn read(&self, key: &Key) -> Option<Value> {
        // check write_buffer first (uncommitted changes visible to owner)
        // then check snapshot for committed values
    }
}
```

### Gate G5 Criteria

```
✅ BEGIN → snapshot created
✅ Own writes in write_buffer visible within transaction
✅ Reads from committed state only (read-your-writes)
✅ v3.9 MVCC implementation is straightforward extension
```

---

## Testing Strategy

### Phase 1 Tests

```
test_begin_commit_rollback
test_concurrent_begin
test_nested_transaction_rejected
test_executor_no_txn_field (struct invariant check)
```

### Phase 2 Tests

```
test_insert_stages_to_buffer
test_update_stages_to_buffer
test_delete_stages_to_buffer
test_buffer_size_limit
```

### Phase 3 Tests

```
test_commit_flushes_to_storage
test_commit_resets_buffer
test_concurrent_commit SSI
```

### Phase 4 Tests

```
test_rollback_discards_buffer
test_rollback_no_storage_change
test_rollback_unknown_txn
```

### Phase 5 Tests

```
test_snapshot_creation
test_read_own_writes
test_read_committed_only
```

---

## Out of Scope (v3.8)

- MVCC full implementation (v3.9)
- Two-phase commit (v3.9+)
- Savepoints
- XA transactions
- Distributed transactions

---

## Dependencies

```
Phase 1 (G1): TransactionManager exists → ✓ (crates/transaction)
Phase 2 (G2): Phase 1 complete
Phase 3 (G3): Phase 2 complete
Phase 4 (G4): Phase 2 complete
Phase 5 (G5): Phase 1 + Phase 2 + Phase 3 + Phase 4 complete
```

---

## Changelog

| Date | Change |
|------|--------|
| 2026-05-30 | Initial draft — 5-phase plan aligned with server-level txn model |