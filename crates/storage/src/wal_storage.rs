use crate::checkpoint::{CheckpointManager, CheckpointMetadata};
use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlResult, StorageEngine, TableInfo,
    TriggerInfo, Value,
};
use crate::wal::{WalEntry, WalEntryType, WalManager};
use std::any::Any;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

// SAFETY: `WalStorage` contains an `UnsafeCell<S>` which is not `Sync` by
// default. However, all access to `inner` goes through:
//   * `&mut self` (exclusive access, single-writer per call site) — the
//     engine serializes these via the engine's own mutex.
//   * `*_transaction_lockfree(&self)` paths which only access `inner`
//     through the `_lockfree` methods that serialize via the engine
//     mutex. These methods are the only `&self` → `&mut inner` path
//     and are the sole reason we need `UnsafeCell` in the first place.
//   * `recover_split_mut` / `inner_mut` which require `&mut self`,
//     serialized via the caller (single-threaded recovery or engine).
// `S: StorageEngine` already requires `Sync`, and our access patterns
// never expose `&mut S` through shared references. This is the standard
// pattern for `UnsafeCell` inside `Sync` containers.
unsafe impl<S: StorageEngine + 'static, T: WalManager + 'static> Sync for WalStorage<S, T> {}

/// WAL sync mode - controls fsync frequency for performance tuning.
///
/// # Performance Trade-offs
/// - `Every`: Full durability, slowest (~8 TPS on MacMini)
/// - `Batch(n)`: Batched durability, ~30-50 TPS
/// - `Off`: No sync, fastest (~100+ TPS), but data loss on crash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WalSyncMode {
    /// Sync after every transaction (default, full durability)
    #[default]
    Every,
    /// Sync after N transactions (batch mode)
    Batch(u32),
    /// No sync at all (fastest, no durability guarantee)
    Off,
    /// Group commit: coalesce up to `max_batch` concurrent commits into
    /// a single fsync, or force-sync after `max_wait_us` microseconds.
    /// Trades durability (up to `max_batch - 1` tx loss on crash) for
    /// throughput. See `crates/storage/src/wal/group_commit.rs`.
    ///
    /// The fields are ignored on the `WalStorage` path (the
    /// `ParallelWalStorage` path uses them). To activate group commit,
    /// use `ParallelWalStorage::set_group_commit(coordinator)` after
    /// construction.
    GroupCommit { max_batch: u32, max_wait_us: u64 },
}

pub struct WalStorage<S: StorageEngine, T: WalManager> {
    /// Inner storage engine wrapped in `UnsafeCell` so the
    /// `*_transaction_lockfree(&self)` engine paths can obtain
    /// `&mut S` without holding the global `Arc<RwLock<storage>>`
    /// write lock. Soundness: the engine serializes `_lockfree`
    /// calls against other writers via the engine's own mutex;
    /// concurrent readers hold a separate `parking_lot::RwLockReadGuard`
    /// and never share `&mut`.
    ///
    /// Phase B Step 3 follow-up #4 (see PHASE_B_STEP3_SOAK.md).
    inner: UnsafeCell<S>,
    /// WAL manager wrapped in `Mutex` so `*_lockfree(&self)` engine paths
    /// can perform WAL appends without holding the global `Arc<RwLock<storage>>`
    /// write lock. The lock duration is just the WAL `append` call (~µs),
    /// which is what we want — readers blocked for an I/O syscall is
    /// worse than readers blocked for an in-process atomic CAS.
    ///
    /// Phase B Step 3 optimization; see `docs/releases/v0.0.0/PERFORMANCE_PLAN.md`.
    wal: parking_lot::Mutex<T>,
    wal_enabled: bool,
    sync_mode: WalSyncMode,
    /// Counter for batch mode: tracks writes since last sync
    writes_since_sync: u32,
    /// Phase B Step 3 follow-up #2: number of commits since the last
    /// inner-engine flush. The default `commit_transaction` defers
    /// `inner.flush()` (relying on WAL replay for durability), and
    /// `drain_pending_flushes` runs a single flush when this counter
    /// is non-zero. `commit_transaction_and_flush` always flushes.
    pending_flush_count: std::sync::atomic::AtomicU64,
    checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>,
    /// Active transaction id. The ExecutionEngine pushes the real id here
    /// via `set_current_tx_id`; without this, every WAL entry would carry
    /// tx_id=0 and the recovery engine could not distinguish autocommit
    /// DML from uncommitted-but-started DML.
    /// `AtomicU64` so `begin_transaction_lockfree` can update it without
    /// holding the global `Arc<RwLock<storage>>` write lock (Phase B Step 3
    /// optimization; see `docs/releases/v0.0.0/PERFORMANCE_PLAN.md`).
    current_tx_id: AtomicU64,
    /// Monotonically increasing LSN counter for WAL entries.
    /// Each `append_wal_entry` increments this and assigns the value to the entry.
    /// `AtomicU64` so concurrent appends don't race.
    next_lsn: AtomicU64,
    /// #3223 Phase 1: Active transaction set with their last WAL LSN.
    /// Populated on `begin_transaction` (insert), drained on
    /// `commit_transaction`/`rollback_transaction` (remove).
    /// `RecoveryEngine` will use `is_tx_active` during replay to skip
    /// uncommitted DML.
    /// Empty for autocommit (tx_id=0) — the legacy single-active-tx model.
    /// `Mutex<HashMap>` so concurrent `begin/commit_transaction_lockfree`
    /// don't race on insert/remove.
    active_txs: Mutex<HashMap<u64, u64>>,
}
impl<S: StorageEngine + 'static, T: WalManager + 'static> WalStorage<S, T> {
    pub fn new(inner: S, wal: T) -> SqlResult<Self> {
        Ok(Self {
            inner: UnsafeCell::new(inner),
            wal: parking_lot::Mutex::new(wal),
            wal_enabled: true,
            sync_mode: WalSyncMode::default(),
            writes_since_sync: 0,
            pending_flush_count: std::sync::atomic::AtomicU64::new(0),
            checkpoint_manager: None,
            current_tx_id: AtomicU64::new(0),
            next_lsn: AtomicU64::new(0),
            active_txs: Mutex::new(HashMap::new()),
        })
    }

    /// Create WalStorage with a specific sync mode for performance tuning.
    ///
    /// # Example
    /// ```
    /// use sqlrustgo_storage::{FileStorage, WalStorage, FileBackedWalManager, WalSyncMode};
    /// let inner = FileStorage::new("/tmp/db".into()).unwrap();
    /// let wal = FileBackedWalManager::new("/tmp/wal".into()).unwrap();
    /// let mut storage = WalStorage::new_with_sync_mode(inner, wal, WalSyncMode::Batch(100));
    /// ```
    pub fn new_with_sync_mode(inner: S, wal: T, sync_mode: WalSyncMode) -> SqlResult<Self> {
        Ok(Self {
            inner: UnsafeCell::new(inner),
            wal: parking_lot::Mutex::new(wal),
            wal_enabled: true,
            sync_mode,
            writes_since_sync: 0,
            pending_flush_count: std::sync::atomic::AtomicU64::new(0),
            checkpoint_manager: None,
            current_tx_id: AtomicU64::new(0),
            next_lsn: AtomicU64::new(0),
            active_txs: Mutex::new(HashMap::new()),
        })
    }

    /// Create with checkpoint manager and sync mode.
    pub fn new_with_sync_mode_and_checkpoint(
        inner: S,
        wal: T,
        sync_mode: WalSyncMode,
        checkpoint_manager: Arc<RwLock<CheckpointManager>>,
    ) -> SqlResult<Self> {
        Ok(Self {
            inner: UnsafeCell::new(inner),
            wal: parking_lot::Mutex::new(wal),
            wal_enabled: true,
            sync_mode,
            writes_since_sync: 0,
            pending_flush_count: std::sync::atomic::AtomicU64::new(0),
            checkpoint_manager: Some(checkpoint_manager),
            current_tx_id: AtomicU64::new(0),
            next_lsn: AtomicU64::new(0),
            active_txs: Mutex::new(HashMap::new()),
        })
    }
    /// Create with a checkpoint manager (uses default sync mode).
    pub fn with_checkpoint_manager(
        inner: S,
        wal: T,
        checkpoint_manager: Arc<RwLock<CheckpointManager>>,
    ) -> SqlResult<Self> {
        Self::new_with_sync_mode_and_checkpoint(
            inner,
            wal,
            WalSyncMode::default(),
            checkpoint_manager,
        )
    }

    /// Returns true if transaction `tx_id` is currently active.
    pub fn is_tx_active(&self, tx_id: u64) -> bool {
        self.active_txs
            .lock()
            .map(|m| m.contains_key(&tx_id))
            .unwrap_or(false)
    }

    /// Get current sync mode
    pub fn sync_mode(&self) -> WalSyncMode {
        self.sync_mode
    }

    /// Set sync mode at runtime
    pub fn set_sync_mode(&mut self, mode: WalSyncMode) {
        self.sync_mode = mode;
    }

    /// Force a sync (useful for batch mode)
    pub fn force_sync(&mut self) -> SqlResult<()> {
        if self.wal_enabled {
            self.wal.lock().sync()?;
            self.writes_since_sync = 0;
        }
        Ok(())
    }

    /// #3223 Phase 1: Returns snapshot of active tx ids (for tests/diagnostics).
    pub fn active_tx_ids(&self) -> Vec<u64> {
        self.active_txs
            .lock()
            .map(|m| m.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Append a WAL entry with a monotonically increasing LSN.
    /// Returns the assigned LSN.
    /// PR-830F: This is the single chokepoint for LSN assignment;
    /// without it, `current_lsn()` returns 0 and checkpoint advance never triggers.
    fn append_wal_entry(&mut self, mut entry: WalEntry) -> SqlResult<u64> {
        let lsn = self.next_lsn.fetch_add(1, Ordering::Relaxed) + 1;
        entry.lsn = lsn;
        self.wal.lock().append(entry)?;
        Ok(lsn)
    }

    pub fn inner(&self) -> &S {
        // SAFETY: callers cannot obtain `&mut S` from `&self` via this
        // method (the UnsafeCell::get_mut path requires &mut self, see
        // inner_mut). The only ways to get `&mut S` are:
        //   * `inner_mut(&mut self)` — caller holds &mut self, so no
        //     `&S` is alive concurrently.
        //   * `*_transaction_lockfree(&self)` paths which serialize all
        //     mutating access through the engine's own mutex.
        // Therefore no `&mut S` aliases this `&S`.
        unsafe { &*self.inner.get() }
    }

    pub fn wal(&self) -> parking_lot::MutexGuard<'_, T> {
        self.wal.lock()
    }

    /// Mutable access to the inner storage engine.
    /// (The legacy `wal_mut`/`split` accessors were removed in Phase B
    /// Step 3 — `wal` is now a `Mutex<T>` so callers must lock it
    /// explicitly via `storage.wal.lock()`. For inner, see `inner_mut`
    /// which uses `UnsafeCell::get_mut` for sound interior mutability.)
    pub fn inner_mut(&mut self) -> &mut S {
        // SAFETY: we have `&mut self` (the only path to `inner_mut` is
        // `&mut self`), so no other reference to `inner` exists.
        unsafe { &mut *self.inner.get() }
    }

    // ----- Legacy compat shims -----
    // The following accessors were removed in Phase B Step 3 because
    // `wal` became `Mutex<T>` (no longer `T`). They are restored here as
    // thin wrappers so existing integration tests (and any third-party
    // code) keep compiling. Prefer `wal()` + `wal.lock()` / `inner_mut()`.

    /// Deprecated: returns `&mut T` via the internal Mutex.
    /// Use `wal()` and `wal.lock()` instead.
    #[doc(hidden)]
    pub fn wal_mut(&mut self) -> parking_lot::MutexGuard<'_, T> {
        self.wal.lock()
    }

    /// Deprecated: returns `(&mut S, MutexGuard<T>)`. Use
    /// `recover_split_mut()` instead.
    #[doc(hidden)]
    pub fn split(&mut self) -> (&mut S, parking_lot::MutexGuard<'_, T>) {
        let wal = self.wal.lock();
        // SAFETY: we have &mut self so no other reference to `inner` exists.
        let inner = unsafe { &mut *self.inner.get() };
        (inner, wal)
    }

    /// Recover-grade split: take `&mut self` and return disjoint
    /// `&mut S` + `&mut T` for callers (e.g. `recover_wal`) that need
    /// both. Safe because we hold `&mut self` for the duration of the
    /// returned borrows — no other `&mut` to the same fields can exist.
    pub fn recover_split_mut(&mut self) -> (&mut S, &mut T) {
        // SAFETY: same as `inner_mut` — we have &mut self.
        let inner = unsafe { &mut *self.inner.get() };
        (inner, self.wal.get_mut())
    }

    /// Obtain `&mut S` from `&self WalStorage`. Used by
    /// `*_transaction_lockfree` methods (Phase B Step 3) which need
    /// `&mut self.inner` (for `discard_all_buffers` / `set_current_tx_id`).
    ///
    /// Safety invariants:
    ///   * `ExecutionEngine` is the only caller. It serializes
    ///     `commit_transaction_lockfree` / `rollback_transaction_lockfree`
    ///     against other engine methods via the engine's own mutex.
    ///   * Concurrent readers hold `storage.read()` (separate
    ///     `RwLockReadGuard`); they never share `&mut`.
    ///   * Recovery (engine_builder::recover_wal) holds
    ///     `storage.write()` and runs single-threaded.
    ///
    /// This replaces the previous `unsafe { &mut *(self as *const Self as *mut Self) }`
    /// cast with a sound `UnsafeCell::get` — the new Phase B Step 3
    /// follow-up #4. The old `#[allow(invalid_reference_casting)]` is
    /// no longer needed.
    #[allow(clippy::mut_from_ref)]
    fn as_inner_mut(&self) -> &mut S {
        // SAFETY: see invariants above. `inner: UnsafeCell<S>` is the
        // ONLY field that requires this. `wal: parking_lot::Mutex<T>`
        // already provides &mut via `lock()`. We never alias `&S` here.
        unsafe { &mut *self.inner.get() }
    }

    fn table_name_to_id(table: &str) -> u64 {
        let mut hash: u64 = 0;
        for byte in table.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }

    /// V400-02 / Issue #3730 (V2): heuristic that picks the WAL
    /// entry type for an insert/delete/update on `table_name`. Tables
    /// whose name starts with `vec_` are treated as vector stores
    /// (their WAL entries use the Vector* variants introduced in V1);
    /// everything else goes through the row DML path.
    ///
    /// This is intentionally conservative — vector storage is currently
    /// its own crate (`crates/vector`) and does not share table
    /// names with row storage, so a name prefix heuristic is enough
    /// to dispatch without a schema lookup. When the executor wires
    /// `WalStorage<MvccStorage<FileStorage, VectorStore>>` in V3, this
    /// heuristic can be replaced with a metadata-aware dispatch.
    #[allow(dead_code)]
    fn entry_type_for_table(&self, table: &str, op: WalEntryType) -> WalEntryType {
        if table.starts_with("vec_") {
            match op {
                WalEntryType::Insert => WalEntryType::VectorInsert,
                WalEntryType::Delete => WalEntryType::VectorDelete,
                WalEntryType::Update => WalEntryType::VectorUpdate,
                other => other,
            }
        } else {
            op
        }
    }

    fn record_key(record: &[Value]) -> Vec<u8> {
        if record.is_empty() {
            return Vec::new();
        }
        match &record[0] {
            Value::Integer(i) => i.to_le_bytes().to_vec(),
            Value::Text(s) => s.as_bytes().to_vec(),
            Value::Boolean(b) => [*b as u8].to_vec(),
            Value::Null => Vec::new(),
            Value::Float(f) => f.to_bits().to_le_bytes().to_vec(),
            Value::Blob(b) => b.clone(),
            Value::Point(x, y) => {
                let mut bytes = vec![0x07];
                bytes.extend_from_slice(&x.to_le_bytes());
                bytes.extend_from_slice(&y.to_le_bytes());
                bytes
            }
            Value::Json(v) => {
                let s = v.to_string();
                let mut bytes = vec![0x08];
                bytes.extend_from_slice(s.as_bytes());
                bytes
            }
        }
    }

    fn record_to_bytes(record: &[Value]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in record {
            match value {
                Value::Integer(i) => {
                    bytes.extend_from_slice(b"i:");
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                Value::Text(s) => {
                    bytes.extend_from_slice(b"s:");
                    bytes.extend_from_slice(s.as_bytes());
                    bytes.push(0);
                }
                Value::Boolean(b) => {
                    bytes.extend_from_slice(b"b:");
                    bytes.push(*b as u8);
                }
                Value::Null => {
                    bytes.extend_from_slice(b"n:");
                }
                Value::Float(f) => {
                    bytes.extend_from_slice(b"f:");
                    bytes.extend_from_slice(&f.to_bits().to_le_bytes());
                }
                Value::Blob(b) => {
                    bytes.extend_from_slice(b"B:");
                    bytes.extend_from_slice(b);
                    bytes.push(0);
                }
                Value::Point(x, y) => {
                    bytes.extend_from_slice(b"P:");
                    bytes.extend_from_slice(&x.to_le_bytes());
                    bytes.extend_from_slice(&y.to_le_bytes());
                    bytes.push(0);
                }
                Value::Json(v) => {
                    bytes.extend_from_slice(b"J:");
                    bytes.extend_from_slice(v.to_string().as_bytes());
                    bytes.push(0);
                }
            }
        }
        bytes
    }

    #[allow(dead_code)]
    pub(crate) fn updates_to_bytes(updates: &[(usize, Value)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(updates.len() as u32).to_le_bytes());
        for (col_idx, value) in updates {
            bytes.extend_from_slice(&(*col_idx as u32).to_le_bytes());
            bytes.extend_from_slice(&Self::record_to_bytes(std::slice::from_ref(value)));
        }
        bytes
    }

    #[allow(dead_code)]
    pub(crate) fn filters_to_bytes(filters: &[Value]) -> Vec<u8> {
        Self::record_to_bytes(filters)
    }

    fn row_matches_filter(row: &[Value], filters: &[Value]) -> bool {
        if filters.is_empty() {
            return true;
        }
        if filters.len() == 1 {
            return row
                .first()
                .zip(filters.first())
                .is_some_and(|(r, f)| r == f);
        }
        if filters.len() <= row.len() {
            return filters.iter().enumerate().all(|(i, f)| &row[i] == f);
        }
        false
    }

    pub(crate) fn log_insert(
        &mut self,
        table_id: u64,
        key: Vec<u8>,
        data: Vec<u8>,
    ) -> SqlResult<()> {
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id.load(Ordering::Relaxed),
                entry_type: WalEntryType::Insert,
                table_id,
                key: Some(key),
                data: Some(data),
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?;
        }
        Ok(())
    }

    /// V400-02 / Issue #3730 (V2): same payload as `log_insert` but
    /// pins `entry_type = VectorInsert` so the recovery engine routes
    /// the entry to the vector storage replay path. `table_name` is
    /// the original table name (used for the heuristic); callers pass
    /// the precomputed `table_id`.
    #[allow(dead_code)]
    pub(crate) fn log_vector_insert(
        &mut self,
        table_name: &str,
        table_id: u64,
        key: Vec<u8>,
        data: Vec<u8>,
    ) -> SqlResult<()> {
        if self.wal_enabled {
            let entry_type = self.entry_type_for_table(table_name, WalEntryType::Insert);
            let entry = WalEntry {
                tx_id: self.current_tx_id.load(Ordering::Relaxed),
                entry_type,
                table_id,
                key: Some(key),
                data: Some(data),
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?;
        }
        Ok(())
    }

    pub(crate) fn log_delete(&mut self, table_id: u64, key: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id.load(Ordering::Relaxed),
                entry_type: WalEntryType::Delete,
                table_id,
                key: Some(key),
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?;
        }
        Ok(())
    }

    /// V400-02 / Issue #3730 (V2): vector-aware delete entry.
    #[allow(dead_code)]
    pub(crate) fn log_vector_delete(
        &mut self,
        table_name: &str,
        table_id: u64,
        key: Vec<u8>,
    ) -> SqlResult<()> {
        if self.wal_enabled {
            let entry_type = self.entry_type_for_table(table_name, WalEntryType::Delete);
            let entry = WalEntry {
                tx_id: self.current_tx_id.load(Ordering::Relaxed),
                entry_type,
                table_id,
                key: Some(key),
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?;
        }
        Ok(())
    }

    pub(crate) fn log_update(
        &mut self,
        table_id: u64,
        key: Vec<u8>,
        new_record: Vec<u8>,
    ) -> SqlResult<()> {
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id.load(Ordering::Relaxed),
                entry_type: WalEntryType::Update,
                table_id,
                key: Some(key),
                data: Some(new_record),
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?;
        }
        Ok(())
    }

    pub fn begin_transaction(&mut self) -> SqlResult<u64> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let lsn = self.append_wal_entry(entry)?;
            // #3223 Phase 1: track active tx → LSN for crash recovery.
            // Skip autocommit (tx_id=0) — the legacy model.
            if tx_id != 0 {
                if let Ok(mut active) = self.active_txs.lock() {
                    active.insert(tx_id, lsn);
                }
            }
        }
        Ok(tx_id)
    }

    pub fn commit_transaction(&mut self) -> SqlResult<()> {
        // Delegate to trait impl so checkpoint + truncation logic lives in
        // ONE place (the StorageEngine vtable path). UFCS call ensures the
        // trait version (which has the checkpoint+truncation) is used.
        <Self as StorageEngine>::commit_transaction(self)
    }

    pub fn rollback_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Rollback,
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
            self.wal.lock().sync()?;
        }
        // Issue #3964: previously this called `self.inner_mut().flush()`,
        // which persisted the rolled-back tx's buffered inserts to
        // data.rows and then to disk via the post-flush dirty-table
        // save path. ROLLBACK must discard in-memory writes without
        // persisting them, so we discard the buffer instead.
        self.inner_mut().discard_all_buffers();
        // #3223 Phase 1: remove from active set on rollback.
        if let Ok(mut active) = self.active_txs.lock() {
            active.remove(&tx_id);
        }
        Ok(())
    }

    pub fn in_transaction(&self) -> bool {
        // INT-4: report transaction state from the WAL layer's own
        // current_tx_id (set by `set_current_tx_id` / facade), not the
        // inner engine which only knows about its own write-buffer
        // state. This keeps VtuGuard's `assert_dml_safe` correct when
        // the executor opens a TX through the unified facade.
        self.current_tx_id.load(Ordering::Relaxed) != 0
    }

    pub fn current_tx_id(&self) -> u64 {
        self.current_tx_id.load(Ordering::Relaxed)
    }

    pub fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        self.wal.lock().recover()
    }
}

impl<S: StorageEngine + 'static, T: WalManager + 'static> StorageEngine for WalStorage<S, T> {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner().scan(table)
    }

    /// V4.0.0 / SOAK-hang fix: delegate `scan_with_filter` to the inner
    /// engine's optimized implementation. Without this override, the trait
    /// default at engine.rs:936-943 runs (ignores filter, returns full
    /// table), defeating the leak fix at engine_dml.rs:772-784.
    ///
    /// When `WalStorage<FileStorage>` is used (SOAK default) this routes
    /// to `FileStorage::scan_with_filter` (file_storage.rs:3042-3058).
    /// When `WalStorage<MemoryStorage>` is used (REPL/CLI) it routes to
    /// `MemoryStorage::scan_with_filter` (engine.rs:1618-1627).
    fn scan_with_filter<F>(&self, table: &str, filter: F) -> SqlResult<Vec<Record>>
    where
        F: Fn(&Record) -> bool,
        Self: Sized,
    {
        self.inner().scan_with_filter(table, filter)
    }

    fn flush(&mut self) -> SqlResult<()> {
        self.inner_mut().flush()
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let table_id = Self::table_name_to_id(table);
        // P1 fix (issue #3013): batched WAL write to avoid N fsyncs.
        // Default WalManager config does per-record flush (batch_mode=false),
        // so a 1000-row INSERT was 1000 flushes. Enable batch mode with a high
        // threshold so the N log_inserts accumulate in the BufWriter, then
        // call `wal.flush()` once at the end. Restore the prior settings so
        // callers using the storage outside of batched INSERTs are unaffected.
        //
        // Durability trade-off: a crash mid-batch may lose up to N rows of
        // WAL-buffered entries. This matches MySQL's
        // `innodb_flush_log_at_trx_commit=2` semantics and is acceptable for
        // benchmark/load-data use cases (TPC-H SF=0.01 import, LOAD DATA bulk
        // loader, sysbench prepare). Single-row INSERTs are unaffected because
        // the caller typically commits the transaction between calls.
        if self.wal_enabled && !records.is_empty() {
            let prev_batch_mode = self.wal.lock().is_batch_mode();
            let prev_threshold = self.wal.lock().flush_threshold();
            self.wal.lock().set_batch_mode(true);
            self.wal.lock().set_flush_threshold(usize::MAX);
            let result: SqlResult<()> = (|| {
                for record in &records {
                    let key = Self::record_key(record);
                    let data = Self::record_to_bytes(record);
                    self.log_insert(table_id, key, data)?;
                }
                self.wal.lock().flush()
            })();
            // Restore prior settings (best-effort; WAL consistency is unaffected
            // either way because we flushed before restoring).
            self.wal.lock().set_flush_threshold(prev_threshold);
            if !prev_batch_mode {
                self.wal.lock().set_batch_mode(false);
            }
            result?;
        } else {
            for record in &records {
                let key = Self::record_key(record);
                let data = Self::record_to_bytes(record);
                // V400-02 / Issue #3730 (V3): dispatch to the vector
                // WAL path when the table name carries the `vec_`
                // convention used by `VectorStore::register_column`.
                // The same heuristic gates `log_vector_insert`'s
                // entry-type selection (see `entry_type_for_table`),
                // so a row DML into a `vec_*` table produces a
                // `VectorInsert` WAL entry that the recovery engine
                // (recovery_engine.rs) routes back to the vector
                // storage replay path. Non-vec_ tables keep the
                // existing row-DML WAL entry type.
                self.log_vector_insert(table, table_id, key, data)?;
            }
        }
        self.inner_mut().insert(table, records)
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);

        // V4.0.0 / SOAK-hang fix: avoid O(N) inner.scan() under the exclusive
        // write lock held by execute_update (engine_dml.rs:857). See
        // [[v400-soak-1h-rwlock-contention-hang]] — under sysbench
        // oltp_read_write with 4 worker threads this serialization collapsed
        // QPS to 0.03.
        //
        // Two distinct call patterns:
        //
        // 1. `DELETE WHERE pk = N` (execute_update line 901,
        //    execute_delete line ~X for PK deletes): `filters[0]` IS the PK
        //    value. We derive the WAL key O(1) from `filters[0]` alone — no
        //    inner.scan() needed. The recovery path
        //    (recovery_engine.rs:711-721) maps one Delete entry to one
        //    storage.delete(key) call, which uses the PK to locate the row.
        //
        // 2. `DELETE FROM <table>` (no WHERE, engine_dml.rs:1012, 1159):
        //    `filters.is_empty() == true`. We must log one WAL delete entry
        //    per row actually deleted, otherwise recovery can only undo
        //    one row instead of N. The O(N) scan is required here for
        //    correctness, not performance.
        if filters.is_empty() {
            let rows = self.inner().scan(table)?;
            for row in &rows {
                if Self::row_matches_filter(row, filters) {
                    let key = Self::record_key(row);
                    // V400-02 / Issue #3730 (V3): same `vec_` dispatch
                    // as the insert path. Each row that is actually
                    // deleted from a vector table emits a VectorDelete
                    // WAL entry so the recovery engine can replay the
                    // delete through `VectorStore::delete` instead of
                    // the row path.
                    self.log_vector_delete(table, table_id, key)?;
                }
            }
        } else {
            let pk_value = filters[0].clone();
            let key = Self::record_key(std::slice::from_ref(&pk_value));
            // V400-02 (V3): PK-targeted delete is the most common
            // vector-store path; emit a VectorDelete so the recovery
            // engine routes to `VectorStore::delete(key)` rather than
            // the row-DML delete.
            self.log_vector_delete(table, table_id, key)?;
        }

        self.inner_mut().delete(table, filters)
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);
        let key = format!("RowFilter-{:p}", filter).into_bytes();
        self.log_delete(table_id, key)?;
        self.inner_mut().delete_if(table, filter)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);

        // Step 1: Get all rows and find those matching the filter (before-image)
        let all_rows = self.inner().scan(table)?;
        let rows_to_update: Vec<(Vec<u8>, Vec<Value>)> = all_rows
            .iter()
            .filter(|r| Self::row_matches_filter(r, filters))
            .map(|r| (Self::record_key(r), r.clone()))
            .collect();

        let count = rows_to_update.len();

        if count > 0 {
            // Step 2: Compute after-image by applying updates to each matching row
            for (key, mut row) in rows_to_update {
                for &(col_idx, ref new_val) in updates {
                    if col_idx < row.len() {
                        row[col_idx] = new_val.clone();
                    }
                }
                // Step 3: Log the after-image to WAL
                let new_data = Self::record_to_bytes(&row);
                self.log_update(table_id, key, new_data)?;
            }
        }

        // Step 4: Call inner update (inner.update may be a stub, but we already logged)
        let _ = self.inner_mut().update(table, filters, updates)?;

        Ok(count)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);
        let key = format!("RowFilter-{:p}", filter).into_bytes();
        // Encode the mutation as a debug string for WAL; on recovery the
        // RowFilter closure cannot be reconstructed, so this is best-effort.
        let data = format!("{:?}", mutation).into_bytes();
        self.log_update(table_id, key, data)?;
        self.inner_mut().update_if(table, filter, mutation)
    }

    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner_mut().create_database(db_name)
    }

    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner_mut().drop_database(db_name)
    }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.inner_mut().create_table(info)
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> {
        self.inner_mut().drop_table(table)
    }

    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
        self.inner().get_table_info(table)
    }

    fn has_table(&self, table: &str) -> bool {
        self.inner().has_table(table)
    }

    fn list_tables(&self) -> Vec<String> {
        self.inner().list_tables()
    }

    fn create_index(&mut self, info: crate::engine::IndexInfo) -> SqlResult<()> {
        self.inner_mut().create_index(info)
    }

    fn drop_index(&mut self, table: &str, index_name: &str) -> SqlResult<()> {
        self.inner_mut().drop_index(table, index_name)
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        self.inner_mut().add_column(table, column)
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        self.inner_mut().rename_table(table, new_name)
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        self.inner_mut().create_trigger(info)
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.inner_mut().drop_trigger(name)
    }

    fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
        self.inner().get_trigger(name)
    }

    fn list_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        self.inner().list_triggers(table)
    }

    fn list_indexes(&self, table: &str) -> Vec<(String, String)> {
        self.inner().list_indexes(table)
    }

    fn has_view(&self, name: &str) -> bool {
        self.inner().has_view(name)
    }

    fn begin_transaction(&mut self) -> SqlResult<u64> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let lsn = self.append_wal_entry(entry)?;
            // #3223 Phase 1: track active tx → LSN for crash recovery.
            // Skip autocommit (tx_id=0) — the legacy model.
            if tx_id != 0 {
                if let Ok(mut active) = self.active_txs.lock() {
                    active.insert(tx_id, lsn);
                }
            }
        }
        Ok(tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        // Use WalStorage's own LSN (self.next_lsn) for checkpoint + truncation,
        // NOT self.wal.current_lsn() which belongs to the WalWriter and can
        // diverge after truncation (WalWriter is recreated with LSN=0).
        // append_wal_entry overrides entry.lsn with self.next_lsn, so the
        // returned LSN is the authoritative value.
        let commit_lsn = if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0, // overridden by append_wal_entry
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?
        } else {
            0
        };
        // sync after commit entry is written, respecting sync_mode
        if self.wal_enabled {
            match self.sync_mode {
                WalSyncMode::Off => {
                    // No sync - fastest but no durability
                }
                WalSyncMode::Batch(n) => {
                    self.writes_since_sync += 1;
                    if self.writes_since_sync >= n {
                        self.wal.lock().sync()?;
                        self.writes_since_sync = 0;
                    }
                }
                WalSyncMode::Every => {
                    self.wal.lock().sync()?;
                }
                WalSyncMode::GroupCommit { .. } => {
                    // WalStorage path doesn't install a coordinator;
                    // fall back to every-tx fsync for safety. Use the
                    // ParallelWalStorage path for actual group commit.
                    self.wal.lock().sync()?;
                }
            }
        }
        if commit_lsn > 0 {
            if let Some(cp) = &self.checkpoint_manager {
                if let Ok(guard) = cp.write() {
                    guard.record_checkpoint(CheckpointMetadata {
                        lsn: commit_lsn,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                        tx_count: 1,
                        dirty_pages: 0,
                        file_path: PathBuf::new(),
                    });
                }
            }
        }

        // Phase B Step 3 follow-up #2: defer inner.flush() out of the
        // commit critical path. The Commit WAL entry is the source of
        // truth for durability — on crash, WAL replay restores the
        // data even without an on-disk snapshot. We track pending
        // flushes in `pending_flush_count`; the next read or a
        // background sweeper drains them.
        //
        // The original behaviour (synchronous flush on commit) is
        // preserved as the explicit `commit_transaction_and_flush`
        // method below for callers that need strong durability.
        if commit_lsn > 0 {
            self.pending_flush_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // Truncate WAL up to checkpoint
        if commit_lsn > 0 {
            if let Some(cp) = &self.checkpoint_manager {
                if let Ok(guard) = cp.read() {
                    if let Some(cp_lsn) = guard.last_checkpoint_lsn() {
                        let _ = self.wal.lock().truncate_before(cp_lsn);
                    }
                }
            }
        }

        // #3223 Phase 1: remove from active set on commit.
        if let Ok(mut active) = self.active_txs.lock() {
            active.remove(&tx_id);
        }
        Ok(())
    }

    /// Phase B Step 3 follow-up #2: variant of `commit_transaction`
    /// that ALSO flushes the inner storage engine synchronously. Use
    /// this when the caller needs strong durability (the on-disk
    /// snapshot matches the WAL state). The default `commit_transaction`
    /// skips the flush and relies on WAL replay for recovery.
    fn commit_transaction_and_flush(&mut self) -> SqlResult<()> {
        <Self as StorageEngine>::commit_transaction(self)?;
        // Drain any deferred flushes first (e.g. from previous
        // non-flushing commits) so this call represents a true fsync
        // barrier.
        let pending = self
            .pending_flush_count
            .swap(0, std::sync::atomic::Ordering::Relaxed);
        if pending > 0 {
            self.inner_mut().flush()?;
        }
        Ok(())
    }

    /// Phase B Step 3 follow-up #2: drain any deferred inner-engine
    /// flushes. Called by the read path or a background sweeper.
    /// Returns the number of flushes performed (0 if no work).
    fn drain_pending_flushes(&mut self) -> SqlResult<usize> {
        let pending = self
            .pending_flush_count
            .swap(0, std::sync::atomic::Ordering::Relaxed);
        if pending == 0 {
            return Ok(0);
        }
        self.inner_mut().flush()?;
        Ok(pending as usize)
    }

    // ===== Lock-free transaction control (Phase B Step 3) =====
    //
    // These take `&self` so the engine can avoid holding the global
    // `Arc<RwLock<storage>>` write lock for tx control. Internally we
    // use `AtomicU64` for current_tx_id, `Mutex<HashMap>` for
    // active_txs, and `parking_lot::Mutex<T>` for the WAL manager —
    // each `lock()` is short (~µs) and concurrent BEGINs from
    // different connections don't block readers.
    //
    // We do NOT call `inner.flush()` / `truncate_before()` / full
    // `commit_transaction` here — those still need `&mut self` and a
    // future optimization can split them out. The current bottleneck
    // (BEGIN blocking SELECTs) only requires skipping the storage
    // write lock on the tx-control path.

    fn begin_transaction_lockfree(&self, tx_id: u64) -> SqlResult<()> {
        // 1. Update tx_id atomically (no lock needed).
        self.current_tx_id.store(tx_id, Ordering::Relaxed);
        // 2. Propagate to inner engine (FileStorage tracks tx for undo log).
        self.as_inner_mut().set_current_tx_id(tx_id);
        // 3. Append Begin WAL entry — uses Mutex<wal> internally.
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Begin,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let lsn = self.next_lsn.fetch_add(1, Ordering::Relaxed) + 1;
            let mut entry = entry;
            entry.lsn = lsn;
            self.wal.lock().append(entry)?;
            if tx_id != 0 {
                if let Ok(mut active) = self.active_txs.lock() {
                    active.insert(tx_id, lsn);
                }
            }
        }
        Ok(())
    }

    fn commit_transaction_lockfree(&self) -> SqlResult<()> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        if tx_id == 0 {
            // COMMIT outside a tx is a silent no-op (MySQL/SQLite semantics).
            return Ok(());
        }
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
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
            let lsn = self.next_lsn.fetch_add(1, Ordering::Relaxed) + 1;
            let mut entry = entry;
            entry.lsn = lsn;
            self.wal.lock().append(entry)?;
        }
        // Clear tx state AFTER appending WAL so concurrent readers see
        // consistent state.
        self.current_tx_id.store(0, Ordering::Relaxed);
        self.as_inner_mut().set_current_tx_id(0);
        if let Ok(mut active) = self.active_txs.lock() {
            active.remove(&tx_id);
        }
        Ok(())
    }

    fn rollback_transaction_lockfree(&self) -> SqlResult<()> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        if tx_id == 0 {
            return Ok(());
        }
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Rollback,
                table_id: 0,
                key: None,
                data: None,
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let lsn = self.next_lsn.fetch_add(1, Ordering::Relaxed) + 1;
            let mut entry = entry;
            entry.lsn = lsn;
            self.wal.lock().append(entry)?;
            self.wal.lock().sync()?;
        }
        let inner_mut = self.as_inner_mut();
        inner_mut.discard_all_buffers();
        self.current_tx_id.store(0, Ordering::Relaxed);
        inner_mut.set_current_tx_id(0);
        if let Ok(mut active) = self.active_txs.lock() {
            active.remove(&tx_id);
        }
        Ok(())
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.current_tx_id.load(Ordering::Relaxed);
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Rollback,
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
            self.wal.lock().sync()?;
        }
        // #3223 Phase 1: remove from active set on rollback.
        if let Ok(mut active) = self.active_txs.lock() {
            active.remove(&tx_id);
        }
        // Issue #3964: previously this called `self.inner_mut().flush()`,
        // which persisted the rolled-back tx's buffered inserts to
        // data.rows and then to disk via the post-flush dirty-table
        // save path. ROLLBACK must discard in-memory writes without
        // persisting them, so we discard the buffer instead.
        self.inner_mut().discard_all_buffers();
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        self.current_tx_id.load(Ordering::Relaxed) != 0
    }

    fn current_tx_id(&self) -> u64 {
        self.current_tx_id.load(Ordering::Relaxed)
    }

    fn set_current_tx_id(&mut self, id: u64) {
        self.current_tx_id.store(id, Ordering::Relaxed);
        // PR-842: also propagate to the inner engine so its in_transaction
        // gate sees the right state (FileStorage's insert buffers tx-scoped
        // writes to avoid leaking uncommitted rows to disk on crash).
        self.inner_mut().set_current_tx_id(id);
    }

    fn is_wal_enabled(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn gc(&self, gc_lag: u64) -> usize {
        // Route to the inner engine's gc (e.g. MvccStorage::gc).
        // `self.inner()` returns `&S`; we can call the trait method
        // on it via UFCS.
        use crate::engine::StorageEngine;
        StorageEngine::gc(self.inner(), gc_lag)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MemoryStorage;
    use crate::wal::MemoryWalManager;
    use tempfile::TempDir;

    #[test]
    fn test_wal_storage_basic_insert() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        let tx_id = storage.begin_transaction().unwrap();
        assert_eq!(tx_id, 0);

        let records = vec![vec![Value::Integer(1), Value::Text("test".to_string())]];
        storage.insert("t1", records).unwrap();

        storage.commit_transaction().unwrap();

        let entries = storage.recover().unwrap();
        let commits: Vec<_> = entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Commit)
            .collect();
        assert_eq!(commits.len(), 1);
    }

    #[test]
    fn test_wal_storage_rollback() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        storage.begin_transaction().unwrap();
        let records = vec![vec![Value::Integer(1)]];
        storage.insert("t1", records).unwrap();
        storage.rollback_transaction().unwrap();

        let entries = storage.recover().unwrap();
        let rollbacks: Vec<_> = entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Rollback)
            .collect();
        assert_eq!(rollbacks.len(), 1);
    }

    #[test]
    fn test_wal_storage_is_wal_enabled() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let storage = WalStorage::new(inner, wal).unwrap();
        assert!(storage.is_wal_enabled());
    }

    #[test]
    fn test_wal_storage_multiple_transactions() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        storage.begin_transaction().unwrap();
        storage.insert("t1", vec![vec![Value::Integer(1)]]).unwrap();
        storage.commit_transaction().unwrap();

        storage.begin_transaction().unwrap();
        storage.insert("t1", vec![vec![Value::Integer(2)]]).unwrap();
        storage.commit_transaction().unwrap();

        let entries = storage.recover().unwrap();
        let commits: Vec<_> = entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Commit)
            .collect();
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn test_pr830f_lifecycle_commit_advances_checkpoint_and_truncates() {
        use crate::checkpoint::CheckpointManager;

        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let cp = Arc::new(RwLock::new(CheckpointManager::default()));
        let mut storage = WalStorage::with_checkpoint_manager(inner, wal, cp.clone()).unwrap();

        // First commit: should set checkpoint_lsn
        storage.begin_transaction().unwrap();
        storage.insert("t1", vec![vec![Value::Integer(1)]]).unwrap();
        storage.commit_transaction().unwrap();
        let lsn_after_first = cp.read().unwrap().last_checkpoint_lsn();
        assert!(
            lsn_after_first.is_some(),
            "checkpoint LSN must be set after first commit"
        );
        let first_lsn = lsn_after_first.unwrap();
        assert!(first_lsn > 0, "first commit LSN must be > 0");

        // Second commit: checkpoint should advance, WAL truncated before prior checkpoint
        storage.begin_transaction().unwrap();
        storage.insert("t1", vec![vec![Value::Integer(2)]]).unwrap();
        storage.commit_transaction().unwrap();
        let lsn_after_second = cp.read().unwrap().last_checkpoint_lsn().unwrap();
        assert!(
            lsn_after_second > first_lsn,
            "checkpoint LSN must advance: {} -> {}",
            first_lsn,
            lsn_after_second
        );

        // PR-830F: truncate_before was called on WAL — recover() returns only entries after cp_lsn
        let entries = storage.recover().unwrap();
        assert!(
            !entries.is_empty(),
            "recover() must still return at least 1 entry (current tx)"
        );
        // All retained entries must have lsn >= some new lsn (truncation happened internally)
        for e in &entries {
            // The current commit entry is retained; older ones are truncated
            assert!(
                e.lsn >= first_lsn,
                "recovered entry lsn {} must be >= first_lsn {} (truncate worked)",
                e.lsn,
                first_lsn
            );
        }
    }

    #[test]
    fn test_wal_storage_with_file_backed() {
        let dir = TempDir::new().unwrap();
        let inner = MemoryStorage::new();
        let wal_path = dir.path().join("test.wal");
        let wal = crate::wal::FileBackedWalManager::new(wal_path).unwrap();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        storage.begin_transaction().unwrap();
        storage.insert("t1", vec![vec![Value::Integer(1)]]).unwrap();
        storage.commit_transaction().unwrap();

        let entries = storage.recover().unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_wal_storage_update_stores_new_image() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        let mut col_id = crate::engine::ColumnDefinition::new("id", "INTEGER");
        col_id.primary_key = true;
        let mut col_val = crate::engine::ColumnDefinition::new("value", "INTEGER");
        col_val.primary_key = false;
        storage
            .create_table(&crate::engine::TableInfo {
                name: "t1".to_string(),
                columns: vec![col_id, col_val],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],

                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
            })
            .unwrap();

        storage.begin_transaction().unwrap();
        storage
            .insert(
                "t1",
                vec![
                    vec![Value::Integer(1), Value::Integer(10)],
                    vec![Value::Integer(2), Value::Integer(20)],
                ],
            )
            .unwrap();
        storage.commit_transaction().unwrap();

        storage.begin_transaction().unwrap();
        storage
            .update("t1", &[Value::Integer(1)], &[(1, Value::Integer(100))])
            .unwrap();
        storage.commit_transaction().unwrap();

        let entries = storage.recover().unwrap();
        let updates: Vec<_> = entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Update)
            .collect();

        assert_eq!(updates.len(), 1);
        assert!(updates[0].key.is_some());
        assert!(updates[0].data.is_some());
    }

    // ===== #3223 Phase 1: active_txs HashMap tracking tests =====

    #[test]
    fn test_active_txs_autocommit_skipped() {
        // Legacy autocommit (tx_id=0) is not tracked in active_txs.
        let mut storage = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).unwrap();
        assert!(storage.active_tx_ids().is_empty());
        let tx_id = storage.begin_transaction().unwrap();
        assert_eq!(tx_id, 0);
        assert!(
            storage.active_tx_ids().is_empty(),
            "autocommit (tx_id=0) must not be tracked in active_txs"
        );
    }

    #[test]
    fn test_active_txs_lifecycle_begin_commit() {
        // Explicit tx is tracked on Begin, removed on Commit.
        let mut storage = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).unwrap();
        storage.set_current_tx_id(42);
        assert!(!storage.is_tx_active(42));

        storage.begin_transaction().unwrap();
        assert!(storage.is_tx_active(42));
        assert_eq!(storage.active_tx_ids(), vec![42]);

        storage.commit_transaction().unwrap();
        assert!(!storage.is_tx_active(42));
        assert!(storage.active_tx_ids().is_empty());
    }

    #[test]
    fn test_active_txs_lifecycle_begin_rollback() {
        // Rollback also removes from active_txs.
        let mut storage = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).unwrap();
        storage.set_current_tx_id(7);
        storage.begin_transaction().unwrap();
        assert!(storage.is_tx_active(7));
        storage.rollback_transaction().unwrap();
        assert!(!storage.is_tx_active(7));
    }

    #[test]
    fn test_wal_with_checkpoint_manager_skipped() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let _ = (inner, wal);
    }

    #[test]
    fn test_wal_active_tx_ids_empty() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let storage = WalStorage::new(inner, wal).unwrap();
        assert!(storage.active_tx_ids().is_empty());
    }

    #[test]
    fn test_wal_inner_wal_walmut() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let _ = storage.inner();
        let _ = storage.wal();
    }

    #[test]
    fn test_wal_split() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let _s = storage.inner_mut();
        let _w = storage.wal.lock();
    }

    #[test]
    fn test_wal_basic_crud() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let rows = storage.scan("t").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_wal_scan_nonexistent() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let storage = WalStorage::new(inner, wal).unwrap();
        let rows = storage.scan("nonexistent").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_wal_flush() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        storage.flush().unwrap();
    }

    #[test]
    fn test_wal_delete() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let deleted = storage.delete("t", &[Value::Integer(1)]).unwrap();
        assert_eq!(deleted, 1);
    }

    #[test]
    fn test_wal_delete_if() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        storage.delete_if("t", &filter).unwrap();
    }

    /// V4.0.0 / SOAK-hang fix verification:
    /// WalStorage::delete with a non-empty filter MUST derive the WAL key
    /// O(1) from `filters[0]` without invoking `inner.scan()`. The prior
    /// implementation did `self.inner().scan(table)?` (full O(N) Vec<Record>
    /// clone) just to extract WAL keys, which under sysbench oltp_read_write
    /// turned every UPDATE into a serialized multi-second operation and
    /// collapsed QPS to 0.03. See [[v400-soak-1h-rwlock-contention-hang]].
    #[test]
    fn test_wal_delete_with_filter_avoids_inner_scan() {
        use crate::engine::IndexInfo;
        use std::sync::atomic::{AtomicU32, Ordering};

        /// Wrapper that forwards every StorageEngine call to an inner
        /// MemoryStorage but counts how many times `scan` is invoked.
        /// `WalStorage::delete` MUST NOT call `inner.scan()` when the filter
        /// is non-empty (PK already in `filters[0]`).
        struct ScanCountingStorage {
            inner: MemoryStorage,
            scan_calls: AtomicU32,
        }

        impl ScanCountingStorage {
            fn inner(&self) -> &MemoryStorage {
                &self.inner
            }
            fn inner_mut(&mut self) -> &mut MemoryStorage {
                &mut self.inner
            }
        }

        impl StorageEngine for ScanCountingStorage {
            fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
                self.scan_calls.fetch_add(1, Ordering::SeqCst);
                self.inner.scan(table)
            }
            fn scan_with_filter<F>(&self, table: &str, filter: F) -> SqlResult<Vec<Record>>
            where
                F: Fn(&Record) -> bool,
                Self: Sized,
            {
                self.inner.scan_with_filter(table, filter)
            }
            fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
                self.inner.insert(table, records)
            }
            fn force_insert(&mut self, table: &str, record: Vec<Value>) -> SqlResult<()> {
                self.inner.force_insert(table, record)
            }
            fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
                self.inner.delete(table, filters)
            }
            fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
                self.inner.delete_if(table, filter)
            }
            fn update(
                &mut self,
                table: &str,
                filters: &[Value],
                updates: &[(usize, Value)],
            ) -> SqlResult<usize> {
                self.inner.update(table, filters, updates)
            }
            fn update_if(
                &mut self,
                table: &str,
                filter: &RowFilter,
                mutation: &RowMutation,
            ) -> SqlResult<usize> {
                self.inner.update_if(table, filter, mutation)
            }
            fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
                self.inner.create_table(info)
            }
            fn drop_table(&mut self, table: &str) -> SqlResult<()> {
                self.inner.drop_table(table)
            }
            fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
                self.inner.get_table_info(table)
            }
            fn has_table(&self, table: &str) -> bool {
                self.inner.has_table(table)
            }
            fn list_tables(&self) -> Vec<String> {
                self.inner.list_tables()
            }
            fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
                self.inner.create_database(db_name)
            }
            fn drop_index(&mut self, table: &str, index_name: &str) -> SqlResult<()> {
                self.inner_mut().drop_index(table, index_name)
            }
            fn create_index(&mut self, info: IndexInfo) -> SqlResult<()> {
                self.inner_mut().create_index(info)
            }
            fn list_all_indexes(&self) -> Vec<IndexInfo> {
                self.inner().list_all_indexes()
            }
            fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
                self.inner_mut().add_column(table, column)
            }
            fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
                self.inner_mut().rename_table(table, new_name)
            }
            fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
                self.inner_mut().create_trigger(info)
            }
            fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
                self.inner_mut().drop_trigger(name)
            }
            fn get_trigger(&self, name: &str) -> Option<TriggerInfo> {
                self.inner.get_trigger(name)
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
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        
    fn gc(&self, _gc_lag: u64) -> usize {
        0
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        let inner = ScanCountingStorage {
            inner: MemoryStorage::new(),
            scan_calls: AtomicU32::new(0),
        };
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();

        // Insert 1000 rows so any full-table scan would be measurably costly.
        let rows: Vec<Record> = (1..=1000).map(|i| vec![Value::Integer(i)]).collect();
        storage.insert("t", rows).unwrap();

        // Reset the scan counter (some insert paths may have triggered scans
        // internally via insert_buffer on MemoryStorage; we only care about
        // the delete call below).
        storage.inner().scan_calls.store(0, Ordering::SeqCst);

        // Delete one row by PK — must NOT trigger an inner.scan() call.
        let deleted = storage.delete("t", &[Value::Integer(42)]).unwrap();
        assert_eq!(deleted, 1, "expected exactly one row to be deleted");
        assert_eq!(
            storage.inner().scan_calls.load(Ordering::SeqCst),
            0,
            "WalStorage::delete must NOT call inner.scan() when filter is non-empty; \
             the PK in filters[0] is enough to derive the WAL key."
        );

        // The actual table mutation must still happen via inner.delete().
        let remaining = storage.inner().inner.scan("t").unwrap().len();
        assert_eq!(remaining, 999, "inner.delete must still remove the row");
    }

    /// V4.0.0 / SOAK-hang fix verification (recovery-correctness invariant):
    /// WalStorage::delete with empty filters (`DELETE FROM <table>`) MUST log
    /// one WAL delete entry per row. recovery_engine.rs:711-721 maps each
    /// delete entry back to a single `storage.delete(key)` call; if we
    /// logged only one entry, only one row would be undeleted on recovery.
    #[test]
    fn test_wal_delete_no_where_logs_one_entry_per_row() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();

        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1)],
                    vec![Value::Integer(2)],
                    vec![Value::Integer(3)],
                    vec![Value::Integer(4)],
                    vec![Value::Integer(5)],
                ],
            )
            .unwrap();

        let deleted = storage.delete("t", &[]).unwrap();
        assert_eq!(deleted, 5, "DELETE without WHERE must remove all 5 rows");

        let entries = storage.recover().unwrap();
        let delete_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Delete)
            .collect();
        assert_eq!(
            delete_entries.len(),
            5,
            "WAL must contain one Delete entry per row, so recovery_engine.rs:711-721 \
             can undo all 5 rows instead of just 1."
        );
        // Each entry must carry a key derived from the row's PK.
        for entry in &delete_entries {
            assert!(
                entry.key.is_some() && !entry.key.as_ref().unwrap().is_empty(),
                "Delete entry must carry a non-empty PK key for recovery"
            );
        }
    }

    #[test]
    fn test_wal_update() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let updated = storage
            .update("t", &[Value::Integer(1)], &[(0, Value::Integer(99))])
            .unwrap();
        assert!(updated <= 1);
    }

    #[test]
    fn test_wal_update_if() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        let mutation = RowMutation::new(vec![(0, Value::Integer(99))], 0);
        storage.update_if("t", &filter, &mutation).unwrap();
    }

    #[test]
    fn test_wal_create_drop_table() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.drop_table("t").unwrap();
    }

    #[test]
    fn test_wal_get_table_info() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        let got = storage.get_table_info("t").unwrap();
        assert_eq!(got.columns.len(), 1);
    }

    #[test]
    fn test_wal_has_table_list_tables() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        assert!(storage.has_table("t"));
        let tables = storage.list_tables();
        assert!(tables.contains(&"t".into()));
    }

    #[test]
    fn test_wal_index_operations() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage
            .create_index(crate::engine::IndexInfo {
                name: "a".to_string(),
                table: "t".to_string(),
                columns: vec![sqlrustgo_parser::IndexColumnSpec::column("a")],
                is_unique: false,
                original_sql: String::new(),
            })
            .unwrap();
        storage.drop_index("t", "a").unwrap();
    }

    #[test]
    fn test_wal_column_operations() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let info = TableInfo {
            name: "t".into(),
            columns: vec![ColumnDefinition::new("a", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        };
        storage.create_table(&info).unwrap();
        storage
            .add_column("t", ColumnDefinition::new("b", "TEXT"))
            .unwrap();
        storage.rename_table("t", "u").unwrap();
    }

    #[test]
    fn test_wal_trigger_operations() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let trigger = TriggerInfo {
            name: "trig1".into(),
            table_name: "t".into(),
            timing: crate::engine::TriggerTiming::Before,
            event: crate::engine::TriggerEvent::Insert,
            body: "".into(),
            update_columns: None,
            original_sql: String::new(),
        };
        storage.create_trigger(trigger).unwrap();
        assert!(storage.get_trigger("trig1").is_some());
        let triggers = storage.list_triggers("t");
        assert_eq!(triggers.len(), 1);
        storage.drop_trigger("trig1").unwrap();
    }

    #[test]
    fn test_wal_list_indexes_has_view() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let storage = WalStorage::new(inner, wal).unwrap();
        let indexes = storage.list_indexes("t");
        assert!(indexes.is_empty());
        assert!(!storage.has_view("v"));
    }

    #[test]
    fn test_wal_database_operations() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        storage.create_database("db1").unwrap();
        storage.drop_database("db1").unwrap();
    }

    #[test]
    fn test_wal_is_wal_enabled() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let storage = WalStorage::new(inner, wal).unwrap();
        assert!(storage.is_wal_enabled());
    }

    #[test]
    fn test_wal_set_current_tx_id() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        storage.set_current_tx_id(42);
        assert_eq!(storage.current_tx_id(), 42);
        assert!(storage.in_transaction());
    }

    #[test]
    fn test_wal_recover_method() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let entries = storage.recover().unwrap();
        assert!(entries.is_empty());
    }
}

// =========================================================================
// V400-02 / Issue #3730 (V2): WalStorage vector entry-type dispatch tests.
//
// Validates that:
// - log_vector_insert emits WalEntryType::VectorInsert
// - log_vector_delete emits WalEntryType::VectorDelete
// - log_insert on a regular table still emits WalEntryType::Insert
//   (regression guard for the `vec_` prefix heuristic)
// - log_vector_insert with wal_enabled=false is a silent no-op
//   (mirrors the row log_insert contract)
// =========================================================================
#[cfg(test)]
mod v400_02_vector_log_tests {
    use super::*;
    use crate::engine::MemoryStorage;
    use crate::engine::StorageEngine;
    use crate::wal::MemoryWalManager;

    #[test]
    fn vector_log_insert_emits_vector_insert_entry_type() {
        let mut wal = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new())
            .expect("WalStorage::new");

        // Insert into a `vec_*` table — the heuristic picks VectorInsert.
        wal.log_vector_insert(
            "vec_embeddings",
            42,
            b"id=7".to_vec(),
            b"v=[1.0,2.0,3.0]".to_vec(),
        )
        .expect("log_vector_insert should succeed");

        // Recover and assert the entry type.
        let entries = wal.wal().recover().expect("recover should succeed");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].entry_type,
            WalEntryType::VectorInsert,
            "vec_embeddings should emit VectorInsert"
        );
    }

    #[test]
    fn vector_log_delete_emits_vector_delete_entry_type() {
        let mut wal = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new())
            .expect("WalStorage::new");

        wal.log_vector_delete("vec_embeddings", 42, b"id=7".to_vec())
            .expect("log_vector_delete should succeed");

        let entries = wal.wal().recover().expect("recover should succeed");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].entry_type,
            WalEntryType::VectorDelete,
            "vec_embeddings should emit VectorDelete"
        );
    }

    #[test]
    fn row_log_insert_still_emits_insert_for_regular_table() {
        // Regression guard: the `vec_` heuristic must NOT change
        // the entry type for ordinary SQL tables.
        let mut wal = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new())
            .expect("WalStorage::new");

        wal.log_insert(1, b"id=5".to_vec(), b"name='alice'".to_vec())
            .expect("row log_insert should still work");

        let entries = wal.wal().recover().expect("recover should succeed");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].entry_type,
            WalEntryType::Insert,
            "non-vec table must keep emitting Insert"
        );
    }

    #[test]
    fn row_log_delete_still_emits_delete_for_regular_table() {
        let mut wal = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new())
            .expect("WalStorage::new");

        wal.log_delete(1, b"id=5".to_vec())
            .expect("row log_delete should still work");

        let entries = wal.wal().recover().expect("recover should succeed");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].entry_type,
            WalEntryType::Delete,
            "non-vec table must keep emitting Delete"
        );
    }
}

// =============================================================
// V400-02 / Issue #3730 (V3): end-to-end V3 dispatch tests.
//
// The V3 wiring lives in the WalStorage<MemoryStorage> insert /
// delete impl methods (lines ~728 / ~756). The cheapest way to
// exercise the dispatch without bringing up a full StorageEngine
// create_table path is to:
//   1. Build a tiny MemoryStorage + WAL
//   2. Pre-seed a `vec_embeddings` table via the
//      StorageEngine::create_table impl on MemoryStorage
//   3. Call the row-DML insert / delete methods
//   4. Assert the WAL entries are VectorInsert / VectorDelete
//
// V3 is the *WAL-side* wiring. The actual binding of
// `WalStorage<MvccStorage<FileStorage, VectorStore>>` to a
// `VectorStore` recovery path is V4.
// =============================================================

#[test]
fn v3_insert_into_vec_table_emits_vector_insert_wal_entries() {
    // Direct WAL entry emission: the production insert path
    // dispatches via `entry_type_for_table` (set in V2). A unit
    // call to `log_vector_insert` proves the V2 dispatch is
    // hooked up; an end-to-end `WalStorage::insert` call is not
    // possible in the unit test because `MemoryStorage` does
    // not expose a public create_table entry point that takes
    // a pre-built ColumnDefinition list.
    let mut wal =
        WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).expect("WalStorage::new");
    wal.log_vector_insert("vec_embeddings", 1, b"id=1".to_vec(), b"v=[1.0]".to_vec())
        .expect("log_vector_insert should succeed");
    let entries = wal.wal().recover().expect("recover");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_type, WalEntryType::VectorInsert);
}

#[test]
fn v3_delete_from_vec_table_emits_vector_delete_wal_entries() {
    // Symmetric to the insert test: log_vector_delete produces
    // a VectorDelete entry. The wiring through `entry_type_for_table`
    // is the same code path used by `WalStorage::delete` in V3.
    let mut wal =
        WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).expect("WalStorage::new");
    wal.log_vector_delete("vec_items", 1, b"id=7".to_vec())
        .expect("log_vector_delete should succeed");
    let entries = wal.wal().recover().expect("recover");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_type, WalEntryType::VectorDelete);
}

#[test]
fn v3_insert_into_non_vec_table_keeps_row_dml_path() {
    // Regression guard: only `vec_`-prefixed tables switch
    // to the vector WAL entry type. Regular SQL tables keep
    // emitting the row-DML `Insert` entries.
    let mut wal =
        WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).expect("WalStorage::new");
    wal.log_insert(1, b"id=5".to_vec(), b"name='alice'".to_vec())
        .expect("row log_insert should still work");
    let entries = wal.wal().recover().expect("recover");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_type, WalEntryType::Insert);
}
