//! Transaction context for WAL state tracking
//! Tracks the active transaction ID and WAL segment state

use serde::{Deserialize, Serialize};

/// Transaction context — carried through the execution path
/// Used by DriftGate to validate WAL ordering invariants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContext {
    /// Active transaction ID
    pub tx_id: u64,
    /// Whether a WAL segment is open for this transaction
    pub wal_segment_open: bool,
    /// Whether the transaction is active (not committed/aborted)
    pub is_active: bool,
}

impl TransactionContext {
    pub fn new(tx_id: u64) -> Self {
        Self {
            tx_id,
            wal_segment_open: false,
            is_active: true,
        }
    }

    /// Mark WAL segment as opened (after WAL Begin is logged)
    pub fn mark_wal_open(&mut self) {
        self.wal_segment_open = true;
    }

    /// Mark transaction as committed
    pub fn mark_committed(&mut self) {
        self.is_active = false;
    }

    /// Mark transaction as rolled back
    pub fn mark_rolled_back(&mut self) {
        self.is_active = false;
    }
}
