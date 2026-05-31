# WAL Lifecycle Contract v1

> **Contract ID**: WAL_LIFECYCLE_CONTRACT_v1
> **PR**: PR-830F — WAL Lifecycle Controller
> **Version**: v3.8.0 Alpha
> **Status**: ACTIVE

---

## 1. Contract Overview

This contract defines the WAL lifecycle management behavior in SQLRustGo, including checkpoint-based truncation safety.

**Scope**: WAL lifecycle — checkpoint tracking, truncation gate, safe deletion.

**Out of Scope**: Parallel replay, background checkpoint scheduler.

---

## 2. Components

### 2.1 WalTruncationGate

```rust
pub trait WalTruncationGate: Send + Sync {
    fn safe_truncate_lsn(&self) -> Option<u64>;
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.safe_truncate_lsn().is_some_and(|cp_lsn| wal_lsn <= cp_lsn)
    }
}
```

**Rule WLC-001**: `can_truncate(lsn)` returns `true` only if `lsn <= safe_truncate_lsn()`.

---

## 3. CheckpointManager Integration

### 3.1 CheckpointManager

```rust
pub struct CheckpointManager {
    last_checkpoint: Arc<RwLock<Option<CheckpointMetadata>>>,
}

impl CheckpointManager {
    pub fn last_checkpoint_lsn(&self) -> Option<u64>;
    pub fn record_checkpoint(&self, metadata: CheckpointMetadata);
}
```

**Rule WLC-002**: `last_checkpoint_lsn()` returns `Some(lsn)` if a checkpoint exists, `None` otherwise.

**Rule WLC-003**: `record_checkpoint()` updates the last checkpoint atomically.

### 3.2 ExecutionEngine Integration

```rust
pub struct ExecutionEngine<S: StorageEngine> {
    checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>,
}

impl ExecutionEngine<S> {
    pub fn advance_checkpoint(&self, lsn: u64);
    pub fn try_truncate_wal(&self, wal: &mut dyn WalManager);
}
```

**Rule WLC-004**: `advance_checkpoint(lsn)` records a checkpoint at the given LSN if `checkpoint_manager` is `Some`.

**Rule WLC-005**: `try_truncate_wal()` calls `wal.truncate_before(lsn)` where `lsn = checkpoint_manager.last_checkpoint_lsn()`.

---

## 4. WAL Truncation Flow

```text
1. Transaction commits → advance_checkpoint(commit_lsn)
2. try_truncate_wal() checks checkpoint_manager.last_checkpoint_lsn()
3. If Some(cp_lsn): wal.truncate_before(cp_lsn)
4. WAL entries with lsn < cp_lsn are safely removed
```

**Rule WLC-006**: WAL entries with `lsn >= checkpoint_lsn` are NEVER truncated.

**Rule WLC-007**: If no checkpoint exists, `can_truncate()` returns `false`.

---

## 5. WAL Retention Policy

**Rule WLC-008**: WAL is an **Event Log**, not a Redo Log.

**Rule WLC-009**: WAL truncation is ONLY safe after checkpoint is established.

**Rule WLC-010**: Crash after recovery but before checkpoint → no data loss (WAL preserved).

---

## 6. Known Limitations

| Limitation | Description | Future Fix |
|------------|-------------|------------|
| Manual checkpoint | No automatic background checkpoint | PR-830F-C |
| Sequential replay | No parallel recovery replay | PR-830F-C |

---

## 7. Dependencies

| PR | Dependency Type | Description |
|----|----------------|-------------|
| PR-830E | Required | RecoveryState + StatefulRecoveryEngine |
| PR-830F-C | Future | Parallel replay + background checkpoint |

---

## 8. Verification

| Test | Description | Status |
|------|-------------|--------|
| `test_truncation_gate_blocks_before_checkpoint` | No checkpoint = no truncation | ✅ |
| `test_truncation_gate_allows_after_checkpoint` | Truncation allowed at checkpoint | ✅ |

---

## 9. Contract History

| Version | Date | Changes |
|---------|------|---------|
| v1 | 2026-05-31 | Initial contract for PR-830F |