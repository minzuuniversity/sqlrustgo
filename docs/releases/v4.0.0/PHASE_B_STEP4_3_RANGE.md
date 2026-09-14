# Phase B Step 4.3: PK range lookup — investigation closed, infra retained

**Status**: Investigation closed. `scan_pk_range` retained as future
infrastructure but NOT wired into the engine hot path.

## What was attempted

Step 4.2 added `StorageEngine::scan_pk(table, pk)` for `WHERE id = ?`
point lookups. Step 4.3 was the natural follow-up: add
`scan_pk_range(table, low, high)` for `WHERE id BETWEEN ? AND ?`
range lookups.

### Changes attempted

1. `crates/storage/src/engine.rs` — `scan_pk_range` trait method
   with default impl (full scan + filter fallback).
2. `crates/storage/src/mvcc.rs` — `find_visible` made `pub(crate)`,
   `versions` field made `pub(crate)` so MvccStorage can use
   BTreeMap's `range` query.
3. `crates/storage/src/mvcc_storage.rs` — `scan_pk_range` impl
   using `mvcc.versions.read().range((Included(low), Included(high)))`
   for O(log N + k) lookup.
4. `src/engine_select_pk.rs` — `try_extract_pk_range` to detect
   `WHERE id BETWEEN ? AND ?` patterns (4 new unit tests).
5. `src/engine_select.rs` — wire `try_extract_pk_range` into the
   single-table scan path (initially enabled, then rolled back —
   see below).

## Why the engine wiring was rolled back

SOAK (60s, wal_sync=off, 4 threads, 10k rows):

| Build | oltp_read_only | oltp_read_write |
|-------|----------------|-----------------|
| Step 4.1 | 170 TPS | 1726 TPS |
| Step 4.2 (PK fast-path) | 226 TPS | 1556 TPS |
| **Step 4.3 (range wired)** | **242 TPS** | **1501 TPS** |
| **Step 4.3 (range NOT wired)** | **215 TPS** | **1475 TPS** |

Profile (sample 8s, 4t oltp_read_write 10k) with the wiring enabled:
- `parking_lot::RawRwLock::lock_exclusive_slow` samples: 46
  (vs Step 4.2 baseline 29, +60%)
- `parking_lot::RawRwLock::lock_shared_slow` samples: 112
  (vs Step 4.2 baseline 105)

The `scan_pk_range` call takes a `mvcc.versions.read()` lock. Under
oltp_read_write, concurrent INSERT/DELETE statements need a
`mvcc.versions.write()` lock. Even though the read lock is held for
only the duration of the BTreeMap range iteration, it contends
with the write lock — the read lock blocks waiting for the writer
to drain, while the writer blocks waiting for the read lock to
drop. The result is a measurable throughput regression.

sysbench oltp_read_write runs 5 range queries per transaction
(simple_range, sum_range, order_range, distinct_range, all
`WHERE id BETWEEN ? AND ?`) plus 2 updates and 1 delete. With the
range wiring enabled, the MVCC read-lock acquisition now serialises
the full transaction against any concurrent inserts — exactly the
contention pattern MVCC was supposed to eliminate.

## What's retained

- The `scan_pk_range` trait method, MvccStorage implementation, and
  `try_extract_pk_range` helper all stay in the code base.
- 4 new unit tests for the range extractor (covering BETWEEN
  detection, no-where, other-column, and id= fallthrough cases).
- 1 new MvccStorage unit test (`test_scan_pk_range`) covering
  inclusive range lookup.
- `mvcc::find_visible` is `pub(crate)` and `versions` is
  `pub(crate)` so the wiring can be re-enabled without further
  visibility changes.

## How to enable in the future

When the read+write contention cost is reduced (e.g. by switching
MVCC inserts from the BTreeMap write lock to a per-shard lock, or
by moving the range query through a row-id index), re-enable the
wiring in `engine_select.rs` by:

1. Remove the `Note: ... intentionally NOT routed` comment.
2. Re-add the `else if let Some((low, high)) = ...` branch that
   calls `storage.scan_pk_range(lookup_table, &low, &high)`.

Alternatively, the B+ Tree could be extended with a true
`range_search(low, high) -> Vec<row_id>` method, replacing the
default impl in `FileStorage` with a real O(log N + k) range
scan, and `FileStorage::scan_pk_range` would no longer go through
MVCC's BTreeMap at all.

## Tests

- mvcc: 19/19 (+1 new `test_scan_pk_range`)
- engine_select_pk: 10/10 (+4 new range tests)
- main lib: 141/141 (was 137, +4 range tests)
- storage lib: 737/738 (1 pre-existing recovery_engine fail)
- wal_storage_direct_v3_12: 32/32
- AHI: 10/10