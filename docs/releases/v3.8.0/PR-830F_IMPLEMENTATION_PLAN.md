# PR-830F Implementation Plan — WAL Lifecycle Controller

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement WAL lifecycle controller with CheckpointManager integration and WAL truncation gate.

**Architecture:**
- Add `WalTruncationGate` trait to `crates/storage/src/wal.rs`
- Extend `CheckpointManager` with `last_checkpoint_lsn()` method
- Integrate `checkpoint_manager` into `ExecutionEngine` with commit hooks
- Wire WAL truncation trigger into `engine_builder.rs`

**Tech Stack:** Rust, sqlrustgo-storage, sqlrustgo-transaction

---

## Bite-Sized Tasks

### Task 1: Add WalTruncationGate trait to wal.rs

**Files:**
- Modify: `crates/storage/src/wal.rs`

**Step 1: Add trait definition**

```rust
/// WAL truncation safety gate
pub trait WalTruncationGate: Send + Sync {
    /// Returns the LSN below which WAL entries can be safely deleted.
    /// Returns None if no checkpoint has been established.
    fn safe_truncate_lsn(&self) -> Option<u64>;

    /// Check if a given LSN can be truncated
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.safe_truncate_lsn().map_or(false, |cp_lsn| wal_lsn <= cp_lsn)
    }
}
```

**Step 2: Verify compilation**

Run: `cargo build -p sqlrustgo-storage`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/storage/src/wal.rs
git commit -m "feat(storage): add WalTruncationGate trait"
```

---

### Task 2: Add last_checkpoint_lsn to CheckpointManager

**Files:**
- Modify: `crates/storage/src/checkpoint.rs`

**Step 1: Add method**

```rust
impl CheckpointManager {
    /// Get the last checkpoint LSN
    pub fn last_checkpoint_lsn(&self) -> Option<u64> {
        self.last_checkpoint.read().unwrap().as_ref().map(|c| c.lsn)
    }

    /// Check if a WAL LSN can be truncated (implements WalTruncationGate)
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.last_checkpoint_lsn()
            .map_or(false, |cp_lsn| wal_lsn <= cp_lsn)
    }
}
```

**Step 2: Implement WalTruncationGate for CheckpointManager**

```rust
impl WalTruncationGate for CheckpointManager {
    fn safe_truncate_lsn(&self) -> Option<u64> {
        self.last_checkpoint_lsn()
    }
}
```

**Step 3: Add unit tests**

```rust
#[test]
fn test_truncation_gate_blocks_before_checkpoint() {
    let gate = CheckpointManager::default();
    assert!(!gate.can_truncate(1000));
}

#[test]
fn test_truncation_gate_allows_after_checkpoint() {
    let temp = TempDir::new().unwrap();
    let mut manager = CheckpointManager::with_dir(temp.path().to_path_buf()).unwrap();
    manager.record_checkpoint(CheckpointMetadata {
        lsn: 1000,
        timestamp: 0,
        tx_count: 1,
        dirty_pages: 0,
        file_path: PathBuf::new(),
    });
    assert!(gate.can_truncate(500));
    assert!(gate.can_truncate(1000));
    assert!(!gate.can_truncate(1500));
}
```

**Step 4: Run tests**

Run: `cargo test -p sqlrustgo-storage test_truncation_gate`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/storage/src/checkpoint.rs
git commit -m "feat(storage): add last_checkpoint_lsn and WalTruncationGate impl"
```

---

### Task 3: Add truncate_before to FileBackedWalManager

**Files:**
- Modify: `crates/storage/src/wal.rs`

**Step 1: Add method to FileBackedWalManager**

```rust
impl FileBackedWalManager {
    /// Truncate WAL entries with LSN < `lsn`
    pub fn truncate_before(&mut self, lsn: u64) -> SqlResult<()> {
        // Keep entries with lsn >= lsn, delete entries with lsn < lsn
        self.entries.retain(|e| e.lsn >= lsn);
        Ok(())
    }
}
```

**Step 2: Add to WalManager trait**

```rust
pub trait WalManager: Send + Sync {
    // ... existing methods ...

    /// Truncate WAL entries with LSN < `lsn`
    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()>;
}
```

**Step 3: Implement for MemoryWalManager**

```rust
impl WalManager for MemoryWalManager {
    // ... existing methods ...

    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()> {
        self.entries.retain(|e| e.lsn >= lsn);
        Ok(())
    }
}
```

**Step 4: Run tests**

Run: `cargo build -p sqlrustgo-storage`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add crates/storage/src/wal.rs
git commit -m "feat(storage): add truncate_before to WalManager"
```

---

### Task 4: Integrate CheckpointManager into ExecutionEngine

**Files:**
- Modify: `src/execution_engine.rs`

**Step 1: Add checkpoint_manager field**

```rust
use sqlrustgo_storage::checkpoint::CheckpointManager;

pub struct ExecutionEngine<S: StorageEngine> {
    // ... existing fields ...
    checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>, // NEW
}
```

**Step 2: Add advance_checkpoint method**

```rust
impl<S: StorageEngine> ExecutionEngine<S> {
    /// Advance checkpoint after commit
    pub fn advance_checkpoint(&self, lsn: u64) {
        if let Some(cp) = &self.checkpoint_manager {
            if let Ok(mut guard) = cp.write() {
                guard.record_checkpoint(CheckpointMetadata {
                    lsn,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    tx_count: 1,
                    dirty_pages: 0,
                    file_path: PathBuf::new(),
                });
            }
        }
    }

    /// Try to truncate WAL up to checkpoint
    pub fn try_truncate_wal(&self, wal: &mut dyn WalManager) {
        if let Some(cp) = &self.checkpoint_manager {
            if let Ok(guard) = cp.read() {
                if let Some(lsn) = guard.last_checkpoint_lsn() {
                    wal.truncate_before(lsn).ok();
                }
            }
        }
    }
}
```

**Step 3: Run clippy**

Run: `cargo clippy -p sqlrustgo --all-features -- -D warnings`
Expected: 0 warnings

**Step 4: Commit**

```bash
git add src/execution_engine.rs
git commit -m "feat(sqlrustgo): integrate CheckpointManager into ExecutionEngine"
```

---

### Task 5: Wire up engine builder

**Files:**
- Modify: `src/engine_builder.rs`

**Step 1: Add factory method with checkpoint**

```rust
impl ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>> {
    /// Create a WAL-backed engine with CheckpointManager
    pub fn with_wal_and_checkpoint(
        data_dir: PathBuf,
        checkpoint_dir: PathBuf,
    ) -> SqlResult<Self> {
        let inner = FileStorage::new_with_wal(data_dir.clone())
            .map_err(|e| SqlError::ExecutionError(format!("FileStorage init failed: {}", e)))?;
        let wal_path = data_dir.join("sqlrustgo.wal");
        let wal_manager = FileBackedWalManager::new(wal_path)?;
        let wal_storage = WalStorage::new(inner, wal_manager)?;

        let checkpoint_manager = CheckpointManager::with_dir(checkpoint_dir).ok();

        Ok(ExecutionEngine {
            storage: Arc::new(RwLock::new(wal_storage)),
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: checkpoint_manager.map(Arc::new).map(RwLock::new),
        })
    }
}
```

**Step 2: Run build**

Run: `cargo build -p sqlrustgo`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/engine_builder.rs
git commit -m "feat(sqlrustgo): add with_wal_and_checkpoint factory"
```

---

### Task 6: Create PR-830F Contract

**Files:**
- Create: `docs/releases/v3.8.0/PR-830F_CONTRACT.md`

**Step 1: Write contract document**

See PR-830F_SPEC.md Section 8 for contract content.

**Step 2: Commit**

```bash
git add docs/releases/v3.8.0/PR-830F_CONTRACT.md
git commit -m "docs: add PR-830F CONTRACT"
```

---

### Task 7: Run full verification gate

**Step 1: Run storage tests**

Run: `cargo test -p sqlrustgo-storage --lib`
Expected: 275+ passed

**Step 2: Run clippy**

Run: `cargo clippy --all-features -- -D warnings`
Expected: 0 warnings

**Step 3: Run doc link check**

Run: `bash scripts/gate/check_docs_links.sh`
Expected: 0 broken links (in changed files)

---

## Execution Options

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?