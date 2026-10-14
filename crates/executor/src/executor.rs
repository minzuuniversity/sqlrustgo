//! Executor trait - abstraction for query execution
//! Decouples the execution layer for remote execution support

use sqlrustgo_planner::PhysicalPlan;
use sqlrustgo_types::{SqlResult, Value};

/// Execution result containing rows and metadata
#[derive(Debug, Clone)]
pub struct ExecutorResult {
    /// Result rows
    pub rows: Vec<Vec<Value>>,
    /// Number of affected rows (for INSERT/UPDATE/DELETE)
    pub affected_rows: usize,
    /// #5271: the output schema — one entry per result column, in order.
    ///
    /// This exists because a result's shape must not be inferred from its
    /// rows. A query returning zero rows still has columns: `SELECT * FROM t`
    /// over an empty `t(id INT, v INT)` is a two-column result with no rows,
    /// and the protocol layer needs both facts to build a result set. Reading
    /// `rows.first()` to learn the column count gets that exactly backwards.
    ///
    /// Empty means "no result set" (DDL/DML command replies), which is
    /// legitimate. It must never be inferred from an empty `rows`.
    ///
    /// `data_type: None` means the column exists but its semantic type is not
    /// resolved yet — never that the column is missing, and never a licence to
    /// drop the column.
    pub columns: Vec<ResultColumn>,
}

/// #5271: one column of a result set's output schema.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResultColumn {
    /// The column name the client sees.
    pub name: String,
    /// The column's semantic type, when known.
    pub data_type: Option<String>,
}

impl ResultColumn {
    /// A column whose type has not been resolved.
    pub fn unnamed_type(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: None,
        }
    }
}

impl ExecutorResult {
    /// Create a new executor result
    pub fn new(rows: Vec<Vec<Value>>, affected_rows: usize) -> Self {
        Self {
            rows,
            affected_rows,
            columns: Vec::new(),
        }
    }

    /// #5271: a result set with a known output schema.
    ///
    /// `columns` is the authoritative shape. `rows` is checked against it so a
    /// caller that assembles them inconsistently fails here rather than
    /// producing a result set whose Column Count disagrees with its rows.
    pub fn with_columns(
        columns: Vec<ResultColumn>,
        rows: Vec<Vec<Value>>,
        affected_rows: usize,
    ) -> Self {
        debug_assert!(
            columns.is_empty() || rows.iter().all(|r| r.len() == columns.len()),
            "row width {} does not match the {} declared columns",
            rows.first().map(|r| r.len()).unwrap_or(0),
            columns.len(),
        );
        Self {
            rows,
            affected_rows,
            columns,
        }
    }

    /// Create an empty result
    pub fn empty() -> Self {
        Self {
            rows: vec![],
            affected_rows: 0,
            columns: Vec::new(),
        }
    }
}

/// Executor trait - abstraction for executing physical plans
/// Enables decoupling execution layer and supports remote execution
pub trait Executor: Send + Sync {
    /// Execute a physical plan and return results
    fn execute(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult>;

    /// Get executor name
    fn name(&self) -> &str;

    /// Check if executor is ready
    fn is_ready(&self) -> bool;
}

/// Volcano-style iterator executor trait for streaming query execution
/// Provides a pull-based streaming model where each call to next()
/// returns the next batch of rows
pub trait VolcanoExecutor: Send + Sync {
    /// Initialize the executor and prepare for iteration
    fn init(&mut self) -> SqlResult<()>;

    /// Get the next batch of rows
    /// Returns None when iteration is complete
    fn next(&mut self) -> SqlResult<Option<ExecutorResult>>;

    /// Clean up resources after iteration is complete
    fn close(&mut self) -> SqlResult<()>;

    /// Get the schema of the output
    fn schema(&self) -> &sqlrustgo_planner::Schema;

    /// Get the name of this executor
    fn name(&self) -> &str;

    /// Check if the executor has been initialized
    fn is_initialized(&self) -> bool;

    /// Downcast to concrete type for type-specific operations
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_planner::PhysicalPlan;

    /// Mock executor for testing
    pub struct MockExecutor;

    impl MockExecutor {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for MockExecutor {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Executor for MockExecutor {
        fn execute(&self, _plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
            Ok(ExecutorResult::empty())
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn is_ready(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_executor_trait() {
        let _executor: Box<dyn Executor> = Box::new(MockExecutor::new());
        assert!(MockExecutor::new().is_ready());
        assert_eq!(MockExecutor::new().name(), "mock");
    }

    #[test]
    fn test_executor_result() {
        let result = ExecutorResult::new(vec![], 0);
        assert!(result.rows.is_empty());

        let result = ExecutorResult::new(vec![vec![Value::Integer(1)]], 1);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.affected_rows, 1);
    }

    #[test]
    fn test_executor_result_empty() {
        let result = ExecutorResult::empty();
        assert!(result.rows.is_empty());
        assert_eq!(result.affected_rows, 0);
    }

    #[test]
    fn test_executor_result_with_rows() {
        let rows = vec![
            vec![Value::Integer(1), Value::Text("Alice".to_string())],
            vec![Value::Integer(2), Value::Text("Bob".to_string())],
        ];
        let result = ExecutorResult::new(rows.clone(), 0);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_executor_result_affected_rows() {
        let result = ExecutorResult::new(vec![], 100);
        assert_eq!(result.affected_rows, 100);
    }

    #[test]
    fn test_executor_send_sync() {
        fn _check<T: Send + Sync>() {}
        _check::<MockExecutor>();
        _check::<ExecutorResult>();
    }

    #[test]
    fn test_executor_result_clone() {
        let result = ExecutorResult::new(vec![vec![Value::Integer(1)]], 1);
        let cloned = result.clone();
        assert_eq!(cloned.rows.len(), 1);
    }

    #[test]
    fn test_mock_executor_name() {
        let executor = MockExecutor::new();
        assert_eq!(executor.name(), "mock");
    }

    #[test]
    fn test_mock_executor_is_ready() {
        let executor = MockExecutor::new();
        assert!(executor.is_ready());
    }

    #[test]
    fn test_executor_result_with_null() {
        let rows = vec![
            vec![Value::Null, Value::Integer(1)],
            vec![Value::Text("test".to_string()), Value::Null],
        ];
        let result = ExecutorResult::new(rows, 0);
        assert_eq!(result.rows.len(), 2);
        assert!(matches!(result.rows[0][0], Value::Null));
    }

    #[test]
    fn test_executor_result_with_float() {
        let rows = vec![
            vec![Value::Float(3.14), Value::Integer(1)],
            vec![Value::Float(-2.5), Value::Integer(2)],
        ];
        let result = ExecutorResult::new(rows, 0);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_executor_result_with_blob() {
        let rows = vec![vec![Value::Blob(vec![0xDE, 0xAD, 0xBE, 0xEF])]];
        let result = ExecutorResult::new(rows, 0);
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_executor_result_large_affected_rows() {
        let result = ExecutorResult::new(vec![], 1_000_000);
        assert_eq!(result.affected_rows, 1_000_000);
    }

    #[test]
    fn test_executor_result_zero_rows() {
        let result = ExecutorResult::new(vec![], 0);
        assert!(result.rows.is_empty());
        assert_eq!(result.affected_rows, 0);
    }
}

#[cfg(test)]
mod result_schema_tests {
    use super::*;

    /// #5271: a result set's shape is a property of the schema, not of the
    /// rows. A query over an empty table has no rows and still has columns —
    /// if the protocol layer has to infer the column count from `rows`, this
    /// is exactly the case it gets wrong.
    #[test]
    fn zero_rows_with_two_columns_keeps_its_columns() {
        let result = ExecutorResult::with_columns(
            vec![
                ResultColumn {
                    name: "id".into(),
                    data_type: Some("INTEGER".into()),
                },
                ResultColumn {
                    name: "v".into(),
                    data_type: Some("INTEGER".into()),
                },
            ],
            vec![],
            0,
        );
        assert_eq!(result.columns.len(), 2);
        assert!(result.rows.is_empty());
    }

    /// `columns` empty means "no result set" — legitimate for DDL and DML, and
    /// must not be confused with a result set that happens to have no rows.
    #[test]
    fn command_replies_have_no_columns() {
        let r = ExecutorResult::new(vec![], 3);
        assert!(r.columns.is_empty());
        assert_eq!(r.affected_rows, 3);
    }

    /// A column whose type is not resolved yet is still a column.
    #[test]
    fn unresolved_type_does_not_remove_the_column() {
        let r = ExecutorResult::with_columns(
            vec![ResultColumn::unnamed_type("expr_1")],
            vec![vec![Value::Integer(1)]],
            1,
        );
        assert_eq!(r.columns.len(), 1);
        assert_eq!(r.columns[0].name, "expr_1");
        assert_eq!(r.columns[0].data_type, None);
    }
}
