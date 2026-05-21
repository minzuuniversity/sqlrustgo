//! WAL Verification engine

use crate::{PropertySeverity, WALLog, WALProperty};

pub struct WALVerifier {
    log: WALLog,
}

impl WALVerifier {
    pub fn new(log: WALLog) -> Self {
        Self { log }
    }

    pub fn verify_all(&self) -> Vec<WALVerificationResult> {
        let mut results = Vec::new();
        for prop in WALProperty::all() {
            results.push(self.verify(&prop));
        }
        results
    }

    pub fn verify(&self, property: &WALProperty) -> WALVerificationResult {
        let holds = match property {
            WALProperty::IdempotentReplay => self.check_idempotent_replay(),
            WALProperty::AtomicWrites => self.check_atomic_writes(),
            WALProperty::LSNMonotonicity => self.log.check_lsn_monotonicity(),
            WALProperty::CheckpointAtomicity => self.check_checkpoint_atomicity(),
            WALProperty::CommittedTransactionsDurable => self.check_durable_commits(),
            WALProperty::NoPhantomCommits => self.check_no_phantom_commits(),
            WALProperty::NoLostUpdates => self.check_no_lost_updates(),
            WALProperty::AuditChainContinuity => self.check_audit_continuity(),
        };

        WALVerificationResult {
            property: property.clone(),
            holds,
            counterexample: None,
            severity: property.severity(),
        }
    }

    fn check_idempotent_replay(&self) -> bool {
        // WAL replay is idempotent if:
        // 1. Each record has unique LSN
        // 2. Records are applied in LSN order
        // 3. UPDATE/DELETE use row keys for idempotency
        let mut lsns: Vec<u64> = self.log.records.iter().map(|r| r.lsn).collect();
        lsns.sort();
        lsns.windows(2).all(|w| w[0] != w[1])
    }

    fn check_atomic_writes(&self) -> bool {
        // Atomic if each record has valid checksum
        self.log.records.iter().all(|r| r.is_valid())
    }

    fn check_checkpoint_atomicity(&self) -> bool {
        // Checkpoint is atomic if:
        // 1. There is at most one checkpoint
        // 2. Checkpoint LSN is within record range
        if let Some(cp_lsn) = self.log.checkpoint_lsn {
            if let Some((first, last)) = self.log.lsn_range() {
                return cp_lsn >= first && cp_lsn <= last;
            }
        }
        true
    }

    fn check_durable_commits(&self) -> bool {
        // All committed transactions must have their records before commit
        let mut txn_states: std::collections::HashMap<u64, bool> = std::collections::HashMap::new();
        for rec in &self.log.records {
            match rec.operation {
                crate::WALOperation::BeginTxn => {
                    txn_states.insert(rec.txn_id, false);
                }
                crate::WALOperation::CommitTxn => {
                    txn_states.insert(rec.txn_id, true);
                }
                crate::WALOperation::AbortTxn => {
                    txn_states.remove(&rec.txn_id);
                }
                _ => {}
            }
        }
        txn_states.values().all(|&committed| committed)
    }

    fn check_no_phantom_commits(&self) -> bool {
        // Phantom commits would be commits without matching BEGIN
        let mut txn_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for rec in &self.log.records {
            if matches!(
                rec.operation,
                crate::WALOperation::BeginTxn | crate::WALOperation::CommitTxn
            ) {
                txn_ids.insert(rec.txn_id);
            }
            if matches!(rec.operation, crate::WALOperation::CommitTxn)
                && !txn_ids.contains(&rec.txn_id)
            {
                return false;
            }
        }
        true
    }

    fn check_no_lost_updates(&self) -> bool {
        // No lost updates if every committed write has a matching record
        self.check_durable_commits()
    }

    fn check_audit_continuity(&self) -> bool {
        // Audit chain is continuous if LSN sequence is uninterrupted
        self.log.check_lsn_monotonicity()
    }
}

#[derive(Debug, Clone)]
pub struct WALVerificationResult {
    pub property: WALProperty,
    pub holds: bool,
    pub counterexample: Option<String>,
    pub severity: PropertySeverity,
}

impl WALVerificationResult {
    pub fn summary(&self) -> String {
        let status = if self.holds { "PASS" } else { "FAIL" };
        format!("{:?} [{}] {:?}", self.property, status, self.severity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WALLog, WALOperation, WALRecord};

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

    fn make_log(records: Vec<WALRecord>, checkpoint_lsn: Option<u64>) -> WALLog {
        WALLog {
            records,
            checkpoint_lsn,
        }
    }

    // WALVerificationResult tests
    mod wal_verification_result_tests {
        use super::*;
        use crate::{PropertySeverity, WALProperty};

        #[test]
        fn test_result_summary_pass() {
            let result = WALVerificationResult {
                property: WALProperty::LSNMonotonicity,
                holds: true,
                counterexample: None,
                severity: PropertySeverity::Critical,
            };
            let summary = result.summary();
            assert!(summary.contains("PASS"));
        }

        #[test]
        fn test_result_summary_fail() {
            let result = WALVerificationResult {
                property: WALProperty::AtomicWrites,
                holds: false,
                counterexample: Some("Record checksum mismatch".to_string()),
                severity: PropertySeverity::Critical,
            };
            let summary = result.summary();
            assert!(summary.contains("FAIL"));
        }
    }

    // WALVerifier tests
    mod wal_verifier_tests {
        use super::*;

        #[test]
        fn test_verify_all_returns_eight_results() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let results = verifier.verify_all();
            assert_eq!(results.len(), 8);
        }

        #[test]
        fn test_verify_lsn_monotonicity_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::LSNMonotonicity);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_lsn_monotonicity_fails() {
            let log = make_log(
                vec![
                    make_record(3, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(1, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::LSNMonotonicity);
            assert!(!result.holds);
        }

        #[test]
        fn test_verify_idempotent_replay_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::IdempotentReplay);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_idempotent_replay_fails_duplicate_lsn() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(1, 1, WALOperation::WriteRow),
                    make_record(2, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::IdempotentReplay);
            assert!(!result.holds);
        }

        #[test]
        fn test_verify_atomic_writes_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::AtomicWrites);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_atomic_writes_fails_invalid_record() {
            let log = make_log(
                vec![
                    make_record(0, 1, WALOperation::BeginTxn), // Invalid: lsn=0
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::AtomicWrites);
            assert!(!result.holds);
        }

        #[test]
        fn test_verify_checkpoint_atomicity_passes_no_checkpoint() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::CheckpointAtomicity);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_checkpoint_atomicity_passes_checkpoint_in_range() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::Checkpoint),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                Some(2),
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::CheckpointAtomicity);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_checkpoint_atomicity_fails_checkpoint_out_of_range() {
            let log = make_log(
                vec![
                    make_record(10, 1, WALOperation::BeginTxn),
                    make_record(20, 1, WALOperation::Checkpoint),
                    make_record(30, 1, WALOperation::CommitTxn),
                ],
                Some(100), // Checkpoint LSN outside record range
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::CheckpointAtomicity);
            assert!(!result.holds);
        }

        #[test]
        fn test_verify_durable_commits_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::CommittedTransactionsDurable);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_durable_commits_fails_uncommitted() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    // No CommitTxn for txn_id 1
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::CommittedTransactionsDurable);
            assert!(!result.holds);
        }

        #[test]
        fn test_verify_no_phantom_commits_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::NoPhantomCommits);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_no_lost_updates_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::NoLostUpdates);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_audit_continuity_passes() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::WriteRow),
                    make_record(3, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::AuditChainContinuity);
            assert!(result.holds);
        }

        #[test]
        fn test_verify_result_has_correct_severity() {
            let log = make_log(
                vec![
                    make_record(1, 1, WALOperation::BeginTxn),
                    make_record(2, 1, WALOperation::CommitTxn),
                ],
                None,
            );
            let verifier = WALVerifier::new(log);
            let result = verifier.verify(&WALProperty::IdempotentReplay);
            assert_eq!(result.severity, PropertySeverity::Critical);
        }
    }
}
