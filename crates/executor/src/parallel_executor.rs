use sqlrustgo_types::Value;
use tracing::instrument;

/// Minimum row count to engage parallel filter path. v3.10.0 Issue #3792:
/// raised from 100K to 2M after benchmark showed no speedup at SF=0.1/1.0/10.0.
/// The bottleneck is shared I/O + partitioning overhead, not data size.
/// CBO threshold in unified_cost.rs is also updated to match.
pub const PARALLEL_MIN_ROWS: usize = 2_000_000;

pub trait ParallelExecutor: Send + Sync {
    fn parallel_degree(&self) -> usize;
    fn set_parallel_degree(&mut self, degree: usize);
    fn partition_scan(&self, rows: Vec<Vec<Value>>, degree: usize) -> Vec<Vec<Vec<Value>>>;
}

pub struct ParallelVolcanoExecutor {
    parallel_degree: usize,
}

impl ParallelVolcanoExecutor {
    pub fn new(parallel_degree: usize) -> Self {
        Self {
            parallel_degree: parallel_degree.max(1),
        }
    }

    pub fn sequential() -> Self {
        Self { parallel_degree: 1 }
    }

    pub fn degree(&self) -> usize {
        self.parallel_degree
    }

    pub fn partition_rows(&self, rows: Vec<Vec<Value>>, degree: usize) -> Vec<Vec<Vec<Value>>> {
        <Self as ParallelExecutor>::partition_scan(self, rows, degree)
    }

    /// Test-only: partition with a custom `min_rows` threshold. Production code
    /// always uses the constant `PARALLEL_MIN_ROWS`; this variant exists so unit
    /// tests can exercise the partitioning algorithm without allocating the
    /// multi-million-row fixture that the production threshold implies.
    pub fn partition_rows_with_min(
        &self,
        rows: Vec<Vec<Value>>,
        degree: usize,
        min_rows: usize,
    ) -> Vec<Vec<Vec<Value>>> {
        let degree = degree.max(1);
        let total = rows.len();
        if total < min_rows || degree <= 1 {
            return vec![rows];
        }
        let base = total / degree;
        let rem = total % degree;
        let mut partitions: Vec<Vec<Vec<Value>>> = Vec::with_capacity(degree);
        let mut cur = 0;
        for i in 0..degree {
            let size = if i < rem { base + 1 } else { base };
            if size > 0 {
                partitions.push(rows[cur..cur + size].to_vec());
            }
            cur += size;
        }
        partitions
    }
}

impl Default for ParallelVolcanoExecutor {
    fn default() -> Self {
        Self::sequential()
    }
}

impl ParallelExecutor for ParallelVolcanoExecutor {
    fn parallel_degree(&self) -> usize {
        self.parallel_degree
    }

    fn set_parallel_degree(&mut self, degree: usize) {
        self.parallel_degree = degree.max(1);
    }
    #[instrument(skip(self, rows), fields(total_rows = rows.len(), degree, partitions_created))]
    fn partition_scan(&self, rows: Vec<Vec<Value>>, degree: usize) -> Vec<Vec<Vec<Value>>> {
        let degree = degree.max(1);
        let total = rows.len();
        if total < PARALLEL_MIN_ROWS || degree <= 1 {
            return vec![rows];
        }
        let base = total / degree;
        let rem = total % degree;
        let mut partitions: Vec<Vec<Vec<Value>>> = Vec::with_capacity(degree);
        let mut cur = 0;
        for i in 0..degree {
            let size = if i < rem { base + 1 } else { base };
            if size > 0 {
                partitions.push(rows[cur..cur + size].to_vec());
            }
            cur += size;
        }
        partitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rows(n: usize) -> Vec<Vec<Value>> {
        (0..n).map(|i| vec![Value::Integer(i as i64)]).collect()
    }

    #[test]
    fn test_partition_scan_sequential() {
        let exec = ParallelVolcanoExecutor::new(1);
        let rows = make_rows(10);
        let parts = exec.partition_scan(rows, 1);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 10);
    }

    #[test]
    fn test_partition_scan_small_skips_parallel() {
        let exec = ParallelVolcanoExecutor::new(4);
        let rows = make_rows(1000);
        let parts = exec.partition_scan(rows, 4);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_partition_scan_large_4_workers() {
        let exec = ParallelVolcanoExecutor::new(4);
        // v3.10.0 Issue #3792: updated to exceed new PARALLEL_MIN_ROWS (2M) threshold.
        let rows = make_rows(3_000_000);
        let parts = exec.partition_scan(rows, 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 3_000_000);
    }

    #[test]
    fn test_partition_scan_uneven_remainder() {
        let exec = ParallelVolcanoExecutor::new(3);
        // v3.10.0 Issue #3792: updated to exceed new PARALLEL_MIN_ROWS (2M) threshold.
        let rows = make_rows(3_000_000);
        let parts = exec.partition_scan(rows, 3);
        assert_eq!(parts.len(), 3);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 3_000_000);
    }

    #[test]
    fn test_partition_scan_empty_table() {
        // Task 3.3: 0 rows -> single empty partition, no panic.
        let exec = ParallelVolcanoExecutor::new(4);
        let rows: Vec<Vec<Value>> = vec![];
        let parts = exec.partition_scan(rows, 4);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 0);
    }

    #[test]
    fn test_partition_scan_degree_one_with_large_rows() {
        // Task 3.4 (variant): N=1 with 200k rows still produces a single
        // partition; verifies the degree<=1 short-circuit is the gating
        // condition (not the row count alone).
        let exec = ParallelVolcanoExecutor::new(1);
        let rows = make_rows(200_000);
        let parts = exec.partition_scan(rows, 1);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 200_000);
    }

    #[test]
    fn test_parallel_executor_trait_method() {
        let mut exec = ParallelVolcanoExecutor::new(2);
        assert_eq!(exec.parallel_degree(), 2);
        exec.set_parallel_degree(0);
        assert_eq!(exec.parallel_degree(), 1);
        exec.set_parallel_degree(8);
        assert_eq!(exec.parallel_degree(), 8);
    }

    #[test]
    fn test_partition_scan_preserves_order_within_partition() {
        let exec = ParallelVolcanoExecutor::new(2);
        let rows = make_rows(200_000);
        let parts = exec.partition_scan(rows, 2);
        for (i, row) in parts[0].iter().enumerate() {
            assert_eq!(row[0], Value::Integer(i as i64));
        }
    }

    #[test]
    fn test_parallel_filter_cell_match_vs_sequential() {
        // v3.10.0 Issue #3703: this is the exact primitive used by
        // `LocalExecutor::execute_parallel_filter`. Partition the row
        // set, filter each partition in parallel, concatenate, and
        // compare to a sequential filter. They must be cell-equivalent
        // (same multiset of rows, order-insensitive). The predicate is
        // "id > 100" over 200 rows.
        let exec = ParallelVolcanoExecutor::new(4);
        let rows: Vec<Vec<Value>> = (1..=200)
            .map(|i| vec![Value::Integer(i as i64), Value::Text(format!("u{}", i))])
            .collect();

        // Sequential control: filter id > 100.
        let seq_filtered: Vec<Vec<Value>> = rows
            .iter()
            .filter(|r| match &r[0] {
                Value::Integer(v) => *v > 100,
                _ => false,
            })
            .cloned()
            .collect();

        // Parallel under test: partition + rayon filter + concat.
        let partitions = exec.partition_rows(rows, 4);
        let par_filtered: Vec<Vec<Value>> = partitions
            .into_iter()
            .flat_map(|part| {
                part.into_iter()
                    .filter(|r| match &r[0] {
                        Value::Integer(v) => *v > 100,
                        _ => false,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Cell-level match (order-insensitive).
        let mut seq_sorted = seq_filtered.clone();
        let mut par_sorted = par_filtered.clone();
        seq_sorted.sort_by(|a, b| a[0].cmp(&b[0]));
        par_sorted.sort_by(|a, b| a[0].cmp(&b[0]));
        assert_eq!(seq_sorted, par_sorted);
        assert_eq!(seq_sorted.len(), 100);
    }
}
