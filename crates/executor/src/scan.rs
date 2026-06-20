use sqlrustgo_planner::{PhysicalPlan, Schema};
use sqlrustgo_storage::predicate::Predicate;
use sqlrustgo_types::{SqlResult, Value};

pub trait ScanExecutor: Send {
    fn init(&mut self) -> SqlResult<()>;
    fn next(&mut self) -> SqlResult<Option<Vec<Value>>>;
    fn schema(&self) -> &Schema;
    fn name(&self) -> &str;
    fn close(&mut self) -> SqlResult<()>;
}

#[derive(Debug, Clone)]
pub struct ScanStats {
    pub rows_scanned: u64,
    pub rows_returned: u64,
    pub used_index: bool,
}

impl Default for ScanStats {
    fn default() -> Self {
        Self {
            rows_scanned: 0,
            rows_returned: 0,
            used_index: false,
        }
    }
}

pub trait IndexScanable {
    fn can_use_index(&self, predicate: &Predicate) -> bool;
    fn estimate_index_cost(&self, predicate: &Predicate) -> f64;
    fn estimate_seq_cost(&self) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock ScanExecutor that returns a fixed set of rows
    struct MockScanExecutor {
        data: Vec<Vec<Value>>,
        idx: usize,
        initialized: bool,
        schema: Schema,
    }

    impl MockScanExecutor {
        fn new(data: Vec<Vec<Value>>) -> Self {
            Self {
                data,
                idx: 0,
                initialized: false,
                schema: Schema::empty(),
            }
        }
    }

    impl ScanExecutor for MockScanExecutor {
        fn init(&mut self) -> SqlResult<()> {
            self.initialized = true;
            Ok(())
        }

        fn next(&mut self) -> SqlResult<Option<Vec<Value>>> {
            if self.idx >= self.data.len() {
                return Ok(None);
            }
            let row = self.data[self.idx].clone();
            self.idx += 1;
            Ok(Some(row))
        }

        fn schema(&self) -> &Schema {
            &self.schema
        }

        fn name(&self) -> &str {
            "MockScan"
        }

        fn close(&mut self) -> SqlResult<()> {
            self.initialized = false;
            self.idx = 0;
            Ok(())
        }
    }

    #[test]
    fn test_scan_executor_name() {
        let executor = MockScanExecutor::new(vec![]);
        assert_eq!(executor.name(), "MockScan");
    }

    #[test]
    fn test_scan_executor_schema() {
        let fields = vec![
            Field::new("id".to_string(), DataType::Integer),
            Field::new("name".to_string(), DataType::Text),
        ];
        let schema = Schema::new(fields);
        let executor = MockScanExecutor {
            data: vec![],
            idx: 0,
            initialized: false,
            schema,
        };
        assert_eq!(executor.schema().fields.len(), 2);
    }

    #[test]
    fn test_scan_executor_init() {
        let mut executor = MockScanExecutor::new(vec![]);
        assert!(!executor.init().unwrap());
    }

    #[test]
    fn test_scan_executor_next_empty() {
        let mut executor = MockScanExecutor::new(vec![]);
        executor.init().unwrap();
        let result = executor.next().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_executor_next_returns_rows() {
        let mut executor = MockScanExecutor::new(vec![
            vec![Value::Integer(1), Value::Text("a".to_string())],
            vec![Value::Integer(2), Value::Text("b".to_string())],
        ]);
        executor.init().unwrap();

        let row1 = executor.next().unwrap().unwrap();
        assert_eq!(row1[0], Value::Integer(1));
        assert_eq!(row1[1], Value::Text("a".to_string()));

        let row2 = executor.next().unwrap().unwrap();
        assert_eq!(row2[0], Value::Integer(2));
        assert_eq!(row2[1], Value::Text("b".to_string()));

        let row3 = executor.next().unwrap();
        assert!(row3.is_none());
    }

    #[test]
    fn test_scan_executor_close_resets() {
        let mut executor = MockScanExecutor::new(vec![
            vec![Value::Integer(1)],
        ]);
        executor.init().unwrap();
        executor.next().unwrap();
        executor.close().unwrap();
        executor.init().unwrap();
        let result = executor.next().unwrap().unwrap();
        assert_eq!(result[0], Value::Integer(1));
    }

    #[test]
    fn test_scan_stats_default() {
        let stats = ScanStats::default();
        assert_eq!(stats.rows_scanned, 0);
        assert_eq!(stats.rows_returned, 0);
        assert!(!stats.used_index);
    }

    #[test]
    fn test_scan_stats_update() {
        let mut stats = ScanStats::default();
        stats.rows_scanned = 100;
        stats.rows_returned = 50;
        stats.used_index = true;
        assert_eq!(stats.rows_scanned, 100);
        assert_eq!(stats.rows_returned, 50);
        assert!(stats.used_index);
    }
}
