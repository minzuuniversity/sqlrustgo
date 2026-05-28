//! Exchange Operator trait and related types.
//!
//! This module defines the ExchangeOperator trait which extends the base
//! Operator trait with distributed execution capabilities.

use crate::pipeline::operator::{Operator, Schema};

/// Exchange type defines the data distribution pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeType {
    /// No exchange - data stays local
    Local,
    /// Broadcast - all data sent to all partitions
    Broadcast,
    /// Shuffle - data redistributed based on partitioning
    Shuffle,
}

/// Partitioning strategy for data distribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Partitioning {
    /// Hash-based partitioning on specified columns
    HashPartition {
        /// Column names used for hashing
        columns: Vec<String>,
    },
    /// Range-based partitioning on specified columns
    RangePartition {
        /// Column names used for range calculation
        columns: Vec<String>,
    },
    /// Round-robin distribution
    RoundRobin,
}

impl Partitioning {
    /// Returns the number of partition columns for this partitioning strategy.
    pub fn num_columns(&self) -> usize {
        match self {
            Partitioning::HashPartition { columns } => columns.len(),
            Partitioning::RangePartition { columns } => columns.len(),
            Partitioning::RoundRobin => 0,
        }
    }
}

/// Trait for exchange operators that handle data distribution.
///
/// Exchange operators extend the base Operator trait with capabilities
/// for distributing data across multiple partitions in a distributed
/// query execution environment.
pub trait ExchangeOperator: Operator {
    /// Returns the number of partitions this exchange produces.
    fn partition_count(&self) -> usize;

    /// Returns the partition ID for the current thread/context.
    fn current_partition(&self) -> usize;

    /// Returns the exchange type being performed.
    fn exchange_type(&self) -> ExchangeType;

    /// Returns the partitioning strategy used for this exchange.
    fn partitioning(&self) -> Partitioning;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_type_variants() {
        assert!(matches!(ExchangeType::Local, ExchangeType::Local));
        assert!(matches!(ExchangeType::Broadcast, ExchangeType::Broadcast));
        assert!(matches!(ExchangeType::Shuffle, ExchangeType::Shuffle));
    }

    #[test]
    fn test_partitioning_num_columns() {
        let hash = Partitioning::HashPartition {
            columns: vec!["a".into(), "b".into()],
        };
        assert_eq!(hash.num_columns(), 2);

        let range = Partitioning::RangePartition {
            columns: vec!["x".into()],
        };
        assert_eq!(range.num_columns(), 1);

        let rr = Partitioning::RoundRobin;
        assert_eq!(rr.num_columns(), 0);
    }

    #[test]
    fn test_partitioning_clone_eq() {
        let p1 = Partitioning::HashPartition {
            columns: vec!["id".into()],
        };
        let p2 = p1.clone();
        assert_eq!(p1, p2);

        let p3 = Partitioning::RoundRobin;
        let p4 = p3.clone();
        assert_eq!(p3, p4);
    }
}
