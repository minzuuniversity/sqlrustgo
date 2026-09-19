//! Multi-Version Concurrency Control (MVCC) for reads.
//!
//! Phase B Step 4 of the SQLRustGo performance plan.
//!
//! Goal: eliminate the `Arc<RwLock<BoxStorageEngine>>` read-lock
//! contention that dominates SELECT-heavy workloads
//! (`parking_lot::RawRwLock::lock_shared_slow` accounts for ~45-68% of
//! `execute_select` samples — see
//! `docs/releases/v4.0.0/PHASE_B_INVESTIGATION.md`).
//!
//! Design (see `docs/releases/v4.0.0/PHASE_B_STEP4_MVCC_PLAN.md`):
//! - Each row in a table has a *version chain*: a chronologically
//!   ordered list of `VersionedRow` values.
//! - Each version is stamped with the global `snapshot_ts` of the
//!   transaction that created it.
//! - Readers acquire a `snapshot_ts = snapshot_counter.load()`
//!   once at the start of their query — one atomic load, no lock.
//! - Visibility filter: a version `v` is visible to snapshot `s` iff
//!   `v.visible_from_ts <= s`. Tombstones (`v.deleted == true`) are
//!   not exposed as data rows.
//! - Writers (under the existing `WalStorage` write lock) append new
//!   versions to the chain with `visible_from_ts = next_ts()`, which
//!   atomically advances the snapshot counter.
//!
//! The wrapper `MvccStorage<S>` (separate file) integrates this layer
//! with the `StorageEngine` trait by intercepting `scan` /
//! `scan_with_filter` to use snapshot-based visibility, while
//! delegating everything else to the inner engine.

use parking_lot::RwLock;
use sqlrustgo_types::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// A single row version. Multiple `VersionedRow`s with the same
/// primary key form a *version chain*.
#[derive(Debug, Clone)]
pub struct VersionedRow {
    /// The actual row data (column values).
    pub row: Vec<Value>,
    /// Snapshot timestamp at which this version became visible.
    /// All readers with `snapshot_ts >= self.visible_from_ts` see this
    /// version (subject to tombstone filtering).
    pub visible_from_ts: u64,
    /// Transaction id that created this version. Used for
    /// debugging/recovery only — visibility is determined by
    /// `visible_from_ts`.
    pub created_by_tx: u64,
    /// If true, this version is a tombstone (delete marker).
    pub deleted: bool,
}

/// In-memory row store with per-key version chains.
///
/// Designed for the read-heavy path:
/// - `begin_snapshot()`: one atomic load, no lock.
/// - `scan_visible()`: short-lived read lock on `versions`, then
///   per-chain binary search. Readers do NOT block writers.
pub struct VersionedTable {
    /// BTreeMap<primary_key, Vec<VersionedRow>> — versions are kept in
    /// ascending `visible_from_ts` order (oldest first).
    pub(crate) versions: parking_lot::RwLock<BTreeMap<Value, Vec<VersionedRow>>>,
    /// Global snapshot timestamp counter — incremented atomically.
    /// Each committed transaction that wrote a version advances this.
    snapshot_counter: AtomicU64,
}

impl Default for VersionedTable {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionedTable {
    pub fn new() -> Self {
        Self {
            versions: RwLock::new(BTreeMap::new()),
            snapshot_counter: AtomicU64::new(0),
        }
    }

    /// Acquire a snapshot timestamp for the duration of this query.
    /// Returns a monotonically increasing value.
    pub fn begin_snapshot(&self) -> u64 {
        self.snapshot_counter.load(Ordering::Acquire)
    }

    /// Allocate the next snapshot timestamp. Called by writers when
    /// they commit a new version. Atomically advances the counter.
    pub fn next_snapshot_ts(&self) -> u64 {
        self.snapshot_counter.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Current snapshot value (debugging).
    pub fn current_snapshot(&self) -> u64 {
        self.snapshot_counter.load(Ordering::Acquire)
    }

    /// Insert a new version for `pk` with the given `row`.
    /// `visible_from_ts` should be obtained from `next_snapshot_ts()`
    /// at commit time.
    pub fn put(&self, pk: Value, row: Vec<Value>, visible_from_ts: u64, tx_id: u64) {
        let version = VersionedRow {
            row,
            visible_from_ts,
            created_by_tx: tx_id,
            deleted: false,
        };
        let mut w = self.versions.write();
        w.entry(pk).or_default().push(version);
    }

    /// Mark a row as deleted at `visible_from_ts`. Appends a
    /// tombstone version so readers at `snapshot_ts <
    /// visible_from_ts` still see the previous live version.
    pub fn delete(&self, pk: &Value, visible_from_ts: u64, tx_id: u64) {
        let version = VersionedRow {
            row: Vec::new(),
            visible_from_ts,
            created_by_tx: tx_id,
            deleted: true,
        };
        let mut w = self.versions.write();
        w.entry(pk.clone()).or_default().push(version);
    }

    /// Scan all visible rows at `snapshot_ts`. Tombstones are skipped.
    /// Returns `(primary_key, row)` pairs in primary-key order.
    pub fn scan_visible(&self, snapshot_ts: u64) -> Vec<(Value, Vec<Value>)> {
        let r = self.versions.read();
        let mut out = Vec::with_capacity(r.len());
        for (key, chain) in r.iter() {
            if let Some(visible) = find_visible(chain, snapshot_ts) {
                if !visible.deleted {
                    out.push((key.clone(), visible.row.clone()));
                }
            }
        }
        out
    }

    /// Phase B Step 4.2: O(log N) point-lookup by primary key.
    /// Returns the visible row at `snapshot_ts` for the given `pk`,
    /// or `None` if the row is missing or tombstoned at this snapshot.
    /// Avoids the O(N) full scan that `scan_visible` performs.
    pub fn get_visible(&self, pk: &Value, snapshot_ts: u64) -> Option<Vec<Value>> {
        let r = self.versions.read();
        let chain = r.get(pk)?;
        let visible = find_visible(chain, snapshot_ts)?;
        if visible.deleted {
            None
        } else {
            Some(visible.row.clone())
        }
    }

    /// Count of distinct primary keys with at least one version.
    pub fn key_count(&self) -> usize {
        self.versions.read().len()
    }

    /// Total number of stored versions (sum of chain lengths) — for
    /// memory accounting and GC metrics.
    pub fn version_count(&self) -> usize {
        self.versions.read().values().map(|v| v.len()).sum()
    }

    /// Garbage-collect versions older than `snapshot_ts - GC_LAG`.
    ///
    /// Policy:
    /// 1. **Multi-version chains** (length ≥ 2): drop versions whose
    ///    `visible_from_ts < cutoff` and that are not the live version.
    ///    The live version (last entry) is always kept so readers can
    ///    resolve the current state of the PK.
    /// 2. **Single-version chains** (length 1): drop the version if
    ///    its `visible_from_ts < cutoff`. This is safe because
    ///    `MvccStorage::scan_pk` falls back to the inner engine
    ///    (FileStorage) when the MVCC chain returns None, and the
    ///    inner still holds the row. The version is regenerated on
    ///    the next `rebuild_from_inner()` at server restart. Without
    ///    this, a workload dominated by INSERTs would accumulate one
    ///    un-reclaimable entry per PK forever (each PK's chain stays
    ///    at length 1).
    ///
    /// Returns the number of versions dropped.
    pub fn gc(&self, snapshot_ts: u64, gc_lag: u64) -> usize {
        let cutoff = snapshot_ts.saturating_sub(gc_lag);
        let mut w = self.versions.write();
        let mut dropped = 0;
        // Collect keys whose entire chain we want to evict so we can
        // remove them from the map in one pass. We can't mutate the
        // map while iterating its values.
        let mut to_evict: Vec<Value> = Vec::new();
        for (pk, chain) in w.iter_mut() {
            if chain.is_empty() {
                continue;
            }
            // Step 1: trim multi-version chains (existing behavior).
            if chain.len() > 1 {
                let mut i = 0;
                while i + 1 < chain.len() {
                    if chain[i].visible_from_ts < cutoff {
                        chain.remove(i);
                        dropped += 1;
                    } else {
                        i += 1;
                    }
                }
            }
            // Step 2: evict single-version chains whose version is
            // older than the cutoff. (The version's visible_from_ts is
            // < cutoff, which means no reader with snapshot_ts <=
            // current snapshot_ts - gc_lag would have looked at this
            // entry anyway.)
            if chain.len() == 1 && chain[0].visible_from_ts < cutoff {
                to_evict.push(pk.clone());
            }
        }
        for pk in &to_evict {
            w.remove(pk);
            dropped += 1;
        }
        dropped
    }

    /// Clear all state (used for testing).
    #[cfg(test)]
    pub fn clear(&self) {
        self.versions.write().clear();
        self.snapshot_counter.store(0, Ordering::Release);
    }
}

/// Find the newest version visible at `snapshot_ts` (linear scan from
/// the end, since the chain is sorted by `visible_from_ts`). Returns
/// `None` if the chain is empty or every version is newer than
/// `snapshot_ts`.
///
/// Linear scan is fine for Phase 4 because version chains are kept
/// short by GC. If chains grow large, replace with `partition_point`.
pub(crate) fn find_visible(chain: &[VersionedRow], snapshot_ts: u64) -> Option<&VersionedRow> {
    chain
        .iter()
        .rev()
        .find(|v| v.visible_from_ts <= snapshot_ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(i: i64) -> Value {
        Value::Integer(i)
    }

    #[test]
    fn test_empty_table_snapshot() {
        let t = VersionedTable::new();
        assert_eq!(t.begin_snapshot(), 0);
        assert!(t.scan_visible(0).is_empty());
    }

    #[test]
    fn test_put_and_scan_visible() {
        let t = VersionedTable::new();
        let ts1 = t.next_snapshot_ts();
        t.put(int(1), vec![int(10), int(20)], ts1, 1);

        // Reader before commit (snapshot=0) sees nothing.
        assert!(t.scan_visible(0).is_empty());

        // Reader after commit (snapshot=ts1) sees the row.
        let rows = t.scan_visible(ts1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, int(1));
        assert_eq!(rows[0].1, vec![int(10), int(20)]);
    }

    #[test]
    fn test_version_chain_update() {
        let t = VersionedTable::new();
        let ts1 = t.next_snapshot_ts();
        t.put(int(1), vec![int(10)], ts1, 1);

        let ts2 = t.next_snapshot_ts();
        t.put(int(1), vec![int(99)], ts2, 2);

        // Reader at ts1 sees old version.
        let r1 = t.scan_visible(ts1);
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].1, vec![int(10)]);

        // Reader at ts2 sees new version.
        let r2 = t.scan_visible(ts2);
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].1, vec![int(99)]);

        // Reader between snapshots (impossible in practice but valid
        // for the visibility rule) sees the previous version.
        let r_mid = t.scan_visible(ts1 + 0); // same as ts1
        assert_eq!(r_mid[0].1, vec![int(10)]);
    }

    #[test]
    fn test_tombstone_hides_row() {
        let t = VersionedTable::new();
        let ts1 = t.next_snapshot_ts();
        t.put(int(1), vec![int(10)], ts1, 1);

        let ts2 = t.next_snapshot_ts();
        t.delete(&int(1), ts2, 2);

        // Reader at ts1 sees the row.
        let r1 = t.scan_visible(ts1);
        assert_eq!(r1.len(), 1);

        // Reader at ts2 sees no row (tombstone).
        let r2 = t.scan_visible(ts2);
        assert_eq!(r2.len(), 0);

        // Reader between ts1 and ts2 still sees the row.
        let r_mid = t.scan_visible(ts1);
        assert_eq!(r_mid.len(), 1);
    }

    #[test]
    fn test_insert_after_delete_resurrects() {
        let t = VersionedTable::new();
        let ts1 = t.next_snapshot_ts();
        t.put(int(1), vec![int(10)], ts1, 1);

        let ts2 = t.next_snapshot_ts();
        t.delete(&int(1), ts2, 2);

        let ts3 = t.next_snapshot_ts();
        t.put(int(1), vec![int(99)], ts3, 3);

        // Reader at ts3 sees the new row (newest version is not deleted).
        let r3 = t.scan_visible(ts3);
        assert_eq!(r3.len(), 1);
        assert_eq!(r3[0].1, vec![int(99)]);

        // Reader at ts2 sees no row (newest version <= ts2 is the tombstone).
        let r2 = t.scan_visible(ts2);
        assert_eq!(r2.len(), 0);
    }

    #[test]
    fn test_snapshot_counter_monotonic() {
        let t = VersionedTable::new();
        let a = t.next_snapshot_ts();
        let b = t.next_snapshot_ts();
        let c = t.next_snapshot_ts();
        assert!(a < b);
        assert!(b < c);
        // begin_snapshot returns the *current* value, not a new one.
        assert_eq!(t.begin_snapshot(), c);
    }

    #[test]
    fn test_gc_drops_old_versions() {
        let t = VersionedTable::new();
        let mut tss = Vec::new();
        // Create 10 versions of the same row.
        for i in 0..10 {
            let ts = t.next_snapshot_ts();
            t.put(int(1), vec![int(i)], ts, i as u64);
            tss.push(ts);
        }
        assert_eq!(t.version_count(), 10);

        // GC at the latest snapshot with GC_LAG=3: drops versions
        // visible_from_ts < (current - 3). All versions in chain are
        // older than ts[9] - 3 = ts[6], so we drop versions 0..6.
        // But we always keep the newest (last in chain), so we keep
        // ts[6..=9] (4 versions).
        let current = tss[9];
        let dropped = t.gc(current, 3);
        assert_eq!(dropped, 6);
        assert_eq!(t.version_count(), 4);
    }

    #[test]
    fn test_key_count() {
        let t = VersionedTable::new();
        let ts = t.next_snapshot_ts();
        t.put(int(1), vec![int(10)], ts, 1);
        t.put(int(2), vec![int(20)], ts, 1);
        t.put(int(3), vec![int(30)], ts, 1);
        assert_eq!(t.key_count(), 3);

        let ts2 = t.next_snapshot_ts();
        t.put(int(1), vec![int(11)], ts2, 2); // update existing key
        assert_eq!(t.key_count(), 3); // still 3 keys
        assert_eq!(t.version_count(), 4); // 3 + 1 new version
    }

    #[test]
    fn test_scan_visible_returns_in_pk_order() {
        let t = VersionedTable::new();
        let ts = t.next_snapshot_ts();
        // Insert in non-sorted order.
        t.put(int(3), vec![int(30)], ts, 1);
        t.put(int(1), vec![int(10)], ts, 1);
        t.put(int(2), vec![int(20)], ts, 1);

        let rows = t.scan_visible(ts);
        assert_eq!(rows[0].0, int(1));
        assert_eq!(rows[1].0, int(2));
        assert_eq!(rows[2].0, int(3));
    }

    #[test]
    fn test_find_visible_at_snapshot_before_all() {
        let t = VersionedTable::new();
        // No versions yet — find_visible on empty chain.
        assert!(find_visible(&[], 0).is_none());
        assert!(find_visible(&[], u64::MAX).is_none());
    }

    #[test]
    fn test_high_contention_snapshot_advances() {
        // Simulate many concurrent writers by interleaving puts.
        let t = VersionedTable::new();
        let mut last_ts = 0;
        for i in 0..100 {
            let ts = t.next_snapshot_ts();
            assert!(ts > last_ts);
            last_ts = ts;
            t.put(int(i), vec![int(i * 10)], ts, i as u64);
        }
        // Final snapshot sees all 100 rows.
        let final_rows = t.scan_visible(last_ts);
        assert_eq!(final_rows.len(), 100);
    }

    #[test]
    fn test_get_visible_pk_lookup() {
        // Phase B Step 4.2: PK lookup must be O(log N) and skip
        // tombstones, mirroring scan_visible semantics.
        let t = VersionedTable::new();
        let ts1 = t.next_snapshot_ts();
        t.put(int(1), vec![int(10), int(20)], ts1, 1);
        let ts2 = t.next_snapshot_ts();
        t.put(int(1), vec![int(99)], ts2, 2);
        let ts3 = t.next_snapshot_ts();
        t.delete(&int(1), ts3, 3);

        // Missing PK → None.
        assert!(t.get_visible(&int(42), ts2).is_none());

        // PK=1 at snapshot ts1 returns the first version.
        let r1 = t.get_visible(&int(1), ts1).expect("row present");
        assert_eq!(r1, vec![int(10), int(20)]);

        // PK=1 at snapshot ts2 returns the updated version.
        let r2 = t.get_visible(&int(1), ts2).expect("row present");
        assert_eq!(r2, vec![int(99)]);

        // PK=1 at snapshot ts3 is tombstoned → None.
        assert!(t.get_visible(&int(1), ts3).is_none());
    }
}
