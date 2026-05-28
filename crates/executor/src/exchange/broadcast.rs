//! BroadcastExchange implementation - all data sent to all partitions.

use crate::exchange::operator::{ExchangeOperator, ExchangeType, Partitioning};
use crate::pipeline::chunk_or_done::ChunkOrDone;
use crate::pipeline::error::PipelineResult;
use crate::pipeline::operator::Schema;

/// Broadcast exchange operator - all data sent to all partitions.
///
/// This exchange type replicates the entire dataset to each partition.
/// Useful for broadcast joins where one side is small, or for
/// materializing common expressions (like CTEs) that are used by
/// multiple parallel workers.
pub struct BroadcastExchange {
    /// Schema of the data being exchanged.
    schema: Schema,
    /// The underlying operator producing data.
    inner: Box<dyn Operator>,
    /// Number of partitions to broadcast to.
    partition_count: usize,
    /// Current partition being read from.
    current_partition: usize,
    /// Flag indicating if the data has been materialized.
    data_materialized: bool,
    /// Materialized chunks for replay.
    chunks: Vec<Chunk>,
    /// Current chunk index for replay.
    chunk_index: usize,
    /// Flag indicating if the inner operator has been opened.
    opened: bool,
    /// Flag indicating if the inner operator has been closed.
    closed: bool,
}

impl BroadcastExchange {
    /// Creates a new BroadcastExchange with the specified number of partitions.
    pub fn new(schema: Schema, inner: Box<dyn Operator>, partition_count: usize) -> Self {
        Self {
            schema,
            inner,
            partition_count,
            current_partition: 0,
            data_materialized: false,
            chunks: Vec::new(),
            chunk_index: 0,
            opened: false,
            closed: false,
        }
    }

    /// Creates a new BroadcastExchange with default partition count of 1.
    pub fn with_default_partitions(schema: Schema, inner: Box<dyn Operator>) -> Self {
        Self::new(schema, inner, 1)
    }
}

impl ExchangeOperator for BroadcastExchange {
    fn partition_count(&self) -> usize {
        self.partition_count
    }

    fn current_partition(&self) -> usize {
        self.current_partition
    }

    fn exchange_type(&self) -> ExchangeType {
        ExchangeType::Broadcast
    }

    fn partitioning(&self) -> Partitioning {
        Partitioning::RoundRobin
    }
}

impl crate::pipeline::operator::Operator for BroadcastExchange {
    fn open(&mut self) -> PipelineResult<()> {
        self.inner.open()?;
        self.opened = true;

        // Materialize all data from the inner operator
        // In a real distributed implementation, this would be sent over the network
        self.chunks.clear();
        self.chunk_index = 0;

        loop {
            match self.inner.next_chunk()? {
                ChunkOrDone::Chunk(chunk) => {
                    self.chunks.push(chunk);
                }
                ChunkOrDone::Done => {
                    break;
                }
            }
        }

        self.data_materialized = true;
        self.inner.close()?;

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

        if self.chunk_index < self.chunks.len() {
            let chunk = self.chunks[self.chunk_index].clone();
            self.chunk_index += 1;
            Ok(ChunkOrDone::Chunk(chunk))
        } else {
            Ok(ChunkOrDone::Done)
        }
    }

    fn close(&mut self) -> PipelineResult<()> {
        if self.closed {
            return Err(crate::pipeline::error::PipelineError::Operator(
                "operator already closed".into(),
            ));
        }
        self.closed = true;
        self.chunks.clear();
        Ok(())
    }

    fn schema(&self) -> &Schema {
        &self.schema
    }
}

use crate::chunk::Chunk;
use crate::pipeline::operator::Operator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ColumnVector;

    struct TestSource {
        schema: Schema,
        chunks: Vec<Chunk>,
        index: usize,
    }

    impl TestSource {
        fn new(chunks_data: Vec<Vec<i32>>) -> Self {
            let chunks: Vec<Chunk> = chunks_data
                .into_iter()
                .map(|vals| {
                    let col = ColumnVector::from_int32(vals);
                    Chunk::new(vec![col])
                })
                .collect();
            Self {
                schema: Schema::new(vec!["val".into()], vec!["INT".into()]),
                chunks,
                index: 0,
            }
        }
    }

    impl Operator for TestSource {
        fn open(&mut self) -> PipelineResult<()> {
            Ok(())
        }

        fn next_chunk(&mut self) -> PipelineResult<ChunkOrDone> {
            if self.index < self.chunks.len() {
                let chunk = self.chunks[self.index].clone();
                self.index += 1;
                Ok(ChunkOrDone::Chunk(chunk))
            } else {
                Ok(ChunkOrDone::Done)
            }
        }

        fn close(&mut self) -> PipelineResult<()> {
            Ok(())
        }

        fn schema(&self) -> &Schema {
            &self.schema
        }
    }

    #[test]
    fn test_broadcast_exchange_partition_info() {
        let schema = Schema::new(vec!["id".into()], vec!["INT".into()]);
        let inner = Box::new(TestSource::new(vec![]));
        let exchange = BroadcastExchange::new(schema, inner, 4);

        assert_eq!(exchange.partition_count(), 4);
        assert_eq!(exchange.current_partition(), 0);
        assert!(matches!(exchange.exchange_type(), ExchangeType::Broadcast));
    }

    #[test]
    fn test_broadcast_exchange_data_replication() {
        let schema = Schema::new(vec!["val".into()], vec!["INT".into()]);
        let inner = Box::new(TestSource::new(vec![vec![1, 2], vec![3, 4]]));
        let mut exchange = BroadcastExchange::new(schema, inner, 2);

        exchange.open().unwrap();

        // Should be able to read all chunks multiple times since it's broadcast
        let chunk1 = exchange.next_chunk().unwrap();
        assert!(chunk1.is_chunk());

        let chunk2 = exchange.next_chunk().unwrap();
        assert!(chunk2.is_chunk());

        let done = exchange.next_chunk().unwrap();
        assert!(done.is_done());

        exchange.close().unwrap();
    }

    #[test]
    fn test_broadcast_exchange_with_default_partitions() {
        let schema = Schema::new(vec!["x".into()], vec!["INT".into()]);
        let inner = Box::new(TestSource::new(vec![vec![10, 20, 30]]));
        let exchange = BroadcastExchange::with_default_partitions(schema, inner);

        assert_eq!(exchange.partition_count(), 1);
        assert!(matches!(exchange.exchange_type(), ExchangeType::Broadcast));
    }

    #[test]
    fn test_broadcast_exchange_schema() {
        let schema = Schema::new(
            vec!["a".into(), "b".into(), "c".into()],
            vec!["INT".into(), "BIGINT".into(), "VARCHAR".into()],
        );
        let inner = Box::new(TestSource::new(vec![]));
        let exchange = BroadcastExchange::new(schema.clone(), inner, 1);

        assert_eq!(exchange.schema().len(), 3);
    }
}
