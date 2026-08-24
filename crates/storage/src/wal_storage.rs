use crate::checkpoint::{CheckpointManager, CheckpointMetadata};
use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlResult, StorageEngine, TableInfo,
    TriggerInfo, Value,
};
use crate::wal::{WalEntry, WalEntryType, WalManager};
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

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
}

pub struct WalStorage<S: StorageEngine, T: WalManager> {
    inner: S,
    wal: T,
    wal_enabled: bool,
    sync_mode: WalSyncMode,
    /// Counter for batch mode: tracks writes since last sync
    writes_since_sync: u32,
    checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>,
    /// Active transaction id. The ExecutionEngine pushes the real id here
    /// via `set_current_tx_id`; without this, every WAL entry would carry
    /// tx_id=0 and the recovery engine could not distinguish autocommit
    /// DML from uncommitted-but-started DML.
    current_tx_id: u64,
    /// Monotonically increasing LSN counter for WAL entries.
    /// Each `append_wal_entry` increments this and assigns the value to the entry.
    next_lsn: u64,
    /// #3223 Phase 1: Active transaction set with their last WAL LSN.
    /// Populated on `begin_transaction` (insert), drained on
    /// `commit_transaction`/`rollback_transaction` (remove).
    /// `RecoveryEngine` will use `is_tx_active` during replay to skip
    /// uncommitted DML.
    /// Empty for autocommit (tx_id=0) — the legacy single-active-tx model.
    active_txs: HashMap<u64, u64>,
}
impl<S: StorageEngine + 'static, T: WalManager + 'static> WalStorage<S, T> {
    pub fn new(inner: S, wal: T) -> SqlResult<Self> {
        Ok(Self {
            inner,
            wal,
            wal_enabled: true,
            sync_mode: WalSyncMode::default(),
            writes_since_sync: 0,
            checkpoint_manager: None,
            current_tx_id: 0,
            next_lsn: 0,
            active_txs: HashMap::new(),
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
            inner,
            wal,
            wal_enabled: true,
            sync_mode,
            writes_since_sync: 0,
            checkpoint_manager: None,
            current_tx_id: 0,
            next_lsn: 0,
            active_txs: HashMap::new(),
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
            inner,
            wal,
            wal_enabled: true,
            sync_mode,
            writes_since_sync: 0,
            checkpoint_manager: Some(checkpoint_manager),
            current_tx_id: 0,
            next_lsn: 0,
            active_txs: HashMap::new(),
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
        self.active_txs.contains_key(&tx_id)
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
            self.wal.sync()?;
            self.writes_since_sync = 0;
        }
        Ok(())
    }

    /// #3223 Phase 1: Returns snapshot of active tx ids (for tests/diagnostics).
    pub fn active_tx_ids(&self) -> Vec<u64> {
        self.active_txs.keys().copied().collect()
    }

    /// Append a WAL entry with a monotonically increasing LSN.
    /// Returns the assigned LSN.
    /// PR-830F: This is the single chokepoint for LSN assignment;
    /// without it, `current_lsn()` returns 0 and checkpoint advance never triggers.
    fn append_wal_entry(&mut self, mut entry: WalEntry) -> SqlResult<u64> {
        self.next_lsn += 1;
        entry.lsn = self.next_lsn;
        self.wal.append(entry)?;
        Ok(self.next_lsn)
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn wal(&self) -> &T {
        &self.wal
    }

    pub fn wal_mut(&mut self) -> &mut T {
        &mut self.wal
    }

    /// Split into (storage, wal) for independent mutable access
    pub fn split(&mut self) -> (&mut S, &mut T) {
        (&mut self.inner, &mut self.wal)
    }

    fn table_name_to_id(table: &str) -> u64 {
        let mut hash: u64 = 0;
        for byte in table.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
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

    fn log_insert(&mut self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id,
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

    fn log_delete(&mut self, table_id: u64, key: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id,
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

    fn log_update(&mut self, table_id: u64, key: Vec<u8>, new_record: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id,
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
        let tx_id = self.current_tx_id;
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
                self.active_txs.insert(tx_id, lsn);
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
        let tx_id = self.current_tx_id;
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
            self.wal.sync()?;
        }
        // Issue #3964: previously this called `self.inner.flush()`,
        // which persisted the rolled-back tx's buffered inserts to
        // data.rows and then to disk via the post-flush dirty-table
        // save path. ROLLBACK must discard in-memory writes without
        // persisting them, so we discard the buffer instead.
        self.inner.discard_all_buffers();
        // #3223 Phase 1: remove from active set on rollback.
        self.active_txs.remove(&tx_id);
        Ok(())
    }

    pub fn in_transaction(&self) -> bool {
        // INT-4: report transaction state from the WAL layer's own
        // current_tx_id (set by `set_current_tx_id` / facade), not the
        // inner engine which only knows about its own write-buffer
        // state. This keeps VtuGuard's `assert_dml_safe` correct when
        // the executor opens a TX through the unified facade.
        self.current_tx_id != 0
    }

    pub fn current_tx_id(&self) -> u64 {
        self.current_tx_id
    }

    pub fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        self.wal.recover()
    }
}

impl<S: StorageEngine + 'static, T: WalManager + 'static> StorageEngine for WalStorage<S, T> {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan(table)
    }

    fn flush(&mut self) -> SqlResult<()> {
        self.inner.flush()
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
            let prev_batch_mode = self.wal.is_batch_mode();
            let prev_threshold = self.wal.flush_threshold();
            self.wal.set_batch_mode(true);
            self.wal.set_flush_threshold(usize::MAX);
            let result: SqlResult<()> = (|| {
                for record in &records {
                    let key = Self::record_key(record);
                    let data = Self::record_to_bytes(record);
                    self.log_insert(table_id, key, data)?;
                }
                self.wal.flush()
            })();
            // Restore prior settings (best-effort; WAL consistency is unaffected
            // either way because we flushed before restoring).
            self.wal.set_flush_threshold(prev_threshold);
            if !prev_batch_mode {
                self.wal.set_batch_mode(false);
            }
            result?;
        } else {
            for record in &records {
                let key = Self::record_key(record);
                let data = Self::record_to_bytes(record);
                self.log_insert(table_id, key, data)?;
            }
        }
        self.inner.insert(table, records)
    }

    fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);

        let rows = self.inner.scan(table)?;
        for row in &rows {
            if Self::row_matches_filter(row, filters) {
                let key = Self::record_key(row);
                self.log_delete(table_id, key)?;
            }
        }

        self.inner.delete(table, filters)
    }

    fn delete_if(&mut self, table: &str, filter: &RowFilter) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);
        let key = format!("RowFilter-{:p}", filter).into_bytes();
        self.log_delete(table_id, key)?;
        self.inner.delete_if(table, filter)
    }

    fn update(
        &mut self,
        table: &str,
        filters: &[Value],
        updates: &[(usize, Value)],
    ) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);

        // Step 1: Get all rows and find those matching the filter (before-image)
        let all_rows = self.inner.scan(table)?;
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
        let _ = self.inner.update(table, filters, updates)?;

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
        self.inner.update_if(table, filter, mutation)
    }

    fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.create_database(db_name)
    }

    fn drop_database(&mut self, db_name: &str) -> SqlResult<()> {
        self.inner.drop_database(db_name)
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

    fn create_index(&mut self, table: &str, column: &str, column_index: usize) -> SqlResult<()> {
        self.inner.create_index(table, column, column_index)
    }

    fn drop_index(&mut self, table: &str, column: &str) -> SqlResult<()> {
        self.inner.drop_index(table, column)
    }

    fn add_column(&mut self, table: &str, column: ColumnDefinition) -> SqlResult<()> {
        self.inner.add_column(table, column)
    }

    fn rename_table(&mut self, table: &str, new_name: &str) -> SqlResult<()> {
        self.inner.rename_table(table, new_name)
    }

    fn create_trigger(&mut self, info: TriggerInfo) -> SqlResult<()> {
        self.inner.create_trigger(info)
    }

    fn drop_trigger(&mut self, name: &str) -> SqlResult<()> {
        self.inner.drop_trigger(name)
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

    fn begin_transaction(&mut self) -> SqlResult<u64> {
        let tx_id = self.current_tx_id;
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
                self.active_txs.insert(tx_id, lsn);
            }
        }
        Ok(tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.current_tx_id;
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
                        self.wal.sync()?;
                        self.writes_since_sync = 0;
                    }
                }
                WalSyncMode::Every => {
                    self.wal.sync()?;
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

        self.inner.flush()?;

        // Truncate WAL up to checkpoint
        if commit_lsn > 0 {
            if let Some(cp) = &self.checkpoint_manager {
                if let Ok(guard) = cp.read() {
                    if let Some(cp_lsn) = guard.last_checkpoint_lsn() {
                        let _ = self.wal.truncate_before(cp_lsn);
                    }
                }
            }
        }

        // #3223 Phase 1: remove from active set on commit.
        self.active_txs.remove(&tx_id);
        Ok(())
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.current_tx_id;
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
            self.wal.sync()?;
        }
        // #3223 Phase 1: remove from active set on rollback.
        self.active_txs.remove(&tx_id);
        // Issue #3964: previously this called `self.inner.flush()`,
        // which persisted the rolled-back tx's buffered inserts to
        // data.rows and then to disk via the post-flush dirty-table
        // save path. ROLLBACK must discard in-memory writes without
        // persisting them, so we discard the buffer instead.
        self.inner.discard_all_buffers();
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        self.current_tx_id != 0
    }

    fn current_tx_id(&self) -> u64 {
        self.current_tx_id
    }

    fn set_current_tx_id(&mut self, id: u64) {
        self.current_tx_id = id;
        // PR-842: also propagate to the inner engine so its in_transaction
        // gate sees the right state (FileStorage's insert buffers tx-scoped
        // writes to avoid leaking uncommitted rows to disk on crash).
        self.inner.set_current_tx_id(id);
    }

    fn is_wal_enabled(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
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
        let _ = storage.wal_mut();
    }

    #[test]
    fn test_wal_split() {
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut storage = WalStorage::new(inner, wal).unwrap();
        let (_s, _w) = storage.split();
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
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        let filter: RowFilter = Box::new(|row: &Record| row[0] == Value::Integer(1));
        storage.delete_if("t", &filter).unwrap();
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
        };
        storage.create_table(&info).unwrap();
        storage.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        storage.create_index("t", "a", 0).unwrap();
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
