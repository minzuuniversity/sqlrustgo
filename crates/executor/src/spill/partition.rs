//! Partitioned spill manager for hash join operations.
//!
//! Partitions data by hash bucket, spilling each partition independently.

use super::manager::{DiskSpillManager, Spillable};
use crate::chunk::Chunk;
use std::collections::HashMap;

/// Partitioned spill manager for hash join.
///
/// Manages multiple spill managers, one per partition. Each partition
/// is spilled independently to allow parallel processing during hash join.
pub struct PartitionedSpill {
    /// Number of partitions.
    num_partitions: usize,
    /// Spill managers for each partition.
    partitions: Vec<DiskSpillManager>,
    /// Current partition write indices.
    write_indices: HashMap<usize, usize>,
    /// Max memory per partition.
    max_memory_bytes: usize,
}

impl PartitionedSpill {
    /// Create a new PartitionedSpill with the given number of partitions.
    pub fn new(num_partitions: usize, max_memory_bytes: usize) -> std::io::Result<Self> {
        let mut partitions = Vec::with_capacity(num_partitions);
        for i in 0..num_partitions {
            let manager =
                DiskSpillManager::new(max_memory_bytes, &format!("partition_spill_{}", i))?;
            partitions.push(manager);
        }

        Ok(Self {
            num_partitions,
            partitions,
            write_indices: HashMap::new(),
            max_memory_bytes,
        })
    }

    /// Returns the number of partitions.
    pub fn num_partitions(&self) -> usize {
        self.num_partitions
    }

    /// Spill a chunk to the specified partition.
    pub fn spill_to_partition(
        &mut self,
        partition_id: usize,
        chunk: &Chunk,
    ) -> std::io::Result<()> {
        if partition_id >= self.num_partitions {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid partition id: {}", partition_id),
            ));
        }

        self.partitions[partition_id].spill_chunk(chunk)?;
        *self.write_indices.entry(partition_id).or_insert(0) += 1;
        Ok(())
    }

    /// Get the next chunk from a specific partition.
    pub fn next_chunk_for_partition(
        &mut self,
        partition_id: usize,
    ) -> std::io::Result<Option<Chunk>> {
        if partition_id >= self.num_partitions {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid partition id: {}", partition_id),
            ));
        }

        self.partitions[partition_id].next_chunk()
    }

    /// Get total spilled count across all partitions.
    pub fn total_spilled_count(&self) -> usize {
        self.partitions.iter().map(|p| p.spilled_count()).sum()
    }

    /// Get memory usage for a specific partition.
    pub fn partition_memory_usage(&self, partition_id: usize) -> usize {
        if partition_id < self.num_partitions {
            self.partitions[partition_id].current_memory_usage()
        } else {
            0
        }
    }

    /// Check if a partition is in spilling mode.
    pub fn is_partition_spilling(&self, partition_id: usize) -> bool {
        if partition_id < self.num_partitions {
            self.partitions[partition_id].spilled_count() > 0
        } else {
            false
        }
    }

    /// Close all partitions and clean up resources.
    pub fn close_all(&mut self) -> std::io::Result<()> {
        for partition in &mut self.partitions {
            partition.close()?;
        }
        self.write_indices.clear();
        Ok(())
    }

    /// Get the hash bucket for a value.
    /// Used to determine which partition to write to.
    pub fn hash_bucket(value: i64, num_partitions: usize) -> usize {
        let hash = (value as u64).wrapping_mul(0x9e3779b97f4a7c15);
        (hash % num_partitions as u64) as usize
    }
}

impl Drop for PartitionedSpill {
    fn drop(&mut self) {
        let _ = self.close_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, ColumnVector, PrimitiveType, PrimitiveValue};

    fn create_test_chunk_with_values(values: Vec<i64>) -> Chunk {
        let col = ColumnVector::from_int64(values);
        Chunk::new(vec![col])
    }

    #[test]
    fn test_partitioned_spill_basic() {
        let mut spill = PartitionedSpill::new(4, 1024).unwrap();

        let chunk1 = create_test_chunk_with_values(vec![1, 2, 3]);
        let chunk2 = create_test_chunk_with_values(vec![4, 5, 6]);

        // Spill to different partitions
        spill.spill_to_partition(0, &chunk1).unwrap();
        spill.spill_to_partition(2, &chunk2).unwrap();

        assert_eq!(spill.num_partitions(), 4);
        assert_eq!(spill.total_spilled_count(), 2);

        spill.close_all().unwrap();
    }

    #[test]
    fn test_hash_bucket() {
        let num_partitions = 8;

        // Same values should go to same partition
        let p1 = PartitionedSpill::hash_bucket(100, num_partitions);
        let p2 = PartitionedSpill::hash_bucket(100, num_partitions);
        assert_eq!(p1, p2);

        // Different values may go to different partitions
        let _p3 = PartitionedSpill::hash_bucket(200, num_partitions);
        // Just check it returns a valid partition
        assert!(p1 < num_partitions);
    }

    #[test]
    fn test_invalid_partition() {
        let mut spill = PartitionedSpill::new(4, 1024).unwrap();
        let chunk = create_test_chunk_with_values(vec![1, 2, 3]);

        let result = spill.spill_to_partition(10, &chunk);
        assert!(result.is_err());

        let result = spill.next_chunk_for_partition(10);
        assert!(result.is_err());

        spill.close_all().unwrap();
    }
}
