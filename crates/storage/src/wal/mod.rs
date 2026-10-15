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
pub mod group_commit;
pub mod memory_wal_manager;

pub use file_backed_wal_manager::FileBackedWalManager;
pub use group_commit::GroupCommitCoordinator;
pub use memory_wal_manager::MemoryWalManager;

pub use crate::wal_legacy::{
    WalEntry, WalEntryType, WalManager as LegacyWalManager, WalReader, WalWriter,
};

use crate::engine::SqlResult;
use std::sync::{Arc, Mutex};

/// #5317: shared `FileBackedWalManager` handle for the production stack.
///
/// The previous design had `FileStorage::new_with_wal` open its own
/// `FileBackedWalManager` internally while `ParallelWalStorage` /
/// `WalStorage` opened a second, independent handle over the same
/// `sqlrustgo.wal` file. Two independent writers racing on one file is
/// exactly the failure mode the comment at
/// `crates/mysql-server/src/lib.rs:6966-6972` warns about ("two
/// independent BufWriters over one WAL file would interleave
/// corruptly"), and the production code only enforced that for the
/// `WalSyncMode::GroupCommit` branch. The remaining
/// `WalSyncMode::Off / Batch / Every` branches plus the default
/// `--storage file` stack all opened two handles, which is what issue
/// #5317 captures as the double-FBWM bug.
///
/// `SharedWalHandle` is the contract that fixes it: a single
/// `FileBackedWalManager` is wrapped in `Arc<dyn WalManager>` (so each
/// consumer can hold an `Arc`) and then in `Arc<std::sync::Mutex<_>>`
/// (so concurrent appends serialize on one lock — the same lock that
/// already serializes `WalStorage` / `ParallelWalStorage`'s
/// commit-path fsyncs). All three layers (`FileStorage`,
/// `WalStorage`, `ParallelWalStorage`) hold clones of the same
/// `SharedWalHandle`, so a single physical file handle is the only
/// writer of `sqlrustgo.wal`.
/// `Box<dyn WalManager>` rather than `Arc<dyn WalManager>`: every
/// `WalManager` method takes `&mut self`, and `Arc<dyn WalManager>` is
/// not reachable as `&mut dyn WalManager` (DerefMut is only implemented
/// when the Arc has unique ownership, which `FileStorage` /
/// `WalStorage` deliberately do *not* have). The `Box` lets the inner
/// `std::sync::Mutex` give out `&mut dyn WalManager` through the lock
/// guard. The outer `Arc` is only there to share the lock between
/// `FileStorage` and the wrapping `WalStorage` / `ParallelWalStorage`
/// — the inner `Box` is never cloned.
pub type SharedWalHandle = Arc<Mutex<Box<dyn WalManager>>>;

/// #5317: thin newtype around `SharedWalHandle` that implements
/// `WalManager` by delegating each method through the shared inner
/// mutex. `ParallelWalStorage<S, W>` is generic over `W: WalManager`,
/// but the trait object `Box<dyn WalManager>` does not implement
/// `WalManager` itself — `Box<DynTrait>` DerefMut gives `&mut DynTrait`
/// but no impl exists. The newtype plugs that gap so the shared
/// handle can be the storage layer's WAL instance.
pub struct DynWalManager(pub SharedWalHandle);

impl WalManager for DynWalManager {
    fn append(&mut self, entry: WalEntry) -> SqlResult<()> {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.append(entry)
    }
    fn flush(&mut self) -> SqlResult<()> {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.flush()
    }
    fn sync(&mut self) -> SqlResult<()> {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.sync()
    }
    fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.recover()
    }
    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()> {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.truncate_before(lsn)
    }
    fn current_lsn(&self) -> u64 {
        let g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.current_lsn()
    }
    fn size(&self) -> SqlResult<u64> {
        let g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.size()
    }
    fn is_batch_mode(&self) -> bool {
        let g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.is_batch_mode()
    }
    fn set_batch_mode(&mut self, enable: bool) {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.set_batch_mode(enable);
    }
    fn flush_threshold(&self) -> usize {
        let g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.flush_threshold()
    }
    fn set_flush_threshold(&mut self, threshold: usize) {
        let mut g = self.0.lock().expect("SharedWalHandle inner mutex poisoned");
        g.set_flush_threshold(threshold);
    }
}

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

    /// Truncate WAL entries with LSN < `lsn`
    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()>;

    /// Returns the current LSN (Log Sequence Number) after the last append.
    /// Returns 0 if no entries have been written.
    fn current_lsn(&self) -> u64;

    /// Returns the current WAL file size in bytes.
    /// Returns 0 if the size cannot be determined.
    fn size(&self) -> SqlResult<u64> {
        Ok(0)
    }

    /// Returns `true` if batch mode is currently enabled.
    ///
    /// Batch mode allows the implementor to buffer multiple `append` calls
    /// into a single `flush`, avoiding per-record fsyncs on the hot path.
    /// Used by `WalStorage::insert` to amortize WAL cost over batched DML
    /// (issue #3013). Default: `false` (no batching supported).
    fn is_batch_mode(&self) -> bool {
        false
    }

    /// Enable or disable batch mode.
    fn set_batch_mode(&mut self, enable: bool) {
        let _ = enable;
    }

    /// Returns the number of buffered entries that trigger an automatic flush.
    fn flush_threshold(&self) -> usize {
        100
    }

    /// Set the flush threshold.
    fn set_flush_threshold(&mut self, threshold: usize) {
        let _ = threshold;
    }
}
/// WAL truncation safety gate
pub trait WalTruncationGate: Send + Sync {
    /// Returns the LSN below which WAL entries can be safely deleted.
    /// Returns None if no checkpoint has been established.
    fn safe_truncate_lsn(&self) -> Option<u64>;

    /// Check if a given LSN can be truncated
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.safe_truncate_lsn()
            .is_some_and(|cp_lsn| wal_lsn <= cp_lsn)
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
        table_name: None,
        database_name: None,
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
        table_name: None,
        database_name: None,
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
        table_name: None,
        database_name: None,
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
        table_name: None,
        database_name: None,
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
        table_name: None,
        database_name: None,
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
        table_name: None,
        database_name: None,
    }
}

// =========================================================================
// V400-02 / Issue #3730: vector WAL entry factories (added 2026-09-29 to close
// the v4.0.0->v4.1.0 test-compile drift on the new WalEntryType variants).
// Each factory mirrors the existing row-level make_*_entry shape: the
// `key` field carries the (table, column, vector_id) triple and `data`
// carries the encoded embedding bytes. The index-management variants
// (CreateVectorIndex / DropVectorIndex / RebuildVectorIndex) use `table_id`
// as the index id and stash the column list in `key`.
// =========================================================================

/// Helper to create a VectorInsert entry
pub fn make_vector_insert_entry(
    tx_id: u64,
    table_id: u64,
    column_id: u64,
    vector_id: u64,
    embedding: Vec<u8>,
    lsn: u64,
) -> WalEntry {
    let key = format!("{}:{}:{}", table_id, column_id, vector_id).into_bytes();
    WalEntry {
        tx_id,
        entry_type: WalEntryType::VectorInsert,
        table_id,
        key: Some(key),
        data: Some(embedding),
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        table_name: None,
        database_name: None,
    }
}

/// Helper to create a VectorUpdate entry (overwrite at existing id)
pub fn make_vector_update_entry(
    tx_id: u64,
    table_id: u64,
    column_id: u64,
    vector_id: u64,
    embedding: Vec<u8>,
    lsn: u64,
) -> WalEntry {
    let key = format!("{}:{}:{}", table_id, column_id, vector_id).into_bytes();
    WalEntry {
        tx_id,
        entry_type: WalEntryType::VectorUpdate,
        table_id,
        key: Some(key),
        data: Some(embedding),
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        table_name: None,
        database_name: None,
    }
}

/// Helper to create a VectorDelete entry (delete by id)
pub fn make_vector_delete_entry(
    tx_id: u64,
    table_id: u64,
    column_id: u64,
    vector_id: u64,
    lsn: u64,
) -> WalEntry {
    let key = format!("{}:{}:{}", table_id, column_id, vector_id).into_bytes();
    WalEntry {
        tx_id,
        entry_type: WalEntryType::VectorDelete,
        table_id,
        key: Some(key),
        data: None,
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        table_name: None,
        database_name: None,
    }
}

/// Helper to create a CreateVectorIndex entry
pub fn make_create_vector_index_entry(
    tx_id: u64,
    table_id: u64,
    column_id: u64,
    index_name: &str,
    lsn: u64,
) -> WalEntry {
    let key = format!("{}:{}:{}", table_id, column_id, index_name).into_bytes();
    WalEntry {
        tx_id,
        entry_type: WalEntryType::CreateVectorIndex,
        table_id,
        key: Some(key),
        data: None,
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        table_name: None,
        database_name: None,
    }
}

/// Helper to create a DropVectorIndex entry
pub fn make_drop_vector_index_entry(
    tx_id: u64,
    table_id: u64,
    index_name: &str,
    lsn: u64,
) -> WalEntry {
    let key = format!("{}:{}", table_id, index_name).into_bytes();
    WalEntry {
        tx_id,
        entry_type: WalEntryType::DropVectorIndex,
        table_id,
        key: Some(key),
        data: None,
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        table_name: None,
        database_name: None,
    }
}

/// Helper to create a RebuildVectorIndex entry
pub fn make_rebuild_vector_index_entry(
    tx_id: u64,
    table_id: u64,
    index_name: &str,
    lsn: u64,
) -> WalEntry {
    let key = format!("{}:{}", table_id, index_name).into_bytes();
    WalEntry {
        tx_id,
        entry_type: WalEntryType::RebuildVectorIndex,
        table_id,
        key: Some(key),
        data: None,
        lsn,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        table_name: None,
        database_name: None,
    }
}
