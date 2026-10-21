// #5449：这个文件此前从未被编译过，`use std::collections::HashMap;` 是未使用的
// import（编译告警）。它在这里没有任何引用，直接删掉。

// #5406：二进制定位收敛到共享 helper（`tests/common/bin_resolver.rs`），
// 与 #5477 的 6 处重复逻辑共用同一套规则。这里原本有一份**独立实现**，
// 两份实现会各自漂移 —— 而其中任何一份回退到硬编码家目录路径，
// 差分对照就会重新变成「静默跳过 ⇒ 假绿」。
#[path = "../common/bin_resolver.rs"]
mod bin_resolver;

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

impl Default for DifferentialTest {
    fn default() -> Self {
        Self::new()
    }
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
                rows: rows
                    .into_iter()
                    .map(|r| r.into_iter().map(String::from).collect())
                    .collect(),
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

    pub fn run_with_mysql_oracle(&self) -> DifferentialResult {
        let mut runner = MysqlDifferentialRunner::new();
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

    fn compare_results(
        &self,
        ours: &SqlResult,
        theirs: &SqlResult,
        step: usize,
    ) -> Option<TestFailure> {
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
                message: format!(
                    "Columns mismatch: {:?} vs {:?}",
                    ours.columns, theirs.columns
                ),
            });
        }

        // Compare affected rows
        if self.config.compare_affected_rows && ours.affected_rows != theirs.affected_rows {
            return Some(TestFailure {
                step,
                kind: FailureKind::AffectedRowsMismatch,
                message: format!(
                    "Affected rows mismatch: {} vs {}",
                    ours.affected_rows, theirs.affected_rows
                ),
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
                message: format!(
                    "Row mismatch:\n  Ours:   {:?}\n  Theirs: {:?}",
                    our_rows, their_rows
                ),
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
            writeln!(
                f,
                "FAIL: {} steps ran, {} failures:",
                self.steps_run,
                self.failures.len()
            )?;
            for failure in &self.failures {
                writeln!(
                    f,
                    "  Step {}: {:?} - {}",
                    failure.step, failure.kind, failure.message
                )?;
            }
            Ok(())
        }
    }
}

pub trait DifferentialOracle {
    fn execute_both(&mut self, sql: &str) -> (SqlResult, SqlResult);
}

/// 定位引擎二进制。
///
/// 顺序：`SQLRUSTGO_BIN` → `CARGO_TARGET_DIR` → 仓库相对
/// `target/{debug,release}` → `PATH`。
///
/// #5406：这里原本硬编码的是**开发机上的绝对路径**
/// （形如 `<某人的家目录>/dev/sqlrustgo/target/debug/sqlrustgo`）——
/// 一台个人机器的路径，在任何 CI runner 上都不存在，
/// 于是差分对照在 CI 上**静默降级**为「什么都不做」并报告「通过」。
pub fn resolve_engine_binary() -> Option<String> {
    bin_resolver::resolve_engine_binary().map(|p| p.display().to_string())
}

/// 定位 sqlite3：环境变量 → `CARGO_TARGET_DIR` → `PATH`。
pub fn resolve_sqlite_binary() -> Option<String> {
    bin_resolver::resolve_sqlite_binary().map(|p| p.display().to_string())
}

// #5406 / #5477：通用二进制查找的实现已收敛到共享 helper
// `tests/common/bin_resolver.rs`。这里原本有一份**独立实现** ——
// 两份实现会各自漂移，而其中任何一份回退到硬编码家目录路径，
// 差分对照就会重新变成「静默跳过 ⇒ 假绿」。

/// 二进制缺失时的统一错误（调用方拿得到「该做什么」，而不是一个空结果）。
fn missing_binary_error(which: &str, env_var: &str) -> String {
    bin_resolver::missing_binary_message(which, env_var)
}

pub struct SqliteDifferentialRunner {
    sqlrustgo_bin: Option<String>,
    sqlite_bin: Option<String>,
}

impl Default for SqliteDifferentialRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteDifferentialRunner {
    pub fn new() -> Self {
        Self {
            sqlrustgo_bin: resolve_engine_binary(),
            sqlite_bin: resolve_sqlite_binary(),
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
        use std::process::Command;
        let Some(bin) = self.sqlrustgo_bin.as_deref() else {
            return SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(missing_binary_error("engine", "SQLRUSTGO_BIN")),
                error_code: None,
            };
        };
        let output = Command::new(bin)
            .args([
                "sqlite",
                "--batch",
                "--mode",
                "csv",
                "--headers",
                "true",
                ":memory:",
            ])
            .arg(sql)
            .output();
        match output {
            Ok(out) => parse_csv_output(&String::from_utf8_lossy(&out.stdout)),
            Err(e) => SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(format!("Failed to execute: {}", e)),
                error_code: None,
            },
        }
    }

    fn execute_sqlite(&self, sql: &str) -> SqlResult {
        use std::process::Command;
        let Some(bin) = self.sqlite_bin.as_deref() else {
            return SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(missing_binary_error("sqlite3", "SQLITE_BIN")),
                error_code: None,
            };
        };
        let output = Command::new(bin)
            .args(["-csv", "-header", ":memory:"])
            .arg(sql)
            .output();
        match output {
            Ok(out) => parse_csv_output(&String::from_utf8_lossy(&out.stdout)),
            Err(e) => SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(format!("Failed to execute: {}", e)),
                error_code: None,
            },
        }
    }
}

pub struct MysqlDifferentialRunner {
    sqlrustgo_bin: Option<String>,
    mysql_host: String,
    mysql_port: u16,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
}

impl Default for MysqlDifferentialRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MysqlDifferentialRunner {
    pub fn new() -> Self {
        Self {
            sqlrustgo_bin: resolve_engine_binary(),
            mysql_host: std::env::var("MYSQL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            mysql_port: std::env::var("MYSQL_PORT")
                .unwrap_or_else(|_| "3306".to_string())
                .parse()
                .unwrap_or(3306),
            mysql_user: std::env::var("MYSQL_USER").unwrap_or_else(|_| "root".to_string()),
            mysql_password: std::env::var("MYSQL_PASSWORD").unwrap_or_else(|_| "".to_string()),
            mysql_database: std::env::var("MYSQL_DATABASE").unwrap_or_else(|_| "test".to_string()),
        }
    }
}

impl DifferentialOracle for MysqlDifferentialRunner {
    fn execute_both(&mut self, sql: &str) -> (SqlResult, SqlResult) {
        let ours = self.execute_sqlrustgo(sql);
        let theirs = self.execute_mysql(sql);
        (ours, theirs)
    }
}

impl MysqlDifferentialRunner {
    fn execute_sqlrustgo(&self, sql: &str) -> SqlResult {
        use std::process::Command;
        let Some(bin) = self.sqlrustgo_bin.as_deref() else {
            return SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(missing_binary_error("engine", "SQLRUSTGO_BIN")),
                error_code: None,
            };
        };
        let output = Command::new(bin)
            .args([
                "sqlite",
                "--batch",
                "--mode",
                "csv",
                "--headers",
                "true",
                ":memory:",
            ])
            .arg(sql)
            .output();
        match output {
            Ok(out) => parse_csv_output(&String::from_utf8_lossy(&out.stdout)),
            Err(e) => SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(format!("Failed to execute: {}", e)),
                error_code: None,
            },
        }
    }

    fn execute_mysql(&self, sql: &str) -> SqlResult {
        use std::process::Command;
        let output = Command::new("mysql")
            .args([
                "-h",
                &self.mysql_host,
                "-P",
                &self.mysql_port.to_string(),
                "-u",
                &self.mysql_user,
                &format!("-p{}", self.mysql_password),
                &self.mysql_database,
                "-e",
                sql,
                "--batch",
                "--raw",
            ])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.is_empty() && !out.status.success() {
                    SqlResult {
                        columns: Vec::new(),
                        rows: Vec::new(),
                        affected_rows: 0,
                        error: Some(stderr.to_string()),
                        error_code: Some(1),
                    }
                } else {
                    parse_tab_output(&stdout)
                }
            }
            Err(e) => SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                error: Some(format!("Failed to execute: {}", e)),
                error_code: None,
            },
        }
    }
}

fn parse_csv_output(output: &str) -> SqlResult {
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return SqlResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: 0,
            error: None,
            error_code: None,
        };
    }

    let columns: Vec<String> = lines[0]
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect();

    let rows: Vec<Vec<String>> = lines[1..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            l.split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect()
        })
        .collect();

    SqlResult {
        columns,
        rows,
        affected_rows: 0,
        error: None,
        error_code: None,
    }
}

fn parse_tab_output(output: &str) -> SqlResult {
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return SqlResult {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: 0,
            error: None,
            error_code: None,
        };
    }

    let columns: Vec<String> = lines[0].split('\t').map(|s| s.trim().to_string()).collect();

    let rows: Vec<Vec<String>> = lines[1..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').map(|s| s.trim().to_string()).collect())
        .collect();

    SqlResult {
        columns,
        rows,
        affected_rows: 0,
        error: None,
        error_code: None,
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

        // #5406：原来这里只有 `assert!(test.steps.len() == 3)` ——
        // 那是在断言**构造器自己刚放进去的东西**，与框架是否有用无关。
        // 现在断言的是框架真正承诺的三件事：步骤被记下来了、
        // 期望值被挂在了**最后一条**语句上、描述被保留。
        assert_eq!(test.steps.len(), 3, "every statement must become a step");
        let last = test.steps.last().expect("at least one step");
        assert_eq!(last.sql, "SELECT * FROM t");
        assert!(
            last.expected_result.is_some(),
            "expect_rows must attach the expectation to the step it belongs to"
        );
        assert_eq!(
            last.expected_result.as_ref().unwrap().rows,
            vec![vec!["1".to_string(), "alice".to_string()]]
        );
        assert!(
            test.steps[0].expected_result.is_none(),
            "a plain .sql() step must not inherit a later expectation"
        );
    }

    #[test]
    fn test_mysql_oracle_available() {
        let runner = MysqlDifferentialRunner::new();
        // #5406：原来只有 `assert!(!runner.mysql_host.is_empty())` ——
        // 默认值是 "127.0.0.1"，**永远**非空 ⇒ 断言恒真。
        // 现在断言的是「这个 host 真的能被解析成一个 socket 地址」。
        let addr = format!("{}:{}", runner.mysql_host, runner.mysql_port);
        assert!(
            addr.parse::<std::net::SocketAddr>().is_ok(),
            "MYSQL_HOST/MYSQL_PORT must form a valid socket address, got {:?}",
            addr
        );
        assert!(!runner.mysql_user.is_empty());
        assert!(!runner.mysql_database.is_empty());
    }

    /// #5406：二进制缺失时必须给出**明确的错误**，而不是一个空结果 ——
    /// 「空结果」会被读成「查询没有返回行」，也就是又一次假绿。
    #[test]
    fn test_missing_binary_yields_an_explicit_error() {
        let mut runner = SqliteDifferentialRunner {
            sqlrustgo_bin: None,
            sqlite_bin: None,
        };
        let (ours, theirs) = runner.execute_both("SELECT 1");
        for (side, res) in [("engine", &ours), ("sqlite3", &theirs)] {
            let err = res.error.as_deref().unwrap_or_else(|| {
                panic!(
                    "{} side must report an error when the binary is missing",
                    side
                )
            });
            assert!(
                err.contains("not found"),
                "{} side error must say the binary was not found, got {:?}",
                side,
                err
            );
            assert!(
                err.contains("SQLRUSTGO_BIN") || err.contains("SQLITE_BIN"),
                "{} side error must name the env var to set, got {:?}",
                side,
                err
            );
            assert!(res.rows.is_empty(), "{} side must not fabricate rows", side);
        }
    }

    /// #5406：二进制解析**不得**回退到任何机器相关的绝对路径。
    #[test]
    fn test_binary_resolution_has_no_machine_specific_default() {
        let src = include_str!("differential_framework.rs");
        // 拼在运行时构造这个字面量 —— 否则**本断言自己**就把它写进了源文件，
        // 于是这个测试永远失败（自指）。
        for needle in [format!("/{}", "Users"), format!("/{}", "home")] {
            assert!(
                !src.contains(&needle),
                "differential_framework.rs must not contain a machine-specific absolute path ({})",
                needle
            );
        }
    }
}
