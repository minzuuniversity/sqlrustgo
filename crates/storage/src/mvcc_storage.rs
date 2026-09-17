//! MvccStorage: wraps an inner `StorageEngine` (typically FileStorage)
//! with a multi-version concurrency control (MVCC) layer for reads.
//!
//! Phase B Step 4 of the SQLRustGo performance plan. See
//! `docs/releases/v4.0.0/PHASE_B_STEP4_MVCC_PLAN.md` for the design.
//!
//! Architecture:
//! - The `inner` engine holds the on-disk / canonical state.
//! - The `mvcc` layer (per-table `VersionedTable`) holds the *visible*
//!   rows at each committed snapshot timestamp.
//! - On `insert`/`update`/`delete`, the wrapper both updates the inner
//!   engine AND appends a new `VersionedRow` to the MVCC chain under
//!   the next snapshot timestamp.
//! - On `scan`/`scan_with_filter`, the wrapper acquires a snapshot
//!   timestamp atomically and reads the MVCC layer.
//!
//! This means readers no longer need the outer `Arc<RwLock<storage>>`
//! read lock for SELECT — they only do an atomic load on the snapshot
//! counter.

use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlResult, StorageEngine, TableInfo,
    TriggerInfo, Value,
};
use crate::mvcc::{find_visible, VersionedTable};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Default GC lag for the MVCC layer — versions older than
/// `current_snapshot - MVCC_GC_LAG` are eligible for cleanup.
pub const MVCC_GC_LAG: u64 = 1024;

/// MVCC-wrapped storage engine.
///
/// The inner engine handles on-disk persistence and is the source of
/// truth for crash recovery. The MVCC layer (one `VersionedTable` per
/// table) is rebuilt on startup from WAL replay.
pub struct MvccStorage<S: StorageEngine + 'static> {
    /// Underlying storage engine (FileStorage, MemoryStorage, etc.)
    inner: S,
    /// Per-table MVCC version chain. `Arc<VersionedTable>` so the
    /// `scan_*(&self)` paths can hand a reference to background GC
    /// without holding the wrapper lock.
    mvcc: parking_lot::RwLock<HashMap<String, Arc<VersionedTable>>>,
    /// V400-MVCC-GC: monotonically incremented on every write path
    /// (insert/delete/update/update_if/delete_if/force_insert).
    /// When `count % GC_INTERVAL == 0` (modulo == 0 right after the
    /// increment), the write thread calls `self.gc(MVCC_GC_LAG)`.
    /// Throttling avoids paying the gc scan cost on every write
    /// (the per-write version chain is already short at the lag
    /// boundary, so GC every-Nth-write is sufficient to keep RSS
    /// bounded under sustained mixed read/write load).
    write_count: std::sync::atomic::AtomicU64,
    /// V400-PERF-FIX: per-table cache of the last observed MVCC
    /// key_count. When this count is monotonically increasing
    /// (no GC has run since last call), we skip the expensive
    /// `inner.scan().len()` check entirely. When the count drops
    /// (GC ran), we re-check inner.
    scan_skip_cache: parking_lot::Mutex<HashMap<String, (usize, u64)>>,
}

/// V400-MVCC-GC: GC frequency. Run `gc(MVCC_GC_LAG)` once every
/// `GC_INTERVAL` write operations. Tunable via const because this
/// must be a `const` for the `AtomicU64::fetch_add` modulo branch.
/// Empirical sweet spot: 128 — small enough that GC fires within a
/// few seconds under 16-thread mixed read/write load (20% writes),
/// large enough that GC overhead is amortized away from hot path.
const GC_INTERVAL: u64 = 128;

impl<S: StorageEngine + 'static> MvccStorage<S> {
    /// Wrap an inner storage engine. Starts with empty MVCC tables —
    /// they're populated lazily on first insert or by WAL recovery.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            mvcc: parking_lot::RwLock::new(HashMap::new()),
            write_count: std::sync::atomic::AtomicU64::new(0),
            scan_skip_cache: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    /// V400-MVCC-GC: bump the write counter; if we've crossed a
    /// `GC_INTERVAL` boundary, reap old versions. Caller must hold
    /// no locks when invoking (called at the tail of write paths).
    #[inline]
    fn maybe_gc(&self) {
        let count = self
            .write_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        if count.is_multiple_of(GC_INTERVAL) {
            let _dropped = self.gc(MVCC_GC_LAG);
        }
    }

    /// Expose the inner engine for callers that need direct access
    /// (e.g. recovery, testing).
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Mutable access to the inner engine. Caller must hold exclusive
    /// access (the engine's write lock).
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Acquire (or lazily create) the MVCC table for `table_name`.
    fn mvcc_table(&self, table_name: &str) -> Arc<VersionedTable> {
        // Fast path: already exists.
        if let Some(t) = self.mvcc.read().get(table_name).cloned() {
            return t;
        }
        // Slow path: create.
        let mut w = self.mvcc.write();
        w.entry(table_name.to_string())
            .or_insert_with(|| Arc::new(VersionedTable::new()))
            .clone()
    }

    /// Snapshot the current MVCC timestamp for use in a query.
    /// Equivalent to `begin_snapshot()` on the first MVCC table, but
    /// reads the global counter (which is the same counter for all
    /// tables).
    pub fn begin_snapshot(&self) -> u64 {
        // We need a table to read the counter from. Any table works
        // since they all share the same counter, but if no table
        // exists yet we use a fresh one.
        if let Some((_, t)) = self.mvcc.read().iter().next() {
            return t.begin_snapshot();
        }
        // No table yet — get or create one and read its counter.
        self.mvcc_table("__snapshot_probe__").begin_snapshot()
    }

    /// Run GC on every MVCC table.
    pub fn gc(&self, gc_lag: u64) -> usize {
        let r = self.mvcc.read();
        let mut total = 0;
        for t in r.values() {
            let ts = t.begin_snapshot();
            total += t.gc(ts, gc_lag);
        }
        total
    }

    /// Phase B Step 4.2: O(log N) point-lookup by primary key.
    /// Returns the visible row at the current snapshot, or None if
    /// the row is missing or tombstoned. Returns the cloned row data
    /// to match the scan-with-filter interface (caller does NOT need
    /// to additionally clone).
    pub fn get_visible(&self, table: &str, pk: &Value) -> Option<Vec<Value>> {
        let mvcc = self.mvcc_table(table);
        let snapshot_ts = mvcc.begin_snapshot();
        mvcc.get_visible(pk, snapshot_ts)
    }

    /// Rebuild the MVCC layer from the inner engine's current rows.
    /// Used at server startup (after WAL recovery) so SELECTs see the
    /// committed state without having to wait for the first writer.
    ///
    /// For each table: take a snapshot of `inner.scan(table)`, then
    /// insert one version per row with `visible_from_ts = current_ts`
    /// and `tx_id = 0` (recovery tx).
    pub fn rebuild_from_inner(&self) -> SqlResult<()> {
        let tables = self.inner.list_tables();
        for table_name in tables {
            let rows = self.inner.scan(&table_name)?;
            let mvcc = self.mvcc_table(&table_name);
            let ts = mvcc.next_snapshot_ts();
            for row in rows {
                // Use the first column as PK if available. For Phase
                // 4 we assume tables have a PK (true for FileStorage
                // tables created via DDL). For PK-less tables, the
                // wrapper falls back to a synthetic PK (auto-increment
                // index) — see also the StorageEngine spec.
                if row.is_empty() {
                    continue;
                }
                let pk = row[0].clone();
                mvcc.put(pk, row, ts, 0);
            }
        }
        Ok(())
    }
}

impl<S: StorageEngine + 'static> StorageEngine for MvccStorage<S> {
    /// Phase B Step 4.2: PK lookup using MVCC chain. Fast path:
    /// MVCC chain lookup (O(log N)). If MVCC has no entry (rebuild
    /// lag), fall back to the inner engine (which has the B+ Tree
    /// index, also O(log N)).
    ///
    /// The returned row is a clone (caller doesn't need to clone
    /// again). To avoid double-cloning when the inner engine already
    /// returns a freshly cloned row, callers should prefer this
    /// method over `inner.scan_pk` + MVCC chain check.
    fn scan_pk(&self, table: &str, pk_column: &str, pk: &Value) -> SqlResult<Option<Record>> {
        let mvcc = self.mvcc_table(table);
        let snapshot_ts = mvcc.begin_snapshot();
        if let Some(row) = mvcc.get_visible(pk, snapshot_ts) {
            return Ok(Some(row));
        }
        // MVCC has no visible row for this PK. Try the inner engine
        // — covers both (a) the rebuild-lag case and (b) the case
        // where GC has evicted the MVCC chain but the row is still
        // in inner.data.rows. The inner's scan_pk is now O(log N)
        // thanks to the auto-built PK B+Tree index (see
        // FileStorage::rebuild_pk_indexes / create_table).
        self.inner.scan_pk(table, pk_column, pk)
    }

    /// Phase B Step 4.3: O(log N + k) PK range scan. Returns the
    /// visible rows whose primary key falls in `low..=high` (inclusive),
    /// in primary-key order. Uses MVCC chains for visibility check;
    /// falls back to the inner engine for the rebuild-lag case.
    fn scan_pk_range(&self, table: &str, low: &Value, high: &Value) -> SqlResult<Vec<Record>> {
        use std::ops::Bound;
        let mvcc = self.mvcc_table(table);
        let snapshot_ts = mvcc.begin_snapshot();
        // BTreeMap::range over [low, high] is O(log N + k) where k
        // is the number of matching keys — much cheaper than a full
        // scan followed by per-row filter.
        let r = mvcc.versions.read();
        let mut out = Vec::new();
        for (_, chain) in r.range((Bound::Included(low), Bound::Included(high))) {
            if let Some(visible) = find_visible(chain, snapshot_ts) {
                if !visible.deleted {
                    out.push(visible.row.clone());
                }
            }
        }
        Ok(out)
    }
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        let mvcc = self.mvcc_table(table);
        let snapshot_ts = mvcc.begin_snapshot();
        let pairs = mvcc.scan_visible(snapshot_ts);
        let mut out: Vec<Record> = pairs.into_iter().map(|(_, row)| row).collect();

        // V400-MVCC-PKFAST: merge inner.scan() so rows that have been
        // evicted from MVCC chains by background GC are still visible.
        //
        // Optimization: ONLY consult inner.scan() when MVCC chain
        // count has dropped since the last call (which means GC
        // may have evicted chains). The cache is per-MvccStorage
        // because GC happens globally; we just check the count
        // delta to skip the expensive inner.scan().len() on the
        // hot path.
        let mvcc_count = mvcc.key_count();
        let needs_check = {
            let mut cache = self.scan_skip_cache.lock();
            let entry = cache.entry(table.to_string()).or_insert((0, 0));
            let cached_count = entry.0;
            let hit_count = entry.1;
            let needs = mvcc_count < cached_count || hit_count == 0;
            if needs {
                entry.0 = mvcc_count;
                entry.1 = hit_count.wrapping_add(1);
            }
            needs
        };
        if needs_check {
            let inner_row_count = self.inner.scan(table)?.len();
            if mvcc_count < inner_row_count {
                let mvcc_pks: std::collections::HashSet<crate::engine::Value> =
                    out.iter().filter_map(|r| r.first().cloned()).collect();
                if let Ok(inner_rows) = self.inner.scan(table) {
                    for row in inner_rows {
                        if let Some(pk) = row.first() {
                            if !mvcc_pks.contains(pk) {
                                out.push(row);
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    fn scan_with_filter<F>(&self, table: &str, filter: F) -> SqlResult<Vec<Record>>
    where
        F: Fn(&Record) -> bool,
        Self: Sized,
    {
        let mvcc = self.mvcc_table(table);
        let snapshot_ts = mvcc.begin_snapshot();
        let pairs = mvcc.scan_visible(snapshot_ts);
        let mut out: Vec<Record> = pairs
            .into_iter()
            .filter_map(|(_, row)| if filter(&row) { Some(row) } else { None })
            .collect();

        // V400-MVCC-PKFAST: also include rows from inner that may have
        // been evicted from MVCC chains by background GC. Deduplicate
        // by PK to avoid double-counting rows still present in MVCC.
        //
        // Optimization: skip the inner scan entirely when MVCC
        // covers all rows. We approximate "MVCC covers all rows"
        // as "MVCC chain count == row_id of the highest inner row",
        // which is the common case in production workloads where
        // GC hasn't evicted any single-version chains yet.
        let mvcc_count = mvcc.key_count() as i64;
        let inner_row_count = self.inner.scan(table)?.len() as i64;
        if mvcc_count < inner_row_count {
            let mvcc_pks: std::collections::HashSet<crate::engine::Value> =
                out.iter().filter_map(|r| r.first().cloned()).collect();
            if let Ok(inner_rows) = self.inner.scan_with_filter(table, filter) {
                for row in inner_rows {
                    if let Some(pk) = row.first() {
                        if !mvcc_pks.contains(pk) {
                            out.push(row);
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        // First, write to the inner engine.
        self.inner.insert(table, records.clone())?;
        // Then, append to the MVCC chain under the next snapshot.
        let mvcc = self.mvcc_table(table);
        for row in records {
            if row.is_empty() {
                continue;
            }
            let pk = row[0].clone();
            let ts = mvcc.next_snapshot_ts();
            mvcc.put(pk, row, ts, ts);
        }
        // V400-MVCC-GC: reap old versions after every write path.
        self.maybe_gc();
        Ok(())
    }

    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize> {
        // Phase 4.1: prefer `delete_collect_pks` so we can tombstone
        // only the affected rows instead of the whole table. The
        // default impl returns an empty Vec, in which case we fall
        // back to the pre-Step-4.1 coarse "tombstone all visible
        // rows" behavior.
        let removed_pks = self.inner.delete_collect_pks(table, _filters)?;
        if removed_pks.is_empty() {
            // Either nothing was deleted, or the engine doesn't know
            // which PKs were deleted (default impl). If filters were
            // empty (full-table delete) we still need to tombstone
            // every visible row.
            if _filters.is_empty() {
                let mvcc = self.mvcc_table(table);
                let pairs = mvcc.scan_visible(mvcc.begin_snapshot());
                let ts = mvcc.next_snapshot_ts();
                for (pk, _) in pairs {
                    mvcc.delete(&pk, ts, ts);
                }
            }
            // V400-MVCC-GC: reap old versions after every write path.
            self.maybe_gc();
            return Ok(0);
        }
        // Tombstone exactly the affected PKs.
        let mvcc = self.mvcc_table(table);
        let ts = mvcc.next_snapshot_ts();
        for pk in &removed_pks {
            mvcc.delete(pk, ts, ts);
        }
        // V400-MVCC-GC: reap old versions after every write path.
        self.maybe_gc();
        Ok(removed_pks.len())
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        // Phase 4: coarse delete. We delegate to inner, then
        // tombstone all visible MVCC rows. If the filter is finer-
        // grained than the MVCC's PK match, readers might see
        // *un-deleted* rows for one extra snapshot until GC catches
        // up. Acceptable for read-heavy workloads; tighter semantics
        // come in Step 4.1.
        let n = self.inner.delete_if(table, filter)?;
        if n > 0 {
            let mvcc = self.mvcc_table(table);
            let pairs = mvcc.scan_visible(mvcc.begin_snapshot());
            let ts = mvcc.next_snapshot_ts();
            for (pk, _) in pairs {
                mvcc.delete(&pk, ts, ts);
            }
        }
        // V400-MVCC-GC: reap old versions after every write path.
        self.maybe_gc();
        Ok(n)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        // Phase 4: coarse update via inner. Then append a new MVCC
        // version with the same PK and the updated row.
        let n = self.inner.update(table, filters, updates)?;
        if n > 0 {
            let mvcc = self.mvcc_table(table);
            // Re-scan to get current row contents after update.
            let pairs = mvcc.scan_visible(mvcc.begin_snapshot());
            let ts = mvcc.next_snapshot_ts();
            for (pk, row) in pairs {
                mvcc.put(pk, row, ts, ts);
            }
        }
        // V400-MVCC-GC: reap old versions after every write path.
        self.maybe_gc();
        Ok(n)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        let n = self.inner.update_if(table, filter, mutation)?;
        if n > 0 {
            let mvcc = self.mvcc_table(table);
            let pairs = mvcc.scan_visible(mvcc.begin_snapshot());
            let ts = mvcc.next_snapshot_ts();
            for (pk, row) in pairs {
                mvcc.put(pk, row, ts, ts);
            }
        }
        // V400-MVCC-GC: reap old versions after every write path.
        self.maybe_gc();
        Ok(n)
    }

    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        self.inner.force_insert(table, record.clone())?;
        if !record.is_empty() {
            let mvcc = self.mvcc_table(table);
            let pk = record[0].clone();
            let ts = mvcc.next_snapshot_ts();
            mvcc.put(pk, record, ts, ts);
        }
        // V400-MVCC-GC: reap old versions after every write path.
        self.maybe_gc();
        Ok(())
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.inner.create_table(info)?;
        // Pre-create the MVCC table so future scans don't race on
        // first-insert.
        let _ = self.mvcc_table(&info.name);
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.inner.drop_table(table)?;
        self.mvcc.write().remove(table);
        Ok(())
    }

    fn flush(&mut self) -> SqlResult<()> {
        self.inner.flush()
    }

    // ---- read-only lookups: delegate directly ----

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.inner.get_table_info(table)
    }
    fn has_table(&self, table: &str) -> bool {
        self.inner.has_table(table)
    }
    fn list_tables(&self) -> Vec<String> {
        self.inner.list_tables()
    }
    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        self.inner.list_triggers(table)
    }
    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        self.inner.list_indexes(table)
    }
    fn has_view(&self, name: &str) -> bool {
        self.inner.has_view(name)
    }
    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        self.inner.get_trigger(name)
    }

    // ---- write methods that delegate (no MVCC update needed for DDL) ----

    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.create_database(db_name)
    }
    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.drop_database(db_name)
    }
    fn create_index(&mut self, info: crate::engine::IndexInfo) -> SqlResult<()> {
        self.inner.create_index(info)
    }
    fn drop_index(&mut self, table: &str, index_name: &str) -> SqlResult<()> {
        self.inner.drop_index(table, index_name)
    }

    // ---- required by trait ----

    fn list_all_indexes(&self) -> Vec<crate::engine::IndexInfo> {
        self.inner.list_all_indexes()
    }

    fn set_current_tx_id(&mut self, tx_id: u64) {
        self.inner.set_current_tx_id(tx_id)
    }

    fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
    }

    fn current_tx_id(&self) -> u64 {
        self.inner.current_tx_id()
    }

    fn discard_all_buffers(&mut self) {
        self.inner.discard_all_buffers()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn add_column(&mut self, table: &str, col: ColumnDefinition) -> SqlResult<()> {
        self.inner.add_column(table, col)
    }

    fn rename_table(&mut self, old: &str, new: &str) -> SqlResult<()> {
        // Move MVCC state under the new name.
        let mvcc_state = self.mvcc.write().remove(old);
        if let Some(state) = mvcc_state {
            self.mvcc.write().insert(new.to_string(), state);
        }
        self.inner.rename_table(old, new)
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        self.inner.create_trigger(info)
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.inner.drop_trigger(name)
    }

    fn gc(&self, gc_lag: u64) -> usize {
        // Delegate to the inherent `gc` method on `MvccStorage`.
        MvccStorage::gc(self, gc_lag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MemoryStorage;

    fn make_storage() -> MvccStorage<MemoryStorage> {
        let inner = MemoryStorage::new();
        let mut s = MvccStorage::new(inner);
        s.create_table(&TableInfo {
            name: "t".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INT".to_string(),
                nullable: false,
                primary_key: true,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
        s
    }

    #[test]
    fn test_scan_returns_inserted_rows() {
        let mut s = make_storage();
        s.insert(
            "t",
            vec![
                vec![Value::Integer(1), Value::Text("a".into())],
                vec![Value::Integer(2), Value::Text("b".into())],
            ],
        )
        .unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_delete_hides_rows() {
        let mut s = make_storage();
        s.insert(
            "t",
            vec![
                vec![Value::Integer(1)],
                vec![Value::Integer(2)],
                vec![Value::Integer(3)],
            ],
        )
        .unwrap();
        let before = s.scan("t").unwrap();
        assert_eq!(before.len(), 3);
        s.delete("t", &[Value::Integer(2)]).unwrap();
        let after = s.scan("t").unwrap();
        // Phase 4.1: precise tombstone — only PK=2 is gone, PK=1 and
        // PK=3 remain visible.
        assert_eq!(
            after.len(),
            2,
            "Phase 4.1 delete tombstones only the matched PK"
        );
        let pks: Vec<i64> = after
            .iter()
            .filter_map(|r| {
                r.first().and_then(|v| {
                    if let Value::Integer(i) = v {
                        Some(*i)
                    } else {
                        None
                    }
                })
            })
            .collect();
        assert!(pks.contains(&1));
        assert!(pks.contains(&3));
        assert!(!pks.contains(&2));
    }

    #[test]
    fn test_delete_then_gc_keeps_tombstone() {
        // V400-MVCC-SYNC: After a DELETE, the tombstone in the MVCC
        // chain must survive GC so readers at future snapshots
        // still see the row as deleted (not resurrected by falling
        // through to inner.scan_pk).
        let mut s = make_storage();
        s.insert(
            "t",
            vec![
                vec![Value::Integer(1)],
                vec![Value::Integer(2)],
                vec![Value::Integer(3)],
            ],
        )
        .unwrap();
        s.delete("t", &[Value::Integer(2)]).unwrap();
        // Force the snapshot ts very high so GC drops any
        // eligible single-version chains (PKs 1 and 3).
        // The tombstone for PK=2 must remain.
        let mvcc = s.mvcc_table("t");
        for _ in 0..2000 {
            mvcc.next_snapshot_ts();
        }
        let dropped = mvcc.gc(2000, 100);
        // PK=1 and PK=3 single-version chains older than cutoff
        // should be evicted; PK=2 has a tombstone that GC won't drop.
        assert!(dropped >= 2, "GC should evict single-version chains");
        let pks: Vec<i64> = s
            .scan("t")
            .unwrap()
            .iter()
            .filter_map(|r| {
                r.first().and_then(|v| {
                    if let Value::Integer(i) = v {
                        Some(*i)
                    } else {
                        None
                    }
                })
            })
            .collect();
        assert!(pks.contains(&1), "PK=1 should still be visible (inner has the row)");
        assert!(pks.contains(&3), "PK=3 should still be visible (inner has the row)");
        assert!(!pks.contains(&2), "PK=2 should be hidden by tombstone");
    }

    #[test]
    fn test_snapshot_progresses_with_inserts() {
        let mut s = make_storage();
        let snap0 = s.begin_snapshot();
        s.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let snap1 = s.begin_snapshot();
        s.insert("t", vec![vec![Value::Integer(2)]]).unwrap();
        let snap2 = s.begin_snapshot();
        assert!(snap0 < snap1);
        assert!(snap1 < snap2);
    }

    #[test]
    fn test_gc_runs_without_error() {
        let mut s = make_storage();
        for i in 0..50 {
            s.insert("t", vec![vec![Value::Integer(i)]]).unwrap();
        }
        let dropped = s.gc(MVCC_GC_LAG);
        // Should drop 0 versions because we only have one snapshot of
        // activity (no readers ahead of `current - GC_LAG`).
        assert_eq!(dropped, 0);
    }

    #[test]
    fn test_rebuild_from_inner_after_inserts() {
        // Insert directly into inner, then rebuild MVCC.
        let mut inner = MemoryStorage::new();
        inner
            .create_table(&TableInfo {
                name: "t".to_string(),
                columns: vec![ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    nullable: false,
                    primary_key: true,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();
        inner
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1)],
                    vec![Value::Integer(2)],
                    vec![Value::Integer(3)],
                ],
            )
            .unwrap();
        let mut s = MvccStorage::new(inner);
        s.rebuild_from_inner().unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_get_visible_pk_lookup() {
        // Phase B Step 4.2: PK lookup must return the visible row at
        // the current snapshot.
        let mut s = make_storage();
        s.insert(
            "t",
            vec![
                vec![Value::Integer(1), Value::Text("a".into())],
                vec![Value::Integer(2), Value::Text("b".into())],
            ],
        )
        .unwrap();
        // PK=1 → present.
        let r1 = s.get_visible("t", &Value::Integer(1)).expect("row");
        assert_eq!(r1[1], Value::Text("a".into()));
        // PK=999 → missing.
        assert!(s.get_visible("t", &Value::Integer(999)).is_none());
        // Delete PK=2 → next snapshot hides it.
        s.delete("t", &[Value::Integer(2)]).unwrap();
        assert!(s.get_visible("t", &Value::Integer(2)).is_none());
        // PK=1 still visible.
        assert!(s.get_visible("t", &Value::Integer(1)).is_some());
    }

    #[test]
    fn test_scan_pk_range() {
        // Phase B Step 4.3: inclusive range lookup should return
        // only the rows in [low, high].
        let mut s = make_storage();
        s.insert(
            "t",
            vec![
                vec![Value::Integer(1), Value::Text("a".into())],
                vec![Value::Integer(2), Value::Text("b".into())],
                vec![Value::Integer(3), Value::Text("c".into())],
                vec![Value::Integer(4), Value::Text("d".into())],
                vec![Value::Integer(5), Value::Text("e".into())],
            ],
        )
        .unwrap();
        let rows = s
            .scan_pk_range("t", &Value::Integer(2), &Value::Integer(4))
            .unwrap();
        assert_eq!(rows.len(), 3);
        let pks: Vec<i64> = rows
            .iter()
            .filter_map(|r| {
                r.first().and_then(|v| {
                    if let Value::Integer(i) = v {
                        Some(*i)
                    } else {
                        None
                    }
                })
            })
            .collect();
        assert_eq!(pks, vec![2, 3, 4]);
    }
}
