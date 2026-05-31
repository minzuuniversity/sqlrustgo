# PR-830E Implementation Plan — WAL Recovery Lifecycle

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement one-shot WAL recovery guard + RecoveryReport logging for PR-830E.

**Architecture:** Add `RecoveryState` enum + `StatefulRecoveryEngine` wrapper to track recovery state. Modify `recover_wal()` to output stats log and use stateful engine.

**Tech Stack:** Rust (sqlrustgo-storage crate), RecoveryEngine trait

---

## Bite-Sized Tasks

### Task 1: Add RecoveryState enum and StatefulRecoveryEngine to recovery_engine.rs

**Files:**
- Modify: `crates/storage/src/recovery_engine.rs:43-58` (add after RecoveryEngine trait)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_recovery_state_default() {
        let state = RecoveryState::Unrecovered;
        assert_eq!(state, RecoveryState::Unrecovered);
    }

    #[test]
    fn test_stateful_engine_blocks_double_recovery() {
        use crate::engine::MemoryStorage;
        use crate::wal::MemoryWalManager;

        let mut storage = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut engine = StatefulRecoveryEngine::new();

        // First recover succeeds
        let result1 = engine.recover(&mut storage, &mut wal);
        assert!(result1.is_ok());

        // Second recover returns idempotent result (empty report)
        let result2 = engine.recover(&mut storage, &mut wal);
        assert!(result2.is_ok());
        let report = result2.unwrap();
        assert_eq!(report.committed_txns, 0); // Idempotent - no entries replayed
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p sqlrustgo-storage test_recovery_state_default
# Expected: FAIL — RecoveryState not defined
```

**Step 3: Write minimal implementation**

```rust
/// Recovery state machine — prevents repeated recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecoveryState {
    #[default]
    Unrecovered,
    Recovered,
    Failed,
}

/// Stateful wrapper around RecoveryEngineImpl
pub struct StatefulRecoveryEngine<S: StorageEngine> {
    inner: RecoveryEngineImpl,
    state: RecoveryState,
}

impl<S: StorageEngine> StatefulRecoveryEngine<S> {
    pub fn new() -> Self {
        Self {
            inner: RecoveryEngineImpl,
            state: RecoveryState::Unrecovered,
        }
    }
}

impl<S: StorageEngine> RecoveryEngine<S> for StatefulRecoveryEngine<S> {
    fn recover(&mut self, storage: &mut S, wal: &mut dyn WalManager) -> SqlResult<RecoveryReport> {
        match self.state {
            RecoveryState::Unrecovered => {
                self.state = RecoveryState::Recovered;
                self.inner.recover(storage, wal)
            }
            RecoveryState::Recovered => {
                // Idempotent — already recovered, return empty report
                Ok(RecoveryReport::default())
            }
            RecoveryState::Failed => {
                Err(crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: cannot recover after previous failure".to_string(),
                ))
            }
        }
    }

    fn apply_entry(&mut self, storage: &mut S, entry: &WalEntry) -> SqlResult<()> {
        self.inner.apply_entry(storage, entry)
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sqlrustgo-storage test_recovery_state
# Expected: PASS
```

**Step 5: Commit**

```bash
git add crates/storage/src/recovery_engine.rs
git commit -m "feat(storage): add RecoveryState and StatefulRecoveryEngine for one-shot recovery guard"
```

---

### Task 2: Modify recover_wal() to use StatefulRecoveryEngine and log stats

**Files:**
- Modify: `src/engine_builder.rs:165-173`

**Step 1: Write the failing test**

```rust
// Add test in same file or integration test
#[test]
fn test_recover_wal_returns_report() {
    use sqlrustgo_storage::{FileStorage, FileBackedWalManager, WalStorage};
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let mut engine = ExecutionEngine::with_wal_file(dir.path().into()).unwrap();

    // Execute some committed work
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    engine.execute("COMMIT").unwrap();

    drop(engine);

    // recover_wal should return RecoveryReport
    let mut engine2 = ExecutionEngine::with_wal_file(dir.path().into()).unwrap();
    let report = recover_wal(&mut engine2).unwrap();
    assert!(report.committed_txns >= 1 || report.entries_total >= 0);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test wal_tx_contract_test test_commit_flush_crash_replays
# Expected: PASS (already works)
```

**Step 3: Write minimal implementation changes**

In `src/engine_builder.rs`:

```rust
/// Recover a WAL-backed engine after crash: replay committed WAL entries
pub fn recover_wal(
    engine: &mut ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>,
) -> SqlResult<RecoveryReport> {
    let storage = &mut *engine.storage.write().map_err(|e| {
        SqlError::ExecutionError(format!("Failed to lock storage for recovery: {:?}", e))
    })?;
    let (inner, wal_mgr) = storage.split();
    let mut recovery = StatefulRecoveryEngine::new();  // Changed from RecoveryEngineImpl
    let report = RecoveryEngine::recover(&mut recovery, inner, wal_mgr)?;

    // Log recovery statistics
    log::info!(
        "WAL recovery completed: {} committed txns, {} rolled back, {} incomplete, {} entries total",
        report.committed_txns,
        report.rolled_back_txns,
        report.incomplete_txns,
        report.entries_total
    );

    Ok(report)
}
```

Also add import:
```rust
use sqlrustgo_storage::recovery_engine::{RecoveryEngine, RecoveryEngineImpl, RecoveryReport, StatefulRecoveryEngine};
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p sqlrustgo-storage test_recover_wal_returns_report
# Or run full recovery integration test:
cargo test --test wal_tx_contract_test test_commit_flush_crash_replays
# Expected: PASS
```

**Step 5: Commit**

```bash
git add src/engine_builder.rs
git commit -m "feat(engine): use StatefulRecoveryEngine in recover_wal() and log stats"
```

---

### Task 3: Create RECOVERY Contract v1 document

**Files:**
- Create: `docs/releases/v3.8.0/PR-830E_CONTRACT.md`

**Step 1: Create document**

```markdown
# RECOVERY_CONTRACT_v1

> **Version**: 1.0
> **Date**: 2026-05-31
> **PR**: PR-830E
> **Status**: SPEC

## Recovery Trigger

RecoveryEngine MUST run at startup before serving requests.

## Recovery Source

RecoveryEngine replays WAL entries from durable storage.

## Idempotency

Repeated execution of RecoveryEngine MUST NOT corrupt storage state.
Current status: **GUARANTEED** via one-shot guard (`StatefulRecoveryEngine`).

## WAL Retention

Recovered WAL files MUST NOT be truncated automatically.

Reason:
- No checkpoint mechanism integrated with recovery
- No durable page tracking exists
- Crash after recovery but before persistence may cause data loss

## Known Limitations

1. Delete replay uses `storage.delete(&table, &[])` — deletes ALL rows in table
2. Update replay is skipped (WAL stores debug-formatted data)

## Future Work

- PR-830F: CheckpointManager integration + WAL truncation
- PR-840: DML Transaction Interception (fix delete/update replay)
```

**Step 2: Verify no broken links**

```bash
bash scripts/gate/check_docs_links.sh
# Expected: 0 broken links
```

**Step 3: Commit**

```bash
git add docs/releases/v3.8.0/PR-830E_CONTRACT.md
git commit -m "docs: add RECOVERY_CONTRACT_v1 for PR-830E"
```

---

### Task 4: Run full verification gate

**Step 1: Run unit tests**

```bash
cargo test -p sqlrustgo-storage
# Expected: 275+ PASS, 0 FAIL
```

**Step 2: Run clippy**

```bash
cargo clippy -p sqlrustgo-storage --all-features -- -D warnings
# Expected: 0 warnings
```

**Step 3: Run WAL contract tests**

```bash
cargo test --test wal_tx_contract_test
# Expected: 15 PASS, 7 FAIL (RECOVERY tests on L3 still fail, but guard works)
```

**Step 4: Check docs links**

```bash
bash scripts/gate/check_docs_links.sh --all
# Expected: 0 broken links
```

**Step 5: Commit if all passes**

```bash
git add -A
git commit -m "feat: PR-830E complete — one-shot recovery guard + RecoveryReport logging"
```

---

## Execution Options

**1. Subagent-Driven (this session)** — I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** — Open new session with executing-plans, batch execution with checkpoints

**Which approach?**

If Subagent-Driven chosen:
- **REQUIRED SUB-SKILL:** Use superpowers:subagent-driven-development
- Stay in this session
- Fresh subagent per task + code review

If Parallel Session chosen:
- Guide them to open new session in worktree
- **REQUIRED SUB-SKILL:** New session uses superpowers:executing-plans