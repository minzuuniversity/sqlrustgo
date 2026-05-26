//! Report Generator Agent
//!
//! Generates GMP reports from natural language queries.

pub mod agent;
pub mod formatter;
pub mod intent;
pub mod types;

pub use agent::ReportGenerator;
pub use types::{ReportFormat, ReportRequest, ReportResult, ReportType};
