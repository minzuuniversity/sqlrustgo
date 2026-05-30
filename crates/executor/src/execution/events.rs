//! Execution events for telemetry tracking

use serde::{Deserialize, Serialize};

/// Execution event types tracked by telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEventType {
    TxnBegin,
    TxnCommit,
    TxnRollback,
    WalBegin,
    WalCommit,
    StorageMutation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    event_type: ExecutionEventType,
    trace_id: Option<String>,
    txn_id: Option<u64>,
    timestamp: i64,
}

impl ExecutionEvent {
    pub fn event_type(&self) -> &str {
        match &self.event_type {
            ExecutionEventType::TxnBegin => "TxnBegin",
            ExecutionEventType::TxnCommit => "TxnCommit",
            ExecutionEventType::TxnRollback => "TxnRollback",
            ExecutionEventType::WalBegin => "WalBegin",
            ExecutionEventType::WalCommit => "WalCommit",
            ExecutionEventType::StorageMutation => "StorageMutation",
        }
    }

    pub fn txn_id(&self) -> Option<u64> {
        self.txn_id
    }
}

/// DML operation types
#[derive(Debug, Clone)]
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
