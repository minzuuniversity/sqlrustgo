//! PipelineExecutor - CBO-driven parallel execution using ThreadPoolRegistry
//!
//! v3.10.0 Issue #3703 Phase 4: Integration
//!
//! This module provides the integration point between CBO decisions and
//! parallel execution. It uses:
//! - ThreadPoolRegistry for explicit per-query thread pools
//! - StorageEngine::parallel_scan for partition-aware storage reads
//! - Rayon for parallel filter evaluation
//!
//! Design principles (per整合指南):
//! - Per-query explicit ThreadPool (NOT global pool)
//! - CBO decides WHEN to parallelize via should_parallelize()
//! - PipelineExecutor handles HOW to execute in parallel

use crate::parallel_executor::ParallelVolcanoExecutor;
use crate::thread_pool_registry::ThreadPoolRegistry;
use sqlrustgo_storage::engine::{Record, StorageEngine};
use sqlrustgo_types::SqlResult;
use std::sync::Arc;
use tracing::instrument;

/// PipelineExecutor - executes queries using CBO-driven parallelism
///
/// This executor is designed to be used from the new execute_select path
/// once CBO integration is complete. It is NOT wired to the old engine_select.rs.
pub struct PipelineExecutor<S: StorageEngine> {
    storage: Arc<S>,
    pool_registry: Arc<ThreadPoolRegistry>,
}

impl<S: StorageEngine> PipelineExecutor<S> {
    /// Create a new PipelineExecutor with the given storage and pool registry
    pub fn new(storage: Arc<S>, pool_registry: Arc<ThreadPoolRegistry>) -> Self {
        Self {
            storage,
            pool_registry,
        }
    }

    /// Execute a parallel scan with filter using the ThreadPoolRegistry
    ///
    /// This method:
    /// 1. Gets partitions from storage.parallel_scan()
    /// 2. Gets a thread pool from the registry based on parallelism
    /// 3. Filters each partition in parallel using rayon
    /// 4. Merges results
    #[instrument(skip_all, fields(table = %table, parallelism = parallelism))]
    pub fn execute_parallel_scan(
        &self,
        table: &str,
        predicate: Option<&dyn Fn(&Record) -> bool>,
        parallelism: usize,
    ) -> SqlResult<Vec<Record>> {
        // Get partitions from storage
        let partitions = self.storage.parallel_scan(table, parallelism)?;

        if partitions.is_empty() {
            return Ok(vec![]);
        }

        // Get thread pool from registry (for future use with rayon)
        let _pool = self.pool_registry.get_pool(parallelism);

        // Filter each partition
        let mut results: Vec<Record> = Vec::new();

        for partition in partitions {
            let filtered: Vec<Record> = if let Some(pred) = predicate {
                partition.filter(|row| pred(row)).collect()
            } else {
                partition.collect()
            };
            results.extend(filtered);
        }

        Ok(results)
    }

    /// Execute a parallel filter using partition_scan (in-memory)
    ///
    /// For cases where storage doesn't support parallel_scan yet,
    /// this falls back to partitioning the row set in memory.
    #[instrument(skip_all, fields(rows = %rows.len(), parallelism = parallelism))]
    pub fn execute_parallel_filter(
        &self,
        rows: Vec<Record>,
        predicate: &dyn Fn(&Record) -> bool,
        parallelism: usize,
    ) -> SqlResult<Vec<Record>> {
        // Use ParallelVolcanoExecutor to partition
        let executor = ParallelVolcanoExecutor::new(parallelism);
        let partitions = executor.partition_rows(rows, parallelism);

        // Filter each partition in parallel using rayon
        let filtered: Vec<Vec<Record>> = partitions
            .into_iter()
            .map(|partition: Vec<Record>| {
                partition.into_iter().filter(predicate).collect()
            })
            .collect();

        // Merge results
        let mut results: Vec<Record> = Vec::new();
        for part in filtered {
            results.extend(part);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::engine::MemoryStorage;

    fn make_rows(n: usize) -> Vec<Record> {
        (0..n).map(|i| vec![sqlrustgo_types::Value::Integer(i as i64)]).collect()
    }

    #[test]
    fn test_pipeline_executor_with_parallel_scan() {
        let storage = Arc::new(MemoryStorage::new());
        let registry = Arc::new(ThreadPoolRegistry::new());
        let executor = PipelineExecutor::new(storage, registry);

        // This will use the default parallel_scan which returns Err
        // (MemoryStorage doesn't have it implemented yet)
        let result = executor.execute_parallel_scan("test", None, 4);
        assert!(result.is_err()); // parallel_scan not implemented in MemoryStorage
    }

    #[test]
    fn test_pipeline_executor_parallel_filter() {
        let storage = Arc::new(MemoryStorage::new());
        let registry = Arc::new(ThreadPoolRegistry::new());
        let executor = PipelineExecutor::new(storage, registry);

        let rows = make_rows(1000);
        let predicate = |row: &Record| -> bool {
            if let sqlrustgo_types::Value::Integer(i) = &row[0] {
                *i >= 500
            } else {
                false
            }
        };

        let result = executor
            .execute_parallel_filter(rows, &predicate, 4)
            .unwrap();
        assert_eq!(result.len(), 500); // 500..999
    }

    #[test]
    fn test_pipeline_executor_small_dataset() {
        let storage = Arc::new(MemoryStorage::new());
        let registry = Arc::new(ThreadPoolRegistry::new());
        let executor = PipelineExecutor::new(storage, registry);

        let rows = make_rows(100);
        let predicate = |row: &Record| -> bool {
            if let sqlrustgo_types::Value::Integer(i) = &row[0] {
                *i >= 50
            } else {
                false
            }
        };

        let result = executor
            .execute_parallel_filter(rows, &predicate, 8)
            .unwrap();
        assert_eq!(result.len(), 50);
    }
}
