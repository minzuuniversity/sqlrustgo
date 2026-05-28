//! LocalExchange implementation - data stays local without network transfer.

use crate::exchange::operator::{ExchangeOperator, ExchangeType, Partitioning};
use crate::pipeline::chunk_or_done::ChunkOrDone;
use crate::pipeline::error::PipelineResult;
use crate::pipeline::operator::Schema;

/// Local exchange operator - data remains on the local partition.
///
/// This exchange type is used when no data redistribution is needed,
/// typically for single-partition queries or when data is already
/// partitioned appropriately.
pub struct LocalExchange {
    /// Schema of the data being exchanged.
    schema: Schema,
    /// The underlying operator producing data.
    inner: Box<dyn Operator>,
    /// Flag indicating if the inner operator has been opened.
    opened: bool,
    /// Flag indicating if the inner operator has been closed.
    closed: bool,
}

impl LocalExchange {
    /// Creates a new LocalExchange.
    pub fn new(schema: Schema, inner: Box<dyn Operator>) -> Self {
        Self {
            schema,
            inner,
            opened: false,
            closed: false,
        }
    }
}

impl ExchangeOperator for LocalExchange {
    fn partition_count(&self) -> usize {
        // Local exchange always produces 1 partition (self)
        1
    }

    fn current_partition(&self) -> usize {
        // Local exchange is always partition 0
        0
    }

    fn exchange_type(&self) -> ExchangeType {
        ExchangeType::Local
    }

    fn partitioning(&self) -> Partitioning {
        Partitioning::RoundRobin
    }
}

impl crate::pipeline::operator::Operator for LocalExchange {
    fn open(&mut self) -> PipelineResult<()> {
        self.inner.open()?;
        self.opened = true;
        Ok(())
    }

    fn next_chunk(&mut self) -> PipelineResult<ChunkOrDone> {
        if !self.opened {
            return Err(crate::pipeline::error::PipelineError::Operator(
                "operator not opened".into(),
            ));
        }
        if self.closed {
            return Err(crate::pipeline::error::PipelineError::Operator(
                "operator already closed".into(),
            ));
        }
        self.inner.next_chunk()
    }

    fn close(&mut self) -> PipelineResult<()> {
        if self.closed {
            return Err(crate::pipeline::error::PipelineError::Operator(
                "operator already closed".into(),
            ));
        }
        let result = self.inner.close();
        self.closed = true;
        result
    }

    fn schema(&self) -> &Schema {
        &self.schema
    }
}

use crate::pipeline::operator::Operator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, ColumnVector};
    use crate::pipeline::operator::{Operator, Schema};

    struct TestOperator {
        schema: Schema,
        remaining: usize,
    }

    impl TestOperator {
        fn new(num_chunks: usize) -> Self {
            Self {
                schema: Schema::new(vec!["val".into()], vec!["INT".into()]),
                remaining: num_chunks,
            }
        }
    }

    impl Operator for TestOperator {
        fn open(&mut self) -> PipelineResult<()> {
            Ok(())
        }

        fn next_chunk(&mut self) -> PipelineResult<ChunkOrDone> {
            if self.remaining == 0 {
                return Ok(ChunkOrDone::Done);
            }
            self.remaining -= 1;
            let col = ColumnVector::from_int32(vec![1, 2, 3]);
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
    fn test_local_exchange_partition_info() {
        let schema = Schema::new(vec!["id".into()], vec!["INT".into()]);
        let inner = Box::new(TestOperator::new(0));
        let exchange = LocalExchange::new(schema, inner);

        assert_eq!(exchange.partition_count(), 1);
        assert_eq!(exchange.current_partition(), 0);
        assert!(matches!(exchange.exchange_type(), ExchangeType::Local));
    }

    #[test]
    fn test_local_exchange_passthrough() {
        let schema = Schema::new(vec!["val".into()], vec!["INT".into()]);
        let inner = Box::new(TestOperator::new(2));
        let mut exchange = LocalExchange::new(schema, inner);

        exchange.open().unwrap();

        let chunk1 = exchange.next_chunk().unwrap();
        assert!(chunk1.is_chunk());

        let chunk2 = exchange.next_chunk().unwrap();
        assert!(chunk2.is_chunk());

        let done = exchange.next_chunk().unwrap();
        assert!(done.is_done());

        exchange.close().unwrap();
    }

    #[test]
    fn test_local_exchange_schema() {
        let schema = Schema::new(
            vec!["a".into(), "b".into()],
            vec!["INT".into(), "VARCHAR".into()],
        );
        let inner = Box::new(TestOperator::new(0));
        let exchange = LocalExchange::new(schema.clone(), inner);

        assert_eq!(exchange.schema().len(), 2);
        assert_eq!(exchange.schema().columns[0], "a");
    }
}
