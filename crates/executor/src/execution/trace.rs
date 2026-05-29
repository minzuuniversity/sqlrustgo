use sqlrustgo_types::SqlError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmlOperation {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftViolationType {
    WalDrift,
    TxnDrift,
    GraphDrift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSeverity {
    Low,
    Medium,
    Critical,
}

#[derive(Debug, Clone)]
pub struct DriftViolation {
    pub violation_id: String,
    pub trace_id: String,
    pub event_id: Option<String>,
    pub violation_type: DriftViolationType,
    pub severity: DriftSeverity,
    pub description: String,
    pub detected_at: i64,
}

impl DriftViolation {
    pub fn new(
        trace_id: String,
        violation_type: DriftViolationType,
        severity: DriftSeverity,
        description: String,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let violation_id = format!("{}_{}", trace_id, timestamp);
        Self {
            violation_id,
            trace_id,
            event_id: None,
            violation_type,
            severity,
            description,
            detected_at: timestamp,
        }
    }

    pub fn with_event_id(mut self, event_id: String) -> Self {
        self.event_id = Some(event_id);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryType {
    Rollback,
    Patch,
    Rewire,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum RecoveryConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    pub plan_id: String,
    pub trace_id: String,
    pub violation_id: String,
    pub recovery_type: RecoveryType,
    pub confidence: RecoveryConfidence,
    pub steps: Vec<String>,
    pub created_at: i64,
}

impl RecoveryPlan {
    pub fn new(
        trace_id: String,
        violation_id: String,
        recovery_type: RecoveryType,
        confidence: RecoveryConfidence,
        steps: Vec<String>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let plan_id = format!("{}_{}", trace_id, timestamp);
        Self {
            plan_id,
            trace_id,
            violation_id,
            recovery_type,
            confidence,
            steps,
            created_at: timestamp,
        }
    }

    pub fn to_cypher(&self) -> serde_json::Value {
        let recovery_type = match self.recovery_type {
            RecoveryType::Rollback => "ROLLBACK",
            RecoveryType::Patch => "PATCH",
            RecoveryType::Rewire => "REWIRE",
            RecoveryType::Ignore => "IGNORE",
        };
        let confidence = match self.confidence {
            RecoveryConfidence::High => "HIGH",
            RecoveryConfidence::Medium => "MEDIUM",
            RecoveryConfidence::Low => "LOW",
        };
        serde_json::json!({
            "statement": "CREATE (p:RecoveryPlan {plan_id: $id, trace_id: $trace_id, violation_id: $violation_id, type: $type, confidence: $confidence, steps: $steps, created_at: $ts})",
            "parameters": {
                "id": self.plan_id,
                "trace_id": self.trace_id,
                "violation_id": self.violation_id,
                "type": recovery_type,
                "confidence": confidence,
                "steps": self.steps.join("->"),
                "ts": self.created_at,
            }
        })
    }
}

#[derive(Debug, Clone)]
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
    pub fn event_type(&self) -> &'static str {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxnStep {
    Begin,
    WalPrepare,
    StorageMutation,
    WalCommit,
    Commit,
}

#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    steps: Vec<TxnStep>,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn push(&mut self, step: TxnStep) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[TxnStep] {
        &self.steps
    }

    pub fn validate_order(&self) -> Result<(), SqlError> {
        let expected = vec![
            TxnStep::Begin,
            TxnStep::WalPrepare,
            TxnStep::StorageMutation,
            TxnStep::WalCommit,
            TxnStep::Commit,
        ];

        if self.steps != expected {
            return Err(SqlError::ExecutionError(format!(
                "TXN_ORDER_VIOLATION: expected {:?}, got {:?}",
                expected, self.steps
            )));
        }
        Ok(())
    }
}

impl Default for ExecutionTrace {
    fn default() -> Self {
        Self::new()
    }
}