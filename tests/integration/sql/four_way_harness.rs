//! G17 4-Way TPC-H Horizontal Comparison — Harness
//!
//! Compares 4 DB engines on the same TPC-H 22 queries with the same data:
//! 1. SQLRustGo (in-process via ExecutionEngine)
//! 2. SQLite (in-process via rusqlite)
//! 3. MariaDB (subprocess via `mysql` CLI to localhost:3306)
//! 4. PostgreSQL (subprocess via `psql` CLI to localhost:5432)
//!
//! Each engine produces a (query_id, row_count, sorted_row_set) tuple.
//! Cross-engine comparison: row_count equality + first-N-rows equality
//! (since full output capture would be O(rows) memory).
//!
//! Refs: V390_TEST_PLAN_ROUND2_REVIEW §G17 (4-way horizontal comparison)
//!       docs/discovery/2026-06-05-tpch-22-mysql-server-comprehensive-report.md
//!
//! Output: docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md

// Shared 4-way harness used by multiple test files; per-test only some engines
// are exercised, so a subset of helpers is unused in any single TU.
#![allow(dead_code)]

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// The shared external-binary resolver (#5477). It is also the owner of the
// **external database clients** (`psql` / `mysql`) as of #5500: before that
// issue this harness hardcoded one machine's homebrew prefix and one
// machine's user name, so on any other machine every external query either
// failed to spawn or failed to authenticate — and the failure was recorded
// as an **empty result set**, which reads exactly like "the engine returned
// no rows".
#[path = "../../common/bin_resolver.rs"]
mod bin_resolver;

/// Why an external database client did not produce a result.
///
/// The three cases must stay distinguishable, because the callers treat
/// them differently:
///
/// * [`ExternalToolError::Unavailable`] — the client is **not installed /
///   not on PATH**. The comparison simply cannot run ⇒ the target reports
///   **NOT-RUN** with a reason code. It is *not* a pass, and it is *not*
///   an empty result set.
/// * [`ExternalToolError::Spawn`] — the client was found but could not be
///   started (wrong architecture, not executable, …). Same treatment as
///   `Unavailable`: the service is not usable.
/// * [`ExternalToolError::Failed`] — the client **ran** and reported a
///   failure (bad SQL, connection refused, …). This is a **real failure**:
///   it must never be folded into an empty result set, because "the query
///   failed" and "the query matched nothing" are different claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalToolError {
    /// The client binary could not be located (or could not be started).
    Unavailable { tool: &'static str, detail: String },
    /// The client ran and reported a failure.
    Failed { tool: &'static str, detail: String },
}

impl ExternalToolError {
    /// Is the external service simply not usable here?
    ///
    /// `true` ⇒ the caller should report **NOT-RUN**, not a pass and not
    /// an empty result.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, ExternalToolError::Unavailable { .. })
    }

    /// The tool this error is about.
    pub fn tool(&self) -> &'static str {
        match self {
            ExternalToolError::Unavailable { tool, .. } => tool,
            ExternalToolError::Failed { tool, .. } => tool,
        }
    }
}

impl fmt::Display for ExternalToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExternalToolError::Unavailable { tool, detail } => {
                write!(f, "{tool} unavailable: {detail}")
            }
            ExternalToolError::Failed { tool, detail } => {
                write!(f, "{tool} failed: {detail}")
            }
        }
    }
}

/// The default database name for the external engines.
///
/// Overridable with `PGDATABASE` (PostgreSQL) / `SQLRUSTGO_MYSQL_DATABASE`
/// (MySQL). The **name** is a convention, not a machine-specific fact —
/// the *user* and the *password* are, and those are never hardcoded.
const DEFAULT_EXTERNAL_DB: &str = "tpch_test";

/// 4 DB engines compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Engine {
    SqlRustGo,
    Sqlite,
    MariaDb,
    PostgreSql,
}

impl Engine {
    pub fn name(self) -> &'static str {
        match self {
            Engine::SqlRustGo => "sqlrustgo",
            Engine::Sqlite => "sqlite",
            Engine::MariaDb => "mariadb",
            Engine::PostgreSql => "postgresql",
        }
    }
    pub fn all() -> [Engine; 4] {
        [
            Engine::SqlRustGo,
            Engine::Sqlite,
            Engine::MariaDb,
            Engine::PostgreSql,
        ]
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// TPC-H 8-table schema (all 4 engines use this DDL adapted to their dialect).
/// `engine_specific_ddl` is the per-engine DDL.
pub fn tpc_h_schema(engine: Engine) -> Vec<String> {
    match engine {
        Engine::SqlRustGo | Engine::Sqlite => vec![
            "DROP TABLE IF EXISTS region".into(),
            "DROP TABLE IF EXISTS nation".into(),
            "DROP TABLE IF EXISTS supplier".into(),
            "DROP TABLE IF EXISTS customer".into(),
            "DROP TABLE IF EXISTS part".into(),
            "DROP TABLE IF EXISTS partsupp".into(),
            "DROP TABLE IF EXISTS orders".into(),
            "DROP TABLE IF EXISTS lineitem".into(),
            "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)".into(),
            "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)".into(),
            "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)".into(),
            "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)".into(),
            "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)".into(),
            "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT)".into(),
            "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)".into(),
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)".into(),
        ],
        Engine::MariaDb | Engine::PostgreSql => vec![
            "DROP TABLE IF EXISTS region".into(),
            "DROP TABLE IF EXISTS nation".into(),
            "DROP TABLE IF EXISTS supplier".into(),
            "DROP TABLE IF EXISTS customer".into(),
            "DROP TABLE IF EXISTS part".into(),
            "DROP TABLE IF EXISTS partsupp".into(),
            "DROP TABLE IF EXISTS orders".into(),
            "DROP TABLE IF EXISTS lineitem".into(),
            "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name VARCHAR(25) NOT NULL, r_comment VARCHAR(152))".into(),
            "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name VARCHAR(25) NOT NULL, n_regionkey INTEGER NOT NULL, n_comment VARCHAR(152))".into(),
            "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name CHAR(25) NOT NULL, s_address VARCHAR(40) NOT NULL, s_nationkey INTEGER NOT NULL, s_phone CHAR(15) NOT NULL, s_acctbal DECIMAL(15,2) NOT NULL, s_comment VARCHAR(101))".into(),
            "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name VARCHAR(25) NOT NULL, c_address VARCHAR(40) NOT NULL, c_nationkey INTEGER NOT NULL, c_phone CHAR(15) NOT NULL, c_acctbal DECIMAL(15,2) NOT NULL, c_mktsegment CHAR(10), c_comment VARCHAR(117))".into(),
            "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name VARCHAR(55) NOT NULL, p_mfgr CHAR(25) NOT NULL, p_brand CHAR(10) NOT NULL, p_type VARCHAR(25) NOT NULL, p_size INTEGER NOT NULL, p_container CHAR(10) NOT NULL, p_retailprice DECIMAL(15,2) NOT NULL, p_comment VARCHAR(23))".into(),
            "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost DECIMAL(15,2) NOT NULL, ps_comment VARCHAR(199))".into(),
            "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus CHAR(1) NOT NULL, o_totalprice DECIMAL(15,2) NOT NULL, o_orderdate DATE NOT NULL, o_orderpriority CHAR(15), o_clerk CHAR(15), o_shippriority INTEGER, o_comment VARCHAR(79))".into(),
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity DECIMAL(15,2) NOT NULL, l_extendedprice DECIMAL(15,2) NOT NULL, l_discount DECIMAL(15,2) NOT NULL, l_tax DECIMAL(15,2) NOT NULL, l_returnflag CHAR(1), l_linestatus CHAR(1), l_shipdate DATE, l_commitdate DATE, l_receiptdate DATE, l_shipinstruct CHAR(25), l_shipmode CHAR(10), l_comment VARCHAR(44))".into(),
        ],
    }
}

/// Per-table (col_count) — for the parser to know how to split TBL rows.
pub const TABLE_COLS: &[(&str, usize)] = &[
    ("region", 3),
    ("nation", 4),
    ("supplier", 7),
    ("customer", 8),
    ("part", 9),
    ("partsupp", 5),
    ("orders", 9),
    ("lineitem", 16),
];

/// Load a single TBL file as INSERT statements (one INSERT per row) for
/// the given engine. Returns the SQL statements.
pub fn load_tbl_inserts(table: &str, cols: usize, tbl_path: &Path, _engine: Engine) -> Vec<String> {
    let content = match fs::read_to_string(tbl_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() < cols {
            continue;
        }
        let vals: Vec<String> = (0..cols)
            .map(|i| {
                let s = fields[i].trim();
                if s.is_empty() {
                    "NULL".to_string()
                } else if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
                    // TPC-H TBL files contain integers and floats in the
                    // same unquoted column. Distinguish via parse: integers
                    // (e.g. "1234") parse as i64, floats (e.g. "6322.20")
                    // only parse as f64. Both must be unquoted so the
                    // engine stores them as Integer/Float (not Text).
                    s.to_string()
                } else {
                    format!("'{}'", s.replace('\'', "''"))
                }
            })
            .collect();
        let sql = format!("INSERT INTO {} VALUES ({});", table, vals.join(", "));
        out.push(sql);
    }
    out
}

/// One TPC-H query result from one engine.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub engine: Engine,
    pub query: u8,
    pub row_count: usize,
    pub duration_ms: u128,
    pub error: Option<String>,
    /// First N sorted rows (canonical form for cross-engine comparison).
    pub sample_rows: Vec<String>,
}

impl QueryResult {
    pub fn passed(&self) -> bool {
        self.error.is_none()
    }
}

/// Compare row_counts across engines for a single query.
pub fn compare_row_counts(results: &[QueryResult]) -> bool {
    let pass: Vec<usize> = results
        .iter()
        .filter(|r| r.passed())
        .map(|r| r.row_count)
        .collect();
    if pass.is_empty() {
        return false;
    }
    let first = pass[0];
    pass.iter().all(|c| *c == first)
}

/// Locate the MySQL client — #5500.
///
/// Explicit environment variable first, then `PATH`. Never a hardcoded
/// absolute path: that path exists on exactly one machine.
fn mysql_binary() -> Result<PathBuf, ExternalToolError> {
    bin_resolver::resolve_mysql_binary().ok_or_else(|| ExternalToolError::Unavailable {
        tool: "mysql",
        detail: bin_resolver::missing_external_tool_message("mysql", bin_resolver::MYSQL_BIN_ENV),
    })
}

/// Locate the PostgreSQL client — #5500.
fn psql_binary() -> Result<PathBuf, ExternalToolError> {
    bin_resolver::resolve_psql_binary().ok_or_else(|| ExternalToolError::Unavailable {
        tool: "psql",
        detail: bin_resolver::missing_external_tool_message("psql", bin_resolver::PSQL_BIN_ENV),
    })
}

/// A non-empty, trimmed environment variable.
fn env_trimmed(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The database to talk to.
///
/// `PGDATABASE` (PostgreSQL) / `SQLRUSTGO_MYSQL_DATABASE` (MySQL) override
/// the default. The **name** is a convention shared by both sides of the
/// comparison; the *user* and the *password* are machine facts, and those
/// are never hardcoded.
fn mysql_database() -> String {
    env_trimmed("SQLRUSTGO_MYSQL_DATABASE").unwrap_or_else(|| DEFAULT_EXTERNAL_DB.to_string())
}

fn psql_database() -> String {
    env_trimmed("PGDATABASE").unwrap_or_else(|| DEFAULT_EXTERNAL_DB.to_string())
}

/// The MySQL client arguments (without the binary path).
///
/// The **user** comes from `SQLRUSTGO_MYSQL_USER` and is omitted entirely
/// when that variable is unset — the client then applies its own default.
/// The **password** is handed to the child process through `MYSQL_PWD`
/// rather than on the command line, so it does not show up in the process
/// list and so no credential literal ever appears in this file.
fn mysql_args(database: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(user) = env_trimmed("SQLRUSTGO_MYSQL_USER") {
        args.push("-u".to_string());
        args.push(user);
    }
    if let Some(db) = database {
        args.push("-D".to_string());
        args.push(db.to_string());
    }
    args.push("--batch".to_string());
    args.push("--skip-column-names".to_string());
    args
}

/// The PostgreSQL client arguments (without the binary path).
///
/// The **user** is left to `libpq`: `psql` reads `PGUSER` (and `PGHOST`,
/// `PGPORT`) from the environment by itself, so this harness never has to
/// name a user at all. When `PGUSER` is unset the client falls back to the
/// operating-system user, which is the right default on any machine.
fn psql_args(database: &str) -> Vec<String> {
    vec![
        "-d".to_string(),
        database.to_string(),
        "-t".to_string(),
        "-A".to_string(),
    ]
}

/// A `mysql` invocation carrying the harness's credential arguments.
fn mysql_command(bin: &Path, database: Option<&str>) -> Command {
    let mut cmd = Command::new(bin);
    cmd.args(mysql_args(database));
    if let Some(pw) = env_trimmed("SQLRUSTGO_MYSQL_PASSWORD") {
        cmd.env("MYSQL_PWD", pw);
    }
    cmd
}

/// A `psql` invocation carrying the harness's database argument.
fn psql_command(bin: &Path, database: &str) -> Command {
    let mut cmd = Command::new(bin);
    cmd.args(psql_args(database));
    cmd
}

/// Run a single SQL statement via the mysql CLI (for MariaDB).
///
/// A missing client is reported as [`ExternalToolError::Unavailable`] —
/// never as an empty result set.
pub fn run_mysql_sql(sql: &str) -> Result<String, ExternalToolError> {
    let bin = mysql_binary()?;
    let database = mysql_database();
    let output = mysql_command(&bin, Some(&database))
        .arg("--execute")
        .arg(sql)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ExternalToolError::Unavailable {
            tool: "mysql",
            detail: format!("mysql spawn: {}", e),
        })?;
    if !output.status.success() {
        return Err(ExternalToolError::Failed {
            tool: "mysql",
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a single SQL statement via psql (for PostgreSQL).
///
/// A missing client is reported as [`ExternalToolError::Unavailable`] —
/// never as an empty result set.
pub fn run_psql_sql(sql: &str) -> Result<String, ExternalToolError> {
    let bin = psql_binary()?;
    let database = psql_database();
    let output = psql_command(&bin, &database)
        .arg("-c")
        .arg(sql)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ExternalToolError::Unavailable {
            tool: "psql",
            detail: format!("psql spawn: {}", e),
        })?;
    if !output.status.success() {
        return Err(ExternalToolError::Failed {
            tool: "psql",
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Is the external client for `engine` usable on this machine?
///
/// The in-process engines ([`Engine::SqlRustGo`], [`Engine::Sqlite`]) have
/// no external dependency and are always usable.
///
/// #5500: this is the **single** place that decides "is the external
/// service there?". Callers use it to turn an absent client into a
/// disclosed NOT-RUN instead of a comparison against nothing.
pub fn external_client_available(engine: Engine) -> bool {
    match engine {
        Engine::MariaDb => bin_resolver::resolve_mysql_binary().is_some(),
        Engine::PostgreSql => bin_resolver::resolve_psql_binary().is_some(),
        _ => true,
    }
}

/// How many `lineitem` rows the fixture on disk holds.
///
/// The "is it already loaded?" check must compare the external database
/// against **the very fixture the engine side loads**. A hardcoded row
/// count would answer a question about a *different* dataset — which is
/// how "the external DB holds something else" turns into "the comparison
/// ran and found differences" (issue #5500).
fn fixture_lineitem_rows(data_dir: &Path) -> Option<u64> {
    let content = fs::read_to_string(data_dir.join("lineitem.tbl")).ok()?;
    Some(content.lines().filter(|l| !l.trim().is_empty()).count() as u64)
}

/// The `lineitem` row count of the external database, if it can be read.
fn external_lineitem_rows(engine: Engine) -> Result<Option<u64>, ExternalToolError> {
    match engine {
        Engine::MariaDb => {
            let bin = mysql_binary()?;
            let db = mysql_database();
            let out = mysql_command(&bin, Some(&db))
                .args(["-N", "-B", "-e", "SELECT COUNT(*) FROM lineitem;"])
                .output()
                .map_err(|e| ExternalToolError::Unavailable {
                    tool: "mysql",
                    detail: format!("mysql spawn: {}", e),
                })?;
            if !out.status.success() {
                return Ok(None);
            }
            Ok(String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<u64>()
                .ok())
        }
        Engine::PostgreSql => {
            let bin = psql_binary()?;
            let db = psql_database();
            let out = psql_command(&bin, &db)
                .args(["-t", "-A", "-c", "SELECT COUNT(*) FROM lineitem;"])
                .output()
                .map_err(|e| ExternalToolError::Unavailable {
                    tool: "psql",
                    detail: format!("psql spawn: {}", e),
                })?;
            if !out.status.success() {
                return Ok(None);
            }
            Ok(String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<u64>()
                .ok())
        }
        _ => Ok(None),
    }
}

/// Set up a database for an external engine (create DB, schema, load data).
///
/// Idempotent: detects if data is already loaded and skips the bulk
/// INSERT phase. The detection compares the database's `lineitem` row
/// count against **the fixture's** row count, not against a constant.
pub fn setup_external_db(engine: Engine, data_dir: &Path) -> Result<(), ExternalToolError> {
    // First, ensure the client is usable at all. A missing client must be
    // reported as "unavailable" so the caller can turn it into a NOT-RUN
    // instead of silently comparing against an empty result set.
    let mysql_bin = if engine == Engine::MariaDb {
        Some(mysql_binary()?)
    } else {
        None
    };
    let psql_bin = if engine == Engine::PostgreSql {
        Some(psql_binary()?)
    } else {
        None
    };

    // Then, ensure the database exists.
    match engine {
        Engine::MariaDb => {
            let bin = mysql_bin.as_ref().expect("mysql binary resolved above");
            mysql_command(bin, None)
                .args([
                    "-e",
                    &format!("CREATE DATABASE IF NOT EXISTS {};", mysql_database()),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .map_err(|e| ExternalToolError::Unavailable {
                    tool: "mysql",
                    detail: format!("create db: {}", e),
                })?;
        }
        Engine::PostgreSql => {
            let bin = psql_bin.as_ref().expect("psql binary resolved above");
            // Idempotent: try CREATE DATABASE, ignore "already exists" error
            psql_command(bin, "postgres")
                .args(["-c", &format!("CREATE DATABASE {};", psql_database())])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .map_err(|e| ExternalToolError::Unavailable {
                    tool: "psql",
                    detail: format!("create db: {}", e),
                })?;
        }
        _ => {
            return Err(ExternalToolError::Unavailable {
                tool: "external db",
                detail: "setup_external_db only for MariaDB/PostgreSQL".into(),
            })
        }
    }

    // Detect: is the DB already populated with **this** fixture?
    let expected_rows = fixture_lineitem_rows(data_dir);
    let actual_rows = external_lineitem_rows(engine)?;
    let already_loaded = match (expected_rows, actual_rows) {
        (Some(expected), Some(actual)) => expected == actual,
        _ => false,
    };
    if already_loaded {
        eprintln!(
            "[{}] Already loaded ({} lineitem rows, matching the fixture) — skipping",
            engine.name(),
            expected_rows.unwrap_or(0)
        );
        return Ok(());
    }

    // Apply schema (idempotent: IF NOT EXISTS on each CREATE)
    for stmt in tpc_h_schema(engine) {
        let safe = if stmt.starts_with("CREATE TABLE") {
            stmt.replacen("CREATE TABLE", "CREATE TABLE IF NOT EXISTS", 1)
        } else {
            stmt
        };
        match engine {
            Engine::MariaDb => {
                let bin = mysql_bin.as_ref().expect("mysql binary resolved above");
                mysql_command(bin, Some(&mysql_database()))
                    .args(["-e", &safe])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map_err(|e| ExternalToolError::Unavailable {
                        tool: "mysql",
                        detail: format!("schema: {}", e),
                    })?;
            }
            Engine::PostgreSql => {
                let bin = psql_bin.as_ref().expect("psql binary resolved above");
                psql_command(bin, &psql_database())
                    .args(["-c", &safe])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map_err(|e| ExternalToolError::Unavailable {
                        tool: "psql",
                        detail: format!("schema: {}", e),
                    })?;
            }
            _ => {}
        }
    }

    // Per-engine column type widening (TPC-H SF=1 simplified data has
    // some long values that need wider types)
    if engine == Engine::PostgreSql {
        let bin = psql_bin.as_ref().expect("psql binary resolved above");
        for alter in [
            "ALTER TABLE part ALTER COLUMN p_type TYPE VARCHAR(35);",
            "ALTER TABLE part ALTER COLUMN p_comment TYPE VARCHAR(60);",
        ] {
            let _ = psql_command(bin, &psql_database())
                .args(["-c", alter])
                .output();
        }
    }

    // Load data: use LOAD DATA (MariaDB) or COPY (PG) for speed
    for (table, _cols) in TABLE_COLS {
        let tbl_path = data_dir.join(format!("{}.tbl", table));
        let r = match engine {
            Engine::MariaDb => {
                let bin = mysql_bin.as_ref().expect("mysql binary resolved above");
                mysql_command(bin, Some(&mysql_database()))
                    .args([
                        "--local-infile=1",
                        "-e",
                        &format!(
                            "LOAD DATA LOCAL INFILE '{}' INTO TABLE {} \
                             FIELDS TERMINATED BY '|' LINES TERMINATED BY '\\n';",
                            tbl_path.display(),
                            table
                        ),
                    ])
                    .output()
                    .map_err(|e| ExternalToolError::Unavailable {
                        tool: "mysql",
                        detail: format!("load {}: {}", table, e),
                    })?
            }
            Engine::PostgreSql => {
                let bin = psql_bin.as_ref().expect("psql binary resolved above");
                psql_command(bin, &psql_database())
                    .args([
                        "-c",
                        &format!(
                            "\\copy {} FROM '{}' WITH (FORMAT csv, DELIMITER '|')",
                            table,
                            tbl_path.display()
                        ),
                    ])
                    .output()
                    .map_err(|e| ExternalToolError::Unavailable {
                        tool: "psql",
                        detail: format!("load {}: {}", table, e),
                    })?
            }
            _ => continue,
        };
        if !r.status.success() {
            eprintln!(
                "[{}] {} load error: {}",
                engine.name(),
                table,
                String::from_utf8_lossy(&r.stderr)
                    .chars()
                    .take(200)
                    .collect::<String>()
            );
        }
    }

    eprintln!("[{}] Data load complete", engine.name());
    Ok(())
}
