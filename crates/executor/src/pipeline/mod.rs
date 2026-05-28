//! Pipeline module: unified execution architecture.

pub mod chunk_or_done;
pub mod error;
pub mod operator;
pub mod pipeline;

// Re-exports
pub use chunk_or_done::ChunkOrDone;
pub use error::{PipelineError, PipelineResult};
pub use operator::{Operator, OperatorHelpers, Schema};
pub use pipeline::Pipeline;
