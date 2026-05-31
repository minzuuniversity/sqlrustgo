use crate::engine::SqlResult;
use crate::wal::{WalEntry, WalManager};

pub struct MemoryWalManager {
    entries: Vec<WalEntry>,
}

impl MemoryWalManager {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
}

impl Default for MemoryWalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WalManager for MemoryWalManager {
    fn append(&mut self, entry: WalEntry) -> SqlResult<()> {
        self.entries.push(entry);
        Ok(())
    }

    fn flush(&mut self) -> SqlResult<()> {
        Ok(())
    }

    fn sync(&mut self) -> SqlResult<()> {
        Ok(())
    }

    fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        Ok(std::mem::take(&mut self.entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal_legacy::WalEntryType;

    #[test]
    fn test_memory_wal_append() {
        let mut wal = MemoryWalManager::new();
        let entry = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 1000,
        };
        wal.append(entry).unwrap();
        assert_eq!(wal.entries.len(), 1);
    }

    #[test]
    fn test_memory_wal_recover() {
        let mut wal = MemoryWalManager::new();
        let entry1 = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 1000,
        };
        let entry2 = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 2,
            timestamp: 1001,
        };
        wal.append(entry1).unwrap();
        wal.append(entry2).unwrap();

        let recovered = wal.recover().unwrap();
        assert_eq!(recovered.len(), 2);
        assert_eq!(wal.entries.len(), 0);
    }

    #[test]
    fn test_memory_wal_flush_sync() {
        let mut wal = MemoryWalManager::new();
        wal.flush().unwrap();
        wal.sync().unwrap();
    }
}