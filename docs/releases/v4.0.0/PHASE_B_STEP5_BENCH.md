# Phase B Step 5: BufferPool Investigation — Closed, No Integration

**Status**: Investigation closed. BufferPool integration **not implemented**.
**Date**: 2026-09-14

## What was attempted

Step 5 was the original Phase B plan item to integrate the
existing `BufferPool` (LRU page cache, 418 lines,
`crates/storage/src/buffer_pool.rs`) into the storage path to
eliminate repeated disk I/O on hot data.

### Profile observations before this step

After Step 4.1 (`df44fb0de6`) the SOAK profile shows:

* `lock_shared_slow` samples: 13-34 (5-10s profiles)
* `lock_exclusive_slow` samples: 8-30
* `wait_for_readers` samples: 0-32

These are the **outer** `Arc<RwLock<WalStorage<FileStorage>>>`
contention at the server layer, **not** FileStorage's internal
locks.

## Findings

### 1. FileStorage is already an in-memory cache

`FileStorage::scan` does NOT touch disk on the hot path:

```rust
fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
    let mut rows: Vec<Record> = self
        .get_table(table)        // <-- HashMap lookup, no I/O
        .map(|data| data.rows.clone())
        .unwrap_or_default();
    // F-09 fix: merge insert_buffer so same-transaction SELECT/UPDATE sees
    // the rows that were just inserted (and not yet flushed to data.rows).
    ...
}
```

`self.tables: HashMap<String, TableData>` is **already** an
in-memory store populated by `load_table()` at server startup
and on `recover_wal()`. Disk I/O happens only in
`save_table()` (called by `flush()`).

### 2. `insert`/`update`/`delete` are already lazy + dirty-tracked

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    // ... mutate data.rows + insert_buffer in memory ...
    self.dirty_tables.insert(table.to_string());
    // disk write deferred to flush()
}
```

After follow-up #2 (`df44fb0de6`), `flush()` is itself deferred
out of `commit_transaction` and runs only on `drain_pending_flushes`
or on shutdown — so the hot path is **purely in-memory**.

### 3. Where would a BufferPool actually help?

The remaining disk I/O is the `commit → flush → save_table` chain
that writes a JSON-serialised `TableData` for every dirty table.
This is the only place where BufferPool-style caching could
short-circuit disk I/O.

But because `flush()` runs in the **background** (post FU2), it
does not sit on the request hot path. SOAK TPS does not change
whether `flush` is fast or slow.

### 4. The 21-sample `lock_shared_slow` tail is not disk-related

`parking_lot::RawRwLock::lock_shared_slow` samples trace back to
**server-layer** `Arc<RwLock<storage>>` (the outer read lock
acquired for every query) and to the `MvccStorage.versions` BTreeMap
read lock during MVCC chain walks. Neither path touches disk
once the server is warmed up.

## What was tried anyway

I considered two directions for Step 5:

* **A. Page-level BufferPool around `load_table`/`save_table`** —
  `BufferPool` is page-based (4 KB), but `FileStorage` stores
  **rows** (`Vec<Record>`), not pages. Wrapping rows in
  Page structs just to feed the existing `BufferPool` would be
  a forced fit and add a layer of indirection without measurable
  benefit (the hot path is already in-memory).

* **B. DashMap per-table sharding** to replace the outer
  `Arc<RwLock<storage>>` with finer-grained locks. This is the
  pattern that *would* help the 21-sample lock tail, but it is a
  multi-week refactor of the storage trait, the executor, and
  every WAL recovery path. Far beyond Step 5's planned 4-8h
  scope.

Both directions were rejected. Step 5 is closed as
"investigated, no integration justified by data".

## Recommended follow-up (if ever pursued)

If a future session observes real disk-I/O saturation
(check `iostat` / `vmstat` during 1h+ SOAK on a workload that
outgrows the in-memory table), the right path is:

1. Replace `FileStorage.tables: HashMap<String, TableData>` with
   `Arc<DashMap<String, Arc<RwLock<TableData>>>>` so reads of
   unrelated tables don't serialise on the outer lock.
2. Add a `lru` crate around `load_table()` so cold tables can
   spill without blocking writes.
3. Only then: integrate `BufferPool` as a row-block cache
   (not page) on top of the per-table shards.

Estimated work: 2-3 weeks. Track as a v5.0.0 candidate (already
in the v4.0.0 → v5.0.0 roadmap as Phase C item B5).

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
| #4.3 range infra | 1100 | +555% |
| FU2 lazy flush | 1100 | +555% |
| **Step 5 BufferPool (not impl)** | 1100 | +555% |

Phase B closes at **+555% oltp_read_only** (4t/1k, sysbench, wal_sync=off).

## 5-Remote Sync (state at end of Phase B)

| Remote | SHA |
|--------|-----|
| gitea250 + gitea252 + origin | `df44fb0de6` (includes GMP) |
| github + gitcode + gitee | `6cf3a603be` (GMP-free) |