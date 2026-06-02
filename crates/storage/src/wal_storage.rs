use crate::checkpoint::{CheckpointManager, CheckpointMetadata};
use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlResult, StorageEngine, TableInfo,
    TriggerInfo, Value,
};
use crate::wal::{WalEntry, WalEntryType, WalManager};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct WalStorage<S: StorageEngine, T: WalManager> {
    inner: S,
    wal: T,
    wal_enabled: bool,
    checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>,
    /// Monotonically increasing LSN counter for WAL entries.
    /// Each `append_wal_entry` increments this and assigns the value to the entry.
    next_lsn: u64,
}

impl<S: StorageEngine, T: WalManager> WalStorage<S, T> {
    pub fn new(inner: S, wal: T) -> SqlResult<Self> {
        Ok(Self {
            inner,
            wal,
            wal_enabled: true,
            checkpoint_manager: None,
            next_lsn: 0,
        })
    }

    pub fn with_checkpoint_manager(
        inner: S,
        wal: T,
        checkpoint_manager: Arc<RwLock<CheckpointManager>>,
    ) -> SqlResult<Self> {
        Ok(Self {
            inner,
            wal,
            wal_enabled: true,
            checkpoint_manager: Some(checkpoint_manager),
            next_lsn: 0,
        })
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
                tx_id: self.inner.current_tx_id(),
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
                tx_id: self.inner.current_tx_id(),
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
                tx_id: self.inner.current_tx_id(),
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
        let tx_id = self.inner.current_tx_id();
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
            self.append_wal_entry(entry)?;
        }
        Ok(tx_id)
    }

    pub fn commit_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.inner.current_tx_id();
        let commit_lsn = if self.wal_enabled {
            let lsn = self.wal.current_lsn();
            let entry = WalEntry {
                tx_id,
                entry_type: WalEntryType::Commit,
                table_id: 0,
                key: None,
                data: None,
                lsn,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.append_wal_entry(entry)?;
            self.wal.sync()?;
            self.wal.current_lsn()
        } else {
            0
        };

        // Advance checkpoint so truncation can proceed
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

        Ok(())
    }

    pub fn rollback_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.inner.current_tx_id();
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
        self.inner.flush()?;
        Ok(())
    }

    pub fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
    }

    pub fn current_tx_id(&self) -> u64 {
        self.inner.current_tx_id()
    }

    pub fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        self.wal.recover()
    }
}

impl<S: StorageEngine, T: WalManager> StorageEngine for WalStorage<S, T> {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        self.inner.scan(table)
    }

    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        let table_id = Self::table_name_to_id(table);
        for record in &records {
            let key = Self::record_key(record);
            let data = Self::record_to_bytes(record);
            self.log_insert(table_id, key, data)?;
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
        let tx_id = self.inner.current_tx_id();
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
            self.append_wal_entry(entry)?;
        }
        Ok(tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.inner.current_tx_id();
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
            self.append_wal_entry(entry)?;
            self.wal.sync()?;
        }
        self.inner.flush()?;
        Ok(())
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        let tx_id = self.inner.current_tx_id();
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
        self.inner.flush()?;
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        self.inner.in_transaction()
    }

    fn current_tx_id(&self) -> u64 {
        self.inner.current_tx_id()
    }

    fn set_current_tx_id(&mut self, id: u64) {
        self.inner.set_current_tx_id(id);
    }

    fn is_wal_enabled(&self) -> bool {
        true
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
}
