use sqlrustgo_parser::{parse, Statement};
use sqlrustgo_storage::Value;
use sqlrustgo_types::{SqlError, SqlResult};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub rows: Vec<Vec<Value>>,
    pub affected_rows: usize,
    pub started_transaction: bool,
    pub auto_committed: bool,
}

impl ExecutionResult {
    pub fn new(rows: Vec<Vec<Value>>, affected_rows: usize) -> Self {
        Self {
            rows,
            affected_rows,
            started_transaction: false,
            auto_committed: false,
        }
    }

    pub fn with_tx_metadata(
        rows: Vec<Vec<Value>>,
        affected_rows: usize,
        started_transaction: bool,
        auto_committed: bool,
    ) -> Self {
        Self {
            rows,
            affected_rows,
            started_transaction,
            auto_committed,
        }
    }

    pub fn empty() -> Self {
        Self {
            rows: vec![],
            affected_rows: 0,
            started_transaction: false,
            auto_committed: false,
        }
    }
}

pub trait SqlExecutor: Send + Sync {
    fn execute(&mut self, sql: &str) -> SqlResult<ExecutionResult>;

    fn begin(&mut self) -> Result<u64, SqlError>;

    fn commit(&mut self) -> Result<Option<u64>, SqlError>;

    fn rollback(&mut self) -> Result<(), SqlError>;

    fn is_in_transaction(&self) -> bool;

    fn current_tx_id(&self) -> Option<u64>;

    fn execute_read(&mut self, sql: &str) -> SqlResult<ExecutionResult> {
        let statement = parse(sql).map_err(|e| SqlError::ParseError(e.to_string()))?;
        match statement {
            Statement::Select(_) | Statement::Show(_) | Statement::Describe(_) => {
                self.execute_select_internal(sql)
            }
            _ => self.execute(sql),
        }
    }

    fn execute_select_internal(&mut self, _sql: &str) -> SqlResult<ExecutionResult> {
        self.execute(_sql)
    }

    fn name(&self) -> &'static str;
}

pub fn is_dml_statement(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Insert(_) | Statement::Update(_) | Statement::Delete(_)
    )
}

pub fn is_read_only_statement(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Select(_) | Statement::Show(_) | Statement::Describe(_)
    )
}

pub fn is_transaction_statement(stmt: &Statement) -> bool {
    matches!(stmt, Statement::Transaction(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dml_statement() {
        let insert = parse("INSERT INTO t VALUES (1)").unwrap();
        assert!(is_dml_statement(&insert));

        let update = parse("UPDATE t SET x = 1").unwrap();
        assert!(is_dml_statement(&update));

        let delete = parse("DELETE FROM t WHERE x = 1").unwrap();
        assert!(is_dml_statement(&delete));

        let select = parse("SELECT * FROM t").unwrap();
        assert!(!is_dml_statement(&select));
    }

    #[test]
    fn test_is_read_only_statement() {
        let select = parse("SELECT * FROM t").unwrap();
        assert!(is_read_only_statement(&select));

        let show = parse("SHOW TABLES").unwrap();
        assert!(is_read_only_statement(&show));

        let insert = parse("INSERT INTO t VALUES (1)").unwrap();
        assert!(!is_read_only_statement(&insert));
    }

    #[test]
    fn test_execution_result() {
        let result = ExecutionResult::new(vec![vec![Value::Integer(1)]], 0);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.affected_rows, 0);
        assert!(!result.started_transaction);
        assert!(!result.auto_committed);

        let result = ExecutionResult::with_tx_metadata(vec![], 1, true, true);
        assert_eq!(result.affected_rows, 1);
        assert!(result.started_transaction);
        assert!(result.auto_committed);
    }

    #[test]
    fn test_execution_result_empty() {
        let result = ExecutionResult::empty();
        assert!(result.rows.is_empty());
        assert_eq!(result.affected_rows, 0);
    }

    #[test]
    fn test_is_transaction_statement() {
        let begin = parse("BEGIN").unwrap();
        assert!(is_transaction_statement(&begin));

        let commit = parse("COMMIT").unwrap();
        assert!(is_transaction_statement(&commit));

        let rollback = parse("ROLLBACK").unwrap();
        assert!(is_transaction_statement(&rollback));

        let select = parse("SELECT * FROM t").unwrap();
        assert!(!is_transaction_statement(&select));

        let insert = parse("INSERT INTO t VALUES (1)").unwrap();
        assert!(!is_transaction_statement(&insert));
    }

    #[test]
    fn test_is_dml_statement_all_variants() {
        let insert = parse("INSERT INTO t VALUES (1, 2, 3)").unwrap();
        assert!(is_dml_statement(&insert));

        let update = parse("UPDATE t SET x = 1, y = 2 WHERE z = 3").unwrap();
        assert!(is_dml_statement(&update));

        let delete = parse("DELETE FROM t WHERE x = 1 AND y = 2").unwrap();
        assert!(is_dml_statement(&delete));
    }

    #[test]
    fn test_is_read_only_statement_all_variants() {
        let select = parse("SELECT * FROM t WHERE x = 1").unwrap();
        assert!(is_read_only_statement(&select));

        let show = parse("SHOW TABLES").unwrap();
        assert!(is_read_only_statement(&show));

        let describe = parse("DESCRIBE t").unwrap();
        assert!(is_read_only_statement(&describe));

        let create_table = parse("CREATE TABLE t (x INT)").unwrap();
        assert!(!is_read_only_statement(&create_table));

        let drop_table = parse("DROP TABLE t").unwrap();
        assert!(!is_read_only_statement(&drop_table));
    }

    #[test]
    fn test_execution_result_null_values() {
        let result = ExecutionResult::new(vec![
            vec![Value::Null, Value::Integer(1)],
            vec![Value::Text("test".to_string()), Value::Null],
        ], 0);
        assert_eq!(result.rows.len(), 2);
        assert!(matches!(result.rows[0][0], Value::Null));
        assert!(matches!(result.rows[1][1], Value::Null));
    }

    #[test]
    fn test_execution_result_large_affected_rows() {
        let result = ExecutionResult::new(vec![], 1_000_000);
        assert_eq!(result.affected_rows, 1_000_000);
    }

    #[test]
    fn test_execution_result_with_blob() {
        let result = ExecutionResult::new(
            vec![vec![Value::Blob(vec![0xDE, 0xAD, 0xBE, 0xEF])]],
            0,
        );
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execution_result_with_float() {
        let result = ExecutionResult::new(vec![
            vec![Value::Float(3.14), Value::Integer(1)],
            vec![Value::Float(-2.5), Value::Integer(2)],
        ], 0);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execution_result_clone() {
        let result = ExecutionResult::new(vec![vec![Value::Integer(1)]], 1);
        let cloned = result.clone();
        assert_eq!(cloned.rows.len(), 1);
        assert_eq!(cloned.affected_rows, 1);
    }

    #[test]
    fn test_sql_executor_trait_send_sync() {
        fn _check<T: Send + Sync>() {}
        _check::<ExecutionResult>();
    }
}
