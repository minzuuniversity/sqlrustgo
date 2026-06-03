//! WalTransactionalFacade — WAL-aware implementation of TransactionalFacade
//!
//! **STATUS (2026-06-03)**: STUB — facade design pending rewrite.
//!
//! The previous 195-line implementation had multiple compile errors against
//! the current API surface:
//! - `WalStorage<S>` generic parameter mismatch (needs `<S, T: WalManager>`)
//! - `WalStorage::new(inner, PathBuf)` signature wrong (real API takes a
//!   `WalManager` trait object, not a path)
//! - `WalStorage::new_without_wal` does not exist
//! - `sqlrustgo_transaction::TransactionManager::new()` does not exist
//! - `parking_lot` not in executor's Cargo.toml dependencies
//! - `sqlrustgo_transaction` not in executor's Cargo.toml dependencies
//! - `LocalExecutor` no longer at crate root
//! - `sqlrustgo_planner::create_physical_plan` does not exist
//!
//! This stub preserves the trait-object shape so the trait (`TransactionalFacade`)
//! compiles and the module can be mounted. The implementation is `unimplemented!()`
//! pending a real facade redesign that aligns with the current API surface.
//!
//! See docs/releases/v3.8.0/F06_FACADE_DESIGN_NOTES.md for the rewrite plan.

#![allow(unused_variables, dead_code)]

use std::sync::Arc;

use super::drift_gate::DriftGate;
use super::transaction_context::TransactionContext;
use super::transactional_facade::TransactionalFacade;
use super::write_op::WriteOp;
use crate::sql_executor::ExecutionResult;
use sqlrustgo_types::SqlResult;

/// WalTransactionalFacade — wraps a storage engine + WAL manager + transaction manager
/// and exposes a unified `TransactionalFacade` interface.
///
/// The concrete storage type is kept abstract (`S: StorageEngine`) so the facade
/// can be parameterised by any engine (in-memory, file, columnar, etc.).
pub struct WalTransactionalFacade<S> {
    _storage: Arc<std::sync::Mutex<S>>,
    _drift_gate: DriftGate,
    _marker: std::marker::PhantomData<()>,
}

impl<S> WalTransactionalFacade<S> {
    /// Construct a new facade wrapping `storage`.
    ///
    /// **STATUS**: stub — real WAL wiring pending.
    pub fn new(storage: S) -> Self {
        Self {
            _storage: Arc::new(std::sync::Mutex::new(storage)),
            _drift_gate: DriftGate::new("wal-facade-stub".to_string()),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Send + Sync> TransactionalFacade for WalTransactionalFacade<S> {
    fn begin(&self) -> SqlResult<u64> {
        unimplemented!("WalTransactionalFacade::begin — see F06_FACADE_DESIGN_NOTES.md")
    }
    fn commit(&self) -> SqlResult<Option<u64>> {
        unimplemented!("WalTransactionalFacade::commit — see F06_FACADE_DESIGN_NOTES.md")
    }
    fn rollback(&self) -> SqlResult<()> {
        unimplemented!("WalTransactionalFacade::rollback — see F06_FACADE_DESIGN_NOTES.md")
    }
    fn is_in_transaction(&self) -> bool {
        false
    }
    fn current_tx_id(&self) -> Option<u64> {
        None
    }
    fn execute_write(&self, _ctx: &TransactionContext, _op: WriteOp) -> SqlResult<ExecutionResult> {
        unimplemented!("WalTransactionalFacade::execute_write — see F06_FACADE_DESIGN_NOTES.md")
    }
    fn execute_read(&self, _sql: &str) -> SqlResult<ExecutionResult> {
        unimplemented!("WalTransactionalFacade::execute_read — see F06_FACADE_DESIGN_NOTES.md")
    }
    fn validate_operation(
        &self,
        op: &WriteOp,
        ctx: &TransactionContext,
    ) -> Result<(), crate::execution::DriftViolation> {
        self._drift_gate.validate(op, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies the stub compiles and the trait wiring is in place.
    /// Real implementation tests will replace this once the rewrite lands.
    #[test]
    fn facade_stub_compiles_and_basic_state() {
        // We can't easily construct a real StorageEngine here without a
        // type parameter; instead, verify the type-level wiring by
        // instantiating the facade with the unit type.
        let _facade: WalTransactionalFacade<()> = WalTransactionalFacade::new(());
        // is_in_transaction / current_tx_id are implemented (no tx state)
        assert!(!_facade.is_in_transaction());
        assert_eq!(_facade.current_tx_id(), None);
    }
}
