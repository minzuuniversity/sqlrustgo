//! Workflow V2 Engine for SQLRustGo GMP
//!
//! Next-generation workflow engine with state machines, parallel gates, and full audit trail.

pub mod definition;
pub mod engine;
pub mod state;
pub mod transitions;

pub use definition::{WorkflowDef, StepDef, GateDef, WorkflowVersion};
pub use engine::WorkflowEngine;
pub use state::{WorkflowInstance, StepStatus, InstanceStatus};
pub use transitions::TransitionResult;
