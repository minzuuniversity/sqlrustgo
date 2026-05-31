# RECOVERY Contract v1

> **Contract ID**: RECOVERY_CONTRACT_v1
> **PR**: PR-830E — WAL Recovery Lifecycle
> **Version**: v3.8.0 Alpha
> **Status**: DRAFT — For Review

---

## 1. Contract Overview

This contract defines the behavior and guarantees of the WAL Recovery subsystem in SQLRustGo.

**Scope**: Startup recovery — replay of WAL entries after crash restart.

**Out of Scope**: WAL truncation, checkpoint management, DML replay fixes.

---

## 2. Recovery Trigger

| Rule | Description |
|------|-------------|
| RECOVERY-001 | `RecoveryEngine::recover()` MUST be called at database startup before serving any user requests. |
| RECOVERY-002 | `with_wal_recovery()` calls `recover_wal()` automatically — callers do NOT need to manually invoke recovery. |
| RECOVERY-003 | Recovery MUST run on a freshly opened `WalStorage<FileStorage, FileBackedWalManager>` — no in-flight transactions. |

---

## 3. Recovery Source

| Rule | Description |
|------|-------------|
| RECOVERY-004 | RecoveryEngine replays WAL entries from durable storage (`.wal` file). |
| RECOVERY-005 | Only committed transactions are replayed (BEGIN → COMMIT sequence). |
| RECOVERY-006 | Rolled-back transactions are skipped (BEGIN → ROLLBACK sequence). |
| RECOVERY-007 | Incomplete transactions (no COMMIT/ROLLBACK) are marked as `incomplete_txns` in RecoveryReport. |

---

## 4. Idempotency

| Rule | Description |
|------|-------------|
| RECOVERY-008 | Repeated execution of `StatefulRecoveryEngine::recover()` MUST NOT corrupt storage state. |
| RECOVERY-009 | One-shot guard: After successful recovery, subsequent calls return `RecoveryReport::default()` (idempotent). |
| RECOVERY-010 | After failed recovery, subsequent calls MUST return error (no retry). |

**Implementation**: `StatefulRecoveryEngine<S>` wraps `RecoveryEngineImpl` with a `RecoveryState` enum.

```rust
pub enum RecoveryState {
    Unrecovered,  // Initial state
    Recovered,    // After successful recovery
    Failed,       // After failed recovery
}

impl<S: StorageEngine> RecoveryEngine<S> for StatefulRecoveryEngine<S> {
    fn recover(&mut self, storage: &mut S, wal: &mut dyn WalManager) -> SqlResult<RecoveryReport> {
        match self.state {
            RecoveryState::Unrecovered => {
                self.state = RecoveryState::Recovered;
                self.inner.recover(storage, wal)
            }
            RecoveryState::Recovered => Ok(RecoveryReport::default()),
            RecoveryState::Failed => Err(SqlError::ExecutionError(
                "RecoveryEngine: cannot recover after previous failure".to_string(),
            )),
        }
    }
}
```

---

## 5. WAL Retention

| Rule | Description |
|------|-------------|
| RECOVERY-011 | Recovered WAL files MUST NOT be truncated automatically. |
| RECOVERY-012 | WAL is an **Event Log**, not a Redo Log — authoritative history, not a recovery mechanism. |

**Rationale**:
- No checkpoint mechanism integrated with recovery
- No durable page tracking exists
- Crash after recovery but before persistence may cause data loss if WAL is truncated

**Future**: PR-830F will integrate CheckpointManager for WAL lifecycle management (truncation).

---

## 6. RecoveryReport

The `RecoveryReport` struct provides statistics after recovery:

```rust
#[derive(Debug, Default, Clone)]
pub struct RecoveryReport {
    pub entries_total: usize,     // Total WAL entries read
    pub committed_txns: usize,     // Committed transactions replayed
    pub rolled_back_txns: usize,   // Rolled back transactions skipped
    pub incomplete_txns: usize,    // Incomplete (no Commit/Rollback)
    pub rows_inserted: usize,      // Rows inserted during recovery
    pub rows_updated: usize,       // Rows updated during recovery
    pub rows_deleted: usize,       // Rows deleted during recovery
}
```

| Rule | Description |
|------|-------------|
| RECOVERY-013 | `recover_wal()` MUST return `RecoveryReport` to caller. |
| RECOVERY-014 | `recover_wal()` MUST log RecoveryReport stats at INFO level. |

---

## 7. Logging

| Rule | Description |
|------|-------------|
| RECOVERY-015 | `recover_wal()` outputs: `"WAL recovery completed: {committed_txns} committed txns, {rolled_back_txns} rolled back, {incomplete_txns} incomplete, {entries_total} entries total"` |

---

## 8. Known Limitations

| Limitation | Description | Future Fix |
|------------|-------------|------------|
| Delete replay | Uses `storage.delete(&table, &[])` — deletes ALL rows in table, not row-level | PR-840 (DML Transaction Interception) |
| Update replay | Skipped — WAL stores debug-formatted data, not update deltas | WAL format redesign |
| No checkpoint | WAL retention policy not enforced | PR-830F (CheckpointManager) |

---

## 9. Dependencies

| PR | Dependency Type | Description |
|----|----------------|-------------|
| PR-830A | Required | WAL Entry structure definition |
| PR-830B | Required | WalManager trait |
| PR-830C | Required | RecoveryEngine trait |
| PR-830D | Required | RecoveryEngineImpl implementation |
| PR-830F | Future | CheckpointManager integration + WAL truncation |
| PR-840 | Future | DML Transaction Interception (fix delete/update replay) |

---

## 10. Verification

| Test | Description | Status |
|------|-------------|--------|
| `test_recovery_state_default` | Default state is Unrecovered | ✅ PASS |
| `test_stateful_engine_blocks_double_recovery` | Idempotent re-invocation returns default report | ✅ PASS |
| `test_recovery_engine_impl_trait_bounds` | Trait bounds satisfied | ✅ PASS |

---

## 11. Contract History

| Version | Date | Changes |
|---------|------|---------|
| v1 | 2026-05-31 | Initial contract for PR-830E |