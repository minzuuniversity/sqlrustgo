//! CrashPoint defines where in the storage pipeline crashes can be injected

use serde::{Deserialize, Serialize};

/// Represents a point in the storage/engine pipeline where a crash can be injected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrashPoint {
    /// Just before writing to WAL
    WalWrite,
    /// During WAL flush/sync
    WalFlush,
    /// During transaction commit
    TransactionCommit,
    /// During page split
    PageSplit,
    /// During checkpoint creation
    Checkpoint,
    /// During buffer pool eviction
    BufferEvict,
    /// During index update
    IndexUpdate,
    /// During lock acquisition
    LockAcquire,
    /// During audit log write
    AuditWrite,
    /// During signature computation
    SignatureCompute,
    /// After commit but before returning to client
    PostCommit,
}

impl CrashPoint {
    /// Get all crash points as a list
    pub fn all() -> &'static [CrashPoint] {
        &[
            CrashPoint::WalWrite,
            CrashPoint::WalFlush,
            CrashPoint::TransactionCommit,
            CrashPoint::PageSplit,
            CrashPoint::Checkpoint,
            CrashPoint::BufferEvict,
            CrashPoint::IndexUpdate,
            CrashPoint::LockAcquire,
            CrashPoint::AuditWrite,
            CrashPoint::SignatureCompute,
            CrashPoint::PostCommit,
        ]
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::WalWrite => "wal_write",
            Self::WalFlush => "wal_flush",
            Self::TransactionCommit => "transaction_commit",
            Self::PageSplit => "page_split",
            Self::Checkpoint => "checkpoint",
            Self::BufferEvict => "buffer_evict",
            Self::IndexUpdate => "index_update",
            Self::LockAcquire => "lock_acquire",
            Self::AuditWrite => "audit_write",
            Self::SignatureCompute => "signature_compute",
            Self::PostCommit => "post_commit",
        }
    }

    /// Description of what this crash point represents
    pub fn description(&self) -> &'static str {
        match self {
            Self::WalWrite => "WAL write operation (before fsync)",
            Self::WalFlush => "WAL flush/sync operation",
            Self::TransactionCommit => "Transaction commit acknowledgment",
            Self::PageSplit => "B-tree page split operation",
            Self::Checkpoint => "Checkpoint creation",
            Self::BufferEvict => "Buffer pool page eviction",
            Self::IndexUpdate => "Index update operation",
            Self::LockAcquire => "Lock acquisition",
            Self::AuditWrite => "Audit log write",
            Self::SignatureCompute => "Cryptographic signature computation",
            Self::PostCommit => "After commit, before client response",
        }
    }
}

impl std::fmt::Display for CrashPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
