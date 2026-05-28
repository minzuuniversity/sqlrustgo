//! Pipeline: a chain of operators that execute in sequence.

use super::chunk_or_done::ChunkOrDone;
use super::error::PipelineResult;
use super::operator::Schema;
use crate::chunk::Chunk;
use crate::pipeline::operator::Operator;

/// A pipeline is a sequence of operators that process data in order.
/// Each operator's output becomes the next operator's input.
pub struct Pipeline {
    operators: Vec<Box<dyn Operator>>,
}

impl Pipeline {
    /// Creates a new empty pipeline.
    pub fn new() -> Self {
        Self {
            operators: Vec::new(),
        }
    }

    /// Adds an operator to the end of the pipeline.
    pub fn add(&mut self, op: impl Operator + 'static) {
        self.operators.push(Box::new(op));
    }

    /// Returns the number of operators in this pipeline.
    pub fn len(&self) -> usize {
        self.operators.len()
    }

    /// Returns true if the pipeline has no operators.
    pub fn is_empty(&self) -> bool {
        self.operators.is_empty()
    }

    /// Returns the schema of the pipeline's output.
    pub fn output_schema(&self) -> Option<&Schema> {
        self.operators.last().map(|op| op.schema())
    }

    /// Executes the pipeline and collects all output chunks.
    pub fn execute(&mut self) -> PipelineResult<Vec<Chunk>> {
        if self.operators.is_empty() {
            return Ok(vec![]);
        }

        // Open all operators
        for op in &mut *self.operators {
            op.open()?;
        }

        let mut results: Vec<Chunk> = Vec::new();

        // Pull chunks through the pipeline
        loop {
            let chunk_or_done = self.operators.last_mut().unwrap().next_chunk()?;
            match chunk_or_done {
                ChunkOrDone::Chunk(chunk) => {
                    results.push(chunk);
                }
                ChunkOrDone::Done => {
                    break;
                }
            }
        }

        // Close all operators
        for op in &mut *self.operators {
            op.close()?;
        }

        Ok(results)
    }

    /// Executes the pipeline with a custom driver function.
    pub fn execute_with_driver<D: FnMut(Chunk) -> PipelineResult<()>>(
        &mut self,
        driver: &mut D,
    ) -> PipelineResult<()> {
        if self.operators.is_empty() {
            return Ok(());
        }

        for op in &mut *self.operators {
            op.open()?;
        }

        loop {
            let chunk_or_done = self.operators.last_mut().unwrap().next_chunk()?;
            match chunk_or_done {
                ChunkOrDone::Chunk(chunk) => {
                    driver(chunk)?;
                }
                ChunkOrDone::Done => {
                    break;
                }
            }
        }

        for op in &mut *self.operators {
            op.close()?;
        }

        Ok(())
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ColumnVector;
    use crate::pipeline::operator::Operator;

    struct PassthroughOp {
        schema: Schema,
        remaining: usize,
        chunk_size: usize,
    }

    impl PassthroughOp {
        fn new(num_chunks: usize, chunk_size: usize, num_cols: usize) -> Self {
            let columns: Vec<String> = (0..num_cols).map(|i| format!("col_{i}")).collect();
            let types: Vec<String> = vec!["INT".into(); num_cols];
            Self {
                schema: Schema::new(columns, types),
                remaining: num_chunks,
                chunk_size,
            }
        }
    }

    impl Operator for PassthroughOp {
        fn open(&mut self) -> PipelineResult<()> {
            Ok(())
        }

        fn next_chunk(&mut self) -> PipelineResult<ChunkOrDone> {
            if self.remaining == 0 {
                return Ok(ChunkOrDone::Done);
            }
            self.remaining -= 1;
            let col = ColumnVector::from_int32((0..self.chunk_size).map(|i| i as i32).collect());
            Ok(ChunkOrDone::Chunk(Chunk::new(vec![col])))
        }

        fn close(&mut self) -> PipelineResult<()> {
            Ok(())
        }

        fn schema(&self) -> &Schema {
            &self.schema
        }
    }

    #[test]
    fn test_pipeline_empty() {
        let pipeline: Pipeline = Pipeline::new();
        assert!(pipeline.is_empty());
        assert!(pipeline.output_schema().is_none());
    }

    #[test]
    fn test_pipeline_single_operator() {
        let mut pipeline = Pipeline::new();
        pipeline.add(PassthroughOp::new(3, 10, 1));

        let results = pipeline.execute().unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].rows(), 10);
        assert_eq!(results[1].rows(), 10);
        assert_eq!(results[2].rows(), 10);
    }

    #[test]
    fn test_pipeline_multiple_operators() {
        let mut pipeline = Pipeline::new();
        pipeline.add(PassthroughOp::new(2, 5, 1));
        pipeline.add(PassthroughOp::new(2, 5, 1));

        let results = pipeline.execute().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].rows(), 5);
    }

    #[test]
    fn test_pipeline_driver() {
        let mut pipeline = Pipeline::new();
        pipeline.add(PassthroughOp::new(2, 5, 1));

        let mut row_count = 0;
        let result = pipeline.execute_with_driver(&mut |chunk| {
            row_count += chunk.rows();
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(row_count, 10);
    }
}
