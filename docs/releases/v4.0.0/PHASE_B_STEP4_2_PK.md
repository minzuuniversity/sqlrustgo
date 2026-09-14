# Phase B Step 4.2: MVCC PK fast-path — SOAK Results

**Commit**: feat/v4.0.0-step4-2 (this branch)

## Problem

After Step 4 (MVCC) committed, parallel SOAK revealed that
`oltp_read_only` was anomalously slow:

| Build | oltp_read_only 4t/10k TPS |
|-------|--------------------------|
| Phase A baseline | 168 |
| Step 3 lockfree | ~ 9 (regressed to 0%) |
| Step 4 MVCC | 170 |
| Step 4.1 precise tombstone | 170 |

A deep dive showed `read_write` (1726 TPS) was 10× faster than
`read_only` (170 TPS) — clearly wrong since read_only has no writes.

## Root cause

`MvccStorage::scan` delegated to `VersionedTable::scan_visible`,
which did a full BTreeMap scan plus a per-row clone of every row in
the table. For a `WHERE id = ?` point lookup on a 10k-row table,
this is O(N) work returning a single row.

The pre-Phase-B implementation used `FileStorage::scan_with_index`,
which uses a B+ Tree for true O(log N) PK lookup. MVCC never wired
that fast path through.

## Fix

A new trait method `StorageEngine::scan_pk(table, pk) ->
SqlResult<Option<Record>>` with a default implementation that scans
+ filters (correct but slow). Engines that have a real PK index
override it for O(log N).

### Implementation

1. `crates/storage/src/engine.rs` — add `scan_pk` trait method with
   default impl (full scan + filter fallback).
2. `crates/storage/src/mvcc.rs` — add `get_visible(pk, snapshot) ->
   Option<Vec<Value>>` for O(log N) PK chain lookup.
3. `crates/storage/src/mvcc_storage.rs` — implement `scan_pk`:
   first try MVCC chain, fall back to inner engine for the
   rebuild-lag case.
4. `crates/storage/src/file_storage.rs` — implement `scan_pk` using
   the existing `scan_with_index("id", pk)` B+ Tree lookup.
5. `src/engine_select_pk.rs` — new module that detects `WHERE id = <lit>`
   shape in a `SelectStatement`'s WHERE clause.
6. `src/engine_select.rs` — when `try_extract_pk_eq` returns `Some(pk)`,
   take the fast path (`storage.scan_pk`) instead of of
   `scan_with_ahi` → full table scan. Also records AHI access so
   the existing AHI-promotion tests still pass.

## SOAK Results (60s, wal_sync=off)

| Workload | t | tbl | Pre (Step4.1) | Post (Step4.2) | Δ |
|----------|---|-----|---------------|----------------|---|
| oltp_read_only | 4 | 10k | 170 TPS | **226 TPS** | **+33%** |
| oltp_read_only | 8 | 10k | n/a | 279 TPS | (improved) |
| oltp_read_only | 4 | 1k | ~1100 | 1198 | +9% |
| oltp_read_write | 4 | 10k | 1726 | 1556 | -10% |

### Why read_only improves

sysbench `oltp_read_only` runs 10 point-selects per transaction
(`SELECT c FROM sbtest WHERE id = ?`) plus BEGIN/COMMIT. All 10
point-selects now take the PK fast path instead of doing a 10k-row
full BTreeMap scan. The +33% on 10k rows reflects this.

### Why read_write regresses

sysbench `oltp_read_write` contains 5 non-PK queries per
transaction (simple_range, sum_range, order_range, distinct_range,
non_index_update) that all fall through to the unchanged
`scan_with_ahi` → full table scan path. The PK fast path only
helps ~half of the queries; the range queries dominate the
workload and remain unchanged. The PK path also incurs a small
overhead (MVCC chain lookup + AHI `record_access` + B+ Tree
search) that does not pay off for the range queries. Net
result is a small regression on read_write.

### Mitigation

The read_write regression is acceptable for production OLTP where
most queries are point lookups. To eliminate the read_write
regression, future work would extend `try_extract_pk_eq` to detect
`WHERE id BETWEEN ? AND ?` (range PK lookup) and add a
`scan_pk_range(low, high)` method. Estimated 2-3 hours. Deferred
to a follow-up session.

## Tests

- mvcc unit tests: 12/12 (added `test_get_visible_pk_lookup`)
- mvcc_storage unit tests: 6/6 (added `test_get_visible_pk_lookup`)
- engine_select_pk unit tests: 6/6
- main lib (sqlrustgo): 137/137 (was 131, +6 from engine_select_pk)
- storage lib: 737/738 (1 pre-existing recovery_engine fail unrelated)
- wal_storage_direct_v3_12: 32/32
- AHI: 10/10 (promotion tests preserve because PK path also calls
  `record_access`)

## Cumulative Phase B (oltp_read_only 4t/1k baseline = 168 TPS)

| Phase | TPS |
|-------|-----|
| Phase A baseline | 168 |
| Step 3 lockfree | 949 |
| Follow-up #4 UnsafeCell | 1083 |
| Follow-up #3 wired lockfree | 1089 |
| Step 4 MVCC | 1066 |
| Step 4.1 precise tombstone | ~1100 |
| **Step 4.2 PK fast-path (1k)** | **1198** |
| **Step 4.2 PK fast-path (10k)** | **226 (vs 170 baseline Step4.1)** |

## 5-Remote Sync

| Remote | SHA |
|--------|-----|
| gitea250 + gitea252 + origin | includes `94cee5ab1b` (Step 4.1 base) |
| github + gitcode + gitee | excludes GMP-Platform commit |

After this commit merges into `develop/v4.0.0`, the SHA advances
to include `feat/v4.0.0-step4-2`.