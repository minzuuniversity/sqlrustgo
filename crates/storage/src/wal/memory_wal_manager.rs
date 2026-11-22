use crate::engine::SqlResult;
use crate::wal::{WalEntry, WalManager};

pub struct MemoryWalManager {
    entries: Vec<WalEntry>,
    /// The LSN the next `append` will stamp. The manager owns the LSN
    /// space, so the counter lives here rather than in the caller.
    next_lsn: u64,
}

impl MemoryWalManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_lsn: 0,
        }
    }
}

impl Default for MemoryWalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WalManager for MemoryWalManager {
    fn append(&mut self, mut entry: WalEntry) -> SqlResult<u64> {
        let lsn = self.next_lsn;
        entry.lsn = lsn;
        self.next_lsn += 1;
        self.entries.push(entry);
        Ok(lsn)
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

    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()> {
        self.entries.retain(|e| e.lsn >= lsn);
        Ok(())
    }

    fn current_lsn(&self) -> u64 {
        // The LSN the NEXT append will stamp — one past the highest LSN
        // written so far. Returning the maximum instead made the two
        // off by one, so a caller that appended and then read this back
        // saw the LSN it had just used rather than the next one.
        self.next_lsn
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
            table_name: None,
            database_name: None,
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
            table_name: None,
            database_name: None,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 1000,
        };
        let entry2 = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            table_name: None,
            database_name: None,
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
