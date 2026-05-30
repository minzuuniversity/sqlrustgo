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
#[derive(Debug, Clone, Copy, PartialEq)]
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
