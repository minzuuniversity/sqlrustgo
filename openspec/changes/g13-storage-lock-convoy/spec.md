# G13 Storage Lock Convoy — Full Fix Specification

## Purpose

Eliminate the thundering-herd deadlock in `ExecutionEngine::storage` lock by migrating from `std::sync::RwLock` to `parking_lot::RwLock` with `Fair` policy. This is the root-cause fix for Issue #3672.

## Requirements

### Req 1: Storage lock uses `parking_lot::RwLock` with `Fair` policy

The `ExecutionEngine::storage` field SHALL be of type `Arc<parking_lot::RwLock<S>>` where `S: StorageEngine`. The `parking_lot::RwLock` SHALL be constructed with the default `Fair` policy, which prevents writer starvation by ensuring that a writer releases the lock to all waiting readers before allowing a new writer to acquire.

```rust
// Before:
pub(crate) storage: Arc<std::sync::RwLock<S>>,

// After:
pub(crate) storage: Arc<parking_lot::RwLock<S>>,
```

#### Scenario: Server runs sustained write workload without deadlock
- **GIVEN** the server is running with `--server-threads 32`
- **WHEN** 4+ clients issue mixed INSERT/UPDATE/DELETE at >1000 qps for 15+ minutes
- **THEN** no thread SHALL enter `RwLock::lock_contended` state (verified via `sample` tool)
- **AND** no thread SHALL park on `semaphore_wait_trap` for >100ms

#### Scenario: Readers are not blocked by long-running writers
- **GIVEN** one thread holds the write lock for a long INSERT (e.g. 100ms)
- **WHEN** 32 reader threads attempt to acquire the read lock
- **THEN** all 32 readers SHALL acquire the read lock within 1ms of the writer releasing
- **AND** no reader SHALL wait for >1ms due to writer starvation

#### Scenario: `storage_read()` and `storage_write()` helpers work with new lock type
- **WHEN** `storage_read()` is called and the lock is free
- **THEN** it returns a `parking_lot::lock_api::RwLockReadGuard<'_, S>` immediately
- **AND** no `.unwrap()` is needed (the lock cannot be poisoned)
- **AND** no `Result` is returned (try_read is handled internally)

### Req 2: All call sites use the new lock type

The following files SHALL be updated to use `parking_lot::RwLock` consistently:

| File | Lines | Change |
|------|-------|--------|
| `src/execution_engine.rs` | 85, ~250 call sites | Field type + all `.read().unwrap()` / `.write().unwrap()` |
| `src/engine_builder.rs` | 8, ~20 call sites | Engine constructors wrap `Arc<parking_lot::RwLock<S>>` |
| `src/cbo_estimator.rs` | 14, ~10 call sites | Receives `Arc<parking_lot::RwLock<ExecutionStats>>` |
| `src/engine_select.rs` | ~3 call sites | `self.storage.read()` already becomes `self.storage_read()` (existing helper) |
| `src/execution_engine_tests.rs` | 8, ~10 call sites | Test fixtures |
| `crates/executor/src/stored_proc.rs` | 11, ~10 call sites | `StoredProcExecutor` constructor |
| `crates/executor/src/trigger.rs` | 12, ~10 call sites | `TriggerExecutor` constructor |
| `crates/mysql-server/src/lib.rs` | 18, ~30 call sites | Server runtime, `Arc<RwLock<...>>` types |

#### Scenario: No `.read().unwrap()` or `.write().unwrap()` remain
- **WHEN** the project is built with `cargo build --release`
- **THEN** the build SHALL succeed
- **AND** the static analysis SHALL show zero `.read().unwrap()` or `.write().unwrap()` on the storage lock

#### Scenario: Library and binary both build
- **WHEN** `cargo build --release -p sqlrustgo` is run
- **THEN** the build SHALL succeed
- **WHEN** `cargo build --release -p sqlrustgo-mysql-server` is run
- **THEN** the build SHALL succeed

### Req 3: `storage_write()` helper exists for write paths

The `ExecutionEngine` SHALL provide a `storage_write()` method parallel to `storage_read()`:

```rust
pub(crate) fn storage_write(&self) -> parking_lot::lock_api::RwLockWriteGuard<'_, S> {
    for attempt in 0..1000u32 {
        if let Some(g) = self.storage.try_write() {
            if attempt > 100 {
                log::warn!("storage_write contended for {} attempts", attempt);
            }
            return g;
        }
        std::thread::sleep(std::time::Duration::from_micros(100));
    }
    log::error!("storage_write timeout; falling back to blocking write");
    self.storage.write()
}
```

#### Scenario: `storage_write()` is used in INSERT/UPDATE/DELETE paths
- **WHEN** the engine executes an INSERT, UPDATE, or DELETE
- **THEN** it SHALL use `self.storage_write()` instead of `self.storage.write().unwrap()`
- **AND** the call SHALL NOT return a `Result` (no `?` operator needed)

### Req 4: `mysql-server/src/lib.rs` do_command_loop compiles

The `do_command_loop` function in `crates/mysql-server/src/lib.rs` creates and uses `Arc<RwLock<ExecutionEngine<...>>>`. After the type change:

- All `engine.write().map_err(...)` patterns → `engine.write()` (no Result)
- All `if let Ok(...) = ...read()...` patterns → direct `let ... = ...read()`
- The `if let Some(x) = .try_read()` patterns → direct `let x = .read()`

#### Scenario: `do_command_loop` accepts new RwLock type
- **WHEN** the server processes a COM_QUERY packet
- **THEN** `do_command_loop` SHALL compile and run with the new `parking_lot::RwLock<ExecutionEngine<...>>` type
- **AND** no error shall be returned to the client due to type mismatch

### Req 5: Hybrid workload verification (Issue #3648 mix ratio)

The fix SHALL be verified by running the hybrid workload for 15+ minutes with `--oltp-ratio 0.36` to hit Issue #3648's 18-22% mix ratio:

- INSERT count > 0
- UPDATE count > 0
- DELETE count > 0
- `query_errors == 0`
- `crud_errors == 0`
- `mix_ratio_actual` in [18.0, 22.0]
- `alert_triggered == false`

#### Scenario: Hybrid workload passes Issue #3648 acceptance
- **GIVEN** the fix is applied
- **WHEN** `hybrid_soak` is run for 15+ minutes with `--oltp-ratio 0.36` and 4 threads at 50 qps/thread
- **THEN** `SoakReport.mix_ratio_actual` SHALL be in [18.0, 22.0]
- **AND** all 6 acceptance criteria SHALL pass

### Req 6: Lock contention is provably absent

The fix SHALL be verified by sampling the server with macOS's `sample` tool:

- No thread SHALL be in `RwLock::lock_contended` state for >1% of samples
- No thread SHALL be in `semaphore_wait_trap` state for >5% of samples
- The server SHALL respond to `SELECT 1` in <50ms during sustained write load

#### Scenario: Server sample shows no lock contention
- **WHEN** the server is sampled with `sample <pid> 1` during sustained write load
- **THEN** the count of `RwLock::lock_contended` occurrences in the sample output SHALL be 0
- **AND** the count of `semaphore_wait_trap` occurrences in the sample output SHALL be <5

## Acceptance Criteria

All 6 requirements verified by:

1. **Build**: `cargo build --release -p sqlrustgo` and `cargo build --release -p sqlrustgo-mysql-server` both succeed
2. **Unit test**: existing `tests/execution_engine_tests.rs` passes
3. **No deadlock**: hybrid_soak runs 15+ min at 200 qps, `sample` shows 0 `lock_contended`
4. **Mix ratio**: hybrid_soak with `--oltp-ratio 0.36` shows 18-22% CRUD
5. **WAL persistence**: queries return correct row counts after 15+ min
6. **Issue 3672 closure**: comment posted on Issue #3672 linking this change
