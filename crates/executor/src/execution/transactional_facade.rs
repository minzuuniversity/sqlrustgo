use crate::execution::{DriftViolation, TransactionContext, WriteOp};
use crate::sql_executor::ExecutionResult;
use sqlrustgo_types::{SqlError, SqlResult};

pub trait TransactionalFacade: Send + Sync {
    fn begin(&self) -> SqlResult<u64>;
    fn commit(&self) -> SqlResult<Option<u64>>;
    fn rollback(&self) -> SqlResult<()>;
    fn is_in_transaction(&self) -> bool;
    fn current_tx_id(&self) -> Option<u64>;
    fn execute_write(&self, ctx: &TransactionContext, op: WriteOp) -> SqlResult<ExecutionResult>;
    fn execute_read(&self, sql: &str) -> SqlResult<ExecutionResult>;
    fn validate_operation(&self, op: &WriteOp, ctx: &TransactionContext) -> Result<(), DriftViolation>;
}