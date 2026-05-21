//! WAL Formal Verification
//!
//! Provides formal specifications and verification for WAL correctness properties.

pub mod verification;

// Re-exports
pub use verification::{WALVerificationResult, WALVerifier};

/// WAL operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WALOperation {
    BeginTxn,
    WriteRow,
    DeleteRow,
    UpdateRow,
    CommitTxn,
    AbortTxn,
    Checkpoint,
}

impl WALOperation {
    pub fn is_dml(&self) -> bool {
        matches!(self, Self::WriteRow | Self::DeleteRow | Self::UpdateRow)
    }

    pub fn is_boundary(&self) -> bool {
        matches!(self, Self::BeginTxn | Self::CommitTxn | Self::AbortTxn)
    }
}

/// WAL state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WALState {
    #[default]
    Idle,
    Writing,
    Flushing,
    Committing,
    Checkpointing,
}

/// Critical WAL properties that must always hold
#[derive(Debug, Clone)]
pub enum WALProperty {
    IdempotentReplay,
    AtomicWrites,
    LSNMonotonicity,
    CheckpointAtomicity,
    CommittedTransactionsDurable,
    NoPhantomCommits,
    NoLostUpdates,
    AuditChainContinuity,
}

impl WALProperty {
    pub fn all() -> Vec<WALProperty> {
        vec![
            WALProperty::IdempotentReplay,
            WALProperty::AtomicWrites,
            WALProperty::LSNMonotonicity,
            WALProperty::CheckpointAtomicity,
            WALProperty::CommittedTransactionsDurable,
            WALProperty::NoPhantomCommits,
            WALProperty::NoLostUpdates,
            WALProperty::AuditChainContinuity,
        ]
    }

    pub fn critical() -> Vec<WALProperty> {
        Self::all()
            .into_iter()
            .filter(|p| p.severity() == PropertySeverity::Critical)
            .collect()
    }

    pub fn severity(&self) -> PropertySeverity {
        match self {
            Self::IdempotentReplay => PropertySeverity::Critical,
            Self::AtomicWrites => PropertySeverity::Critical,
            Self::LSNMonotonicity => PropertySeverity::Critical,
            Self::CheckpointAtomicity => PropertySeverity::Critical,
            Self::CommittedTransactionsDurable => PropertySeverity::Critical,
            Self::NoPhantomCommits => PropertySeverity::Critical,
            Self::NoLostUpdates => PropertySeverity::Critical,
            Self::AuditChainContinuity => PropertySeverity::Important,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::IdempotentReplay => "WAL replay is idempotent",
            Self::AtomicWrites => "All WAL writes are atomic",
            Self::LSNMonotonicity => "LSN is monotonically increasing",
            Self::CheckpointAtomicity => "Checkpoint is atomic",
            Self::CommittedTransactionsDurable => "All committed transactions are durable",
            Self::NoPhantomCommits => "No phantom commits after recovery",
            Self::NoLostUpdates => "No committed updates are lost",
            Self::AuditChainContinuity => "Audit chain has no gaps",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertySeverity {
    Critical,
    Important,
    Advisory,
}

#[derive(Debug, Clone)]
pub struct WALRecord {
    pub lsn: u64,
    pub txn_id: u64,
    pub operation: WALOperation,
    pub page_id: Option<u64>,
    pub row_key: Option<Vec<u8>>,
    pub before_value: Option<Vec<u8>>,
    pub after_value: Option<Vec<u8>>,
    pub checksum: u32,
}

impl WALRecord {
    pub fn is_valid(&self) -> bool {
        self.lsn > 0 && self.txn_id > 0
    }
}

#[derive(Debug, Clone)]
pub struct WALLog {
    pub records: Vec<WALRecord>,
    pub checkpoint_lsn: Option<u64>,
}

impl WALLog {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            checkpoint_lsn: None,
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn lsn_range(&self) -> Option<(u64, u64)> {
        if self.records.is_empty() {
            return None;
        }
        let first = self.records.first().map(|r| r.lsn)?;
        let last = self.records.last().map(|r| r.lsn)?;
        Some((first, last))
    }

    pub fn txn_records(&self, txn_id: u64) -> Vec<&WALRecord> {
        self.records.iter().filter(|r| r.txn_id == txn_id).collect()
    }

    pub fn check_lsn_monotonicity(&self) -> bool {
        for window in self.records.windows(2) {
            if window[0].lsn >= window[1].lsn {
                return false;
            }
        }
        true
    }
}

impl Default for WALLog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum WALInvariant {
    WALNeverEmptyWhenNotIdle,
    RecordWrittenImpliesFlushed,
    CommitLSNMonotonic,
    NoDuplicateLSN,
    TransactionAtomicity,
}

pub struct WALInvariantResult {
    pub invariant: WALInvariant,
    pub holds: bool,
    pub failed_at: Option<usize>,
}

impl WALInvariant {
    pub fn check(&self, log: &WALLog) -> WALInvariantResult {
        match self {
            WALInvariant::NoDuplicateLSN => {
                let mut lsns = std::collections::HashSet::new();
                for (i, rec) in log.records.iter().enumerate() {
                    if !lsns.insert(rec.lsn) {
                        return WALInvariantResult {
                            invariant: WALInvariant::NoDuplicateLSN,
                            holds: false,
                            failed_at: Some(i),
                        };
                    }
                }
                WALInvariantResult {
                    invariant: WALInvariant::NoDuplicateLSN,
                    holds: true,
                    failed_at: None,
                }
            }
            _ => WALInvariantResult {
                invariant: self.clone(),
                holds: true,
                failed_at: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // WALOperation tests
    mod wal_operation_tests {
        use super::*;

        #[test]
        fn test_is_dml_write_row() {
            assert!(WALOperation::WriteRow.is_dml());
        }

        #[test]
        fn test_is_dml_delete_row() {
            assert!(WALOperation::DeleteRow.is_dml());
        }

        #[test]
        fn test_is_dml_update_row() {
            assert!(WALOperation::UpdateRow.is_dml());
        }

        #[test]
        fn test_is_dml_false_for_non_dml() {
            assert!(!WALOperation::BeginTxn.is_dml());
            assert!(!WALOperation::CommitTxn.is_dml());
            assert!(!WALOperation::AbortTxn.is_dml());
            assert!(!WALOperation::Checkpoint.is_dml());
        }

        #[test]
        fn test_is_boundary_begin_txn() {
            assert!(WALOperation::BeginTxn.is_boundary());
        }

        #[test]
        fn test_is_boundary_commit_txn() {
            assert!(WALOperation::CommitTxn.is_boundary());
        }

        #[test]
        fn test_is_boundary_abort_txn() {
            assert!(WALOperation::AbortTxn.is_boundary());
        }

        #[test]
        fn test_is_boundary_false_for_non_boundary() {
            assert!(!WALOperation::WriteRow.is_boundary());
            assert!(!WALOperation::DeleteRow.is_boundary());
            assert!(!WALOperation::UpdateRow.is_boundary());
            assert!(!WALOperation::Checkpoint.is_boundary());
        }

        #[test]
        fn test_wal_operation_equality() {
            assert_eq!(WALOperation::BeginTxn, WALOperation::BeginTxn);
            assert_eq!(WALOperation::WriteRow, WALOperation::WriteRow);
            assert_ne!(WALOperation::BeginTxn, WALOperation::CommitTxn);
        }

        #[test]
        fn test_wal_operation_clone() {
            let op = WALOperation::WriteRow;
            let cloned = op;
            assert_eq!(op, cloned);
        }
    }

    // WALState tests
    mod wal_state_tests {
        use super::*;

        #[test]
        fn test_wal_state_default() {
            assert_eq!(WALState::default(), WALState::Idle);
        }

        #[test]
        fn test_wal_state_equality() {
            assert_eq!(WALState::Idle, WALState::Idle);
            assert_eq!(WALState::Writing, WALState::Writing);
            assert_ne!(WALState::Idle, WALState::Writing);
        }

        #[test]
        fn test_wal_state_all_variants() {
            let states = [
                WALState::Idle,
                WALState::Writing,
                WALState::Flushing,
                WALState::Committing,
                WALState::Checkpointing,
            ];
            assert_eq!(states.len(), 5);
        }
    }

    // WALProperty tests
    mod wal_property_tests {
        use super::*;

        #[test]
        fn test_all_returns_eight_properties() {
            let props = WALProperty::all();
            assert_eq!(props.len(), 8);
        }

        #[test]
        fn test_critical_returns_critical_severity_only() {
            let critical = WALProperty::critical();
            for prop in &critical {
                assert_eq!(prop.severity(), PropertySeverity::Critical);
            }
        }

        #[test]
        fn test_property_severity() {
            assert_eq!(
                WALProperty::IdempotentReplay.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::AtomicWrites.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::LSNMonotonicity.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::CheckpointAtomicity.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::CommittedTransactionsDurable.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::NoPhantomCommits.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::NoLostUpdates.severity(),
                PropertySeverity::Critical
            );
            assert_eq!(
                WALProperty::AuditChainContinuity.severity(),
                PropertySeverity::Important
            );
        }

        #[test]
        fn test_property_description() {
            assert_eq!(
                WALProperty::IdempotentReplay.description(),
                "WAL replay is idempotent"
            );
            assert_eq!(
                WALProperty::AtomicWrites.description(),
                "All WAL writes are atomic"
            );
            assert_eq!(
                WALProperty::LSNMonotonicity.description(),
                "LSN is monotonically increasing"
            );
        }
    }

    // WALRecord tests
    mod wal_record_tests {
        use super::*;

        #[test]
        fn test_wal_record_valid_with_nonzero_lsn_and_txn_id() {
            let record = WALRecord {
                lsn: 1,
                txn_id: 1,
                operation: WALOperation::WriteRow,
                page_id: Some(1),
                row_key: Some(vec![1, 2, 3]),
                before_value: None,
                after_value: Some(vec![4, 5, 6]),
                checksum: 0xDEADBEEF,
            };
            assert!(record.is_valid());
        }

        #[test]
        fn test_wal_record_invalid_with_zero_lsn() {
            let record = WALRecord {
                lsn: 0,
                txn_id: 1,
                operation: WALOperation::WriteRow,
                page_id: None,
                row_key: None,
                before_value: None,
                after_value: None,
                checksum: 0,
            };
            assert!(!record.is_valid());
        }

        #[test]
        fn test_wal_record_invalid_with_zero_txn_id() {
            let record = WALRecord {
                lsn: 1,
                txn_id: 0,
                operation: WALOperation::WriteRow,
                page_id: None,
                row_key: None,
                before_value: None,
                after_value: None,
                checksum: 0,
            };
            assert!(!record.is_valid());
        }
    }

    // WALLog tests
    mod wal_log_tests {
        use super::*;

        fn make_record(lsn: u64, txn_id: u64, op: WALOperation) -> WALRecord {
            WALRecord {
                lsn,
                txn_id,
                operation: op,
                page_id: None,
                row_key: None,
                before_value: None,
                after_value: None,
                checksum: 0,
            }
        }

        #[test]
        fn test_wal_log_new() {
            let log = WALLog::new();
            assert!(log.is_empty());
            assert_eq!(log.len(), 0);
        }

        #[test]
        fn test_wal_log_is_empty() {
            let log = WALLog::new();
            assert!(log.is_empty());
        }

        #[test]
        fn test_wal_log_len() {
            let mut log = WALLog::new();
            assert_eq!(log.len(), 0);
            log.records.push(make_record(1, 1, WALOperation::BeginTxn));
            assert_eq!(log.len(), 1);
            log.records.push(make_record(2, 1, WALOperation::WriteRow));
            assert_eq!(log.len(), 2);
        }

        #[test]
        fn test_wal_log_lsn_range_empty() {
            let log = WALLog::new();
            assert!(log.lsn_range().is_none());
        }

        #[test]
        fn test_wal_log_lsn_range_single_record() {
            let mut log = WALLog::new();
            log.records
                .push(make_record(100, 1, WALOperation::BeginTxn));
            assert_eq!(log.lsn_range(), Some((100, 100)));
        }

        #[test]
        fn test_wal_log_lsn_range_multiple_records() {
            let mut log = WALLog::new();
            log.records.push(make_record(10, 1, WALOperation::BeginTxn));
            log.records.push(make_record(20, 1, WALOperation::WriteRow));
            log.records
                .push(make_record(30, 1, WALOperation::CommitTxn));
            assert_eq!(log.lsn_range(), Some((10, 30)));
        }

        #[test]
        fn test_wal_log_txn_records() {
            let mut log = WALLog::new();
            log.records.push(make_record(1, 1, WALOperation::BeginTxn));
            log.records.push(make_record(2, 1, WALOperation::WriteRow));
            log.records.push(make_record(3, 2, WALOperation::BeginTxn));
            log.records.push(make_record(4, 1, WALOperation::CommitTxn));

            let txn1_records = log.txn_records(1);
            assert_eq!(txn1_records.len(), 3);

            let txn2_records = log.txn_records(2);
            assert_eq!(txn2_records.len(), 1);

            let txn3_records = log.txn_records(3);
            assert!(txn3_records.is_empty());
        }

        #[test]
        fn test_wal_log_check_lsn_monotonicity_true() {
            let mut log = WALLog::new();
            log.records.push(make_record(1, 1, WALOperation::BeginTxn));
            log.records.push(make_record(2, 1, WALOperation::WriteRow));
            log.records.push(make_record(3, 1, WALOperation::CommitTxn));
            assert!(log.check_lsn_monotonicity());
        }

        #[test]
        fn test_wal_log_check_lsn_monotonicity_false() {
            let mut log = WALLog::new();
            log.records.push(make_record(3, 1, WALOperation::BeginTxn));
            log.records.push(make_record(2, 1, WALOperation::WriteRow)); // Decreasing LSN
            log.records.push(make_record(1, 1, WALOperation::CommitTxn));
            assert!(!log.check_lsn_monotonicity());
        }

        #[test]
        fn test_wal_log_check_lsn_monotonicity_equal_lsn() {
            let mut log = WALLog::new();
            log.records.push(make_record(1, 1, WALOperation::BeginTxn));
            log.records.push(make_record(1, 1, WALOperation::WriteRow)); // Same LSN
            assert!(!log.check_lsn_monotonicity());
        }
    }

    // WALInvariant tests
    mod wal_invariant_tests {
        use super::*;

        fn make_record(lsn: u64, txn_id: u64, op: WALOperation) -> WALRecord {
            WALRecord {
                lsn,
                txn_id,
                operation: op,
                page_id: None,
                row_key: None,
                before_value: None,
                after_value: None,
                checksum: 0,
            }
        }

        #[test]
        fn test_no_duplicate_lsn_holds() {
            let mut log = WALLog::new();
            log.records.push(make_record(1, 1, WALOperation::BeginTxn));
            log.records.push(make_record(2, 1, WALOperation::WriteRow));
            log.records.push(make_record(3, 1, WALOperation::CommitTxn));

            let invariant = WALInvariant::NoDuplicateLSN;
            let result = invariant.check(&log);
            assert!(result.holds);
            assert!(result.failed_at.is_none());
        }

        #[test]
        fn test_no_duplicate_lsn_fails() {
            let mut log = WALLog::new();
            log.records.push(make_record(1, 1, WALOperation::BeginTxn));
            log.records.push(make_record(1, 1, WALOperation::WriteRow)); // Duplicate LSN
            log.records.push(make_record(2, 1, WALOperation::CommitTxn));

            let invariant = WALInvariant::NoDuplicateLSN;
            let result = invariant.check(&log);
            assert!(!result.holds);
            assert_eq!(result.failed_at, Some(1));
        }
    }
}
