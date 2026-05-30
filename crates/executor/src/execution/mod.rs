//! Execution module — VTU Phase 2 contract enforcement
//!
//! Contains: DriftGate (enforcement), TransactionContext, WriteOp, ExecutionEngine (pure)

pub mod context;
pub mod drift;
pub mod drift_gate;
pub mod engine;
pub mod events;
pub mod facade;
pub mod recovery;
pub mod result;
pub mod telemetry;
pub mod trace;
pub mod transaction_context;
pub mod write_op;

// Re-export types for external use
pub use context::QueryContext;
pub use drift::{DriftDetector};
pub use drift_gate::{DriftGate, DriftSeverity, DriftViolation, DriftViolationType, GuardPolicy};
pub use engine::ExecutionEngine;
pub use events::{
    DmlOperation, ExecutionEvent, RecoveryConfidence, RecoveryPlan,
    RecoveryType,
};
pub use result::ExecutionResult;
pub use trace::ExecutionTrace;
pub use transaction_context::TransactionContext;
pub use write_op::WriteOp;
