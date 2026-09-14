# v5.0.0 Phase C: Server-layer lock removal — analysis & plan

**Author**: sqlrustgo core team
**Date**: 2026-09-14
**Status**: Design document — awaiting approval before implementation
**Target**: close the oltp_read_only vs oltp_read_write TPS gap by removing
the server-layer outer `Arc<RwLock<S>>` and making SELECT lock-free.

## Goal

| Workload | Current (v4.0.0 + Phase B) | Target (v5.0.0 Phase C) |
|----------|---------------------------|--------------------------|
| oltp_read_only 4t/10k | 215 TPS | 1500–1700 TPS (≈ 7–8×) |
| oltp_read_only 8t/10k | 273 TPS | 1800–2200 TPS (≈ 7–8×) |
| oltp_read_write 4t/10k | 1690 TPS | 1900–2300 TPS (≈ 1.1–1.4×) |
| oltp_read_write 8t/10k | 1825 TPS | 2100–2500 TPS (≈ 1.1–1.4×) |

Reading should match the write path: the extra cost of `BEGIN + 10×SELECT +
COMMIT` in `oltp_read_only` (12 statements/txn) should be roughly the same
as the per-statement cost in `oltp_read_write` (22 statements/txn) once the
server-side `engine.read()` lock-acquire is removed.

## Background: why is oltp_read_only so slow today?

Per-txn statement count is the dominant factor, **not** lock contention:

* `oltp_read_only`: `BEGIN + 10×point_select + COMMIT` = 12 statements/txn.
* `oltp_read_write`: `BEGIN + 22 statements + COMMIT` ≈ 22/txn.
* Ratio 22/12 ≈ 1.8×, which matches the observed read_write / read_only
  ratio at the txn level for similarly-sized payloads.

`oltp_read_only` runs **12 statements × 8 threads = 96 server-side
`engine.read()` lock acquires per second**. Each acquire costs
~30–50 ns (parking_lot atomic CAS). The parking_lot fast-path retries
are what `sample` reports as `lock_shared_slow` (49 samples in 8s of an
8-thread profile). These are not real contention waits — the OS-level
`__psynch_cvwait` count is only ~20 samples (≈ 1.5% of CPU).

Removing the outer read lock makes the 96 acquires vanish and lets
each connection execute its `BEGIN + 10×SELECT + COMMIT` as 12 straight
forward function calls.

## Current architecture (the bottleneck)

```
┌────────────────────────┐
│  MySQL connection      │
│  thread (N of them)    │
└──────────┬─────────────┘
           │ per query
           ▼
   engine.read() ───────────────────┐  (parking_lot RwLock, 8 threads
           │                          │   contend on a single shared lock)
           ▼                          │
   execute_select()                    │
           │                          │
           ▼                          │
   self.storage.read()                 │
           │                          │
           ▼                          │
   WalStorage.scan() / .scan_pk()       │
```

Two outer RwLock acquisitions per query (`engine.read()` and then
`self.storage.read()`). Both contend across all 8 connection threads.

## Target architecture

```
┌────────────────────────┐
│  MySQL connection      │
│  thread (N of them)    │
└──────────┬─────────────┘
           │ per query
           ▼
   Arc::clone(&engine) → cheap atomic, no lock
           │
           ▼
   &Engine::execute_select() ──── no outer lock
           │
           ▼
   Arc::clone(&storage) → cheap atomic, no lock
           │
           ▼
   &WalStorage::scan() / .scan_pk() ──── read-only &self methods,
                                         no outer lock
```

Writes (`INSERT/UPDATE/DELETE/COMMIT`) take a short-lived write lock
on a per-engine `Mutex<S>` added internally. They do **not** hold any
outer RwLock.

## Implementation plan (estimated 2–3 weeks)

### Phase C.0 — preparatory safety analysis (1 day)

* Catalogue of every `&mut self` method on `StorageEngine` and confirm
  each can be rewritten as `&self` + internal `Mutex`/`RwLock` (most
  already are inside `WalStorage`; the `FileStorage::insert` path
  is the largest).
* Catalogue of every `Arc<RwLock<S>>` call site: confirmed
  ~180 across `src/` and `crates/`. Of those, ~165 are `engine.read()`
  / `storage.read()` (read paths); the remaining ~15 are `engine.write()`
  / `storage.write()` (write paths).

### Phase C.1 — internal locking inside `FileStorage` (3 days)

Add `pub(crate) struct FileStorage { ..., write_lock: parking_lot::Mutex<()> }`.
Convert every `&mut self` method (34 total) to `&self`, taking
`write_lock` internally only when mutating `tables` or `insert_buffer`.

Risk: data races between concurrent read and write. Mitigations:
* `scan` already returns a fresh `Vec<Record>` clone — reads are
  decoupled from in-flight writes by `data.rows.clone()`.
* MVCC layer (if used) clones from the version chain per row.
* DDL (`drop_table`, `create_table`) is rare; holding `write_lock` for
  its duration is fine.

### Phase C.2 — convert `Arc<RwLock<S>>` → `Arc<S>` in `ExecutionEngine` (2 days)

* Change `pub(crate) storage: Arc<parking_lot::RwLock<S>>` to
  `pub(crate) storage: Arc<S>`.
* Update `storage_read()` / `storage_write()` callers — `storage_write`
  becomes `self.storage.write_lock.lock()` (or call a helper that
  returns a serialised handle).
* Update all internal `self.storage.read()` → `&*self.storage` (no
  lock).

### Phase C.3 — server-lib wiring (2 days)

* `handle_connection`, `do_command_loop`, ephemeral test helpers:
  replace `Arc<RwLock<BoxStorageEngine>>` with
  `Arc<BoxStorageEngine>` (4 type-level changes).
* `let eng = engine.read();` → `let eng = &*engine;` in the SELECT
  dispatch site (and equivalent in the SHOW / DESCRIBE branches).
* `let mut eng = engine.write();` → `let mut eng = &mut **Arc::get_mut(&engine).unwrap();` —
  or, simpler, hand the write path a `MutexGuard<ExecutionEngine>`.

### Phase C.4 — WalStorage + commit_lockfree paths (3 days)

`WalStorage::commit_lockfree` and `recover_split_mut` currently take
`&mut self` via `inner_mut()`. Need to verify these still serialise
correctly under the new `&self` model. Specifically:

* `commit_transaction_lockfree(&self)` already takes `&self`. It calls
  `as_inner_mut()` (via `unsafe { &mut *self.inner.get() }`). Verify
  this still holds after we remove the outer RwLock.
* `recover_split_mut(&mut self)` is used by `engine_builder::recover_wal`
  at startup, which holds the outer write lock. With `Arc<S>` and no
  outer RwLock, recovery needs to run single-threaded (no concurrent
  queries during recovery). Verify or guard.

### Phase C.5 — test harness and feature flag (1 day)

Add `Arc<S>` as a generic parameter so callers can opt into the new
mode. Ship a `--server-arc-storage` CLI flag (default off) that
constructs the engine with `Arc<S>` instead of `Arc<RwLock<S>>`.
The flag lets us deploy and rollback safely.

### Phase C.6 — regression + SOAK + commit (2 days)

* `cargo test --lib` (141 tests + new paths from C.1–C.4).
* Parallel 1h SOAK (NEW vs OLD baseline, on `oltp_read_only` and
  `oltp_read_write`).
* Profile with `sample 8 5` on 8-thread 10k-row workload; confirm
  `lock_shared_slow` samples drop to <5 per 5s.

## Risk register

| Risk | Mitigation |
|------|------------|
| Torn reads: a writer mutates `data.rows` while a reader is iterating | `scan` already returns a `data.rows.clone()` — readers get a fresh `Vec`. After C.1, the writer takes `write_lock`; readers either see the old or new table state, never a torn one. |
| MVCC version-chain writes racing with scans | MVCC `versions` is a `parking_lot::RwLock<BTreeMap>` internally (added in Step 4 follow-up #4). `get_visible` takes a read lock; inserts take a write lock. Already safe. |
| `execute_select` holding `&ExecutionEngine` while invoking a method that needs `&mut ExecutionEngine` | Phase C.2 splits `ExecutionEngine` reads (cheap `&self`) from writes (which take `write_lock` separately). The borrow checker enforces this statically. |
| `recover_wal` running concurrently with queries at startup | Current behaviour: `recover_wal` runs synchronously in `with_wal_recovery` before serving any connection. New behaviour unchanged — keep that invariant. |
| WAL replay corruption from concurrent reads | WAL replay only happens during `recover_wal`, before any connection is accepted. Not a hot-path concern. |

## Test plan

| Test | Pass criterion |
|------|----------------|
| `cargo test --lib` | 141 + new tests all pass |
| `cargo test --test wal_storage_direct_v3_12` | 32/32 pass |
| 1h SOAK oltp_read_only 8t/10k | ≥1500 TPS, RSS <80 MB, 0 reconnects, 0 errors |
| 1h SOAK oltp_read_write 8t/10k | ≥2000 TPS, RSS <80 MB, 0 reconnects, 0 errors |
| Concurrent reads + writes crash test | 8 threads × 10 min, then check `data_rows` integrity via `SELECT COUNT(*)` matches expected |
| Profile sample 5s | `lock_shared_slow` < 5 samples; no `lock_exclusive_slow` from `eval_predicate` path |

## Estimated cost & risk

* Engineering: 2–3 weeks of focused work, 1 engineer.
* Risk: medium. The StorageEngine trait touches 180 call sites; an
  interim commit per phase (C.1, C.2, C.3, C.4) keeps the change
  reviewable and bisectable.
* Rollback: `--server-arc-storage` flag default off. If a regression
  appears in production, the engine can be restarted without the
  flag to restore current behaviour. No schema migration, no data
  format change.

## Cumulative Phase B + C outlook

| Phase | Workload | TPS | vs Phase A |
|-------|----------|-----|------------|
| Phase A baseline | oltp_read_only 4t/1k | 168 | — |
| Phase B done | oltp_read_only 4t/1k | ~1100 | +555% |
| **Phase C target** | oltp_read_only 4t/1k | **~2500–3500** | **+1400–2000%** |
| MySQL 8.0 reference | oltp_read_only 4t/1k | ~1500–2000 | — |

Phase C takes sqlrustgo past MySQL 8.0's lower bound on the
oltp_read_only workload — a long-standing target. Reaching
PostgreSQL/SQLite-WAL performance still requires v6.0.0 work
(vectorised execution, planner improvements).

## References

* `docs/releases/v4.0.0/PERFORMANCE_OPTIMIZATION_ROADMAP.md` —
  Phase C roadmap items B5 (BufferPool), C2 (MVCC), C3
  (vectorised execution).
* `docs/releases/v4.0.0/PHASE_B_MINISTEP_READ.md` — Mini Step
  analysis showing why this change is required.
* `docs/releases/v4.0.0/PHASE_B_STEP5_BENCH.md` — Step 5
  BufferPool investigation, leading to C.5.
* `docs/releases/v4.0.0/PHASE_B_STEP6_SHARD.md` — per-table
  sharding investigation, leading to C.1.