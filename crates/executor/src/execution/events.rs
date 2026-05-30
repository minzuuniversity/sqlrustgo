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
#[derive(Debug, Clone)]
pub enum RecoveryType {
    Crash,
    Rollback,
    Replay,
}

/// Recovery confidence level
#[derive(Debug, Clone, Copy)]
pub enum RecoveryConfidence {
    High,
    Medium,
    Low,
}

/// Recovery plan
#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    pub recovery_type: RecoveryType,
    pub confidence: RecoveryConfidence,
    pub steps: Vec<String>,
}
