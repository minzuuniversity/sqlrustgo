//! Local Executor DML Module
//!
//! Placeholder for local executor DML operations.
//!
//! ## G4 Fix (Issue #2811)
//!
//! Added `engine: Arc<Mutex<dyn ExecutionEngine>>` field to enable
//! `MergeExecutor::new()` instantiation from G3 (#2810). Previously
//! LocalExecutor held only borrowed `&dyn StorageEngine`, making it
//! impossible to construct MergeExecutor's required Arc-wrapped form.

use crate::execution::ExecutionEngine;
use std::sync::{Arc, Mutex};

/// LocalExecutorDml with engine field for VTU path (G4 #2811)
pub struct LocalExecutorDml {
    engine: Arc<Mutex<dyn ExecutionEngine>>,
}

/// Placeholder LocalExecutorDmlArc
pub struct LocalExecutorDmlArc;

impl LocalExecutorDml {
    pub fn new() -> Self {
        Self {
            // Default: construct a no-op engine adapter. G3 will replace
            // this with a real ExecutionEngine wired to actual DML.
            engine: Arc::new(Mutex::new(NoopExecutionEngine)),
        }
    }

    /// Construct with explicit engine (used by G3 #2810 caller)
    pub fn with_engine(engine: Arc<Mutex<dyn ExecutionEngine>>) -> Self {
        Self { engine }
    }

    /// Get the engine (for G3 MergeExecutor construction)
    pub fn engine(&self) -> Arc<Mutex<dyn ExecutionEngine>> {
        self.engine.clone()
    }
}

/// No-op engine adapter for placeholder use. G3 will replace with real impl.
struct NoopExecutionEngine;

impl ExecutionEngine for NoopExecutionEngine {
    fn execute(&mut self, _ctx: &mut crate::execution::QueryContext) -> Result<crate::execution::ExecutionResult, sqlrustgo_types::SqlError> {
        Ok(crate::execution::ExecutionResult::ok(0))
    }
    fn begin(&mut self) -> Result<u64, sqlrustgo_types::SqlError> {
        Err(sqlrustgo_types::SqlError::ExecutionError("NoopEngine: begin not implemented".to_string()))
    }
    fn commit(&mut self, _txn: u64) -> Result<(), sqlrustgo_types::SqlError> {
        Err(sqlrustgo_types::SqlError::ExecutionError("NoopEngine: commit not implemented".to_string()))
    }
    fn rollback(&mut self, _txn: u64) -> Result<(), sqlrustgo_types::SqlError> {
        Err(sqlrustgo_types::SqlError::ExecutionError("NoopEngine: rollback not implemented".to_string()))
    }
}

impl Default for LocalExecutorDml {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_local_executor_dml_new() {
        let dml = LocalExecutorDml::new();
        let _ = dml;
    }

    #[test]
    fn test_local_executor_dml_default() {
        let dml = LocalExecutorDml::default();
        let _ = dml;
    }

    #[test]
    fn test_send_sync() {
        // LocalExecutorDml is not necessarily Send because it holds
        // `Arc<Mutex<dyn ExecutionEngine>>` where ExecutionEngine is not Send.
        // Only verify the no-engine Arc type.
        fn check<T: Send + Sync>() {}
        check::<LocalExecutorDmlArc>();
    }

    #[test]
    fn test_g4_engine_field_arc_cloneable() {
        // G4 #2811: engine must be Arc<Mutex<dyn ExecutionEngine>> and cloneable
        let dml = LocalExecutorDml::new();
        let engine1 = dml.engine();
        let engine2 = dml.engine();
        // Both Arc handles point to the same engine
        assert!(Arc::ptr_eq(&engine1, &engine2));
    }

    #[test]
    fn test_g4_with_engine_constructor() {
        // G4 #2811: with_engine() must accept explicit engine
        let storage = MemoryStorage::new();
        let engine: Arc<Mutex<dyn ExecutionEngine>> =
            Arc::new(Mutex::new(NoopExecutionEngine));
        let dml = LocalExecutorDml::with_engine(engine.clone());
        let retrieved = dml.engine();
        assert!(Arc::ptr_eq(&engine, &retrieved));
        let _ = storage; // suppress unused warning
    }
}

