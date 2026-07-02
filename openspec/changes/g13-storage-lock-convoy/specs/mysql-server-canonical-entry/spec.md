# Delta — MySQL Server Storage Lock Migration

## MODIFIED Requirements

### Requirement: Storage lock uses `parking_lot::RwLock` with Fair policy

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

### Requirement: All call sites use the new lock type

The following files SHALL be updated to use `parking_lot::RwLock` consistently:

- `src/execution_engine.rs` (storage field, ~40 call sites)
- `src/engine_builder.rs` (engine constructors, ~20 call sites)
- `src/cbo_estimator.rs` (~10 call sites)
- `src/execution_engine_tests.rs` (~10 test fixture sites)
- `crates/executor/src/stored_proc.rs` (~10 sites)
- `crates/executor/src/trigger.rs` (~10 sites)
- `crates/mysql-server/src/lib.rs` (~30 server runtime sites)

#### Scenario: No `.read().unwrap()` or `.write().unwrap()` remain
- **WHEN** the project is built with `cargo build --release`
- **THEN** the build SHALL succeed
- **AND** the static analysis SHALL show zero `.read().unwrap()` or `.write().unwrap()` on the storage lock

#### Scenario: Library and binary both build
- **WHEN** `cargo build --release -p sqlrustgo` is run
- **THEN** the build SHALL succeed
- **WHEN** `cargo build --release -p sqlrustgo-mysql-server` is run
- **THEN** the build SHALL succeed

### Requirement: `storage_write()` helper exists for write paths

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

## ADDED Requirements

### Requirement: Lock contention is provably absent

The fix SHALL be verified by sampling the server with macOS's `sample` tool:

- No thread SHALL be in `RwLock::lock_contended` state for >1% of samples
- No thread SHALL be in `semaphore_wait_trap` state for >5% of samples
- The server SHALL respond to `SELECT 1` in <50ms during sustained write load

#### Scenario: Server sample shows no lock contention
- **WHEN** the server is sampled with `sample <pid> 1` during sustained write load
- **THEN** the count of `RwLock::lock_contended` occurrences in the sample output SHALL be 0
- **AND** the count of `semaphore_wait_trap` occurrences in the sample output SHALL be <5

### Requirement: Hybrid workload passes Issue #3648 acceptance

The fix SHALL be verified by running the hybrid workload for 15+ minutes with `--oltp-ratio 0.36` to hit Issue #3648's 18-22% mix ratio.

#### Scenario: Hybrid workload passes Issue #3648 acceptance
- **GIVEN** the fix is applied
- **WHEN** `hybrid_soak` is run for 15+ minutes with `--oltp-ratio 0.36` and 4 threads at 50 qps/thread
- **THEN** `SoakReport.mix_ratio_actual` SHALL be in [18.0, 22.0]
- **AND** `SoakReport.query_errors` SHALL be 0
- **AND** `SoakReport.crud_errors` SHALL be 0
- **AND** `SoakReport.alert_triggered` SHALL be false
