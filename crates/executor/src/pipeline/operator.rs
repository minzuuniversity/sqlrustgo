//! Operator trait: the core interface for all pipeline operators.

use super::chunk_or_done::ChunkOrDone;
use super::error::{PipelineError, PipelineResult};
use crate::chunk::Chunk;

/// Schema describes the output of an operator.
#[derive(Debug, Clone)]
pub struct Schema {
    /// Column names.
    pub columns: Vec<String>,
    /// Column types (SQL types as strings).
    pub types: Vec<String>,
}

impl Schema {
    /// Creates a new schema.
    pub fn new(columns: Vec<String>, types: Vec<String>) -> Self {
        debug_assert_eq!(columns.len(), types.len());
        Self { columns, types }
    }

    /// Returns the number of columns.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Returns true if the schema has no columns.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

/// The core trait for all pipeline operators.
///
/// Each operator takes a Chunk as input and produces Chunks as output.
pub trait Operator: Send {
    /// Initialize the operator. Called once before any calls to next_chunk().
    fn open(&mut self) -> PipelineResult<()>;

    /// Return the next chunk of output data.
    ///
    /// Returns `ChunkOrDone::Chunk(chunk)` if there is more data,
    /// or `ChunkOrDone::Done` when the operator has finished.
    fn next_chunk(&mut self) -> PipelineResult<ChunkOrDone>;

    /// Clean up resources held by the operator.
    /// Called once after all data has been produced.
    fn close(&mut self) -> PipelineResult<()>;

    /// Returns the schema of the data produced by this operator.
    fn schema(&self) -> &Schema;
}

/// Extension trait for convenience methods on operators.
pub trait OperatorHelpers {
    /// Returns the number of columns in the output schema.
    fn num_columns(&self) -> usize;
    /// Returns the column names.
    fn column_names(&self) -> Vec<String>;
}

impl<T: Operator> OperatorHelpers for T {
    fn num_columns(&self) -> usize {
        self.schema().len()
    }

    fn column_names(&self) -> Vec<String> {
        self.schema().columns.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_new() {
        let schema = Schema::new(
            vec!["id".into(), "name".into()],
            vec!["INT".into(), "VARCHAR".into()],
        );
        assert_eq!(schema.len(), 2);
        assert_eq!(schema.columns[0], "id");
        assert_eq!(schema.types[1], "VARCHAR");
    }

    #[test]
    fn test_schema_empty() {
        let schema = Schema::new(vec![], vec![]);
        assert!(schema.is_empty());
        assert_eq!(schema.len(), 0);
    }
}
