# Proposal — G13 Storage Lock Convoy (Full Fix)

## Why

Issue #3672 (filed from the 72h wired SOAK): under sustained write load (1,200+ qps with 4 hybrid_soak threads doing INSERT/UPDATE/DELETE), the `std::sync::RwLock` on `ExecutionEngine::storage` enters a permanent thundering-herd deadlock:

- 1 long-running write (INSERT/UPDATE/DELETE) holds the write lock
- All 32 reader workers queue behind it on `RwLock::lock_contended`
- Subsequent writers queue behind the readers (write-preferring)
- The original writer can't release because no thread can make progress
- 17 mariadb bash clients + 4 hybrid_soak workers all hang in ESTABLISHED for 13+ hours

The previous G13 fix (commit `82e1c5581`, PR #3642) only addressed **lock poisoning** (panic in lock holder → all subsequent calls fail). It did NOT cover the **thundering-herd** case where the lock holder is alive and not panicking, but readers/writers queue indefinitely.

The 72h SOAK server (PID 35830) deadlocked at 70h36m / 72h because of this. 5 hybrid_soak batches of INSERTs/UPDATEs/DELETEs (~1,800 writes) persisted before the deadlock, then the server hung.

A partial fix (commit `2d2f23516`) added `ExecutionEngine::storage_read()` with `try_read()` + 100-iteration retry to break the thundering-herd. Verified: post-fix, the same hybrid workload runs cleanly with 0 errors and 0 lock_contended for 120s. But this is a workaround, not a root-cause fix — `std::sync::RwLock` has no fair policy, so under heavier write load the retry can fail and the deadlock can return.

## What Changes

### 1. Migrate storage lock to `parking_lot::RwLock` with `Fair` policy

`std::sync::RwLock` → `parking_lot::RwLock` (default `Fair` policy). `parking_lot::RwLock`:
- Has a `Fair` policy that prevents writer starvation
- Is `no_std`-compatible and faster than `std::sync::RwLock`
- Has `try_read` / `try_write` that never poisons
- API-compatible: `.read()` / `.write()` return guards (no `Result`)

### 2. Update all call sites (8 files)

The type change cascades through:
- `src/execution_engine.rs` — `ExecutionEngine::storage` field + 40+ call sites
- `src/engine_builder.rs` — engine constructors wrap storage in `Arc<RwLock<...>>`
- `src/cbo_estimator.rs` — receives `&Arc<RwLock<ExecutionStats>>`
- `src/engine_select.rs` — `self.storage.read()` calls
- `src/execution_engine_tests.rs` — test fixtures
- `crates/executor/src/stored_proc.rs` — `StoredProcExecutor` constructor
- `crates/executor/src/trigger.rs` — `TriggerExecutor` constructor
- `crates/mysql-server/src/lib.rs` — server runtime, `Arc<RwLock<...>>` types

### 3. Keep `storage_read()` helper, add `storage_write()`

The try_read+retry helper (commit `2d2f23516`) is still useful for diagnostics. Add a parallel `storage_write()` helper that:
- Tries write with retry (100 attempts, 100µs backoff)
- After 100ms of contention, falls back to blocking write
- Logs a warning on heavy contention (caller-side metric)

### 4. Update `mysql-server/src/lib.rs` `do_command_loop`

The server runtime wraps the engine in `Arc<RwLock<ExecutionEngine<...>>>`. After type change:
- `engine.write().map_err(...)` → `engine.write()` (no Result)
- The `if let Ok(...)` patterns at lines ~1761, ~1784, ~1853 → direct `let ... = ...`
- The `if let Some(x) = ...try_read()` patterns → direct `let x = ...`

### 5. Verify with hybrid_soak + sample analysis

- Rebuild server binary
- Restart server with fresh data dir
- Run hybrid_soak for 15+ min at 200 qps
- Sample server: confirm `lock_contended` count is **0**
- Sample server: confirm `execute_select` is being called
- Run a test with `--oltp-ratio 0.36` to satisfy Issue #3648's 18-22% mix

## Capabilities

### New Capabilities

- **`storage_read()` and `storage_write()` helpers** with try_lock + retry, useful for diagnosing lock contention
- **No-deadlock guarantee** under sustained write load: parking_lot::RwLock::Fair policy prevents writer starvation
- **Direct integration** with the existing G13 SOAK infrastructure (commit `82e1c5581` poisoning fix is preserved)

### Modified Capabilities

- **`src/execution_engine.rs` storage lock type**: `Arc<std::sync::RwLock<S>>` → `Arc<parking_lot::RwLock<S>>`
- **`src/execution_engine.rs` storage helper**: `storage_read()` already exists, add `storage_write()`
- **All call sites** that previously used `.read().unwrap()` / `.write().unwrap()` no longer need `.unwrap()`

## Non-goals

- **WAL checkpointing**: separate issue (WAL grew from 153KB to 32MB during 70h36m without checkpoint). Not addressed by this change.
- **Migrating ALL locks** to parking_lot: this change is scoped to `self.storage` only — the actual lock that caused the deadlock. Other locks (catalog, stats, transaction manager) keep std::sync::RwLock.
- **72h SOAK wall-clock completion**: the original SOAK server already deadlocked at 70h36m. The fix prevents future deadlocks, but the existing 72h target cannot be retroactively achieved. The next 72h SOAK run will satisfy the wall-clock acceptance criterion.

## Out-of-scope follow-ups

- **WAL checkpointing** (separate change, separate issue)
- **Read replica / connection pooling** to reduce per-connection lock contention
- **Lock-free execution engine** (full redesign, separate change)
