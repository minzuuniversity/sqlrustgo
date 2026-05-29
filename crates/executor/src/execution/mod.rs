pub mod engine;
pub mod context;
pub mod result;
pub mod facade;
pub mod trace;
pub mod telemetry;
pub mod drift;
pub mod recovery;

pub use engine::ExecutionEngine;
pub use context::QueryContext;
pub use result::ExecutionResult;
pub use facade::ExecutionFacade;
pub use trace::{TxnStep, ExecutionTrace, DmlOperation, ExecutionEvent, DriftViolationType, DriftSeverity, DriftViolation, RecoveryType, RecoveryConfidence, RecoveryPlan};
pub use telemetry::{TelemetryCollector, EventBuffer};
pub use drift::{DriftDetector, GuardPolicy};
pub use recovery::{RecoveryPlanner, ExecutionReplayEngine, SafeExecutionController};