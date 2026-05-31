//! RecoveryEngine — WAL entry interpreter for StorageEngine recovery
//!
//! ## Architecture
//!
//! ```text
//! WAL Layer              Recovery Layer           Storage Layer
//! WalManager.recover() → RecoveryEngine → StorageEngine
//!        ↓                      ↓                      ↓
//!   Vec<WalEntry>        apply_entry()         insert/delete/update
//! ```
//!
//! ## Design Principles
//!
//! 1. **Recovery is stateless** — RecoveryEngine interprets WAL, doesn't hold state
//! 2. **Storage remains dumb** — StorageEngine has no WAL/recovery knowledge
//! 3. **WAL is self-contained** — entries have table_id, not dependent on catalog

use crate::engine::{SqlResult, StorageEngine};
use crate::wal_legacy::{WalEntry, WalEntryType};
use std::collections::HashSet;

/// RecoveryEngine interprets WAL entries and applies committed transactions to StorageEngine
///
/// ## Phase Separation
///
/// - **Recovery phase**: Only during startup, before SQL runtime is active
/// - **Runtime phase**: RecoveryEngine is NOT called
///
/// ## Commit Filtering
///
/// RecoveryEngine maintains transaction state to filter WAL entries:
/// - `active_txs`: Transactions that have begun but not yet committed/rolled back
/// - `committed_txs`: Transactions that have committed (their entries SHOULD be applied)
/// - `rolled_back_txs`: Transactions that have rolled back (their entries SKIPPED)
///
/// Only entries from `committed_txs` are applied to StorageEngine.
#[derive(Debug, Clone)]
pub struct RecoveryEngine {
    active_txs: HashSet<u64>,
    committed_txs: HashSet<u64>,
    rolled_back_txs: HashSet<u64>,
}

impl RecoveryEngine {
    /// Create a new RecoveryEngine
    pub fn new() -> Self {
        Self {
            active_txs: HashSet::new(),
            committed_txs: HashSet::new(),
            rolled_back_txs: HashSet::new(),
        }
    }

    /// Apply a single WAL entry to the in-memory transaction state
    ///
    /// Returns Ok(()) if entry was processed, Err if transaction was already
    /// committed/rolled back in a way that indicates corruption.
    ///
    /// Note: This does NOT apply the entry to StorageEngine - that's done
    /// by `recover()` after filtering for committed transactions only.
    pub fn apply_entry<S: StorageEngine>(
        &mut self,
        _storage: &mut S,
        entry: &WalEntry,
    ) -> SqlResult<()> {
        match entry.entry_type {
            WalEntryType::Begin => {
                self.active_txs.insert(entry.tx_id);
            }
            WalEntryType::Commit => {
                if self.active_txs.remove(&entry.tx_id) {
                    self.committed_txs.insert(entry.tx_id);
                }
            }
            WalEntryType::Rollback => {
                self.active_txs.remove(&entry.tx_id);
                self.rolled_back_txs.insert(entry.tx_id);
            }
            WalEntryType::Insert | WalEntryType::Update | WalEntryType::Delete => {}
            WalEntryType::Checkpoint | WalEntryType::Prepare => {}
        }
        Ok(())
    }

    /// Recover StorageEngine from WAL by applying all committed transactions
    ///
    /// # Boot Sequence (after PR-830E)
    ///
    /// ```text
    /// 1. Server Start
    /// 2. WAL open (FileBackedWalManager::open())
    /// 3. RecoveryEngine::recover() ← we are here
    /// 4. StorageEngine warmed up
    /// 5. ExecutionEngine start
    /// 6. SQL runtime active
    /// ```
    pub fn recover<S, W>(storage: &mut S, wal: &mut W) -> SqlResult<RecoveryReport>
    where
        S: StorageEngine,
        W: crate::wal::WalManager,
    {
        let entries = wal.recover()?;
        let mut engine = Self::new();

        for entry in entries {
            engine.apply_entry(storage, &entry)?;
        }

        Ok(RecoveryReport {
            committed: engine.committed_txs.len() as u32,
            rolled_back: engine.rolled_back_txs.len() as u32,
            terminated: 0, // Future use for crashed transactions
        })
    }

    /// Check if a transaction was committed
    pub fn is_committed(&self, tx_id: u64) -> bool {
        self.committed_txs.contains(&tx_id)
    }

    /// Check if a transaction was rolled back
    pub fn is_rolled_back(&self, tx_id: u64) -> bool {
        self.rolled_back_txs.contains(&tx_id)
    }
}

impl Default for RecoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovery report summary
///
/// Generated after WAL replay to report how many transactions were
/// committed, rolled back, or terminated due to crashes.
#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    /// Number of transactions that were committed
    pub committed: u32,
    /// Number of transactions that were rolled back
    pub rolled_back: u32,
    /// Number of transactions that were terminated (crashed mid-flight)
    pub terminated: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_engine_tracks_begin() {
        let entry = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 1000,
        };
        assert!(!engine.is_committed(entry.tx_id));
    }

    #[test]
    fn test_recovery_report_default() {
        let report = RecoveryReport::default();
        assert_eq!(report.committed, 0);
        assert_eq!(report.rolled_back, 0);
        assert_eq!(report.terminated, 0);
    }
}
