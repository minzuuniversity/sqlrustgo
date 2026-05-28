//! Pipeline error types.

use thiserror::Error;

/// Errors that can occur during pipeline execution.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("operator error: {0}")]
    Operator(String),

    #[error("pipeline error: {0}")]
    Pipeline(String),

    #[error("schema mismatch: {0}")]
    SchemaMismatch(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for pipeline operations.
pub type PipelineResult<T> = Result<T, PipelineError>;
