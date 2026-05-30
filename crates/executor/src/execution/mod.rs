pub mod context;
pub mod drift;
pub mod engine;
pub mod facade;
pub mod recovery;
pub mod result;
pub mod telemetry;
pub mod trace;

pub use context::QueryContext;
pub use drift::{DriftDetector, GuardPolicy};
pub use engine::ExecutionEngine;
pub use facade::ExecutionFacade;
pub use recovery::{ExecutionReplayEngine, RecoveryPlanner, SafeExecutionController};
pub use result::ExecutionResult;
pub use telemetry::{EventBuffer, TelemetryCollector};
pub use trace::{
    DmlOperation, DriftSeverity, DriftViolation, DriftViolationType, ExecutionEvent,
    ExecutionTrace, RecoveryConfidence, RecoveryPlan, RecoveryType, TxnStep,
};
