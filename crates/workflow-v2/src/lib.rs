//! Workflow V2 Engine for SQLRustGo GMP
//!
//! Next-generation workflow engine with state machines, parallel gates, and full audit trail.

pub mod definition;
pub mod engine;
pub mod state;
pub mod transitions;

pub use definition::{GateDef, StepDef, WorkflowDef, WorkflowVersion};
pub use engine::WorkflowEngine;
pub use state::{InstanceStatus, StepStatus, WorkflowInstance};
pub use transitions::TransitionResult;
