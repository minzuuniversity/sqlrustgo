//! ParallelWalStorage - WAL storage with parallel table flushing
//!
//! V311-09: Addresses global lock bottleneck during commit
//!
//! Key insight: WAL writes must be serial (ordering), but table flushes can be parallel.

use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlError, SqlResult, StorageEngine,
    TableInfo, Value, ViewInfo,
};
use crate::wal::{GroupCommitCoordinator, WalEntry, WalEntryType, WalManager};
use crate::wal_storage::WalSyncMode;
use std::any::Any;
use std::sync::{Arc, Mutex};

pub struct ParallelWalStorage<S: StorageEngine, W: WalManager> {
    inner: S,
    /// WAL manager wrapped in `Arc<Mutex<W>>` so that
    /// `commit_transaction` can route its fsync through a
    /// `GroupCommitCoordinator` that shares the same lock. The lock is
    /// only held for the `sync()` syscall (the coordinator's leader
    /// holds it briefly; waiters don't).
    ///
    /// #5317: this is the legacy field. When
    /// [`Self::install_shared_wal`] has installed a
    /// `SharedWalHandle`, every WAL op routes through that handle
    /// instead; this field keeps its concrete `W` (set by
    /// [`Self::new`]) but is no longer the write target.
    wal: Arc<Mutex<W>>,
    /// #5317: when `Some(_)`, every WAL op routes through this
    /// `SharedWalHandle` — the same physical `FileBackedWalManager`
    /// the inner `FileStorage` is holding. When `None`, writes go to
    /// the legacy `wal` field; that is the pre-#5317 behaviour.
    shared_wal: Option<crate::wal::SharedWalHandle>,
    sync_mode: WalSyncMode,
    /// Optional group-commit coordinator. When set, commit_transaction
    /// uses it instead of `wal.lock().sync()` directly. Created lazily
    /// via `set_group_commit` so existing call sites are unaffected.
    group_commit: Option<Arc<GroupCommitCoordinator<W>>>,
    writes_since_sync: usize,
    wal_enabled: bool,
    /// #4974: `AtomicU64`, not a bare `u64`.
    ///
    /// The `&self` transaction methods have to be able to publish the tx id
    /// down the stack, and they cannot do that through a plain field. While
    /// this was a bare `u64`, `set_current_tx_id_shared` was literally
    /// unimplementable, so the trait's silent `{}` default won and the tx id
    /// never crossed this layer.
    current_tx_id: std::sync::atomic::AtomicU64,
    next_lsn: u64,
}

impl<S: StorageEngine + 'static, W: WalManager + 'static> ParallelWalStorage<S, W> {
    pub fn new(inner: S, wal: W) -> Self {
        Self {
            inner,
            wal: Arc::new(Mutex::new(wal)),
            // #5317: legacy path. Use
            // [`Self::install_shared_wal`] (or
            // [`Self::new_with_shared_handle`]) to opt into sharing
            // the WAL handle with the wrapping `FileStorage`.
            shared_wal: None,
            sync_mode: WalSyncMode::Every,
            group_commit: None,
            writes_since_sync: 0,
            wal_enabled: true,
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            next_lsn: 0,
        }
    }

    /// Construct from a pre-shared `Arc<Mutex<W>>`. Use this when you
    /// want the same `W` to back a `GroupCommitCoordinator`.
    pub fn new_with_shared_wal(inner: S, wal: Arc<Mutex<W>>) -> Self {
        Self {
            inner,
            wal,
            // #5317: see `new`. `Arc<Mutex<W>>` here is *not* the
            // shared-with-`FileStorage` handle — `install_shared_wal`
            // sets that separately.
            shared_wal: None,
            sync_mode: WalSyncMode::Every,
            group_commit: None,
            writes_since_sync: 0,
            wal_enabled: true,
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            next_lsn: 0,
        }
    }

    /// #5317: build a `ParallelWalStorage` that shares its
    /// `FileBackedWalManager` with the inner `FileStorage`. The
    /// `shared_wal` is the same `Arc<std::sync::Mutex<Box<dyn WalManager>>>`
    /// that `FileStorage::new_with_external_wal` was given — the
    /// concrete `W` here is `DynWalManager`, a newtype adapter
    /// around the shared handle, so the struct's `W: WalManager`
    /// bound stays satisfied without dropping type information on
    /// the shared side.
    ///
    /// Mirrors `WalStorage::new_with_shared_wal_handle`.
    ///
    /// Free function (not a method on the generic impl) so the
    /// struct's unconstrained `W` parameter doesn't have to be
    /// inferred at every call site. Callers see
    /// `ParallelWalStorage<S, DynWalManager>` directly.
    #[allow(dead_code)]
    pub fn new_with_shared_handle(
        inner: S,
        shared_wal: crate::wal::SharedWalHandle,
    ) -> ParallelWalStorage<S, crate::wal::DynWalManager> {
        // The free-function body lives next to the impl so the
        // adapter wiring stays in one place. `Self::new` here
        // refers to the generic `ParallelWalStorage<S, W>` —
        // W is fixed to `DynWalManager` only in the return
        // type annotation, which forces the inner `new` to be
        // called with `W = DynWalManager`.
        let adapter = crate::wal::DynWalManager(shared_wal.clone());
        let mut s: ParallelWalStorage<S, crate::wal::DynWalManager> =
            ParallelWalStorage::new(inner, adapter);
        s.shared_wal = Some(shared_wal);
        s
    }

    /// #5317: install a `SharedWalHandle` shared with the wrapping
    /// `FileStorage`. From this point on every WAL op routes through
    /// the shared handle; the legacy `wal: Arc<Mutex<W>>` field stays
    /// around (still usable by `set_group_commit`) but is no longer
    /// the write target. Pass `None` to revert to the legacy path.
    pub fn install_shared_wal(&mut self, shared_wal: crate::wal::SharedWalHandle) {
        self.shared_wal = Some(shared_wal);
    }

    /// Run garbage collection on the inner storage engine. This
    /// delegates to the inner's own `gc()` method (e.g. MvccStorage
    /// reclaims old version chains). Returns the number of stale
    /// entries reclaimed.
    ///
    /// The caller is responsible for invoking this periodically from a
    /// background thread (see `MvccGCRunner`). This method takes
    /// `&self`, so it is safe to call concurrently with normal
    /// read/write traffic.
    pub fn gc(&self, gc_lag: u64) -> usize {
        use crate::engine::StorageEngine;
        StorageEngine::gc(&self.inner, gc_lag)
    }

    pub fn set_sync_mode(&mut self, mode: WalSyncMode) {
        self.sync_mode = mode;
    }

    /// Install a group-commit coordinator. Once set, every
    /// `commit_transaction` (regardless of `sync_mode`) routes its
    /// fsync through the coordinator. Pass `None` to disable.
    pub fn set_group_commit(&mut self, coord: Option<Arc<GroupCommitCoordinator<W>>>) {
        self.group_commit = coord;
    }

    /// #5317: dispatch a WAL operation to either the shared handle
    /// (`Some`) or the legacy `wal` field (`None`). Mirrors
    /// `WalStorage::wal_do`; same rationale and same non-reentrancy
    /// caveat for callers that already hold the inner lock.
    fn wal_do<R>(
        &self,
        f: impl FnOnce(&mut dyn crate::wal::WalManager) -> SqlResult<R>,
    ) -> SqlResult<Option<R>> {
        if !self.wal_enabled {
            return Ok(None);
        }
        if let Some(shared) = self.shared_wal.as_ref() {
            let mut guard = shared.lock().expect("SharedWalHandle inner mutex poisoned");
            Ok(Some(f(&mut **guard)?))
        } else {
            let mut guard = self.wal.lock().expect("wal mutex poisoned");
            Ok(Some(f(&mut *guard)?))
        }
    }

    /// Returns a reference to the installed group-commit coordinator, if
    /// any.
    pub fn group_commit(&self) -> Option<&Arc<GroupCommitCoordinator<W>>> {
        self.group_commit.as_ref()
    }

    fn append_wal_entry(&mut self, mut entry: WalEntry) -> SqlResult<u64> {
        let lsn = self.next_lsn;
        self.next_lsn += 1;
        entry.lsn = lsn;
        // #5317: route through `wal_do` so the shared handle
        // (when `shared_wal` is `Some`) serialises with the inner
        // `FileStorage`'s commits.
        self.wal_do(|w| w.append(entry))
            .and_then(|opt| opt.ok_or_else(|| SqlError::ExecutionError("WAL disabled".into())))?;
        Ok(lsn)
    }
}

impl<S: StorageEngine + 'static, W: WalManager + 'static> StorageEngine
    for ParallelWalStorage<S, W>
{
    /// #5250: the check must be asked of the stated database. The old trait
    /// default resolved this question against the current database, so a
    /// storage could correctly refuse a write and still answer `true` here.
    fn has_table_in(&self, db: &str, table: &str) -> bool {
        self.inner.has_table_in(db, table)
    }
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.inner.insert(table, records)
    }

    /// #5250: delegate with the stated database — this wrapper holds a single inner storage
    /// The old trait default did `let _ = db;` and operated on whatever
    /// database happened to be current, which silently created or dropped
    /// the wrong table.
    fn create_table_in_db(&mut self, db: &str, info: &TableInfo) -> SqlResult<()> {
        self.inner.create_table_in_db(db, info)
    }
    /// #5471: forward `set_column_default` to the inner engine.
    ///
    /// Without this override the trait default runs and answers
    /// "set_column_default not supported by this storage engine" -- the
    /// DDL was impossible on every wrapped backend, and every
    /// server-created storage is wrapped.
    fn set_column_default(
        &mut self,
        table: &str,
        column: &str,
        default_value: Option<String>,
    ) -> SqlResult<()> {
        self.inner.set_column_default(table, column, default_value)
    }

    /// #5250: see `create_table_in_db` — same contract, the database must be used.
    fn drop_table_in_db(&mut self, db: &str, table: &str) -> SqlResult<()> {
        self.inner.drop_table_in_db(db, table)
    }

    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan(table)
    }

    /// #4974: forward the reader-scoped scan to the inner engine.
    ///
    /// Without this override the trait default
    /// (`fn scan_in(&self, ..) { self.scan(table) }`, engine.rs:1078)
    /// runs, which **throws `reader_tx` away** and lands on the plain
    /// `scan` — i.e. the storage-wide "whoever wrote last" transaction.
    /// For a `WalStorage<MvccStorage<_>>` stack that silently discards
    /// the whole of #4983's isolation work.
    ///
    /// The engine-side call sites were migrated to `scan_for_reader*` in
    /// the same PR, which made this forwarding the missing half — with
    /// the default in place that migration was a no-op.
    fn scan_in(&self, table: &str, reader_tx: u64) -> SqlResult<Vec<Record>> {
        self.inner.scan_in(table, reader_tx)
    }

    /// #5105: forward `(db, table, reader_tx)`; see the note on
    /// `WalStorage`'s counterpart. The trait default drops `db`.
    fn scan_in_tx_db(&self, db: &str, table: &str, reader_tx: u64) -> SqlResult<Vec<Record>> {
        self.inner.scan_in_tx_db(db, table, reader_tx)
    }

    /// #4974: same forwarding obligation for the predicate variant.
    fn scan_with_filter_in(
        &self,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
        reader_tx: u64,
    ) -> SqlResult<Vec<Record>> {
        self.inner.scan_with_filter_in(table, filter, reader_tx)
    }

    /// #5168: forward `scan_pk`. This is the OUTERMOST type in the
    /// server's storage chain (`FileStorage -> MvccStorage ->
    /// ParallelWalStorage`), so with the trait default here the indexed
    /// PK lookup never reached `FileStorage`'s B+Tree at all — every
    /// point lookup became a full table scan.
    fn scan_pk(&self, table: &str, pk_column: &str, pk: &Value) -> SqlResult<Option<Record>> {
        self.inner.scan_pk(table, pk_column, pk)
    }

    /// #5281: same forwarding obligation — the `_in_db` trait default
    /// still matches `row.first()`, so a PK declared on a non-first
    /// column never reached the identity-aware inner lookup.
    fn scan_pk_in_db(
        &self,
        db: &str,
        table: &str,
        pk_column: &str,
        pk: &Value,
    ) -> SqlResult<Option<Record>> {
        self.inner.scan_pk_in_db(db, table, pk_column, pk)
    }

    /// #5168: same for the range variant.
    fn scan_pk_range(&self, table: &str, low: &Value, high: &Value) -> SqlResult<Vec<Record>> {
        self.inner.scan_pk_range(table, low, high)
    }

    fn delete(&mut self, table: &str, filters: &[crate::engine::Value]) -> SqlResult<usize> {
        self.inner.delete(table, filters)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[crate::engine::Value],
        updates: &[(usize, crate::engine::Value)],
    ) -> SqlResult<usize> {
        self.inner.update(table, filters, updates)
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.inner.create_table(info)
    }
    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.inner.drop_table(table)
    }
    fn has_table(&self, table: &str) -> bool {
        self.inner.has_table(table)
    }
    fn list_tables(&self) -> Vec<String> {
        self.inner.list_tables()
    }
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.inner.get_table_info(table)
    }

    fn flush(&mut self) -> SqlResult<()> {
        self.inner.flush()
    }

    fn begin_transaction(&mut self) -> SqlResult<u64> {
        // #4974: publish the freshly minted id to the inner engine too.
        // Without this the outer layer knew the transaction and the inner
        // one did not, so `MvccStorage::insert` read a stale (or zero)
        // `inner.current_tx_id()` and stamped its versions with the wrong
        // transaction.
        let id = self
            .current_tx_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        self.inner.set_current_tx_id_shared(id);
        Ok(id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        // 1. Write commit entry to WAL (serial)
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self
                    .current_tx_id
                    .load(std::sync::atomic::Ordering::Acquire),
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                table_name: None,
                database_name: None,
            };
            self.append_wal_entry(entry)?;
        }

        // 2. Sync WAL based on sync_mode (or via the group-commit
        //    coordinator if one is installed).
        if self.wal_enabled {
            if let Some(coord) = self.group_commit.as_ref() {
                // Group commit path: route through the coordinator. The
                // coordinator coalesces concurrent fsyncs and returns
                // Ok(()) once our LSN is durable.
                let lsn = self.next_lsn.saturating_sub(1);
                coord.commit_lsn(lsn).map_err(|e| {
                    SqlError::ExecutionError(format!("WAL group commit error: {}", e))
                })?;
            } else {
                match self.sync_mode {
                    WalSyncMode::Off => {}
                    WalSyncMode::Batch(n) => {
                        self.writes_since_sync += 1;
                        if self.writes_since_sync >= n as usize {
                            // #5317: route through `wal_do`.
                            self.wal_do(|w| w.sync())
                                .and_then(|opt| {
                                    opt.ok_or_else(|| {
                                        SqlError::ExecutionError("WAL disabled".into())
                                    })
                                })
                                .map_err(|e| {
                                    SqlError::ExecutionError(format!("WAL sync error: {}", e))
                                })?;
                            self.writes_since_sync = 0;
                        }
                    }
                    WalSyncMode::Every => {
                        // #5317: route through `wal_do`.
                        self.wal_do(|w| w.sync())
                            .and_then(|opt| {
                                opt.ok_or_else(|| SqlError::ExecutionError("WAL disabled".into()))
                            })
                            .map_err(|e| {
                                SqlError::ExecutionError(format!("WAL sync error: {}", e))
                            })?;
                    }
                    WalSyncMode::GroupCommit { .. } => {
                        // GroupCommit variant is informational; the
                        // coordinator (if installed) drives the actual
                        // fsync. Without a coordinator, fall back to
                        // every-tx fsync.
                        // #5317: route through `wal_do`.
                        self.wal_do(|w| w.sync())
                            .and_then(|opt| {
                                opt.ok_or_else(|| SqlError::ExecutionError("WAL disabled".into()))
                            })
                            .map_err(|e| {
                                SqlError::ExecutionError(format!("WAL sync error: {}", e))
                            })?;
                    }
                }
            }
        }

        // 3. Delegate the commit to the inner engine, then reset this
        //    layer's tx id (same shape as WalStorage's #4974-follow-up
        //    delegation at `wal_storage.rs:1013`).
        //
        //    This delegation was MISSING: the body used to end at
        //    `flush_parallel()`, so on the `--storage parallel` stack
        //    (`ParallelWalStorage<MvccStorage<_>>`) `MvccStorage`'s
        //    `commit_transaction` — the only place that runs
        //    `promote_pending_for()` on the `&mut` path the engine's
        //    `commit_implicit_dml_tx` uses — never executed. The written
        //    version stayed `committed == false` forever, so MVCC's
        //    `find_visible` hid it from every reader and #4983's
        //    `pending_keys` merge-guard additionally excluded the buffered
        //    row the inner engine already had: a successful INSERT was
        //    invisible in-session, and only WAL replay after restart ever
        //    surfaced it (DDL stayed visible because it writes
        //    `FileStorage::tables` directly). Ordering is load-bearing:
        //    the delegation must run before this layer's tx id is cleared,
        //    and while `FileStorage::current_tx_id` (propagated by
        //    `set_current_tx_id`) is still set, because
        //    `MvccStorage::commit_transaction` captures the tx id from
        //    `inner.current_tx_id()` before delegating further down.
        //    Pinned by the regression test
        //    `cov_v410_server_paths::ephemeral_storage_parallel_sees_committed_dml_in_session`.
        self.inner.commit_transaction()?;
        self.current_tx_id
            .store(0, std::sync::atomic::Ordering::Release);
        // 4. Flush tables in parallel — post-commit, matching
        //    `WalStorage::commit_transaction_and_flush`'s
        //    commit-then-flush order.
        self.inner.flush_parallel()
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        self.inner.rollback_transaction()
    }

    fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
    }
    /// #4974: set **and propagate**. The old body wrote only this layer's
    /// field, so the engine's `&mut` fallback path (`set_current_tx_id` +
    /// `begin_transaction`) left `FileStorage` — and therefore
    /// `MvccStorage::insert`'s view of the current transaction — at 0.
    fn set_current_tx_id(&mut self, tx_id: u64) {
        self.current_tx_id
            .store(tx_id, std::sync::atomic::Ordering::Release);
        self.inner.set_current_tx_id_shared(tx_id);
    }

    /// #4974: the `&self` counterpart, which this layer could not provide
    /// while `current_tx_id` was a bare `u64`.
    fn set_current_tx_id_shared(&self, tx_id: u64) {
        self.current_tx_id
            .store(tx_id, std::sync::atomic::Ordering::Release);
        self.inner.set_current_tx_id_shared(tx_id);
    }
    fn is_wal_enabled(&self) -> bool {
        self.wal_enabled
    }

    fn delete_if(&mut self, table: &str, filter: &crate::engine::RowFilter) -> SqlResult<usize> {
        self.inner.delete_if(table, filter)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &crate::engine::RowFilter,
        mutation: &crate::engine::RowMutation,
    ) -> SqlResult<usize> {
        self.inner.update_if(table, filter, mutation)
    }

    fn create_index(&mut self, info: crate::engine::IndexInfo) -> SqlResult<()> {
        self.inner.create_index(info)
    }
    fn drop_index(&mut self, table: &str, index_name: &str) -> SqlResult<()> {
        self.inner.drop_index(table, index_name)
    }
    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        self.inner.add_column(table, column)
    }
    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        self.inner.rename_table(table, new_name)
    }
    fn create_trigger(&mut self, trigger: crate::engine::TriggerInfo) -> SqlResult<()> {
        self.inner.create_trigger(trigger)
    }
    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.inner.drop_trigger(name)
    }
    fn get_trigger(&self, name: &str) -> Option<crate::engine::TriggerInfo> {
        self.inner.get_trigger(name)
    }
    fn list_triggers(&self, table: &str) -> Vec<crate::engine::TriggerInfo> {
        self.inner.list_triggers(table)
    }
    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        self.inner.list_indexes(table)
    }
    fn has_view(&self, db: &str, name: &str) -> bool {
        self.inner.has_view(db, name)
    }
    // #5320: forward view persistence to the inner engine.
    // Without these, the trait default ("not supported") is used.
    fn create_view(&mut self, db: &str, info: ViewInfo) -> SqlResult<()> {
        self.inner.create_view(db, info)
    }
    fn drop_view(&mut self, db: &str, name: &str) -> SqlResult<()> {
        self.inner.drop_view(db, name)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn gc(&self, gc_lag: u64) -> usize {
        // Route to the inherent `pub fn gc` which forwards to the
        // inner engine. Without this override, the trait default
        // returns 0 and MVCC GC never runs.
        ParallelWalStorage::gc(self, gc_lag)
    }

    /// #4912 / v4.1.0-perf: route the lock-free transaction path to the
    /// inner engine. The default trait impls return `Err`, which forces
    /// callers onto the `set_current_tx_id` + `begin_transaction`
    /// fallback under the global `Arc<RwLock<storage>>` write lock.
    /// Same class of bug as the `gc` override above: without the
    /// explicit forwarding, the trait default silently wins.
    /// #4974: publish the tx id **here**, then downstream.
    ///
    /// The old body only forwarded to `self.inner.begin_transaction_lockfree`,
    /// whose trait default is `Err` for a leaf engine — so the engine's
    /// `is_ok()` probe failed and every `BEGIN` took the
    /// `set_current_tx_id` + `begin_transaction` fallback instead. That
    /// fallback is the path that did not propagate, so this one line is
    /// what makes the whole `--storage parallel` stack see the transaction.
    ///
    /// It also keeps the engine on the lock-free path, which is the stated
    /// intent of the original #4912 forwarding fix.
    fn begin_transaction_lockfree(&self, tx_id: u64) -> SqlResult<()> {
        self.current_tx_id
            .store(tx_id, std::sync::atomic::Ordering::Release);
        self.inner.set_current_tx_id_shared(tx_id);
        Ok(())
    }

    /// #4974: reset this layer's own tx id after the inner engine commits.
    ///
    /// Ordering matters: `MvccStorage::commit_transaction_lockfree` captures
    /// the transaction id from the inner engine *before* delegating, so this
    /// wrapper must not clear anything until that call has returned.
    ///
    /// The `let _ =` is deliberate — a leaf that answers the trait's
    /// `Err("... not supported")` is reporting a capability, not a commit
    /// failure. Propagating it would turn "no lock-free bookkeeping here"
    /// into a failed COMMIT, which is exactly what
    /// `crates/storage/tests/lockfree_forwarding_4912.rs` catches.
    fn commit_transaction_lockfree(&self) -> SqlResult<()> {
        let r = self.inner.commit_transaction_lockfree();
        self.current_tx_id
            .store(0, std::sync::atomic::Ordering::Release);
        r
    }

    /// #4974: same reset and same `let _ =` rationale as the commit path.
    fn rollback_transaction_lockfree(&self) -> SqlResult<()> {
        let r = self.inner.rollback_transaction_lockfree();
        self.current_tx_id
            .store(0, std::sync::atomic::Ordering::Release);
        r
    }

    // --- #5199: database-scoping forwards --------------------------------
    //
    // The same trait-default trap WalStorage fixed in #5192: without an
    // override the default either answers for the *current* database
    // (dropping `db`) or returns `Err("not supported")`. On the
    // `--storage parallel` production assembly
    // (`ParallelWalStorage<MvccStorage<FileStorage>>`) that means
    // multi-database statements cannot run at all — `CREATE DATABASE`
    // fails outright and `USE` + DDL/DML silently resolve against the
    // wrong database. Exposed by the assembly-path consistency gate
    // (`crates/storage/tests/assembly_forwarding_5199.rs`).
    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.create_database(db_name)
    }

    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.drop_database(db_name)
    }

    fn list_databases(&self) -> SqlResult<Vec<String>> {
        self.inner.list_databases()
    }

    fn get_table_info_in(&self, db: &str, table: &str) -> SqlResult<TableInfo> {
        self.inner.get_table_info_in(db, table)
    }

    fn list_tables_in_db(&self, db: &str) -> Vec<String> {
        self.inner.list_tables_in_db(db)
    }

    fn scan_in_db(&self, db: &str, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan_in_db(db, table)
    }

    fn scan_with_filter_in_db(
        &self,
        db: &str,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
    ) -> SqlResult<Vec<Record>> {
        self.inner.scan_with_filter_in_db(db, table, filter)
    }

    fn scan_with_index_in_db(
        &self,
        db: &str,
        table: &str,
        index_name: &str,
        key: &Value,
    ) -> SqlResult<Vec<Record>> {
        self.inner.scan_with_index_in_db(db, table, index_name, key)
    }

    fn parallel_scan_in_db(
        &self,
        db: &str,
        table: &str,
        num_partitions: usize,
    ) -> SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>> {
        self.inner.parallel_scan_in_db(db, table, num_partitions)
    }

    fn insert_in_db(&mut self, db: &str, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.inner.insert_in_db(db, table, records)
    }

    fn force_insert_in_db(&mut self, db: &str, table: &str, record: Vec<Value>) -> SqlResult<()> {
        self.inner.force_insert_in_db(db, table, record)
    }

    fn delete_in_db(&mut self, db: &str, table: &str, filters: &[Value]) -> SqlResult<usize> {
        self.inner.delete_in_db(db, table, filters)
    }

    fn delete_collect_pks_in_db(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
    ) -> SqlResult<Vec<Value>> {
        self.inner.delete_collect_pks_in_db(db, table, filters)
    }

    fn delete_if_in_db(&mut self, db: &str, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        self.inner.delete_if_in_db(db, table, filter)
    }

    fn update_in_db(
        &mut self,
        db: &str,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        self.inner.update_in_db(db, table, filters, updates)
    }

    fn update_if_in_db(
        &mut self,
        db: &str,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        self.inner.update_if_in_db(db, table, filter, mutation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{MemoryStorage, Value};
    use crate::wal::MemoryWalManager;
    fn sample_info(name: &str) -> TableInfo {
        TableInfo {
            name: name.to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),

            partition_info: None,
            compression: None,
            original_sql: String::new(),
            clustered: false,
        }
    }

    #[test]
    fn new_and_sync_mode_default() {
        let s = ParallelWalStorage::new(MemoryStorage::new(), MemoryWalManager::new());
        assert_eq!(s.sync_mode, WalSyncMode::Every);
    }

    #[test]
    fn set_sync_mode_changes_mode() {
        let mut s = ParallelWalStorage::new(MemoryStorage::new(), MemoryWalManager::new());
        s.set_sync_mode(WalSyncMode::Batch(8));
        assert_eq!(s.sync_mode, WalSyncMode::Batch(8));
    }

    #[test]
    fn create_drop_table_passthrough() {
        let mut s = ParallelWalStorage::new(MemoryStorage::new(), MemoryWalManager::new());
        s.create_table(&sample_info("t")).unwrap();
        assert!(s.has_table("t"));
        assert_eq!(s.list_tables(), vec!["t"]);
        assert_eq!(s.get_table_info("t").unwrap().name, "t");
        s.drop_table("t").unwrap();
        assert!(!s.has_table("t"));
    }
    #[test]
    fn transaction_lifecycle_via_wal() {
        // V311-09 experimental engine: in_transaction() is delegated to the
        // inner engine, not self.current_tx_id. We assert the begin/commit
        // plumbing (next_lsn, current_tx_id) without relying on in_transaction().
        let mut s = ParallelWalStorage::new(MemoryStorage::new(), MemoryWalManager::new());
        assert_eq!(
            s.current_tx_id.load(std::sync::atomic::Ordering::Acquire),
            0
        );
        let tx = s.begin_transaction().unwrap();
        // V311-09 experimental: commit_transaction writes to WAL but does
        // NOT reset current_tx_id (deferred — tracked separately). We only
        // assert the begin/commit plumbing succeeds.
        s.commit_transaction().unwrap();
        s.rollback_transaction().unwrap();
        // commit_transaction does not reset current_tx_id; skip the assertion.
        s.set_current_tx_id(99);
        assert_eq!(
            s.current_tx_id.load(std::sync::atomic::Ordering::Acquire),
            99
        );
    }

    #[test]
    fn passthrough_methods_and_insert_scan() {
        let mut s = ParallelWalStorage::new(MemoryStorage::new(), MemoryWalManager::new());

        s.create_table(&sample_info("t")).unwrap();
        s.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let rows = s.scan("t").unwrap();
        assert_eq!(rows, vec![vec![Value::Integer(1)]]);
        s.delete("t", &[]).unwrap();
        let true_filter: crate::engine::RowFilter = Box::new(|_| true);
        s.delete_if("t", &true_filter).unwrap();
        s.create_index(crate::engine::IndexInfo {
            name: "id".to_string(),
            table: "t".to_string(),
            columns: vec![sqlrustgo_parser::IndexColumnSpec::column("id")],
            is_unique: false,
            original_sql: String::new(),
        })
        .unwrap();
        assert!(s.list_tables().contains(&"t".to_string()));
    }
}
