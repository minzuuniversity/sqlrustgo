use sqlrustgo_types::Value;

pub const PARALLEL_MIN_ROWS: usize = 100_000;

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
        let rows = make_rows(400_000);
        let parts = exec.partition_scan(rows, 4);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 400_000);
    }

    #[test]
    fn test_partition_scan_uneven_remainder() {
        let exec = ParallelVolcanoExecutor::new(3);
        let rows = make_rows(200_000);
        let parts = exec.partition_scan(rows, 3);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 200_000);
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
}
