use sqlrustgo_types::SqlError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmlOperation {
    Insert,
    Update,
    Delete,
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