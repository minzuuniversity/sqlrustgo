# Phase B Step 4.1: Fix oltp_read_write regression with precise MVCC tombstone

**Commit**: `step4.1` (data model enhancement)
**Parent**: Step 4 MVCC (`9e94bfbbe7`)

## Problem

After Step 4 (MVCC) committed, parallel SOAK revealed an
**oltp_read_write 4t/10k regression of -8%**:

| Build | oltp_read_write 4t/10k TPS |
|-------|---------------------------|
|
| Step 3 lockfree (pre-MVCC) | 1729.43 |
| Step 4 MVCC | **1591.05** (-8.0%) |

Profile (sample 5s, 4t, 30s run) showed:

| Function | Step3 samples | MVCC samples |
|----------|---------------|--------------|
| `_xzm_free` (malloc free) | 194 | 217 (+23) |
| `__psynch_cvwait` (kernel wait) | 58 | 69 (+11) |
| `String::clone` | 116 | 120 (+4) |

Memory allocation + kernel wait both up — the MVCC layer is doing
extra work that doesn't translate into throughput.

## Root cause

`MvccStorage::delete` (Phase 4, pre-Step-4.1) had this body:

```rust
fn delete(&mut self, table, _filters) -> SqlResult<usize> {
    let n = self.inner.delete(table, _filters)?;
    if n > 0 {
        let mvcc = self.mvcc_table(table);
        // Append a tombstone for EVERY visible row.
        let pairs = mvcc.scan_visible(mvcc.begin_snapshot());
        let ts = mvcc.next_snapshot_ts();
        for (pk, _) in pairs {
            mvcc.delete(&pk, ts, ts);
        }
    }
    Ok(n)
}
```

The problem: `inner.delete` only returns the **count** of deleted
rows. Without knowing which PKs were deleted, the MVCC wrapper
**tombstones every visible row** in the table — even if the user
only asked to delete 1 row out of 10,000.

For sysbench `oltp_read_write` at table=10k with ~3 DELETEs per
transaction, this means each DELETE does:
- 1 × `scan_visible` (O(N) clone of all visible rows)
- N × `mvcc.delete` (N tombstone version-chain appends)

= O(N) work per DELETE, totaling ~30k extra operations per
transaction.

## Fix: `delete_collect_pks`

A new trait method:

```rust
fn delete_collect_pks(&mut self, table: &str, filters: &[Value])
    -> SqlResult<Vec<Value>>;
```

Returns the **primary keys** of deleted rows (column 0 of each
deleted row). Default implementation returns an empty Vec (signaling
"don't know, fall back to coarse behavior"); engines that can do
better (FileStorage, MemoryStorage) override it.

`MvccStorage::delete` now does:

```rust
let removed_pks = self.inner.delete_collect_pks(table, _filters)?;
if removed_pks.is_empty() {
    // Coarse fallback: only on full-table wipe (filters.is_empty())
    if _filters.is_empty() { /* tombstone all visible */ }
    return Ok(0);
}
// Precise tombstone — only the PKs that were actually deleted.
let mvcc = self.mvcc_table(table);
let ts = mvcc.next_snapshot_ts();
for pk in &removed_pks {
    mvcc.delete(pk, ts, ts);
}
Ok(removed_pks.len())
```

For a DELETE of 1 row in a 10k table: **1 tombstone** instead of
**10,000**.

## SOAK results

| Workload | t | tbl | time | Step3 | Step4.0 | Step4.1 |
|----------|---|-----|------|-------|---------|---------|
| oltp_read_write | 4 | 10k | 60s | 1729.43 | 1591.05 (-8%) | **1726.51** ✓ |
| oltp_read_write | 8 | 10k | 60s | 1722.23 | 1704.07 | **1825.49 (+6%)** |
| oltp_read_only | 4 | 10k | 60s | 157.86 | 155.37 (-1.6%) | **170.08 (+7.7%)** |

**Step 4.1 fixes the 4t regression (1726 vs 1729, ≈Step3) and adds
+6% on 8t and +7.7% on 4t oltp_read_only.**

## Tests

- mvcc_storage unit tests: 5/5 (updated `test_delete_hides_rows`
  to verify only the matched PK is tombstoned, not the whole table).
- mvcc unit tests: 11/11.
- storage lib: 737 pass, 1 pre-existing fail (recovery_engine).
- main lib (sqlrustgo): 131/131.
- wal_storage_direct_v3_12: 32/32.
- AHI: 10/10.

## What it doesn't fix

`update` is still coarse — `MvccStorage::update` re-scans all
visible rows and appends a new version for each. For tables with
many updates and few rows this isn't a problem, but for oltp_read_write
on larger tables it could become one. Step 4.2 would address this.

## 5-Remote Sync

| Remote | SHA (after Step 4.1 merge) |
|--------|-----------------------------|
| gitea250 | see `git log` |
| gitea252 | see `git log` |
| github | see `git log` |
| gitcode | see `git log` |
| gitee | see `git log` |
| origin | see `git log` |