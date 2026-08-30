use std::collections::HashMap;

use crate::mvcc::{Snapshot, TxId};
use crate::savepoint::UndoRecord;
use crate::ssi::{SsiDetectorSync, SsiError};

/// Transaction isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsolationLevel {
    /// Snapshot Isolation - readers see consistent snapshot, writers use first-committer-wins
    #[default]
    SnapshotIsolation,
    /// Serializable - ensures strict serial execution order
    Serializable,
}

/// Current state of a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Transaction is actively executing
    Active,
    /// Transaction has been committed successfully
    Committed,
    /// Transaction was aborted (rolled back)
    Aborted,
}

/// Active transaction with its metadata
#[derive(Debug)]
pub struct ActiveTransaction {
    /// Unique transaction identifier
    pub tx_id: TxId,
    /// MVCC snapshot for this transaction
    pub snapshot: Snapshot,
    /// Current state of the transaction
    pub state: TransactionState,
    /// Keys read by this transaction
    pub read_keys: Vec<Vec<u8>>,
    /// Keys written by this transaction
    pub write_keys: Vec<Vec<u8>>,
    /// SEM-1 (#3172): per-transaction savepoint manager.
    /// SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT operate on this
    /// per-tx state. The savepoints are NOT persisted to WAL in this
    /// iteration; physical rollback of tuple changes is deferred to a
    /// future PR (the API is in place so callers can begin using the
    /// statement-level semantics today).
    pub savepoint_manager: crate::savepoint::SavepointManager,
}

impl ActiveTransaction {
    /// Create a new active transaction
    pub fn new(tx_id: TxId, snapshot: Snapshot) -> Self {
        Self {
            tx_id,
            snapshot,
            state: TransactionState::Active,
            read_keys: Vec::new(),
            write_keys: Vec::new(),
            savepoint_manager: crate::savepoint::SavepointManager::new(),
        }
    }
}

/// Transaction manager with SSI (Serializable Snapshot Isolation) support
pub struct TransactionManager {
    ssi_detector: SsiDetectorSync,
    active_transactions: HashMap<TxId, ActiveTransaction>,
    next_tx_id: u64,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new() -> Self {
        Self {
            ssi_detector: SsiDetectorSync::new(),
            active_transactions: HashMap::new(),
            next_tx_id: 1,
        }
    }

    /// Begin a new transaction with the specified isolation level
    ///
    /// # Arguments
    /// * `isolation` - Isolation level for the new transaction
    ///
    /// # Returns
    /// * `Ok(TxId)` - Transaction ID if successful
    /// * `Err(SsiError)` - If transaction cannot be started
    pub fn begin_transaction(&mut self, _isolation: IsolationLevel) -> Result<TxId, SsiError> {
        let tx_id = TxId::new(self.next_tx_id);
        self.next_tx_id += 1;

        let snapshot_timestamp = tx_id.as_u64();
        let snapshot = Snapshot::new_read_committed(tx_id, snapshot_timestamp);

        let active_tx = ActiveTransaction::new(tx_id, snapshot);
        self.active_transactions.insert(tx_id, active_tx);

        Ok(tx_id)
    }

    /// Record a read operation for SSI detection
    ///
    /// # Arguments
    /// * `tx_id` - Transaction ID
    /// * `key` - Key being read
    pub fn record_read(&mut self, tx_id: TxId, key: Vec<u8>) -> Result<(), SsiError> {
        self.ssi_detector.record_read(tx_id, key.clone());

        if let Some(active_tx) = self.active_transactions.get_mut(&tx_id) {
            active_tx.read_keys.push(key);
        }

        Ok(())
    }

    /// Record a write operation for SSI detection
    ///
    /// # Arguments
    /// * `tx_id` - Transaction ID
    /// * `key` - Key being written
    pub fn record_write(&mut self, tx_id: TxId, key: Vec<u8>) -> Result<(), SsiError> {
        self.ssi_detector.record_write(tx_id, key.clone());

        if let Some(active_tx) = self.active_transactions.get_mut(&tx_id) {
            active_tx.write_keys.push(key);
        }

        Ok(())
    }

    /// Commit a transaction
    ///
    /// # Arguments
    /// * `tx_id` - Transaction ID to commit
    ///
    /// # Returns
    /// * `Ok(())` - If commit succeeds
    /// * `Err(SsiError)` - If serialization failure detected
    pub fn commit(&mut self, tx_id: TxId) -> Result<(), SsiError> {
        self.ssi_detector.validate_commit(tx_id)?;

        if let Some(active_tx) = self.active_transactions.get_mut(&tx_id) {
            active_tx.state = TransactionState::Committed;
        }

        self.ssi_detector.release(tx_id);
        self.active_transactions.remove(&tx_id);

        Ok(())
    }

    pub fn rollback(&mut self, tx_id: TxId) -> Result<(), SsiError> {
        if let Some(active_tx) = self.active_transactions.get_mut(&tx_id) {
            active_tx.state = TransactionState::Aborted;
        }

        self.ssi_detector.release(tx_id);
        self.active_transactions.remove(&tx_id);

        Ok(())
    }

    /// Issue #4581 / B-track case 35-36: top-level ROLLBACK physical
    /// undo. Replays the per-tx undo log in reverse via the `on_undo`
    /// closure (which performs the actual storage operations). Mirrors
    /// `rollback_to_savepoint_with_undo` (the SAVEPOINT path), but
    /// applies to the WHOLE transaction rather than to a named
    /// savepoint. The caller (ExecutionEngine) provides the closure
    /// that drives `storage.delete` / `storage.insert` from each
    /// `UndoRecord`.
    ///
    /// Errors from `on_undo` are logged to stderr but DO NOT abort the
    /// rollback — matches the SAVEPOINT convention at
    /// `savepoint.rs:129` (best-effort physical undo). The transaction
    /// state is marked Aborted and the active entry is removed from
    /// `active_transactions` regardless.
    pub fn rollback_with_undo<F>(
        &mut self,
        tx_id: TxId,
        mut on_undo: F,
    ) -> Result<(), SsiError>
    where
        F: FnMut(&UndoRecord) -> Result<(), String>,
    {
        let active = self
            .active_transactions
            .get_mut(&tx_id)
            .ok_or(SsiError::TransactionNotFound { tx_id })?;
        active.state = TransactionState::Aborted;
        // Take ownership of the undo log so the borrow on `active` is
        // released before the closure runs (the closure re-acquires the
        // storage write lock via the engine's Arc<RwLock<StorageEngine>>).
        let undo_log = active.savepoint_manager.take_undo_log();
        self.ssi_detector.release(tx_id);
        self.active_transactions.remove(&tx_id);
        // Reverse iteration: most recent operation is undone first so
        // referential integrity is preserved (e.g. an INSERT that
        // depended on a row inserted later is undone first, leaving the
        // dependency row intact).
        for record in undo_log.iter().rev() {
            if let Err(e) = on_undo(record) {
                eprintln!(
                    "Transaction {} undo failed (continuing): {}",
                    tx_id.as_u64(),
                    e
                );
            }
        }
        Ok(())
    }

    /// Abort (rollback) a transaction
    ///
    /// # Arguments
    /// * `tx_id` - Transaction ID to abort
    pub fn abort(&mut self, tx_id: TxId) -> Result<(), SsiError> {
        if let Some(active_tx) = self.active_transactions.get_mut(&tx_id) {
            active_tx.state = TransactionState::Aborted;
        }

        self.ssi_detector.release(tx_id);
        self.active_transactions.remove(&tx_id);

        Ok(())
    }

    /// Get the snapshot for a transaction
    ///
    /// # Arguments
    /// * `tx_id` - Transaction ID
    ///
    /// # Returns
    /// * `Some(Snapshot)` - If transaction is active
    /// * `None` - If transaction not found
    pub fn get_snapshot(&self, tx_id: TxId) -> Option<Snapshot> {
        self.active_transactions
            .get(&tx_id)
            .map(|at| at.snapshot.clone())
    }

    /// SEM-1 (#3172): Create or reset a SAVEPOINT in the named transaction.
    ///
    /// The MySQL 5.7 semantics: if a savepoint with this name already
    /// exists, RESET its undo position to the current undo-log length
    /// (so subsequent DML before the next SAVEPOINT/RELEASE is captured
    /// by this name). Otherwise, register a new savepoint.
    ///
    /// # Errors
    /// Returns an error if the transaction is not active.
    pub fn savepoint(&mut self, tx_id: TxId, name: String) -> Result<(), SsiError> {
        let active = self
            .active_transactions
            .get_mut(&tx_id)
            .ok_or(SsiError::TransactionNotFound { tx_id })?;
        active
            .savepoint_manager
            .savepoint(name)
            .map_err(|e| match e {
                crate::savepoint::SavepointError::NotFound => {
                    SsiError::TransactionNotFound { tx_id }
                }
                crate::savepoint::SavepointError::InvalidOperation => SsiError::LockTimeout,
            })
    }

    /// SEM-1 (#3172): ROLLBACK TO SAVEPOINT.
    ///
    /// Legacy no-physical-undo variant retained for backward compatibility.
    /// New callers should prefer [`Self::rollback_to_savepoint_with_undo`]
    /// which lets the orchestrator drive the actual storage reverse-op
    /// via a closure (issue #4519 wired this in v3.12).
    #[deprecated(
        since = "3.12.0",
        note = "Use rollback_to_savepoint_with_undo for physical undo"
    )]
    pub fn rollback_to_savepoint(&mut self, tx_id: TxId, name: &str) -> Result<(), SsiError> {
        self.rollback_to_savepoint_with_undo(tx_id, name, |_| Ok(()))
    }

    /// #4519 (清华 MySQL 课程第 9 章核心): physically undo DML after a
    /// savepoint by handing each `UndoRecord` to the caller-supplied
    /// closure. The closure is invoked in reverse order
    /// (last-write-first) so the post-state matches the pre-savepoint
    /// snapshot.
    ///
    /// The orchestrator (`ExecutionEngine`) supplies a closure that
    /// drives `storage.delete` (Insert undo) or
    /// `storage.insert(old_value)` (Delete / Update undo) — see
    /// `src/execution_engine.rs::execute_savepoint`.
    pub fn rollback_to_savepoint_with_undo<F>(
        &mut self,
        tx_id: TxId,
        name: &str,
        mut on_undo: F,
    ) -> Result<(), SsiError>
    where
        F: FnMut(&UndoRecord) -> Result<(), String>,
    {
        let active = self
            .active_transactions
            .get_mut(&tx_id)
            .ok_or(SsiError::TransactionNotFound { tx_id })?;
        active
            .savepoint_manager
            .rollback_to(name, |rec| on_undo(rec))
            .map_err(|e| match e {
                crate::savepoint::SavepointError::NotFound => SsiError::LockTimeout,
                crate::savepoint::SavepointError::InvalidOperation => SsiError::LockTimeout,
            })
    }

    /// #4519: append a typed `UndoRecord` to the active transaction's
    /// savepoint undo log. No-op when the transaction has no active
    /// savepoints (the executor short-circuits before calling this so
    /// the undo log never grows for savepoint-less transactions).
    pub fn add_undo_record(&mut self, tx_id: TxId, record: UndoRecord) -> Result<(), SsiError> {
        let active = self
            .active_transactions
            .get_mut(&tx_id)
            .ok_or(SsiError::TransactionNotFound { tx_id })?;
        active.savepoint_manager.add_undo(record);
        Ok(())
    }

    /// #4519: returns `true` when the active transaction has at least
    /// one savepoint. The executor uses this guard to decide whether
    /// to record an undo entry for each DML — without an active
    /// savepoint the undo record is wasted memory.
    pub fn has_active_savepoint(&self, tx_id: TxId) -> bool {
        self.active_transactions
            .get(&tx_id)
            .map(|at| at.savepoint_manager.get_savepoint_count() > 0)
            .unwrap_or(false)
    }

    /// SEM-1 (#3172): RELEASE SAVEPOINT.
    ///
    /// Removes the savepoint from the stack. The undo-log entries are
    /// kept (they may still be needed by outer savepoints). A
    /// non-existent savepoint is a no-op (matches MySQL behaviour).
    pub fn release_savepoint(&mut self, tx_id: TxId, name: &str) -> Result<(), SsiError> {
        let active = self
            .active_transactions
            .get_mut(&tx_id)
            .ok_or(SsiError::TransactionNotFound { tx_id })?;
        active
            .savepoint_manager
            .release_savepoint(name)
            .map_err(|e| match e {
                crate::savepoint::SavepointError::NotFound => SsiError::LockTimeout,
                crate::savepoint::SavepointError::InvalidOperation => SsiError::LockTimeout,
            })
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_begin_transaction() {
        let mut mgr = TransactionManager::new();
        let tx_id = mgr.begin_transaction(IsolationLevel::SnapshotIsolation);
        assert!(tx_id.is_ok());
        assert_eq!(tx_id.unwrap().as_u64(), 1);
    }

    #[test]
    fn test_record_read_write() {
        let mut mgr = TransactionManager::new();
        let tx_id = mgr
            .begin_transaction(IsolationLevel::SnapshotIsolation)
            .unwrap();

        mgr.record_read(tx_id, b"key1".to_vec()).unwrap();
        mgr.record_write(tx_id, b"key2".to_vec()).unwrap();

        let active = mgr.active_transactions.get(&tx_id).unwrap();
        assert_eq!(active.read_keys, vec![b"key1".to_vec()]);
        assert_eq!(active.write_keys, vec![b"key2".to_vec()]);
    }

    #[test]
    fn test_commit_transaction() {
        let mut mgr = TransactionManager::new();
        let tx_id = mgr
            .begin_transaction(IsolationLevel::SnapshotIsolation)
            .unwrap();

        mgr.record_read(tx_id, b"key1".to_vec()).unwrap();
        let result = mgr.commit(tx_id);
        assert!(result.is_ok());

        assert!(mgr.active_transactions.get(&tx_id).is_none());
    }

    #[test]
    fn test_rollback_transaction() {
        let mut mgr = TransactionManager::new();
        let tx_id = mgr
            .begin_transaction(IsolationLevel::SnapshotIsolation)
            .unwrap();

        mgr.record_read(tx_id, b"key1".to_vec()).unwrap();
        let result = mgr.rollback(tx_id);
        assert!(result.is_ok());

        assert!(mgr.active_transactions.get(&tx_id).is_none());
    }

    #[test]
    fn test_get_snapshot() {
        let mut mgr = TransactionManager::new();
        let tx_id = mgr
            .begin_transaction(IsolationLevel::SnapshotIsolation)
            .unwrap();

        let snapshot = mgr.get_snapshot(tx_id);
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().tx_id, tx_id);
    }

    #[test]
    fn test_get_snapshot_none() {
        let mgr = TransactionManager::new();
        let snapshot = mgr.get_snapshot(TxId::new(999));
        assert!(snapshot.is_none());
    }

    #[test]
    fn test_isolation_level_default() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::SnapshotIsolation);
    }

    #[test]
    fn test_multiple_transactions() {
        let mut mgr = TransactionManager::new();

        let tx1 = mgr
            .begin_transaction(IsolationLevel::SnapshotIsolation)
            .unwrap();
        let tx2 = mgr.begin_transaction(IsolationLevel::Serializable).unwrap();

        assert_eq!(tx1.as_u64(), 1);
        assert_eq!(tx2.as_u64(), 2);

        mgr.record_read(tx1, b"key1".to_vec()).unwrap();
        mgr.record_write(tx2, b"key2".to_vec()).unwrap();

        mgr.commit(tx1).unwrap();
        mgr.commit(tx2).unwrap();
    }
}
