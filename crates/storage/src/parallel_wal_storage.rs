//! ParallelWalStorage - WAL storage with parallel table flushing
//!
//! V311-09: Addresses global lock bottleneck during commit
//!
//! Key insight: WAL writes must be serial (ordering), but table flushes can be parallel.

use crate::engine::{ColumnDefinition, Record, SqlError, SqlResult, StorageEngine, TableInfo};
use crate::wal::{WalEntry, WalEntryType, WalManager};
use crate::wal_storage::WalSyncMode;

pub struct ParallelWalStorage<S: StorageEngine, W: WalManager> {
    inner: S,
    wal: W,
    sync_mode: WalSyncMode,
    writes_since_sync: usize,
    wal_enabled: bool,
    current_tx_id: u64,
    next_lsn: u64,
}

impl<S: StorageEngine, W: WalManager> ParallelWalStorage<S, W> {
    pub fn new(inner: S, wal: W) -> Self {
        Self {
            inner,
            wal,
            sync_mode: WalSyncMode::Every,
            writes_since_sync: 0,
            wal_enabled: true,
            current_tx_id: 0,
            next_lsn: 0,
        }
    }

    pub fn set_sync_mode(&mut self, mode: WalSyncMode) {
        self.sync_mode = mode;
    }

    fn append_wal_entry(&mut self, mut entry: WalEntry) -> SqlResult<u64> {
        let lsn = self.next_lsn;
        self.next_lsn += 1;
        entry.lsn = lsn;
        self.wal
            .append(entry)
            .map_err(|e| SqlError::ExecutionError(format!("WAL append error: {}", e)))?;
        Ok(lsn)
    }
}

impl<S: StorageEngine, W: WalManager> StorageEngine for ParallelWalStorage<S, W> {
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        self.inner.insert(table, records)
    }

    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan(table)
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
        self.current_tx_id += 1;
        Ok(self.current_tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        // 1. Write commit entry to WAL (serial)
        if self.wal_enabled {
            let entry = WalEntry {
                tx_id: self.current_tx_id,
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

        // 2. Sync WAL based on sync_mode
        if self.wal_enabled {
            match self.sync_mode {
                WalSyncMode::Off => {}
                WalSyncMode::Batch(n) => {
                    self.writes_since_sync += 1;
                    if self.writes_since_sync >= n as usize {
                        self.wal.sync().map_err(|e| {
                            SqlError::ExecutionError(format!("WAL sync error: {}", e))
                        })?;
                        self.writes_since_sync = 0;
                    }
                }
                WalSyncMode::Every => {
                    self.wal
                        .sync()
                        .map_err(|e| SqlError::ExecutionError(format!("WAL sync error: {}", e)))?;
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
    fn set_current_tx_id(&mut self, tx_id: u64) {
        self.current_tx_id = tx_id;
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
}
