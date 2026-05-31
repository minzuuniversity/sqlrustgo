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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context_active() {
        let ctx = TransactionContext::new(1);
        assert_eq!(ctx.tx_id, 1);
        assert!(!ctx.wal_segment_open);
        assert!(ctx.is_active);
    }

    #[test]
    fn test_mark_wal_open() {
        let mut ctx = TransactionContext::new(42);
        ctx.mark_wal_open();
        assert!(ctx.wal_segment_open);
        assert!(ctx.is_active);
    }

    #[test]
    fn test_mark_committed() {
        let mut ctx = TransactionContext::new(1);
        ctx.mark_wal_open();
        ctx.mark_committed();
        assert!(!ctx.is_active);
        assert!(ctx.wal_segment_open);
    }

    #[test]
    fn test_mark_rolled_back() {
        let mut ctx = TransactionContext::new(1);
        ctx.mark_wal_open();
        ctx.mark_rolled_back();
        assert!(!ctx.is_active);
        // wal_segment_open remains true; DriftGate validation catches inactive txns
        assert!(ctx.wal_segment_open);
    }

    #[test]
    fn test_multiple_contexts() {
        let ctx1 = TransactionContext::new(1);
        let ctx2 = TransactionContext::new(2);
        assert_ne!(ctx1.tx_id, ctx2.tx_id);
        assert!(ctx1.is_active);
        assert!(ctx2.is_active);
    }
}
