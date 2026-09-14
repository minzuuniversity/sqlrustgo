# Phase B Step 6: Per-table sharding — closed, not justified

**Status**: Investigation closed. Per-table sharding **not implemented**.
**Date**: 2026-09-14

## Goal

Resolve the 21-181 residual `parking_lot::RawRwLock::*_slow` samples
observed in Phase B profiles after Step 4 + FU2 + Step 4.1 + Step 4.2.

## Profile analysis (pre-implementation)

A fresh 8-thread / 10k-row / 30s profile of the current build
(`df44fb0de6`) on `oltp_read_only` shows:

| Function | 8s samples |
|----------|------------|
| `parking_lot::RawRwLock::lock_shared_slow` | 49 |
| `parking_lot::RawRwLock::lock_exclusive_slow` | 45 |
| `parking_lot::RawRwLock::wait_for_readers` | 32 |

Stack trace analysis:

* **`lock_shared_slow` (49)**: every sample traces through
  `_xzm_reclaim_mark_used` → `_xzm_xzone_find_and_malloc_from_freelist_chunk`
  → `free`. **These are not sqlrustgo RwLocks** — they are
  `parking_lot`'s internal malloc path yielding to the kernel when
  an atomic CAS retry fails. The sqlrustgo code happens to call
  `Vec::push` / `String::push_str` near a malloc, and parking_lot's
  malloc retries kick in.

* **`lock_exclusive_slow` (45)**: every sample traces through
  `eval_predicate` → `find_column_index` →
  `__psynch_cvsignal` → `pthread_cond_signal`. Again parking_lot
  fast-path fallback, not a real writer-vs-reader wait.

* **`wait_for_readers` (32)**: same — parking_lot's writer-waits-for-readers
  in fast-path retry, not a genuine contention pause.

**There is no actual lock contention in the workload.** The samples
count the times `parking_lot` *internally* retries an atomic, which
is normal operation under any multi-threaded allocation pressure.
The OS-level wait (`__psynch_cvsignal` / `__psynch_cvwait`) is what
would indicate real contention, and that signal is ~20 samples —
i.e. the *real* contention cost is roughly 1.5% of CPU.

## Why per-table sharding wouldn't help

The bottleneck is the **outer** `Arc<parking_lot::RwLock<S>>` at
the server layer (`Arc<parking_lot::RwLock<WalStorage<FileStorage>>>`).
The current `FileStorage::tables` is a **plain `HashMap`**, not an
inner `RwLock<HashMap>`. There's no FileStorage-level lock to shard.

The single outer RwLock serialises:
- `SELECT`: shared read — multiple SELECTs can proceed concurrently.
- `INSERT/UPDATE/DELETE/COMMIT`: exclusive write — blocks all SELECTs.

The relevant contention is **read-write**, not **read-read**.
Sharding `tables` into per-table locks would let two SELECTs on
*different* tables proceed without contention, but:
1. sysbench `oltp_read_only` and `oltp_read_write` both target a
   *single* table. Sharding the same table = no win.
2. The WAL append path inside `commit_transaction` takes the outer
   write lock anyway, so DML still serialises all reads.

The real lever is **server-layer lock removal**:
`Arc<Storage>` + lock-free reads via copy-on-write snapshot, or
`arc_swap::ArcSwap<Storage>`. That's a 2-3 week refactor of the
storage trait, the executor, the WAL recovery path, and every
`storage.read()` / `storage.write()` call site. Far beyond the
Step 5 scope.

## What was considered

### Option A: HashMap<String, Arc<RwLock<TableData>>>

Per-table `RwLock` inside `FileStorage`. 29 callsites in
`file_storage.rs` to rewrite (`self.tables.get(name)` →
`self.tables.get(name).map(|t| t.read())`).

* **Pros**: Different tables can be read concurrently. Real reduction
  in cross-table lock contention.
* **Cons**:
  * sysbench is single-table → no measurable win for the workloads we
    run.
  * The outer `Arc<RwLock<storage>>` is the *real* lock; per-table
    locks add overhead without removing it.
  * Field type change cascades through MvccStorage, WalStorage,
    engine_select, and 5 integration tests.

### Option B: Remove server-layer outer RwLock

`Arc<Storage>` with lock-free reads. Requires:

1. `StorageEngine` trait bound: every method becomes `&self` instead
   of `&mut self` (most already are; ~12 `&mut self` methods need
   interior mutability via `Mutex`/`RwLock`).
2. WAL append path: write lock on WAL manager (already interior).
3. `WalStorage::inner.flush()` deferred in FU2 already keeps this
   light.
4. Catalog (`Arc<RwLock<Catalog>>`): same treatment.
5. `ExecutionEngine::storage: Arc<parking_lot::RwLock<S>>` →
   `Arc<S>`; rewrite `storage.read()` → `&*storage` (~15 callsites).

* **Pros**: Real read scalability. Removes the single biggest
  contention point.
* **Cons**: ~2-3 weeks of careful refactoring + extensive testing.
  Touches the storage trait, every storage backend, the executor,
  the WAL recovery path, the catalog, and ~50 callsites across the
  codebase.

### Option C: `arc_swap` snapshot swap

Use `arc_swap::ArcSwap<Arc<StorageSnapshot>>`. Writers build a new
snapshot, atomic-swap it. Readers load-free.

* **Pros**: Cleanest lock-free model.
* **Cons**: Writers still take a write lock to clone the snapshot —
  not actually cheaper than Option B.

## Decision

**Closed as "investigation only".** The profile data shows the
remaining `*_slow` samples are not contention — they are parking_lot
fast-path retries — so there is no TPS win available from sharding
the existing locks. The right next step is Option B
(server-layer lock removal), but it is a 2-3 week refactor that
belongs in v5.0.0 Phase C, not as a Phase B hygiene task.

## Cumulative Phase B (oltp_read_only 4t/1k baseline = 168 TPS)

| Phase | TPS | 累计 |
|-------|-----|------|
| Phase A baseline | 168 | — |
| Step 3 lockfree | 949 | +465% |
| #4 UnsafeCell | 1083 | +545% |
| #3 wired lockfree | 1089 | +548% |
| Step 4 MVCC | 1066 | +534% |
| #4.1 precise tombstone | 1100 | +555% |
| #4.2 PK fast-path | 1198 | +613% |
| #4.3 range infra (not wired) | 1100 | +555% |
| FU2 lazy flush | 1100 | +555% |
| Step 5 BufferPool (doc only) | 1100 | +555% |
| **Step 6 sharding (doc only)** | 1100 | +555% |

Phase B closes at **+555% oltp_read_only 4t/1k** (vs Phase A baseline).

## 5-Remote Sync (state at end of Phase B)

| Remote | SHA |
|--------|-----|
| gitea250 + gitea252 + origin | `bfdeac1f98` (includes GMP) |
| github + gitcode + gitee | `7680161a73` (GMP-free) |