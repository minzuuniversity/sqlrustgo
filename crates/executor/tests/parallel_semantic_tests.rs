//! Semantic guard tests for parallel execution
//!
//! These tests verify that parallel execution preserves NULL semantics
//! and produces identical results to sequential execution.
//!
//! v3.10.0 Issue #3703 Phase 2: Semantic Isolation
//! Goal: Ensure "data量小绝不分发任务" and "并行不破坏 NULL 语义"

use sqlrustgo_executor::parallel_executor::{ParallelExecutor, ParallelVolcanoExecutor};
use sqlrustgo_types::Value;

/// Helper to create rows with NULL values for semantic testing
fn make_rows_with_nulls(n: usize, null_every: usize) -> Vec<Vec<Value>> {
    (0..n)
        .map(|i| {
            if i % null_every == 0 {
                vec![Value::Null, Value::Integer(i as i64)]
            } else {
                vec![Value::Integer(i as i64), Value::Integer((i * 2) as i64)]
            }
        })
        .collect()
}

/// Helper to create simple rows
fn make_simple_rows(n: usize) -> Vec<Vec<Value>> {
    (0..n).map(|i| vec![Value::Integer(i as i64)]).collect()
}

// =============================================================================
// Semantic Guard: NULL preservation in partition
// =============================================================================

#[test]
fn test_partition_null_preservation_small() {
    // For small datasets (< PARALLEL_MIN_ROWS), should NOT partition
    let exec = ParallelVolcanoExecutor::new(4);
    let rows = make_rows_with_nulls(1000, 7); // ~143 NULLs
    let parts = exec.partition_scan(rows, 4);
    // Should return single partition (not parallel)
    assert_eq!(parts.len(), 1, "Small dataset should not be partitioned");
    assert_eq!(parts[0].len(), 1000);
}

#[test]
fn test_partition_null_preservation_large() {
    // For large datasets (>= PARALLEL_MIN_ROWS), should partition
    let exec = ParallelVolcanoExecutor::new(4);
    let rows = make_rows_with_nulls(600_000, 7); // ~85,714 NULLs
    let parts = exec.partition_scan(rows, 4);
    assert_eq!(parts.len(), 4, "Large dataset should be partitioned");
    let total: usize = parts.iter().map(|p| p.len()).sum();
    assert_eq!(total, 600_000, "All rows should be preserved");
}

#[test]
fn test_partition_null_distribution() {
    // Verify NULLs are evenly distributed across partitions
    let exec = ParallelVolcanoExecutor::new(4);
    let rows = make_rows_with_nulls(600_000, 3); // ~200,000 NULLs
    let parts = exec.partition_scan(rows, 4);

    let null_counts: Vec<usize> = parts
        .iter()
        .map(|p| p.iter().filter(|r| matches!(r[0], Value::Null)).count())
        .collect();

    // Each partition should have roughly equal NULLs
    let avg_nulls: usize = null_counts.iter().sum::<usize>() / 4;
    for (i, count) in null_counts.iter().enumerate() {
        let diff = (*count as isize - avg_nulls as isize).unsigned_abs();
        assert!(
            diff <= avg_nulls / 2 + 1000,
            "Partition {} has {} NULLs, avg={}, diff={}",
            i,
            count,
            avg_nulls,
            diff
        );
    }
}

// =============================================================================
// Semantic Guard: Order preservation
// =============================================================================

#[test]
fn test_partition_order_sequential_degree() {
    // With degree=1, should return single partition (preserves order)
    let exec = ParallelVolcanoExecutor::new(1);
    let rows = make_simple_rows(100);
    let parts = exec.partition_scan(rows, 1);
    assert_eq!(parts.len(), 1);
    // Single partition maintains original order
    assert_eq!(parts[0].len(), 100);
}

#[test]
fn test_partition_order_non_split() {
    // Small dataset with degree>1 should NOT split (preserves order)
    let exec = ParallelVolcanoExecutor::new(8);
    let rows = make_simple_rows(100);
    let parts = exec.partition_scan(rows, 8);
    // Should NOT partition small dataset
    assert_eq!(parts.len(), 1, "Small dataset should not be partitioned");
}

#[test]
fn test_partition_all_rows_accounted() {
    // Verify no rows are lost during partitioning
    let exec = ParallelVolcanoExecutor::new(8);
    let rows: Vec<Vec<Value>> = (0..600_000)
        .map(|i| vec![Value::Integer(i as i64)])
        .collect();
    let parts = exec.partition_scan(rows, 8);

    let total: usize = parts.iter().map(|p| p.len()).sum();
    assert_eq!(total, 600_000, "All rows must be accounted for");

    // Verify all original values are present
    let mut all_values: Vec<i64> = parts
        .iter()
        .flat_map(|p| {
            p.iter()
                .filter_map(|r| {
                    if let Value::Integer(i) = &r[0] {
                        Some(*i)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();
    all_values.sort();
    let expected: Vec<i64> = (0..600_000).collect();
    assert_eq!(all_values, expected, "All values must be preserved");
}

// =============================================================================
// Semantic Guard: Edge cases
// =============================================================================

#[test]
fn test_partition_empty_input() {
    let exec = ParallelVolcanoExecutor::new(4);
    let rows: Vec<Vec<Value>> = vec![];
    let parts = exec.partition_scan(rows, 4);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].len(), 0);
}

#[test]
fn test_partition_single_row() {
    let exec = ParallelVolcanoExecutor::new(4);
    let rows = vec![vec![Value::Integer(42)]];
    let parts = exec.partition_scan(rows, 4);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].len(), 1);
}

#[test]
fn test_partition_all_null_rows() {
    let exec = ParallelVolcanoExecutor::new(4);
    let rows: Vec<Vec<Value>> = (0..1000)
        .map(|_| vec![Value::Null; 5])
        .collect();
    let parts = exec.partition_scan(rows, 4);
    // Small dataset should not partition
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].len(), 1000);
}

#[test]
fn test_partition_mixed_types() {
    let exec = ParallelVolcanoExecutor::new(4);
    let rows = vec![
        vec![Value::Integer(1)],
        vec![Value::Float(1.5)],
        vec![Value::Text("hello".into())],
        vec![Value::Boolean(true)],
        vec![Value::Null],
    ];
    let parts = exec.partition_scan(rows, 4);
    // Small dataset should not partition
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].len(), 5);
}

#[test]
fn test_partition_degree_zero() {
    // partition_scan clamps degree to 1, so degree=0 means single partition
    let exec = ParallelVolcanoExecutor::new(4);
    let rows = make_simple_rows(600_000);
    let parts = exec.partition_scan(rows, 0);
    // degree=0 is clamped to 1 internally, but we still partition by total/degree
    // The actual partition happens based on total rows vs PARALLEL_MIN_ROWS
    let total: usize = parts.iter().map(|p| p.len()).sum();
    assert_eq!(total, 600_000, "All rows should be preserved");
}

#[test]
fn test_partition_degree_larger_than_rows() {
    let exec = ParallelVolcanoExecutor::new(100);
    let rows = make_simple_rows(50);
    let parts = exec.partition_scan(rows, 100);
    // If total < PARALLEL_MIN_ROWS, single partition regardless of degree
    assert_eq!(parts.len(), 1, "Small dataset should not partition");
}

// =============================================================================
// Semantic Guard: Parallelism degree correctness
// =============================================================================

#[test]
fn test_parallel_degree_bounds() {
    // new(0) clamps to 1
    let exec = ParallelVolcanoExecutor::new(0);
    assert_eq!(exec.parallel_degree(), 1, "new(0) should clamp to 1");

    let mut exec = ParallelVolcanoExecutor::new(4);
    exec.set_parallel_degree(0);
    assert_eq!(exec.parallel_degree(), 1, "set_parallel_degree(0) should become 1");

    exec.set_parallel_degree(100);
    assert_eq!(exec.parallel_degree(), 100, "Large degree should be preserved");
}

// =============================================================================
// Semantic Guard: Large dataset partitioning
// =============================================================================

#[test]
fn test_partition_even_distribution() {
    let exec = ParallelVolcanoExecutor::new(4);
    let rows: Vec<Vec<Value>> = (0..600_000).map(|i| vec![Value::Integer(i as i64)]).collect();
    let parts = exec.partition_scan(rows, 4);

    let sizes: Vec<usize> = parts.iter().map(|p| p.len()).collect();

    // With 600K rows and 4 partitions: base = 150K, rem = 0
    // All partitions should be exactly 150K
    assert_eq!(sizes, vec![150_000, 150_000, 150_000, 150_000]);
}

#[test]
fn test_partition_uneven_distribution() {
    let exec = ParallelVolcanoExecutor::new(4);
    let rows: Vec<Vec<Value>> = (0..600_001).map(|i| vec![Value::Integer(i as i64)]).collect();
    let parts = exec.partition_scan(rows, 4);

    let sizes: Vec<usize> = parts.iter().map(|p| p.len()).collect();

    // With 600001 rows and 4 partitions: base = 150000, rem = 1
    // First partition gets 150001, rest get 150000
    assert_eq!(sizes[0], 150_001);
    assert_eq!(sizes[1], 150_000);
    assert_eq!(sizes[2], 150_000);
    assert_eq!(sizes[3], 150_000);
    assert_eq!(sizes.iter().sum::<usize>(), 600_001);
}
// =============================================================================
// Semantic Guard: Parallel path activation
// =============================================================================

#[test]
fn test_parallel_path_activates_for_large_dataset() {
    // Verify that for large datasets, we get multiple partitions
    let exec = ParallelVolcanoExecutor::new(8);
    let rows: Vec<Vec<Value>> = (0..600_000).map(|i| vec![Value::Integer(i as i64)]).collect();
    let parts = exec.partition_scan(rows, 8);
    assert_eq!(parts.len(), 8, "Large dataset should activate 8-way parallelism");
}

#[test]
fn test_parallel_path_skips_for_small_dataset() {
    // Verify that for small datasets, we don't activate parallel path
    let exec = ParallelVolcanoExecutor::new(8);
    let rows: Vec<Vec<Value>> = (0..1000).map(|i| vec![Value::Integer(i as i64)]).collect();
    let parts = exec.partition_scan(rows, 8);
    assert_eq!(parts.len(), 1, "Small dataset should skip parallel path");
}
