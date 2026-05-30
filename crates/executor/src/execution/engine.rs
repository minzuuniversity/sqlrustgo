use super::context::QueryContext;
use super::result::ExecutionResult;
use sqlrustgo_types::SqlError;

pub trait ExecutionEngine {
    fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError>;

    fn begin(&mut self) -> Result<u64, SqlError>;
    fn commit(&mut self, txn: u64) -> Result<(), SqlError>;
    fn rollback(&mut self, txn: u64) -> Result<(), SqlError>;
}