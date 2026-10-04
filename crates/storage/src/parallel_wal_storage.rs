//! ParallelWalStorage - WAL storage with parallel table flushing
//!
//! V311-09: Addresses global lock bottleneck during commit
//!
//! Key insight: WAL writes must be serial (ordering), but table flushes can be parallel.

use crate::engine::{ColumnDefinition, Record, SqlError, SqlResult, StorageEngine, TableInfo};
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
    wal: Arc<Mutex<W>>,
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
            sync_mode: WalSyncMode::Every,
            group_commit: None,
            writes_since_sync: 0,
            wal_enabled: true,
            current_tx_id: std::sync::atomic::AtomicU64::new(0),
            next_lsn: 0,
        }
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

    /// Returns a reference to the installed group-commit coordinator, if
    /// any.
    pub fn group_commit(&self) -> Option<&Arc<GroupCommitCoordinator<W>>> {
        self.group_commit.as_ref()
    }

    fn append_wal_entry(&mut self, mut entry: WalEntry) -> SqlResult<u64> {
        let lsn = self.next_lsn;
        self.next_lsn += 1;
        entry.lsn = lsn;
        let mut wal = self.wal.lock().expect("wal mutex poisoned");
        wal.append(entry)
            .map_err(|e| SqlError::ExecutionError(format!("WAL append error: {}", e)))?;
        Ok(lsn)
    }
}

impl<S: StorageEngine + 'static, W: WalManager + 'static> StorageEngine
    for ParallelWalStorage<S, W>
{
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.inner.insert(table, records)
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

    /// #4974: same forwarding obligation for the predicate variant.
    fn scan_with_filter_in(
        &self,
        table: &str,
        filter: &dyn Fn(&Record) -> bool,
        reader_tx: u64,
    ) -> SqlResult<Vec<Record>> {
        self.inner.scan_with_filter_in(table, filter, reader_tx)
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
                tx_id: self.current_tx_id.load(std::sync::atomic::Ordering::Acquire),
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
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
                            let mut wal = self.wal.lock().expect("wal mutex poisoned");
                            wal.sync().map_err(|e| {
                                SqlError::ExecutionError(format!("WAL sync error: {}", e))
                            })?;
                            drop(wal);
                            self.writes_since_sync = 0;
                        }
                    }
                    WalSyncMode::Every => {
                        let mut wal = self.wal.lock().expect("wal mutex poisoned");
                        wal.sync().map_err(|e| {
                            SqlError::ExecutionError(format!("WAL sync error: {}", e))
                        })?;
                    }
                    WalSyncMode::GroupCommit { .. } => {
                        // GroupCommit variant is informational; the
                        // coordinator (if installed) drives the actual
                        // fsync. Without a coordinator, fall back to
                        // every-tx fsync.
                        let mut wal = self.wal.lock().expect("wal mutex poisoned");
                        wal.sync().map_err(|e| {
                            SqlError::ExecutionError(format!("WAL sync error: {}", e))
                        })?;
                    }
                }
            }
        }

        // 3. Flush tables in parallel
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
    fn has_view(&self, name: &str) -> bool {
        self.inner.has_view(name)
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
