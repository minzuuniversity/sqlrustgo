# Tasks — G13 Storage Lock Convoy (Full Fix)

## Phase 1: Spec & Analysis ✅

- [x] 1.1 Read G13 fix commit (82e1c5581) to confirm it covers poisoning only
- [x] 1.2 Read partial fix commit (2d2f23516) for storage_read() helper
- [x] 1.3 Identify all 8 files that need type changes

## Phase 2: Type Migration

### 2.1 Add parking_lot to workspace + crate deps

- [ ] 2.1.1 Add `parking_lot.workspace = true` to top-level `sqlrustgo` package deps
- [ ] 2.1.2 Add `parking_lot.workspace = true` to `crates/executor/Cargo.toml`
- [ ] 2.1.3 Add `parking_lot.workspace = true` to `crates/mysql-server/Cargo.toml`

### 2.2 Switch `src/execution_engine.rs` storage type

- [ ] 2.2.1 Replace `use std::sync::{Arc, RwLock}` with `use parking_lot::RwLock; use std::sync::Arc;`
- [ ] 2.2.2 Change `pub(crate) storage: Arc<RwLock<S>>` → `Arc<parking_lot::RwLock<S>>`
- [ ] 2.2.3 Update `pub fn new(storage: Arc<RwLock<S>>)` etc. (8 constructors)
- [ ] 2.2.4 Remove `.unwrap()` from `self.storage.read()` / `self.storage.write()` (40+ sites)
- [ ] 2.2.5 Remove `.map_err(...)` from `.write().map_err(...)` (3 sites)
- [ ] 2.2.6 Add `storage_write()` helper (parallel to existing `storage_read()`)
- [ ] 2.2.7 Convert INSERT/UPDATE/DELETE paths to use `storage_write()`

### 2.3 Update `src/engine_builder.rs`

- [ ] 2.3.1 Replace `use std::sync::{Arc, RwLock}` with `use parking_lot::RwLock; use std::sync::Arc;`
- [ ] 2.3.2 Update all `Arc::new(RwLock::new(...))` patterns (8 call sites)
- [ ] 2.3.3 Remove `.unwrap()` from `.read().unwrap()` (3 sites)

### 2.4 Update `src/cbo_estimator.rs`

- [ ] 2.4.1 Replace import
- [ ] 2.4.2 Update all `&Arc<RwLock<ExecutionStats>>` to `&Arc<parking_lot::RwLock<ExecutionStats>>`
- [ ] 2.4.3 Remove `.unwrap()` (2 sites)

### 2.5 Update `src/execution_engine_tests.rs`

- [ ] 2.5.1 Replace import
- [ ] 2.5.2 Update `Arc::new(RwLock::new(MemoryStorage::new()))` patterns (4 sites)
- [ ] 2.5.3 Remove `.unwrap()` (1 site)

### 2.6 Update `crates/executor/src/stored_proc.rs`

- [ ] 2.6.1 Replace import
- [ ] 2.6.2 Update `StoredProcExecutor` struct field and constructor
- [ ] 2.6.3 Update `match self.storage.read() { Ok(s) => s, ... }` pattern
- [ ] 2.6.4 Remove `.unwrap()` from `.read().unwrap()` (5+ sites)

### 2.7 Update `crates/executor/src/trigger.rs`

- [ ] 2.7.1 Replace import
- [ ] 2.7.2 Update `TriggerExecutor` struct field
- [ ] 2.7.3 Update `is_wal_enabled` assertion
- [ ] 2.7.4 Remove `.unwrap()` from `.read().unwrap()` (4 sites)
- [ ] 2.7.5 Update `if let Some(info) = .read()...` to direct let

### 2.8 Update `crates/mysql-server/src/lib.rs`

- [ ] 2.8.1 Replace 2x `use std::sync::{Arc, RwLock}` imports
- [ ] 2.8.2 Update `do_command_loop` signature (engine parameter)
- [ ] 2.8.3 Update 4 `Arc::new(RwLock::new(ExecutionEngine::new(...)))` patterns
- [ ] 2.8.4 Update 2 `Arc::new(RwLock::new(MemoryStorage::new()))` patterns
- [ ] 2.8.5 Remove `.read().map_err(...)` patterns (2 sites)
- [ ] 2.8.6 Convert 3 `if let Ok(mut storage) = self.storage.write()` to direct let
- [ ] 2.8.7 Convert `if let Some(x) = .try_read()` patterns to direct let

## Phase 3: Build & Test

- [ ] 3.1 `cargo build --release -p sqlrustgo` succeeds
- [ ] 3.2 `cargo build --release -p sqlrustgo-mysql-server` succeeds
- [ ] 3.3 `cargo build --release -p sqlrustgo-mysql-client` succeeds
- [ ] 3.4 `cargo build --release -p sqlrustgo-cli` succeeds
- [ ] 3.5 `cargo test -p sqlrustgo --lib` passes

## Phase 4: Server Verification

### 4.1 Start server with fresh data dir

- [ ] 4.1.1 Copy TPC-H SF=0.001 fixture to `/tmp/p1_fresh_data/`
- [ ] 4.1.2 Start `sqlrustgo-mysql-server serve` on port 13307 with `--server-threads 8`
- [ ] 4.1.3 Verify `SELECT COUNT(*) FROM orders` returns 150 (or similar) in <10ms

### 4.2 Run hybrid_soak for 15+ min with `--oltp-ratio 0.36`

- [ ] 4.2.1 Start `sqlrustgo-soak` example with 4 threads, 50 qps/thread, 900s duration
- [ ] 4.2.2 Verify QPS reaches 200+ sustained
- [ ] 4.2.3 Verify 0 errors (no EAGAIN, no panics)
- [ ] 4.2.4 Sample server with `sample` tool — confirm 0 `lock_contended` occurrences
- [ ] 4.2.5 Compute `mix_ratio_actual = writes / (writes + reads)` — should be 18-22%
- [ ] 4.2.6 Verify all 6 Issue #3648 acceptance criteria pass

### 4.3 Compare with pre-fix (regression test)

- [ ] 4.3.1 Run hybrid_soak for 5 min at high QPS (300+) — pre-fix deadlocks, post-fix doesn't
- [ ] 4.3.2 Sample server at 1 min, 5 min, 10 min — all should show 0 `lock_contended`
- [ ] 4.3.3 Verify no `semaphore_wait_trap` for >100ms in any thread

## Phase 5: Documentation & Closure

- [ ] 5.1 Generate final `SOAK_72H_REPORT.md` showing:
  - [ ] 5.1.1 Old SOAK state (server deadlocked at 70h36m)
  - [ ] 5.1.2 Fix applied (commit hash, files changed)
  - [ ] 5.1.3 Post-fix hybrid workload results (mix ratio, errors, QPS)
  - [ ] 5.1.4 Sample analysis (lock_contended = 0)
- [ ] 5.2 Post to Gitea:
  - [ ] 5.2.1 Issue #3672: comment marking it fixed
  - [ ] 5.2.2 Issue #3265: comment with final SOAK status
  - [ ] 5.2.3 Issue #3648: comment with mix ratio compliance
- [ ] 5.3 Commit final fix to git
- [ ] 5.4 Archive this openspec change

## Acceptance Criteria

- [ ] All Phase 2-5 tasks complete
- [ ] Build succeeds
- [ ] Server runs without deadlock for 15+ min under sustained write load
- [ ] `sample` tool shows 0 `lock_contended` threads
- [ ] Issue #3648 acceptance: `mix_ratio_actual` in [18.0, 22.0]
- [ ] All 6 Gitea comments posted
- [ ] openspec change archived
