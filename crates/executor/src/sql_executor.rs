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
}