//! Execution events for telemetry tracking

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEvent {
    SqlReceived { sql: String },
    TxnBegin { txn_id: u64 },
    TxnCommit { txn_id: u64 },
    TxnRollback { txn_id: u64 },
    WalBegin { txn_id: u64 },
    WalWrite { txn_id: u64, segment: String },
    WalCommit { txn_id: u64 },
    StorageRead { table: String, rows: usize },
    StorageWrite { table: String, rows: usize },
    StorageMutation { table: String, op: DmlOperation },
    BoundaryCheck { module: String, passed: bool },
    VtuValidate { result: bool },
}

impl ExecutionEvent {
    pub fn event_type(&self) -> &str {
        match self {
            ExecutionEvent::SqlReceived { .. } => "SqlReceived",
            ExecutionEvent::TxnBegin { .. } => "TxnBegin",
            ExecutionEvent::TxnCommit { .. } => "TxnCommit",
            ExecutionEvent::TxnRollback { .. } => "TxnRollback",
            ExecutionEvent::WalBegin { .. } => "WalBegin",
            ExecutionEvent::WalWrite { .. } => "WalWrite",
            ExecutionEvent::WalCommit { .. } => "WalCommit",
            ExecutionEvent::StorageRead { .. } => "StorageRead",
            ExecutionEvent::StorageWrite { .. } => "StorageWrite",
            ExecutionEvent::StorageMutation { .. } => "StorageMutation",
            ExecutionEvent::BoundaryCheck { .. } => "BoundaryCheck",
            ExecutionEvent::VtuValidate { .. } => "VtuValidate",
        }
    }

    pub fn txn_id(&self) -> Option<u64> {
        match self {
            ExecutionEvent::TxnBegin { txn_id } => Some(*txn_id),
            ExecutionEvent::TxnCommit { txn_id } => Some(*txn_id),
            ExecutionEvent::TxnRollback { txn_id } => Some(*txn_id),
            ExecutionEvent::WalBegin { txn_id } => Some(*txn_id),
            ExecutionEvent::WalWrite { txn_id, .. } => Some(*txn_id),
            ExecutionEvent::WalCommit { txn_id } => Some(*txn_id),
            _ => None,
        }
    }
}

/// DML operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DmlOperation {
    Insert,
    Update,
    Delete,
}

/// Recovery types
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryType {
    Crash,
    Rollback,
    Replay,
    Patch,
    Ignore,
    Rewire,
}

/// Recovery confidence level
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum RecoveryConfidence {
    High,
    Medium,
    Low,
}

impl RecoveryConfidence {
    /// Greater-than-or-equal comparison
    pub fn ge(&self, other: &RecoveryConfidence) -> bool {
        *self as u8 >= *other as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_names() {
        assert_eq!(
            ExecutionEvent::SqlReceived { sql: "".into() }.event_type(),
            "SqlReceived"
        );
        assert_eq!(
            ExecutionEvent::TxnBegin { txn_id: 1 }.event_type(),
            "TxnBegin"
        );
        assert_eq!(
            ExecutionEvent::TxnCommit { txn_id: 1 }.event_type(),
            "TxnCommit"
        );
        assert_eq!(
            ExecutionEvent::WalBegin { txn_id: 1 }.event_type(),
            "WalBegin"
        );
        assert_eq!(
            ExecutionEvent::StorageRead {
                table: "t".into(),
                rows: 0
            }
            .event_type(),
            "StorageRead"
        );
        assert_eq!(
            ExecutionEvent::VtuValidate { result: true }.event_type(),
            "VtuValidate"
        );
    }

    #[test]
    fn test_txn_id_extraction() {
        let e = ExecutionEvent::TxnBegin { txn_id: 42 };
        assert_eq!(e.txn_id(), Some(42));

        let e = ExecutionEvent::StorageRead {
            table: "t".into(),
            rows: 5,
        };
        assert_eq!(e.txn_id(), None);
    }

    #[test]
    fn test_all_variants_cover_event_types() {
        let variants = vec![
            ExecutionEvent::SqlReceived {
                sql: "SELECT 1".into(),
            },
            ExecutionEvent::TxnBegin { txn_id: 1 },
            ExecutionEvent::TxnCommit { txn_id: 1 },
            ExecutionEvent::TxnRollback { txn_id: 1 },
            ExecutionEvent::WalBegin { txn_id: 1 },
            ExecutionEvent::WalWrite {
                txn_id: 1,
                segment: "seg_1".into(),
            },
            ExecutionEvent::WalCommit { txn_id: 1 },
            ExecutionEvent::StorageRead {
                table: "t".into(),
                rows: 10,
            },
            ExecutionEvent::StorageWrite {
                table: "t".into(),
                rows: 5,
            },
            ExecutionEvent::StorageMutation {
                table: "t".into(),
                op: DmlOperation::Insert,
            },
            ExecutionEvent::BoundaryCheck {
                module: "wal".into(),
                passed: true,
            },
            ExecutionEvent::VtuValidate { result: false },
        ];
        assert_eq!(variants.len(), 12);
        for v in &variants {
            let tn = v.event_type();
            assert!(!tn.is_empty(), "All event types must have names");
        }
    }

    #[test]
    fn test_recovery_plan_default() {
        let plan = RecoveryPlan::new(
            "trace-1".into(),
            "viol-1".into(),
            RecoveryType::Crash,
            RecoveryConfidence::High,
            vec!["step1".into()],
        );
        assert!(plan.plan_id.starts_with("RP-"));
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn test_recovery_confidence_ge() {
        // Low(2) >= Medium(1) = true, Medium(1) >= High(0) = true
        assert!(RecoveryConfidence::Low.ge(&RecoveryConfidence::High));
        assert!(RecoveryConfidence::Medium.ge(&RecoveryConfidence::High));
        assert!(!RecoveryConfidence::High.ge(&RecoveryConfidence::Low));
    }
}

/// Recovery plan
#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    pub plan_id: String,
    pub trace_id: String,
    pub violation_id: String,
    pub recovery_type: RecoveryType,
    pub confidence: RecoveryConfidence,
    pub steps: Vec<String>,
}

impl RecoveryPlan {
    pub fn new(
        trace_id: String,
        violation_id: String,
        recovery_type: RecoveryType,
        confidence: RecoveryConfidence,
        steps: Vec<String>,
    ) -> Self {
        Self {
            plan_id: format!("RP-{}-{}", violation_id, trace_id),
            trace_id,
            violation_id,
            recovery_type,
            confidence,
            steps,
        }
    }
}
