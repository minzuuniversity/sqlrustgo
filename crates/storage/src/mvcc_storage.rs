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
    /// V400-05: optional cross-model write tracker. When set, every
    /// write path calls the tracker's closure so the V400-05 tracker
    /// can enforce all-or-nothing semantics across SQL + vector +
    /// graph + audit. None means "no cross-model tracking".
    /// Boxed dyn to avoid circular dependency on the transaction crate.
    tx_tracker: Option<Box<dyn CrossModelWriteTracker>>,
}

/// V400-05: abstraction for cross-model write tracking. Implementors
/// live in the transaction crate; storage doesn't depend on transaction
/// to avoid a circular dep. SQL writes register as `kind=0` (Sql).
pub trait CrossModelWriteTracker: Send + Sync {
    fn register_write(&self, kind: u8, description: &str);
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
            tx_tracker: None,
        }
    }

    /// V400-05: attach a cross-model write tracker. Pass `None` to
    /// disable (legacy behavior).
    pub fn with_tx_tracker(mut self, tracker: Option<Box<dyn CrossModelWriteTracker>>) -> Self {
        self.tx_tracker = tracker;
        self
    }

    /// V400-05: get the current tx_tracker (if any).
    pub fn tx_tracker(&self) -> Option<&dyn CrossModelWriteTracker> {
        self.tx_tracker.as_deref()
    }

    /// V400-05: register an SQL write with the attached tracker
    /// (if any). Also fires the process-global tracker if set.
    /// kind=0 means Sql per the tracker contract.
    fn register_sql_write(&self, description: &str) {
        if let Some(tracker) = &self.tx_tracker {
            tracker.register_write(0, description);
        }
        crate::cross_model_tracker::register_sql_write(description);
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
    /// #4974: promote every pending version across all MVCC tables.
    /// #4974: `promote_pending` with the transaction id supplied by the
    /// caller.
    ///
    /// It **has** to be supplied. `FileStorage::commit_transaction` clears
    /// its own `current_tx_id` as part of committing, so a
    /// "delegate to inner, then promote" sequence reads back `0` and
    /// promotes nothing — which is why the committed row stayed invisible
    /// even after the `if r.is_ok()` gate was reached. Capture first,
    /// delegate second.
    fn promote_pending_for(&self, tx_id: u64) {
        if tx_id == 0 {
            return;
        }
        // #5156: each table stamps its own commit from its OWN counter.
        // The previous `self.mvcc_table("__commit_probe__").next_snapshot_ts()`
        // came from a lazily-created table whose counter has no relation to
        // any real table's `visible_from_ts` sequence — and `commit_tx`
        // overwrites that field. A lagging stamp put the tombstone behind
        // versions that were already newer, so `find_visible` returned the
        // older `put` and a committed DELETE silently reverted.
        let tables: Vec<Arc<VersionedTable>> = {
            let g = self.mvcc.read();
            g.values().cloned().collect()
        };
        for t in tables {
            t.commit_tx_auto(tx_id);
        }
    }

    /// #4974: drop every pending version across all MVCC tables.
    /// #4974: same capture-first requirement as
    /// [`promote_pending_for`](Self::promote_pending_for).
    fn discard_pending_for(&self, tx_id: u64) {
        if tx_id == 0 {
            return;
        }
        let tables: Vec<Arc<VersionedTable>> = {
            let g = self.mvcc.read();
            g.values().cloned().collect()
        };
        for t in tables {
            t.rollback_tx(tx_id);
        }
    }

    /// #4983: keys `tx_id` has written but not committed. The inner
    /// engine buffers those in `insert_buffer` and `inner.scan()`
    /// merges them back, so the caller needs to know which to drop.
    fn pending_keys(
        &self,
        table: &str,
        tx_id: u64,
    ) -> std::collections::HashSet<crate::engine::Value> {
        self.mvcc_table(table).pending_keys(tx_id)
    }

    /// #5105: [`pending_keys`](Self::pending_keys) against a stated
    /// database. The pending set is per `(db, table)` because the version
    /// store is, so filtering on the wrong database's set would let one
    /// database's uncommitted row through another database's scan.
    fn pending_keys_in(
        &self,
        db: &str,
        table: &str,
        tx_id: u64,
    ) -> std::collections::HashSet<crate::engine::Value> {
        self.mvcc_table_in(db, table).pending_keys(tx_id)
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Acquire (or lazily create) the MVCC table for `table_name`.
    fn mvcc_table(&self, table_name: &str) -> Arc<VersionedTable> {
        // #5105: the database is resolved from the storage's shared
        // `current_db` here. That is only correct for callers that have no
        // database of their own to state — see [`mvcc_table_in`](Self::mvcc_table_in)
        // for why the per-connection path cannot use it.
        self.mvcc_table_in(&self.inner.current_db(), table_name)
    }

    /// #5105: the version store for `(db, table)`.
    ///
    /// #5025 keyed the store per database, but derived the database from
    /// `inner.current_db()` — one shared value. So the key was scoped, yet
    /// every caller resolved the same database to build it. A read that
    /// knows which database it is asking about must pass it here, or two
    /// databases' tables land in one `VersionedTable` and a scan in one
    /// returns the other's rows.
    fn mvcc_table_in(&self, db: &str, table_name: &str) -> Arc<VersionedTable> {
        let key = crate::engine::scoped_key(db, table_name);
        // Fast path: already exists.
        if let Some(t) = self.mvcc.read().get(&key).cloned() {
            return t;
        }
        // Slow path: create.
        let mut w = self.mvcc.write();
        w.entry(key)
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
        let tx_id = self.inner.current_tx_id();
        let snapshot_ts = mvcc.begin_snapshot();
        mvcc.get_visible(pk, snapshot_ts, tx_id)
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
            let _tx_id = self.inner.current_tx_id();
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
        let tx_id = self.inner.current_tx_id();
        let snapshot_ts = mvcc.begin_snapshot();
        if let Some(row) = mvcc.get_visible(pk, snapshot_ts, tx_id) {
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
        let tx_id = self.inner.current_tx_id();
        let snapshot_ts = mvcc.begin_snapshot();
        // BTreeMap::range over [low, high] is O(log N + k) where k
        // is the number of matching keys — much cheaper than a full
        // scan followed by per-row filter.
        let r = mvcc.versions.read();
        let mut out = Vec::new();
        for (_, chain) in r.range((Bound::Included(low), Bound::Included(high))) {
            if let Some(visible) = find_visible(chain, snapshot_ts, tx_id) {
                if !visible.deleted {
                    out.push(visible.row.clone());
                }
            }
        }
        Ok(out)
    }
    /// #4983 / #4951: scan on behalf of `reader_tx`.
    ///
    /// `scan` cannot answer this on its own: it reads
    /// `inner.current_tx_id()`, which is a single storage-wide value
    /// holding whichever connection wrote last, not the one asking. Two
    /// concurrent connections therefore both resolve to the same
    /// transaction, and an uncommitted write becomes visible to a
    /// connection that did not make it.
    fn scan_in(&self, table: &str, reader_tx: u64) -> SqlResult<Vec<Record>> {
        // #5105: keep the transaction and resolve the database the way the
        // rest of this impl does, so both halves come from the same place.
        self.scan_in_tx_db(&self.inner.current_db(), table, reader_tx)
    }

    /// #5105: snapshot read of `(db, table)` on behalf of `reader_tx`.
    ///
    /// The engine's read path needs both halves at once: `db` selects the
    /// table namespace and `reader_tx` selects which versions are visible.
    /// Before this existed the engine could only have one — routing through
    /// `scan_in_db` kept `db` and dropped `reader_tx` (so a reader saw
    /// another connection's uncommitted rows), while `scan_in` kept
    /// `reader_tx` and resolved `db` from the storage-wide `current_db`.
    ///
    /// It was the *engine* that lost isolation, not this type: callers
    /// reaching `scan_in` directly kept both. See
    /// `tests/mvcc_reader_tx_5105.rs`, which goes through
    /// `ExecutionEngine` and fails without this.
    fn scan_in_tx_db(&self, db: &str, table: &str, reader_tx: u64) -> SqlResult<Vec<Record>> {
        let mvcc = self.mvcc_table_in(db, table);
        let snapshot_ts = mvcc.begin_snapshot();
        let pairs = mvcc.scan_visible(snapshot_ts, reader_tx);
        let mut out: Vec<Record> = pairs.into_iter().map(|(_, row)| row).collect();

        // Merge in rows the inner engine holds that MVCC has no visible
        // version for, so nothing that was committed becomes invisible
        // (see `scan`'s comment for why the merge is unconditional).
        //
        // #4983: rows this transaction wrote but has not committed must
        // be excluded. The inner engine buffers them in `insert_buffer`
        // with no visibility notion of its own, so an unconditional
        // merge hands an uncommitted write straight back to a reader
        // that `scan_visible` had correctly filtered out.
        //
        // #5105: both the pending set and the inner scan name `db`. They
        // read the same rows this method is deciding the visibility of;
        // pulling either from a different database would merge rows this
        // transaction has no business seeing.
        let pending: std::collections::HashSet<crate::engine::Value> =
            self.pending_keys_in(db, table, reader_tx);
        let inner_rows = self.inner.scan_in_db(db, table)?;
        if inner_rows.len() > out.len() {
            let mut present: std::collections::HashSet<crate::engine::Value> =
                out.iter().filter_map(|r| r.first().cloned()).collect();
            for row in inner_rows {
                if let Some(pk) = row.first() {
                    if pending.contains(pk) {
                        continue;
                    }
                    if present.insert(pk.clone()) {
                        out.push(row);
                    }
                }
            }
        }
        Ok(out)
    }

    /// #4974: `scan_with_filter` on behalf of `reader_tx`.
    ///
    /// This override was **missing** while `scan_in` existed, so the
    /// trait default (`engine.rs:1111` → `self.scan_with_filter(...)`)
    /// silently dropped `reader_tx` and fell back to
    /// `inner.current_tx_id()` — the storage-wide "whoever wrote last"
    /// value. Every predicate-filtered read was therefore unisolated.
    fn scan_with_filter_in(
        &self,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
        reader_tx: u64,
    ) -> SqlResult<Vec<Record>> {
        let mvcc = self.mvcc_table(table);
        let snapshot_ts = mvcc.begin_snapshot();
        let mut out: Vec<Record> = mvcc
            .scan_visible(snapshot_ts, reader_tx)
            .into_iter()
            .map(|(_, row)| row)
            .filter(|r| filter(r))
            .collect();

        // Same inner-merge contract as `scan_in`, and the same reason it
        // is unconditional (see `scan`'s comment): a read path that
        // intermittently hides committed rows is not an optimisation.
        let pending: std::collections::HashSet<crate::engine::Value> =
            self.pending_keys(table, reader_tx);
        let inner_rows = self.inner.scan_with_filter(table, filter)?;
        if inner_rows.len() > out.len() {
            let mut present: std::collections::HashSet<crate::engine::Value> =
                out.iter().filter_map(|r| r.first().cloned()).collect();
            for row in inner_rows {
                if let Some(pk) = row.first() {
                    if pending.contains(pk) {
                        continue;
                    }
                    if present.insert(pk.clone()) {
                        out.push(row);
                    }
                }
            }
        }
        Ok(out)
    }

    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        let mvcc = self.mvcc_table(table);
        let tx_id = self.inner.current_tx_id();
        let snapshot_ts = mvcc.begin_snapshot();
        let pairs = mvcc.scan_visible(snapshot_ts, tx_id);
        let mut out: Vec<Record> = pairs.into_iter().map(|(_, row)| row).collect();

        // Merge in rows the inner engine holds that MVCC has no visible
        // version for, so nothing that was committed becomes invisible.
        //
        // #4946: this merge used to be gated on an MVCC-key-count
        // heuristic — "only consult inner.scan() when the chain count has
        // dropped since the last call, which means GC may have evicted
        // chains". That premise is wrong in two ways:
        //
        //   1. `hit_count == 0` made the *first* scan of every table do
        //      the full inner scan, then the count comparison suppressed
        //      it for the next N calls. Which rows a SELECT could see
        //      therefore depended on call history — a plain read could
        //      return fewer rows than the table holds.
        //   2. MVCC chain count and inner row count are not comparable.
        //      MVCC holds one chain per key (with version history); the
        //      inner engine holds one row per committed record. A chain
        //      count can be lower than the row count with no GC involved.
        //
        // The merge is now unconditional: one inner scan per statement,
        // which is what correctness requires. A read path that
        // intermittently hides committed rows is not an optimisation.
        // The PK dedup keeps the result duplicate-free.
        let inner_rows = self.inner.scan(table)?;
        if inner_rows.len() > out.len() {
            let mut present: std::collections::HashSet<crate::engine::Value> =
                out.iter().filter_map(|r| r.first().cloned()).collect();
            for row in inner_rows {
                if let Some(pk) = row.first() {
                    if present.insert(pk.clone()) {
                        out.push(row);
                    }
                }
            }
        }
        Ok(out)
    }

    fn scan_with_filter(
        &self,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
    ) -> SqlResult<Vec<Record>> {
        let mvcc = self.mvcc_table(table);
        let tx_id = self.inner.current_tx_id();
        let snapshot_ts = mvcc.begin_snapshot();
        let pairs = mvcc.scan_visible(snapshot_ts, tx_id);
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
            let tx_id = self.inner.current_tx_id();
            let ts = mvcc.next_snapshot_ts();
            mvcc.put(pk, row, ts, tx_id);
        }
        // V400-05: register SQL write with cross-model transaction tracker
        self.register_sql_write("INSERT");
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
                let tx_id = self.inner.current_tx_id();
                let pairs = mvcc.scan_visible(mvcc.begin_snapshot(), tx_id);
                let tx_id = self.inner.current_tx_id();
                let ts = mvcc.next_snapshot_ts();
                for (pk, _) in pairs {
                    mvcc.delete(&pk, ts, tx_id);
                }
            }
            self.register_sql_write("DELETE");
            // V400-MVCC-GC: reap old versions after every write path.
            self.maybe_gc();
            return Ok(0);
        }
        // Tombstone exactly the affected PKs.
        let mvcc = self.mvcc_table(table);
        let tx_id = self.inner.current_tx_id();
        let ts = mvcc.next_snapshot_ts();
        for pk in &removed_pks {
            mvcc.delete(pk, ts, tx_id);
        }
        self.register_sql_write("DELETE");
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
            let tx_id = self.inner.current_tx_id();
            let pairs = mvcc.scan_visible(mvcc.begin_snapshot(), tx_id);
            let tx_id = self.inner.current_tx_id();
            let ts = mvcc.next_snapshot_ts();
            for (pk, _) in pairs {
                mvcc.delete(&pk, ts, tx_id);
            }
        }
        self.register_sql_write("DELETE");
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
            let tx_id = self.inner.current_tx_id();
            let ts = mvcc.next_snapshot_ts();
            // #4995: read the post-update row contents from the **inner
            // engine**, not from the MVCC chain.
            //
            // The old body did `mvcc.scan_visible(...)` here and re-`put`
            // whatever came back. But `self.inner.update(...)` only
            // touches the inner engine — the MVCC chain still holds the
            // *pre-update* versions, so the loop appended a brand-new
            // version whose contents were the **old row**. Every reader
            // resolves the newest version for a PK from the tail of the
            // chain, so the freshly appended old-row version shadowed the
            // real data and the update was invisible forever.
            //
            // End to end that is: `UPDATE ... SET k=222` returns
            // success, no error, and a following `SELECT` reads the
            // original value back. It reached production through
            // `apply_odku` (`ON DUPLICATE KEY UPDATE`), which is why ODKU
            // "worked" (duplicate detected, statement succeeded) while
            // never actually changing a row.
            //
            // The inner engine is now the authority on current contents;
            // the chain is the authority on *visibility*.
            let updated_rows = self.inner.scan(table)?;
            for row in updated_rows {
                let Some(pk) = row.first().cloned() else {
                    continue;
                };
                // `filters` carries the primary key values of the rows the
                // update actually touched (see `apply_odku`, which passes
                // `pk_values`). An empty filter means the whole table.
                if !filters.is_empty() && !filters.contains(&pk) {
                    continue;
                }
                mvcc.put(pk, row, ts, tx_id);
            }
        }
        self.register_sql_write("UPDATE");
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
            let tx_id = self.inner.current_tx_id();
            let pairs = mvcc.scan_visible(mvcc.begin_snapshot(), tx_id);
            let tx_id = self.inner.current_tx_id();
            let ts = mvcc.next_snapshot_ts();
            for (pk, row) in pairs {
                mvcc.put(pk, row, ts, tx_id);
            }
        }
        self.register_sql_write("UPDATE");
        self.maybe_gc();
        Ok(n)
    }

    fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
        self.inner.force_insert(table, record.clone())?;
        if !record.is_empty() {
            let mvcc = self.mvcc_table(table);
            let pk = record[0].clone();
            let tx_id = self.inner.current_tx_id();
            let ts = mvcc.next_snapshot_ts();
            mvcc.put(pk, record, ts, tx_id);
        }
        self.register_sql_write("FORCE_INSERT");
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
        // #5025: same scoped key `mvcc_table` uses.
        let key = crate::engine::scoped_key(&self.inner.current_db(), table);
        self.mvcc.write().remove(&key);
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

    /// #5025: forward the database switch. A wrapper that answers the
    /// trait default (`Ok(())` that changes nothing) makes `USE` report
    /// success while every query still resolves against the previous
    /// database.
    fn set_current_db(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.set_current_db(db_name)
    }

    fn current_db(&self) -> String {
        self.inner.current_db()
    }
    /// #5009: forward. Without this the MVCC layer would answer the
    /// trait default (empty list) and `SHOW DATABASES` would report
    /// nothing at all when the server runs on the MVCC engine.
    fn list_databases(&self) -> SqlResult<Vec<String>> {
        self.inner.list_databases()
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

    /// #4978: forward the `&mut self` transaction methods.
    ///
    /// `MvccStorage` only implemented the `*_lockfree` trio, so the
    /// plain `begin_transaction` / `commit_transaction` /
    /// `rollback_transaction` fell through to the trait defaults, which
    /// return `Err("Transactions not supported by this storage engine")`.
    ///
    /// `FileStorage` does not implement any `*_lockfree` method either,
    /// so the lockfree path failed as well — meaning **neither** route
    /// worked through this wrapper. Production
    /// (`src/execution_engine_methods.rs:1586-1605`) tries lockfree and
    /// falls back to the plain call, discarding the error with
    /// `let _ =`, so a COMMIT here silently did nothing.
    ///
    /// These take `&mut self` and match the trait signature, so no
    /// interior mutability is involved — `inner_mut` is the only way to
    /// reach `S`, and it already requires the exclusive borrow.
    fn begin_transaction(&mut self) -> SqlResult<u64> {
        self.inner.begin_transaction()
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        // #4974: capture the tx id **before** delegating — `FileStorage`
        // clears it as part of its own commit.
        let tx_id = self.inner.current_tx_id();
        let r = self.inner.commit_transaction();
        // Promote this transaction's pending versions so other
        // connections can see them. Only after the inner engine accepted
        // the commit — a failed commit leaves everything pending, and
        // therefore invisible.
        if r.is_ok() {
            self.promote_pending_for(tx_id);
        }
        r
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.inner.current_tx_id();
        let r = self.inner.rollback_transaction();
        // #4974: drop the pending versions. Nothing was ever visible, so
        // there is nothing to restore.
        if r.is_ok() {
            self.discard_pending_for(tx_id);
        }
        r
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
        // #5025: move the version store with the table.
        let old_key = crate::engine::scoped_key(&self.inner.current_db(), old);
        let mvcc_state = self.mvcc.write().remove(&old_key);
        if let Some(state) = mvcc_state {
            let new_key = crate::engine::scoped_key(&self.inner.current_db(), new);
            self.mvcc.write().insert(new_key, state);
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

    /// #4912 / v4.1.0-perf: delegate the lock-free transaction path to the
    /// inner engine. Without these, `MvccStorage` inherits the `Err`
    /// defaults from the trait and any caller routing through it (e.g.
    /// `--storage parallel`, which wraps `MvccStorage` in
    /// `ParallelWalStorage`) falls back to the global storage write lock
    /// on every BEGIN / COMMIT / ROLLBACK.
    fn begin_transaction_lockfree(&self, tx_id: u64) -> SqlResult<()> {
        self.inner.begin_transaction_lockfree(tx_id)
    }

    /// #4974: **this override was missing the MVCC promotion entirely.**
    ///
    /// The `&mut` twin `commit_transaction` calls `promote_pending()`;
    /// the lockfree variant only forwarded to the inner engine. But the
    /// engine's `commit_transaction` prefers the lockfree path whenever
    /// the storage supports it — which `WalStorage` does — so
    /// `promote_pending()` was **never reached in the server**. Every
    /// version written inside a transaction stayed `committed == false`
    /// for the rest of its life, so:
    ///
    /// ```text
    /// PROBE while_A_uncommitted count=0   <- correct isolation
    /// PROBE after_A_commit     count=0   <- the commit is invisible too
    /// PROBE VERDICT=LOST_WRITE
    /// ```
    ///
    /// This was masked before the read path was fixed: with every read
    /// resolving to the storage-wide "whoever wrote last" transaction,
    /// committed and uncommitted rows looked identical, so a commit that
    /// promoted nothing was unobservable.
    fn commit_transaction_lockfree(&self) -> SqlResult<()> {
        // #4974: capture first, delegate second — see `promote_pending_for`.
        let tx_id = self.inner.current_tx_id();
        self.commit_transaction_lockfree_for(tx_id)
    }

    /// #5099: `commit_transaction_lockfree` for an explicit transaction id.
    ///
    /// This is the point where the identity was being lost: the id was
    /// read back out of the shared `current_tx_id` slot, which under the
    /// caller's READ guard another connection can overwrite at any moment
    /// (concurrent readers are allowed). `promote_pending_for` then
    /// promotes whichever transaction that slot happened to name —
    /// leaving the real one pending and the peer's rows committed.
    fn commit_transaction_lockfree_for(&self, tx_id: u64) -> SqlResult<()> {
        let r = self.inner.commit_transaction_lockfree_for(tx_id);
        // Promote only after the inner engine accepted the commit — a
        // failed commit must leave everything pending, and therefore
        // invisible. Same ordering as `commit_transaction` above.
        if r.is_ok() {
            self.promote_pending_for(tx_id);
        }
        r
    }

    /// #4974: same omission on the rollback side — `rollback_transaction`
    /// calls `discard_pending()`, this one did not, so an aborted
    /// transaction's versions stayed pending (invisible to readers, but
    /// never released, and `pending_keys` kept paying for them on every
    /// subsequent read).
    fn rollback_transaction_lockfree(&self) -> SqlResult<()> {
        let tx_id = self.inner.current_tx_id();
        self.rollback_transaction_lockfree_for(tx_id)
    }

    /// #5099: `rollback_transaction_lockfree` for an explicit transaction
    /// id — see [`Self::commit_transaction_lockfree_for`].
    fn rollback_transaction_lockfree_for(&self, tx_id: u64) -> SqlResult<()> {
        let r = self.inner.rollback_transaction_lockfree_for(tx_id);
        if r.is_ok() {
            self.discard_pending_for(tx_id);
        }
        r
    }

    /// BLK-2: forward the `&self` variants so the lockfree transaction
    /// paths reach the backend without laundering a `&mut` out of the
    /// caller's read guard.
    fn set_current_tx_id_shared(&self, id: u64) {
        self.inner.set_current_tx_id_shared(id);
    }

    fn discard_all_buffers_shared(&self) {
        self.inner.discard_all_buffers_shared();
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
        assert!(
            pks.contains(&1),
            "PK=1 should still be visible (inner has the row)"
        );
        assert!(
            pks.contains(&3),
            "PK=3 should still be visible (inner has the row)"
        );
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

// --- #5025: the MVCC version store is per-database -----------------------
//
// `FileStorage` scoping alone is not enough: `MvccStorage` keeps its own
// `VersionedTable` per name, so without a scoped key two databases'
// same-named tables share one version store and a scan in either returns
// both. This is the one that made isolation hold on disk and then not
// hold on read.

#[cfg(test)]
mod db_isolation_tests {
    use super::*;
    use crate::engine::StorageEngine;
    use crate::file_storage::FileStorage;
    use sqlrustgo_types::Value;
    use std::fs::remove_dir_all;

    fn table(name: &str) -> crate::engine::TableInfo {
        let mut info = crate::engine::TableInfo::default();
        info.name = name.to_string();
        info.columns = vec![crate::ColumnDefinition::new("id", "INTEGER")];
        info
    }

    #[test]
    fn mvcc_version_store_is_scoped_per_database() {
        let dir = std::env::temp_dir().join("mvcc_db_iso_5025");
        let _ = remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let inner = FileStorage::new(dir.clone()).unwrap();
        let mut mvcc = MvccStorage::new(inner);
        mvcc.create_database("d1").unwrap();
        mvcc.create_database("d2").unwrap();

        for (db, id) in [("d1", 1i64), ("d2", 2)] {
            mvcc.set_current_db(db).unwrap();
            mvcc.create_table(&table("t")).unwrap();
            mvcc.insert("t", vec![vec![Value::Integer(id)]]).unwrap();
        }

        mvcc.set_current_db("d1").unwrap();
        assert_eq!(
            mvcc.scan("t").unwrap(),
            vec![vec![Value::Integer(1)]],
            "d1 must not see d2's version"
        );
        mvcc.set_current_db("d2").unwrap();
        assert_eq!(
            mvcc.scan("t").unwrap(),
            vec![vec![Value::Integer(2)]],
            "d2 must not see d1's version"
        );

        let _ = remove_dir_all(&dir);
    }
}
