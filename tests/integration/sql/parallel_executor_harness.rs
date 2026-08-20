//! ParallelExecutor Harness (P3-4 #3183)
//!
//! Shared utilities for parallel/vector executor testing. Provides:
//! - `MockPartitionInfo` — declarative partition metadata
//! - `MockPartitionAgent` — collection of partition infos
//! - `MockPartitionStrategy` (Hash, Range, Key, RoundRobin)
//! - `MockWorkerPool` — worker count + batch size + queue depth
//! - `run_partition` — distribute total_rows across partitions
//! - `run_worker_pool` — distribute work across workers
//!
//! This file is **not** a test target itself (no `#[test]`); shared
//! by `parallel_executor_test.rs`.

#![allow(dead_code)] // helpers consumed by test targets

/// Mock partition info (mirrors ParallelVectorExecutor::PartitionInfo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockPartitionInfo {
    pub partition_id: usize,
    pub num_partitions: usize,
    pub total_rows: usize,
}

impl MockPartitionInfo {
    pub fn new(partition_id: usize, num_partitions: usize, total_rows: usize) -> Self {
        Self {
            partition_id,
            num_partitions,
            total_rows,
        }
    }
}

/// Mock partition agent (collection of partition infos).
#[derive(Debug, Clone, PartialEq)]
pub struct MockPartitionAgent {
    pub partitions: Vec<MockPartitionInfo>,
}

impl MockPartitionAgent {
    pub fn new(num_partitions: usize, total_rows: usize) -> Self {
        let partitions = (0..num_partitions)
            .map(|id| MockPartitionInfo::new(id, num_partitions, total_rows))
            .collect();
        Self { partitions }
    }

    pub fn num_partitions(&self) -> usize {
        self.partitions.len()
    }

    pub fn total_rows(&self) -> usize {
        self.partitions.first().map(|p| p.total_rows).unwrap_or(0)
    }

    pub fn rows_per_partition(&self) -> usize {
        let n = self.num_partitions();
        self.total_rows().checked_div(n).unwrap_or(0)
    }
}

/// Partition strategy kinds (mirrors PartitionStrategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionKind {
    Hash,
    Range,
    Key,
    RoundRobin,
}

/// Mock partition strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockPartitionStrategy {
    pub kind: PartitionKind,
    pub num_shards: u64,
}

impl MockPartitionStrategy {
    pub fn new(kind: PartitionKind, num_shards: u64) -> Self {
        Self { kind, num_shards }
    }

    /// Hash partition a value (simple FNV-like).
    pub fn hash_partition(&self, value: i64) -> u64 {
        let mut h: u64 = 0xCBF29CE484222325;
        let bytes = value.to_le_bytes();
        for b in bytes {
            h = h.wrapping_mul(0x100000001B3).wrapping_add(b as u64);
        }
        h % self.num_shards.max(1)
    }

    /// Range partition a value against boundaries.
    pub fn range_partition(&self, value: i64, boundaries: &[i64]) -> u64 {
        for (i, b) in boundaries.iter().enumerate() {
            if value < *b {
                return i as u64;
            }
        }
        boundaries.len() as u64
    }

    /// RoundRobin partition (cycles through shards).
    pub fn round_robin_partition(&self, value: i64) -> u64 {
        (value.rem_euclid(self.num_shards.max(1) as i64)) as u64
    }

    /// Pick a partition for a value based on the strategy kind.
    pub fn partition(&self, value: i64) -> u64 {
        match self.kind {
            PartitionKind::Hash => self.hash_partition(value),
            PartitionKind::Range => self.range_partition(value, &[]),
            PartitionKind::Key => self.hash_partition(value), // Key uses hash under the hood
            PartitionKind::RoundRobin => self.round_robin_partition(value),
        }
    }
}

/// Mock worker pool config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockWorkerPool {
    pub num_workers: usize,
    pub batch_size: usize,
    pub queue_depth: usize,
}

impl MockWorkerPool {
    pub fn new(num_workers: usize, batch_size: usize) -> Self {
        Self {
            num_workers,
            batch_size,
            queue_depth: num_workers * 2, // default queue = 2x workers
        }
    }

    pub fn with_queue_depth(mut self, depth: usize) -> Self {
        self.queue_depth = depth;
        self
    }

    /// Number of batches a worker can process concurrently.
    pub fn concurrent_batches(&self) -> usize {
        (self.queue_depth / self.num_workers).max(1)
    }

    /// Total throughput (rows/s, assuming 1ms per row).
    pub fn throughput(&self) -> usize {
        self.num_workers * self.batch_size * 1000
    }
}

/// Distribute total_rows across partitions in round-robin.
pub fn run_partition(agent: &MockPartitionAgent, total_rows: usize) -> Vec<Vec<usize>> {
    let n = agent.num_partitions();
    let mut partitions: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..total_rows {
        partitions[i % n].push(i);
    }
    partitions
}

/// Distribute work across workers.
pub fn run_worker_pool(pool: &MockWorkerPool, total_rows: usize) -> Vec<Vec<usize>> {
    let n = pool.num_workers.max(1);
    let mut workers: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..total_rows {
        workers[i % n].push(i);
    }
    workers
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn partition_agent_basic() {
        let agent = MockPartitionAgent::new(4, 1000);
        assert_eq!(agent.num_partitions(), 4);
        assert_eq!(agent.total_rows(), 1000);
        assert_eq!(agent.rows_per_partition(), 250);
    }

    #[test]
    fn hash_partition_deterministic() {
        let s = MockPartitionStrategy::new(PartitionKind::Hash, 4);
        let p1 = s.hash_partition(42);
        let p2 = s.hash_partition(42);
        assert_eq!(p1, p2);
    }

    #[test]
    fn range_partition_boundaries() {
        let s = MockPartitionStrategy::new(PartitionKind::Range, 4);
        assert_eq!(s.range_partition(0, &[10, 20, 30]), 0);
        assert_eq!(s.range_partition(15, &[10, 20, 30]), 1);
        assert_eq!(s.range_partition(50, &[10, 20, 30]), 3);
    }

    #[test]
    fn round_robin_cycles() {
        let s = MockPartitionStrategy::new(PartitionKind::RoundRobin, 4);
        assert_eq!(s.round_robin_partition(0), 0);
        assert_eq!(s.round_robin_partition(1), 1);
        assert_eq!(s.round_robin_partition(2), 2);
        assert_eq!(s.round_robin_partition(3), 3);
        assert_eq!(s.round_robin_partition(4), 0); // wraps
    }

    #[test]
    fn worker_pool_throughput_scales() {
        let p1 = MockWorkerPool::new(1, 100);
        let p4 = MockWorkerPool::new(4, 100);
        assert_eq!(p4.throughput(), p1.throughput() * 4);
    }
}
