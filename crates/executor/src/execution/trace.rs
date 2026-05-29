use sqlrustgo_types::SqlError;

/// DML operation types that share the same VTU execution model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmlOperation {
    Insert,
    Update,
    Delete,
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