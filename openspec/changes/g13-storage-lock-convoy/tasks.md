# Tasks — G13 Storage Lock Convoy (Full Fix)

## Phase 1: Spec & Analysis ✅

- [x] 1.1 Read G13 fix commit (82e1c5581) to confirm it covers poisoning only
- [x] 1.2 Read partial fix commit (2d2f23516) for storage_read() helper
- [x] 1.3 Identify all 8 files that need type changes

## Phase 2: Type Migration

### 2.1 Add parking_lot to workspace + crate deps

- [x] 2.1.1 Add `parking_lot.workspace = true` to top-level `sqlrustgo` package deps
- [x] 2.1.2 Add `parking_lot.workspace = true` to `crates/executor/Cargo.toml`
- [x] 2.1.3 Add `parking_lot.workspace = true` to `crates/mysql-server/Cargo.toml`

### 2.2 Switch `src/execution_engine.rs` storage type

- [x] 2.2.1 Replace `use std::sync::{Arc, RwLock}` with `use parking_lot::RwLock; use std::sync::Arc;`
- [x] 2.2.2 Change `pub(crate) storage: Arc<RwLock<S>>` → `Arc<parking_lot::RwLock<S>>`
- [x] 2.2.3 Update `pub fn new(storage: Arc<RwLock<S>>)` etc. (8 constructors)
- [x] 2.2.4 Remove `.unwrap()` from `self.storage.read()` / `self.storage.write()` (40+ sites)
- [x] 2.2.5 Remove `.map_err(...)` from `.write().map_err(...)` (3 sites)
- [x] 2.2.6 Add `storage_write()` helper (parallel to existing `storage_read()`)
- [x] 2.2.7 Convert INSERT/UPDATE/DELETE paths to use `storage_write()`

### 2.3 Update `src/engine_builder.rs`

- [x] 2.3.1 Replace `use std::sync::{Arc, RwLock}` with `use parking_lot::RwLock; use std::sync::Arc;`
- [x] 2.3.2 Update all `Arc::new(RwLock::new(...))` patterns (8 call sites)
- [x] 2.3.3 Remove `.unwrap()` from `.read().unwrap()` (3 sites)

### 2.4 Update `src/cbo_estimator.rs`

- [x] 2.4.1 Replace import
- [x] 2.4.2 Update all `&Arc<RwLock<ExecutionStats>>` to `&Arc<parking_lot::RwLock<ExecutionStats>>`
- [x] 2.4.3 Remove `.unwrap()` (2 sites)

### 2.5 Update `src/execution_engine_tests.rs`

- [x] 2.5.1 Replace import
- [x] 2.5.2 Update `Arc::new(RwLock::new(MemoryStorage::new()))` patterns (4 sites)
- [x] 2.5.3 Remove `.unwrap()` (1 site)

### 2.6 Update `crates/executor/src/stored_proc.rs`

- [x] 2.6.1 Replace import
- [x] 2.6.2 Update `StoredProcExecutor` struct field and constructor
- [x] 2.6.3 Update `match self.storage.read() { Ok(s) => s, ... }` pattern
- [x] 2.6.4 Remove `.unwrap()` from `.read().unwrap()` (5+ sites)

### 2.7 Update `crates/executor/src/trigger.rs`

- [x] 2.7.1 Replace import
- [x] 2.7.2 Update `TriggerExecutor` struct field
- [x] 2.7.3 Update `is_wal_enabled` assertion
- [x] 2.7.4 Remove `.unwrap()` from `.read().unwrap()` (4 sites)
- [x] 2.7.5 Update `if let Some(info) = .read()...` to direct let

### 2.8 Update `crates/mysql-server/src/lib.rs`

- [x] 2.8.1 Replace 2x `use std::sync::{Arc, RwLock}` imports
- [x] 2.8.2 Update `do_command_loop` signature (engine parameter)
- [x] 2.8.3 Update 4 `Arc::new(RwLock::new(ExecutionEngine::new(...)))` patterns
- [x] 2.8.4 Update 2 `Arc::new(RwLock::new(MemoryStorage::new()))` patterns
- [x] 2.8.5 Remove `.read().map_err(...)` patterns (2 sites)
- [x] 2.8.6 Convert 3 `if let Ok(mut storage) = self.storage.write()` to direct let
- [x] 2.8.7 Convert `if let Some(x) = .try_read()` patterns to direct let

## Phase 3: Build & Test

- [x] 3.1 `cargo build --release -p sqlrustgo` succeeds
- [x] 3.2 `cargo build --release -p sqlrustgo-mysql-server` succeeds
- [x] 3.3 `cargo build --release -p sqlrustgo-mysql-client` succeeds
- [x] 3.4 `cargo build --release -p sqlrustgo-cli` succeeds
- [x] 3.5 `cargo test -p sqlrustgo --lib` passes

## Phase 4: Server Verification

### 4.1 Start server with fresh data dir

- [x] 4.1.1 Copy TPC-H SF=0.001 fixture to `/tmp/p1_fresh_data/`
- [x] 4.1.2 Start `sqlrustgo-mysql-server serve` on port 13307 with `--server-threads 8`
- [x] 4.1.3 Verify `SELECT COUNT(*) FROM orders` returns 150 (or similar) in <10ms

### 4.2 Run hybrid_soak for 15+ min with `--oltp-ratio 0.36`

- [x] 4.2.1 Start `sqlrustgo-soak` example with 4 threads, 50 qps/thread, 900s duration
- [x] 4.2.2 Verify QPS reaches 200+ sustained
- [x] 4.2.3 Verify 0 errors (no EAGAIN, no panics)
- [x] 4.2.4 Sample server with `sample` tool — confirm 0 `lock_contended` occurrences
- [x] 4.2.5 Compute `mix_ratio_actual = writes / (writes + reads)` — should be 18-22%
- [x] 4.2.6 Verify all 6 Issue #3648 acceptance criteria pass

### 4.3 Compare with pre-fix (regression test)

- [x] 4.3.1 Run hybrid_soak for 5 min at high QPS (300+) — pre-fix deadlocks, post-fix doesn't
- [x] 4.3.2 Sample server at 1 min, 5 min, 10 min — all should show 0 `lock_contended`
- [x] 4.3.3 Verify no `semaphore_wait_trap` for >100ms in any thread

## Phase 5: Documentation & Closure

- [x] 5.1 Generate final `SOAK_72H_REPORT.md` showing:
  - [x] 5.1.1 Old SOAK state (server deadlocked at 70h36m)
  - [x] 5.1.2 Fix applied (commit hash, files changed)
  - [x] 5.1.3 Post-fix hybrid workload results (mix ratio, errors, QPS)
  - [x] 5.1.4 Sample analysis (lock_contended = 0)
- [x] 5.2 Post to Gitea:
  - [x] 5.2.1 Issue #3672: comment marking it fixed
  - [x] 5.2.2 Issue #3265: comment with final SOAK status
  - [x] 5.2.3 Issue #3648: comment with mix ratio compliance
- [x] 5.3 Commit final fix to git
- [x] 5.4 Archive this openspec change

## Acceptance Criteria

- [x] All Phase 2-5 tasks complete
- [x] Build succeeds
- [x] Server runs without deadlock for 15+ min under sustained write load
- [x] `sample` tool shows 0 `lock_contended` threads
- [x] Issue #3648 acceptance: `mix_ratio_actual` in [18.0, 22.0]
- [x] All 6 Gitea comments posted
- [x] openspec change archived
