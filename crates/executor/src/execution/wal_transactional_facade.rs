use parking_lot::RwLock;
use sqlrustgo_storage::{StorageEngine, WalStorage};
use sqlrustgo_transaction::{TransactionError, TransactionManager, TxId};
use sqlrustgo_types::SqlError;
use std::path::PathBuf;
use std::sync::Arc;

use super::drift_gate::DriftGate;
use super::transaction_context::TransactionContext;
use super::transactional_facade::TransactionalFacade;
use super::write_op::WriteOp;
use crate::execution::result::ExecutionResult;

pub struct WalTransactionalFacade<S: StorageEngine> {
    storage: Arc<RwLock<WalStorage<S>>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
    drift_gate: DriftGate,
}

impl<S: StorageEngine> WalTransactionalFacade<S> {
    pub fn new(inner: S, wal_path: PathBuf) -> Result<Self, SqlError> {
        let wal_storage = WalStorage::new(inner, wal_path)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(Self {
            storage: Arc::new(RwLock::new(wal_storage)),
            tx_manager: Arc::new(RwLock::new(TransactionManager::new())),
            drift_gate: DriftGate::new("wal-facade".to_string()),
        })
    }

    pub fn new_without_wal(inner: S) -> Self {
        let wal_storage = WalStorage::new_without_wal(inner);
        Self {
            storage: Arc::new(RwLock::new(wal_storage)),
            tx_manager: Arc::new(RwLock::new(TransactionManager::new())),
            drift_gate: DriftGate::new("wal-facade".to_string()),
        }
    }

    pub fn storage(&self) -> Arc<RwLock<WalStorage<S>>> {
        self.storage.clone()
    }

    fn get_current_ctx(&self) -> SqlResult<TransactionContext> {
        let mgr = self.tx_manager.read();
        let ctx = mgr.get_transaction_context()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(TransactionContext::new(ctx.tx_id.raw()))
    }
}

impl<S: StorageEngine> TransactionalFacade for WalTransactionalFacade<S> {
    fn begin(&self) -> SqlResult<u64> {
        let mut storage = self.storage.write();
        storage.begin_transaction()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        let tx_id = self.tx_manager.write().begin()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        storage.log_begin(tx_id.raw())
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(tx_id.raw())
    }

    fn commit(&self) -> SqlResult<Option<u64>> {
        let ctx = self.get_current_ctx()?;
        if let Err(violation) = self.drift_gate.validate_pre_commit(&ctx) {
            return Err(SqlError::ExecutionError(format!(
                "Drift violation blocked commit: {}",
                violation.description
            )));
        }
        let mut storage = self.storage.write();
        storage.commit_transaction()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        storage.log_commit(ctx.tx_id)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        let ts = self.tx_manager.write().commit()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(ts)
    }

    fn rollback(&self) -> SqlResult<()> {
        let mut storage = self.storage.write();
        storage.rollback_transaction()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        self.tx_manager.write().rollback()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))
    }

    fn is_in_transaction(&self) -> bool {
        self.tx_manager.read().is_in_transaction()
    }

    fn current_tx_id(&self) -> Option<u64> {
        self.tx_manager.read().get_current_tx_id().map(|t| t.raw())
    }

    fn execute_write(&self, ctx: &TransactionContext, op: WriteOp) -> SqlResult<ExecutionResult> {
        if let Err(violation) = self.drift_gate.validate(op, ctx) {
            return Err(SqlError::ExecutionError(format!(
                "Drift violation: {}",
                violation.description
            )));
        }
        let mut storage = self.storage.write();
        let affected = match &op {
            WriteOp::Insert { table, columns, values } => {
                storage.insert(table, columns, values)
            }
            WriteOp::Update { table, set, filter } => {
                storage.update(table, set, filter)
            }
            WriteOp::Delete { table, filter } => {
                storage.delete(table, filter)
            }
        }.map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        storage.log_mutation(ctx.tx_id, &op)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(ExecutionResult::new(vec![], affected))
    }

    fn execute_read(&self, sql: &str) -> SqlResult<ExecutionResult> {
        let storage = self.storage.read();
        let plan = sqlrustgo_planner::create_physical_plan(sql)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        let executor = crate::LocalExecutor::new(&*storage);
        let result = executor.execute(&plan)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(ExecutionResult::new(result.rows, result.affected_rows))
    }

    fn validate_operation(&self, op: &WriteOp, ctx: &TransactionContext) -> Result<(), crate::execution::DriftViolation> {
        self.drift_gate.validate(op, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::engine::MemoryStorage;
    use tempfile::TempDir;

    fn create_facade() -> WalTransactionalFacade<MemoryStorage> {
        let dir = TempDir::new().unwrap();
        let storage = MemoryStorage::new();
        WalTransactionalFacade::new(storage, dir.path().join("test.wal")).unwrap()
    }

    #[test]
    fn test_begin_commit() {
        let facade = create_facade();
        let tx_id = facade.begin().unwrap();
        assert!(tx_id > 0);
        assert!(facade.is_in_transaction());
        let ts = facade.commit().unwrap();
        assert!(ts.is_some());
        assert!(!facade.is_in_transaction());
    }

    #[test]
    fn test_rollback() {
        let facade = create_facade();
        facade.begin().unwrap();
        assert!(facade.is_in_transaction());
        facade.rollback().unwrap();
        assert!(!facade.is_in_transaction());
    }

    #[test]
    fn test_write_without_tx_fails() {
        let facade = create_facade();
        let ctx = TransactionContext::new(999);
        let op = WriteOp::Insert {
            table: "t".to_string(),
            columns: vec!["c".to_string()],
            values: vec![vec![]],
        };
        let result = facade.execute_write(&ctx, op);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_mutation_flow() {
        let facade = create_facade();
        let tx_id = facade.begin().unwrap();
        let mut ctx = TransactionContext::new(tx_id);
        ctx.mark_wal_open();
        let op = WriteOp::Insert {
            table: "t".to_string(),
            columns: vec!["c".to_string()],
            values: vec![vec![sqlrustgo_types::Value::Integer(1)]],
        };
        let result = facade.execute_write(&ctx, op);
        assert!(result.is_ok());
    }
}