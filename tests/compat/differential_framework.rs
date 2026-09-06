use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected_rows: u64,
    pub error: Option<String>,
    pub error_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct TestStep {
    pub sql: String,
    pub expected_result: Option<SqlResult>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DifferentialConfig {
    /// Whether to compare column metadata
    pub compare_columns: bool,
    /// Whether to compare affected rows
    pub compare_affected_rows: bool,
    /// Whether to compare error codes
    pub compare_error_codes: bool,
    /// Maximum rows to compare (0 = unlimited)
    pub max_rows: usize,
    /// Whether to sort rows before comparison
    pub sort_rows: bool,
}

impl Default for DifferentialConfig {
    fn default() -> Self {
        Self {
            compare_columns: true,
            compare_affected_rows: true,
            compare_error_codes: true,
            max_rows: 1000,
            sort_rows: true,
        }
    }
}

pub struct DifferentialTest {
    steps: Vec<TestStep>,
    config: DifferentialConfig,
    setup_sql: Vec<String>,
}

impl DifferentialTest {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            config: DifferentialConfig::default(),
            setup_sql: Vec::new(),
        }
    }

    pub fn sql(mut self, sql: &str) -> Self {
        self.steps.push(TestStep {
            sql: sql.to_string(),
            expected_result: None,
            description: None,
        });
        self
    }

    pub fn sql_with_result(mut self, sql: &str, expected: SqlResult) -> Self {
        self.steps.push(TestStep {
            sql: sql.to_string(),
            expected_result: Some(expected),
            description: None,
        });
        self
    }

    pub fn expect_rows(mut self, sql: &str, rows: Vec<Vec<&str>>) -> Self {
        self.steps.push(TestStep {
            sql: sql.to_string(),
            expected_result: Some(SqlResult {
                columns: Vec::new(),
                rows: rows.into_iter().map(|r| r.into_iter().map(String::from).collect()).collect(),
                affected_rows: 0,
                error: None,
                error_code: None,
            }),
            description: None,
        });
        self
    }

    pub fn setup(mut self, sql: &str) -> Self {
        self.setup_sql.push(sql.to_string());
        self
    }

    pub fn with_config(mut self, config: DifferentialConfig) -> Self {
        self.config = config;
        self
    }

    pub fn run_with_sqlite_oracle(&self) -> DifferentialResult {
        let mut runner = SqliteDifferentialRunner::new();
        self.run(&mut runner)
    }

    pub fn run(&self, oracle: &mut dyn DifferentialOracle) -> DifferentialResult {
        let mut result = DifferentialResult {
            passed: true,
            failures: Vec::new(),
            steps_run: 0,
        };

        // Run setup on both systems
        for setup in &self.setup_sql {
            let _ = oracle.execute_both(setup);
        }

        // Run each test step
        for (i, step) in self.steps.iter().enumerate() {
            result.steps_run += 1;

            let (ours, theirs) = oracle.execute_both(&step.sql);

            if let Some(ref expected) = step.expected_result {
                // Compare against expected result
                if let Some(failure) = self.compare_results(&ours, expected, i) {
                    result.failures.push(failure);
                    result.passed = false;
                }
            } else {
                // Compare against each other (differential)
                if let Some(failure) = self.compare_results(&ours, &theirs, i) {
                    result.failures.push(failure);
                    result.passed = false;
                }
            }
        }

        result
    }

    fn compare_results(&self, ours: &SqlResult, theirs: &SqlResult, step: usize) -> Option<TestFailure> {
        // Compare errors first
        match (&ours.error, &theirs.error) {
            (Some(e1), Some(e2)) => {
                if e1 != e2 {
                    return Some(TestFailure {
                        step,
                        kind: FailureKind::ErrorMessageMismatch,
                        message: format!("Error mismatch: '{}' vs '{}'", e1, e2),
                    });
                }
            }
            (None, Some(e)) => {
                return Some(TestFailure {
                    step,
                    kind: FailureKind::UnexpectedError,
                    message: format!("Expected no error, got: {}", e),
                });
            }
            (Some(e), None) => {
                return Some(TestFailure {
                    step,
                    kind: FailureKind::MissingError,
                    message: format!("Expected error '{}', got success", e),
                });
            }
            (None, None) => {}
        }

        // Compare columns
        if self.config.compare_columns && ours.columns != theirs.columns {
            return Some(TestFailure {
                step,
                kind: FailureKind::ColumnMismatch,
                message: format!("Columns mismatch: {:?} vs {:?}", ours.columns, theirs.columns),
            });
        }

        // Compare affected rows
        if self.config.compare_affected_rows && ours.affected_rows != theirs.affected_rows {
            return Some(TestFailure {
                step,
                kind: FailureKind::AffectedRowsMismatch,
                message: format!("Affected rows mismatch: {} vs {}", ours.affected_rows, theirs.affected_rows),
            });
        }

        // Compare rows
        let mut our_rows = ours.rows.clone();
        let mut their_rows = theirs.rows.clone();

        if self.config.sort_rows {
            our_rows.sort();
            their_rows.sort();
        }

        if self.config.max_rows > 0 {
            our_rows.truncate(self.config.max_rows);
            their_rows.truncate(self.config.max_rows);
        }

        if our_rows != their_rows {
            return Some(TestFailure {
                step,
                kind: FailureKind::RowMismatch,
                message: format!("Row mismatch:\n  Ours:   {:?}\n  Theirs: {:?}", our_rows, their_rows),
            });
        }

        None
    }
}

#[derive(Debug)]
pub enum FailureKind {
    RowMismatch,
    ColumnMismatch,
    AffectedRowsMismatch,
    ErrorMessageMismatch,
    UnexpectedError,
    MissingError,
}

#[derive(Debug)]
pub struct TestFailure {
    pub step: usize,
    pub kind: FailureKind,
    pub message: String,
}

#[derive(Debug)]
pub struct DifferentialResult {
    pub passed: bool,
    pub failures: Vec<TestFailure>,
    pub steps_run: usize,
}

impl std::fmt::Display for DifferentialResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.passed {
            write!(f, "PASS: {} steps ran, all matched", self.steps_run)
        } else {
            write!(f, "FAIL: {} steps ran, {} failures:\n",
                self.steps_run, self.failures.len())?;
            for failure in &self.failures {
                write!(f, "  Step {}: {:?} - {}\n",
                    failure.step, failure.kind, failure.message)?;
            }
            Ok(())
        }
    }
}

pub trait DifferentialOracle {
    fn execute_both(&mut self, sql: &str) -> (SqlResult, SqlResult);
}

pub struct SqliteDifferentialRunner {
    sqlrustgo_bin: String,
    sqlite_bin: String,
}

impl SqliteDifferentialRunner {
    pub fn new() -> Self {
        Self {
            sqlrustgo_bin: std::env::var("SQLRUSTGO_BIN")
                .unwrap_or_else(|_| "/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo".to_string()),
            sqlite_bin: std::env::var("SQLITE_BIN")
                .unwrap_or_else(|_| "/usr/bin/sqlite3".to_string()),
        }
    }
}

impl DifferentialOracle for SqliteDifferentialRunner {
    fn execute_both(&mut self, sql: &str) -> (SqlResult, SqlResult) {
        let ours = self.execute_sqlrustgo(sql);
        let theirs = self.execute_sqlite(sql);
        (ours, theirs)
    }
}

impl SqliteDifferentialRunner {
    fn execute_sqlrustgo(&self, sql: &str) -> SqlResult {
        // Implementation would run SQL via sqlrustgo CLI
        // For now, return placeholder
        SqlResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: 0,
            error: None,
            error_code: None,
        }
    }

    fn execute_sqlite(&self, sql: &str) -> SqlResult {
        // Implementation would run SQL via sqlite3 CLI
        // For now, return placeholder
        SqlResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: 0,
            error: None,
            error_code: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_differential() {
        let test = DifferentialTest::new()
            .sql("CREATE TABLE t(id INT, name TEXT)")
            .sql("INSERT INTO t VALUES (1, 'alice')")
            .expect_rows("SELECT * FROM t", vec![vec!["1", "alice"]]);

        // This would run against real databases in production
        // For now, just verify the framework compiles
        assert!(test.steps.len() == 3);
    }
}
