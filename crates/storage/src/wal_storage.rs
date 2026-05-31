use crate::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, SqlError, SqlResult, StorageEngine,
    TableInfo, TriggerInfo, Value,
};
use crate::wal::{WalEntry, WalEntryType, WalManager};

pub struct WalStorage<S: StorageEngine, T: WalManager> {
    inner: S,
    wal: T,
    current_tx_id: u64,
    wal_enabled: bool,
}

impl<S: StorageEngine, T: WalManager> WalStorage<S, T> {
    pub fn new(inner: S, wal: T) -> SqlResult<Self> {
        Ok(Self {
            inner,
            wal,
            current_tx_id: 0,
            wal_enabled: true,
        })
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn wal(&self) -> &T {
        &self.wal
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

    fn log_insert(&mut self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled && self.current_tx_id != 0 {
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
            self.wal.append(entry)?;
        }
        Ok(())
    }

    fn log_delete(&mut self, table_id: u64, key: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled && self.current_tx_id != 0 {
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
            self.wal.append(entry)?;
        }
        Ok(())
    }

    fn log_update(&mut self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
        if self.wal_enabled && self.current_tx_id != 0 {
            let entry = WalEntry {
                tx_id: self.current_tx_id,
                entry_type: WalEntryType::Update,
                table_id,
                key: Some(key),
                data: Some(data),
                lsn: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            self.wal.append(entry)?;
        }
        Ok(())
    }

    pub fn begin_transaction(&mut self) -> SqlResult<u64> {
        if self.current_tx_id != 0 {
            return Err(crate::engine::SqlError::ExecutionError(
                "Transaction already in progress".to_string(),
            ));
        }
        let tx_id = self.current_tx_id + 1;
        self.current_tx_id = tx_id;
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
            self.wal.append(entry)?;
        }
        Ok(tx_id)
    }

    pub fn commit_transaction(&mut self) -> SqlResult<()> {
        if self.current_tx_id == 0 {
            return Err(crate::engine::SqlError::ExecutionError(
                "No transaction in progress".to_string(),
            ));
        }
        let tx_id = self.current_tx_id;
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
            self.wal.append(entry)?;
            self.wal.sync()?;
        }
        self.current_tx_id = 0;
        Ok(())
    }

    pub fn rollback_transaction(&mut self) -> SqlResult<()> {
        if self.current_tx_id == 0 {
            return Err(crate::engine::SqlError::ExecutionError(
                "No transaction in progress".to_string(),
            ));
        }
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
            self.wal.append(entry)?;
            self.wal.sync()?;
        }
        self.current_tx_id = 0;
        Ok(())
    }

    pub fn in_transaction(&self) -> bool {
        self.current_tx_id != 0
    }

    pub fn current_tx_id(&self) -> u64 {
        self.current_tx_id
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
        let key = format!("{:?}", filters).into_bytes();
        self.log_delete(table_id, key)?;
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
        let key = format!("{:?}", filters).into_bytes();
        let data = format!("{:?}", updates).into_bytes();
        self.log_update(table_id, key, data)?;
        self.inner.update(table, filters, updates)
    }

    fn update_if(
        &mut self,
        table: &str,
        filter: &RowFilter,
        mutation: &RowMutation,
    ) -> SqlResult<usize> {
        let table_id = Self::table_name_to_id(table);
        let key = format!("RowFilter-{:p}", filter).into_bytes();
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
        if self.current_tx_id != 0 {
            return Err(SqlError::ExecutionError(
                "Transaction already in progress".to_string(),
            ));
        }
        let tx_id = self.current_tx_id + 1;
        self.current_tx_id = tx_id;
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
            self.wal.append(entry)?;
        }
        Ok(tx_id)
    }

    fn commit_transaction(&mut self) -> SqlResult<()> {
        if self.current_tx_id == 0 {
            return Err(SqlError::ExecutionError(
                "No transaction in progress".to_string(),
            ));
        }
        let tx_id = self.current_tx_id;
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
            self.wal.append(entry)?;
            self.wal.sync()?;
        }
        self.current_tx_id = 0;
        Ok(())
    }

    fn rollback_transaction(&mut self) -> SqlResult<()> {
        if self.current_tx_id == 0 {
            return Err(SqlError::ExecutionError(
                "No transaction in progress".to_string(),
            ));
        }
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
            self.wal.append(entry)?;
            self.wal.sync()?;
        }
        self.current_tx_id = 0;
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
        assert!(tx_id > 0);
        assert!(storage.in_transaction());

        let records = vec![vec![Value::Integer(1), Value::Text("test".to_string())]];
        storage.insert("t1", records).unwrap();

        storage.commit_transaction().unwrap();
        assert!(!storage.in_transaction());

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
}
