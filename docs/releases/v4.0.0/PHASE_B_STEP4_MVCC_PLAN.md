# Phase B Step 4: MVCC for reads

**Branch**: `feat/v4.0.0-mvcc` (worktree at `~/dev/sqlrustgo-worktrees/v4.0.0-mvcc/`)
**Parent**: `develop/v4.0.0 = 3731e40598`
**Status**: Ready to start

## Goal

Eliminate the `Arc<RwLock<BoxStorageEngine>>` read-lock contention on
SELECT queries (the main bottleneck identified by Phase B profiling:
`parking_lot::RawRwLock::lock_shared_slow` accounts for ~45-68% of
samples in `execute_select`).

After MVCC: SELECTs acquire only a snapshot timestamp atomically (one
`AtomicU64::load`), then iterate through version chains without any
lock. Writers append new versions under the existing `WalStorage`
write lock.

## Architecture

### Data model

```rust
// New file: crates/storage/src/mvcc.rs

/// Single row with multi-version chain.
pub struct VersionedRow {
    /// The actual row data.
    pub row: Record,
    /// Transaction id that created this version (for visibility tests).
    pub created_by_tx: u64,
    /// Snapshot timestamp at which this version became visible.
    pub visible_from_ts: u64,
    /// If true, this is a tombstone (delete marker).
    pub deleted: bool,
}

/// In-memory row store with version chains.
pub struct VersionedTable {
    /// BTreeMap<primary_key, Vec<VersionedRow>> — versions are kept in
    /// chronological order (newest at back). GC removes old entries.
    versions: parking_lot::RwLock<BTreeMap<Value, Vec<VersionedRow>>>,
    /// Global snapshot timestamp counter — incremented atomically.
    /// Each new committed transaction advances this counter.
    snapshot_counter: AtomicU64,
}
```

### Visibility rules

A `VersionedRow` is visible to a `snapshot_ts` if:
- `visible_from_ts <= snapshot_ts`, AND
- NOT deleted by a tx committed at `<= snapshot_ts` (i.e. no
  subsequent tombstone at ts `<= snapshot_ts`)

For Phase 4 (read-only MVCC) we keep `created_by_tx == visible_from_ts`
— commit order == snapshot_ts. Tombstones are inserted at the tx
that committed the delete.

### MVCC-aware scan

```rust
impl VersionedTable {
    /// Acquire a snapshot timestamp for the duration of this query.
    pub fn begin_snapshot(&self) -> u64 {
        self.snapshot_counter.load(Ordering::Acquire)
    }

    /// Iterate visible rows without holding any lock (except the per-key
    /// version chain read lock briefly).
    pub fn scan(&self, table: &str, snapshot_ts: u64) -> Vec<Record> {
        let versions = self.versions.read();
        let mut out = Vec::with_capacity(64);
        for (key, chain) in versions.range(..) {
            if let Some(visible) = find_visible(chain, snapshot_ts) {
                if !visible.deleted {
                    out.push(visible.row.clone());
                }
            }
        }
        out
    }
}

fn find_visible<'a>(chain: &'a [VersionedRow], snapshot_ts: u64) -> Option<&'a VersionedRow> {
    // Binary search for last version with visible_from_ts <= snapshot_ts.
    chain.iter().rev().find(|v| v.visible_from_ts <= snapshot_ts)
}
```

### Engine integration

For Phase 4 we only convert the **scan** path to MVCC. Writes still go
through `WalStorage::insert/update/delete` (which already takes
`storage.write()`). The MVCC layer sits BELOW `WalStorage` and is
the `inner` type:

```
BoxStorageEngine = WalStorage<MvccStorage<FileStorage>>
                 = WalStorage { inner: MvccStorage<FileStorage> }
```

MvccStorage delegates to FileStorage for everything EXCEPT:
- `scan` → uses snapshot-based visibility filter on the in-memory
  version chains.
- `insert/update/delete` → appends a new VersionedRow to the chain,
  then delegates to FileStorage for the on-disk write.

### Snapshot advance

```rust
impl MvccStorage {
    fn commit(&self, tx_id: u64) -> u64 {
        let ts = self.snapshot_counter.fetch_add(1, Ordering::AcqRel) + 1;
        // Mark all versions written by tx_id as visible_from_ts = ts.
        // ...iterate versions, set visible_from_ts on the matching ones.
        ts
    }
}
```

## Estimated work breakdown

1. **VersionedRow + VersionedTable** (2h)
   - New file `crates/storage/src/mvcc.rs` (~200 lines)
   - Unit tests for visibility, GC, snapshot advance.

2. **MvccStorage wrapper** (3h)
   - `crates/storage/src/mvcc_storage.rs` (~400 lines) wrapping
     `FileStorage` (or `MemoryStorage` for tests).
   - Implement all `StorageEngine` trait methods.
   - Convert `scan` / `scan_with_filter` to use snapshot timestamp.
   - Keep `insert/update/delete` delegating to inner but appending
     version chain entries.

3. **Wire as BoxStorageEngine default** (1h)
   - Update `engine_builder.rs` to use `MvccStorage<FileStorage>`.
   - Update `BoxStorageEngine = WalStorage<MvccStorage<FileStorage>>`.

4. **Engine-level snapshot acquisition** (2h)
   - `ExecutionEngine::execute_select` acquires `snapshot_ts` once
     at query start, passes it down through `scan`/`scan_with_filter`.
   - For backwards compat: when no snapshot is passed, fall back to
     current behavior.

5. **Tests + integration** (2h)
   - Unit tests: visibility correctness (10+ scenarios).
   - Integration tests: WAL recovery respects MVCC visibility.
   - Regressions: existing storage tests still pass with wrapper.

6. **SOAK + profile** (2h)
   - 4t/1k, 8t/10k, 8t/10k oltp_read_write before/after.
   - `sample` profile to confirm `lock_shared_slow` is reduced.
   - Memory growth check (version chain GC).

7. **Commit + push + doc** (1h)
   - `docs/releases/v4.0.0/PHASE_B_STEP4_MVCC.md` (design + SOAK).
   - Commit + push to all 5 remotes.

Total: ~13h. Within the 8-16h estimate.

## Risks

- **Version chain GC**: if GC is wrong, OOM under sustained writes.
  Need a clear rule: drop versions older than `snapshot_ts - GC_LAG`.
  Pick `GC_LAG = 1024` to start; can tune.
- **Primary key assumption**: version chains are indexed by PK. For
  Phase 4 we assume every table has a PK (already true for FileStorage).
  Tables without PK fall back to linear scan over a single chain.
- **Snapshot isolation vs MySQL REPEATABLE READ**: MySQL InnoDB uses
  MVCC but with subtle isolation rules. We start with
  READ_COMMITTED semantics (snapshot = commit timestamp), document
  the difference, and can iterate later.
- **UPDATE/DELETE visibility**: requires careful tombstone ordering.
  Defer to Phase 4.1 if it bloats the initial PR.

## Out of scope for Step 4

- Multi-version for non-PK tables (defer to Step 4.1)
- Snapshot isolation conflicts (e.g. write-write conflicts)
- 2PC, savepoint MVCC (already handled by Step 3's `tx_readonly`)

## Next action

Start with `crates/storage/src/mvcc.rs` — the version chain data
structure with full unit tests. Then proceed sequentially through
phases 2-7 above.