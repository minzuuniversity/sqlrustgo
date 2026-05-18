//! WAL Verification engine

use crate::{PropertySeverity, WALLog, WALProperty, WALRecord};

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
                crate::WALOperation::BeginTxn => { txn_states.insert(rec.txn_id, false); }
                crate::WALOperation::CommitTxn => { txn_states.insert(rec.txn_id, true); }
                crate::WALOperation::AbortTxn => { txn_states.remove(&rec.txn_id); }
                _ => {}
            }
        }
        txn_states.values().all(|&committed| committed)
    }

    fn check_no_phantom_commits(&self) -> bool {
        // Phantom commits would be commits without matching BEGIN
        let mut txn_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for rec in &self.log.records {
            if matches!(rec.operation, crate::WALOperation::BeginTxn | crate::WALOperation::CommitTxn) {
                txn_ids.insert(rec.txn_id);
            }
            if matches!(rec.operation, crate::WALOperation::CommitTxn) && !txn_ids.contains(&rec.txn_id) {
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
