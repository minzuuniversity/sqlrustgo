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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_point_all_returns_all_points() {
        let points = CrashPoint::all();
        assert_eq!(points.len(), 11);
    }

    #[test]
    fn test_crash_point_name_contains_no_spaces() {
        for point in CrashPoint::all() {
            let name = point.name();
            assert!(
                !name.contains(' '),
                "name '{}' should not contain spaces",
                name
            );
            assert!(
                name.chars().all(|c| c.is_lowercase() || c == '_'),
                "name '{}' should be lowercase with underscores",
                name
            );
        }
    }

    #[test]
    fn test_crash_point_description_is_non_empty() {
        for point in CrashPoint::all() {
            let desc = point.description();
            assert!(
                !desc.is_empty(),
                "description for {:?} should not be empty",
                point
            );
        }
    }

    #[test]
    fn test_crash_point_display_trait() {
        assert_eq!(format!("{}", CrashPoint::WalWrite), "wal_write");
        assert_eq!(format!("{}", CrashPoint::Checkpoint), "checkpoint");
    }

    #[test]
    fn test_crash_point_eq_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CrashPoint::WalWrite);
        set.insert(CrashPoint::WalWrite); // duplicate
        assert_eq!(set.len(), 1);

        set.insert(CrashPoint::WalFlush);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_crash_point_copy() {
        let point = CrashPoint::TransactionCommit;
        let point2 = point;
        assert_eq!(point, point2);
    }
}
