//! WAL Manager trait and implementations
//!
//! This module provides the WAL (Write-Ahead Log) management abstraction.
//! WAL is responsible for durability - logging modifications before applying them.
//!
//! # Architecture
//!
//! ```text
//! StorageEngine (data)      ←→     WalManager (log)
//!        ↓                              ↓
//!   FileStorage/MemoryStorage      FileBackedWalManager/MemoryWalManager
//! ```
//!
//! # Design Principles
//!
//! 1. **WAL ≠ Recovery**: WalManager only records, reads, and validates WAL entries.
//!    Recovery logic belongs in RecoveryEngine (PR-830C).
//! 2. **Decoupled**: WalManager does not depend on StorageEngine.
//! 3. **Testable**: MemoryWalManager enables unit testing without filesystem.

pub mod file_backed_wal_manager;
pub mod memory_wal_manager;

pub use file_backed_wal_manager::FileBackedWalManager;
pub use memory_wal_manager::MemoryWalManager;

pub use crate::wal_legacy::{
    WalEntry, WalEntryType, WalManager as LegacyWalManager, WalReader, WalWriter,
};

use crate::engine::SqlResult;

/// WAL Manager trait for write-ahead log management.
///
/// Implementors must provide:
/// - `append`: Add a WAL entry
/// - `flush`: Flush buffered writes
/// - `sync`: Force sync to durable storage
/// - `recover`: Read and return WAL entries for recovery
pub trait WalManager: Send + Sync {
    /// Append a WAL entry
    fn append(&mut self, entry: WalEntry) -> SqlResult<()>;

    /// Flush buffered writes
    fn flush(&mut self) -> SqlResult<()>;

    /// Force sync to durable storage
    fn sync(&mut self) -> SqlResult<()>;

    /// Recover WAL entries
    ///
    /// Returns all WAL entries for replay by RecoveryEngine.
    /// Does NOT apply entries to storage - that is RecoveryEngine's responsibility.
    fn recover(&mut self) -> SqlResult<Vec<WalEntry>>;
}

/// WAL truncation safety gate
pub trait WalTruncationGate: Send + Sync {
    /// Returns the LSN below which WAL entries can be safely deleted.
    /// Returns None if no checkpoint has been established.
    fn safe_truncate_lsn(&self) -> Option<u64>;

    /// Check if a given LSN can be truncated
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.safe_truncate_lsn().map_or(false, |cp_lsn| wal_lsn <= cp_lsn)
    }
}

/// Helper to create a BEGIN entry
pub fn make_begin_entry(tx_id: u64) -> WalEntry {
    WalEntry {
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
    }
}

/// Helper to create a COMMIT entry
pub fn make_commit_entry(tx_id: u64, lsn: u64) -> WalEntry {
    WalEntry {
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
    }
}

/// Helper to create a ROLLBACK entry
pub fn make_rollback_entry(tx_id: u64, lsn: u64) -> WalEntry {
    WalEntry {
        tx_id,
        entry_type: WalEntryType::Rollback,
        table_id: 0,
        key: None,
        data: None,
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

/// Helper to create an INSERT entry
pub fn make_insert_entry(
    tx_id: u64,
    table_id: u64,
    key: Vec<u8>,
    data: Vec<u8>,
    lsn: u64,
) -> WalEntry {
    WalEntry {
        tx_id,
        entry_type: WalEntryType::Insert,
        table_id,
        key: Some(key),
        data: Some(data),
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

/// Helper to create an UPDATE entry
pub fn make_update_entry(
    tx_id: u64,
    table_id: u64,
    key: Vec<u8>,
    data: Vec<u8>,
    lsn: u64,
) -> WalEntry {
    WalEntry {
        tx_id,
        entry_type: WalEntryType::Update,
        table_id,
        key: Some(key),
        data: Some(data),
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

/// Helper to create a DELETE entry
pub fn make_delete_entry(tx_id: u64, table_id: u64, key: Vec<u8>, lsn: u64) -> WalEntry {
    WalEntry {
        tx_id,
        entry_type: WalEntryType::Delete,
        table_id,
        key: Some(key),
        data: None,
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}
