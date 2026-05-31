# v3.8.0 Architecture — Server-Level Transaction Model

## Status: DRAFT

> Last updated: 2026-05-30
> **Baseline**: `origin/develop/v3.8.0` (commit 44fea01c) — Execution Semantics Freeze (087bb12d)

---

## 1. Design Principles (Immutable)

These three principles are **fixed for v3.8** — no discussion, no deviation.

| # | Principle | Rationale |
|---|-----------|-----------|
| P1 | **LocalExecutor remains stateless** | Executor is a query execution engine. Transaction lifecycle belongs to the server layer, not the execution engine. |
| P2 | **TransactionManager owns transaction state** | Transaction state (active_txn, write_buffer, snapshots) resides in TransactionManager — a server-level component. |
| P3 | **WriteBuffer belongs to TransactionManager** | Buffering staged writes is a transaction concern, not a storage or executor concern. |

Violating any of these principles requires a formal ADR and sign-off from the project lead.

---

## 2. Current Baseline (v3.7.0)

```
COM_QUERY (mysql-server/src/lib.rs:1081)
  │
  └── eng.execute(&q)        ← LocalExecutor (stateless)
        │
        └── StorageEngine    ← dyn trait, no transaction awareness
```

**TransactionManager** exists as `crates/transaction/src/manager.rs` but is **not connected** to the execution path. It is a standalone module.

```
TransactionManager (standalone, not integrated)
  ├── crates/transaction/src/manager.rs
  └── crates/transaction/src/transaction_manager.rs
```

---

## 3. Target Architecture (v3.8.0)

```
mysql-server
  │
  ├── Session/Connection State
  │     └── TransactionManager (server-level)
  │           ├── active_txn: Option<Transaction>
  │           ├── write_buffer: WriteBuffer
  │           └── snapshot_metadata: SnapshotMeta
  │
  ├── SQL Parser
  │
  └── Transaction Dispatch Layer
        │
        ├── BEGIN  ──→ txn_manager.begin()
        ├── COMMIT ──→ txn_manager.commit()
        ├── ROLLBACK ──→ txn_manager.rollback()
        │
        └── Other Statements ──→ LocalExecutor (stateless)
                                      │
                                      └── StorageEngine
```

### Key Properties

- **TransactionManager** is owned by the Session/Connection, not by LocalExecutor
- **LocalExecutor** receives a reference to `&'a dyn StorageEngine` only — no transaction state
- **WriteBuffer** is inside TransactionManager, not inside StorageEngine or LocalExecutor
- All transaction boundaries (BEGIN/COMMIT/ROLLBACK) are handled at the dispatch layer

---

## 4. Component Responsibilities

### TransactionManager (server-level)

```
Responsibilities:
  - Begin / Commit / Rollback transaction lifecycle
  - Manage write_buffer (stage DML writes before commit)
  - Provide transaction snapshot for read consistency
  - SSI conflict detection (existing)

Does NOT:
  - Execute queries
  - Own StorageEngine references
  - Know about parser or protocol
```

### LocalExecutor (stateless query execution)

```
Responsibilities:
  - Execute query against StorageEngine
  - Cache query plans
  - Profile query execution

Does NOT:
  - Own transaction state
  - Manage write buffers
  - Handle BEGIN/COMMIT/ROLLBACK
```

### StorageEngine (trait)

```
Responsibilities:
  - Read/write physical data
  - Atomic page operations

Does NOT:
  - Know about transactions
  - Buffer staged writes
  - Handle snapshot isolation
```

---

## 5. Transaction State Machine

```
                    ┌─────────────┐
                    │    IDLE     │  ← No active transaction
                    └──────┬──────┘
                           │ begin()
                           ▼
                    ┌─────────────┐
              ┌─────│  ACTIVE    │  ← write_buffer open
              │     └──────┬─────┘
              │            │
    commit()  │            │ rollback()
              ▼            ▼
        ┌──────────┐  ┌──────────┐
        │ COMMITTED│  │  ABORTED │
        └──────────┘  └──────────┘
```

---

## 6. WriteBuffer Design

WriteBuffer is part of TransactionManager, not a separate component.

```rust
// Inside TransactionManager
pub struct WriteBuffer {
    staging: Vec<WriteOp>,         // staged writes before commit
    max_size: usize,              // configurable limit
}

pub enum WriteOp {
    Insert { table: TableId, row: Row },
    Update { table: TableId, key: Vec<Value>, new_row: Row },
    Delete { table: TableId, key: Vec<Value> },
}

impl TransactionManager {
    pub fn stage_write(&mut self, op: WriteOp) {
        self.write_buffer.add(op);
    }

    pub fn commit(mut self) -> Result<(), TxError> {
        // flush write_buffer to storage
        // clear buffer
        // transition to COMMITTED
    }

    pub fn rollback(&mut self) {
        // discard write_buffer
        // transition to ABORTED
    }
}
```

---

## 7. Why This Architecture?

### Common mistakes (pre-v3.8)

| Mistaken assumption | Correct model |
|--------------------|---------------|
| "Executor should own txn state" | Executor is stateless; txn belongs to server |
| "StorageEngine should buffer writes" | Storage is physical; buffering is txn-level |
| "WriteBuffer inside LocalExecutor" | WriteBuffer inside TransactionManager |
| "TransactionContext inside engine" | TransactionContext is server-level |

### Correct ownership

```
mysql-server (connection/session)
  └── TransactionManager (txn lifecycle)
        └── WriteBuffer (staged writes)
              │
              └── flush to StorageEngine (on commit)
```

---

## 8. Non-Goals (v3.8)

These are **explicitly out of scope** for v3.8:

- MVCC / snapshot isolation (v3.9)
- Two-phase commit (v3.9+)
- Distributed transactions
- Savepoints
- XA protocol

---

## 9. Verification

The following must remain true after all v3.8 changes:

1. `LocalExecutor` struct has **no** `txn_manager` field
2. `LocalExecutor` struct has **no** `write_buffer` field
3. Transaction lifecycle code is in `mysql-server` or a new `server/` crate, **not** in `executor/`
4. All `BEGIN`/`COMMIT`/`ROLLBACK` parsing results in calls to `TransactionManager`, not `LocalExecutor`

---

## 10. References

- Current TransactionManager: `crates/transaction/src/manager.rs`
- Current LocalExecutor: `crates/executor/src/local_executor.rs`
- Protocol dispatch: `crates/mysql-server/src/lib.rs:1081`
- v3.7.0 baseline commit: `72223a80`

---

## Changelog

| Date | Change |
|------|--------|
| 2026-05-30 | Initial draft — server-level transaction model |