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

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

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
        [Engine::SqlRustGo, Engine::Sqlite, Engine::MariaDb, Engine::PostgreSql]
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
pub fn load_tbl_inserts(table: &str, cols: usize, tbl_path: &Path, engine: Engine) -> Vec<String> {
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
                } else if s.parse::<i64>().is_ok() {
                    s.to_string()
                } else {
                    // Quote string with single quotes
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

/// Run a single SQL statement via the mysql CLI (for MariaDB).
pub fn run_mysql_sql(sql: &str) -> Result<String, String> {
    let output = Command::new("/opt/homebrew/bin/mysql")
        .args(["-u", "tpch", "-ptpch", "-D", "tpch_test", "--batch", "--skip-column-names"])
        .arg("--execute")
        .arg(sql)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("mysql spawn: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a single SQL statement via psql (for PostgreSQL).
pub fn run_psql_sql(sql: &str) -> Result<String, String> {
    let output = Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
        .args(["-U", "liying", "-d", "tpch_test", "-t", "-A"])
        .arg("-c")
        .arg(sql)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("psql spawn: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Set up a database for an external engine (create DB, schema, load data).
/// Idempotent: detects if data is already loaded and skips the bulk
/// INSERT phase.
pub fn setup_external_db(engine: Engine, data_dir: &Path) -> Result<(), String> {
    // First, ensure the database exists
    match engine {
        Engine::MariaDb => {
            Command::new("/opt/homebrew/bin/mysql")
                .args([
                    "-u", "tpch", "-ptpch",
                    "-e", "CREATE DATABASE IF NOT EXISTS tpch_test;",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .map_err(|e| format!("create db: {}", e))?;
        }
        Engine::PostgreSql => {
            // Idempotent: try CREATE DATABASE, ignore "already exists" error
            Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
                .args([
                    "-U", "liying", "-d", "postgres",
                    "-c", "CREATE DATABASE tpch_test;",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .map_err(|e| format!("create db: {}", e))?;
        }
        _ => return Err("setup_external_db only for MariaDB/PostgreSQL".into()),
    }

    // Detect: is the DB already populated? (count lineitem rows)
    let already_loaded: bool = match engine {
        Engine::MariaDb => {
            let out = Command::new("/opt/homebrew/bin/mysql")
                .args([
                    "-u", "tpch", "-ptpch", "-D", "tpch_test", "-N", "-B",
                    "-e", "SELECT COUNT(*) FROM lineitem;",
                ])
                .output()
                .map_err(|e| format!("count: {}", e))?;
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            s == "60000"
        }
        Engine::PostgreSql => {
            let out = Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
                .args([
                    "-U", "liying", "-d", "tpch_test", "-t", "-A",
                    "-c", "SELECT COUNT(*) FROM lineitem;",
                ])
                .output()
                .map_err(|e| format!("count: {}", e))?;
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            s == "60000"
        }
        _ => false,
    };
    if already_loaded {
        eprintln!("[{}] Already loaded (60000 lineitem rows) — skipping", engine.name());
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
                Command::new("/opt/homebrew/bin/mysql")
                    .args(["-u", "tpch", "-ptpch", "-D", "tpch_test", "-e", &safe])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map_err(|e| format!("schema: {}", e))?;
            }
            Engine::PostgreSql => {
                Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
                    .args(["-U", "liying", "-d", "tpch_test", "-c", &safe])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map_err(|e| format!("schema: {}", e))?;
            }
            _ => {}
        }
    }

    // Per-engine column type widening (TPC-H SF=1 simplified data has
    // some long values that need wider types)
    if engine == Engine::PostgreSql {
        let _ = Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
            .args([
                "-U", "liying", "-d", "tpch_test",
                "-c", "ALTER TABLE part ALTER COLUMN p_type TYPE VARCHAR(35);",
            ])
            .output();
        let _ = Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
            .args([
                "-U", "liying", "-d", "tpch_test",
                "-c", "ALTER TABLE part ALTER COLUMN p_comment TYPE VARCHAR(60);",
            ])
            .output();
    }

    // Load data: use LOAD DATA (MariaDB) or COPY (PG) for speed
    for (table, _cols) in TABLE_COLS {
        let tbl_path = data_dir.join(format!("{}.tbl", table));
        let r = match engine {
            Engine::MariaDb => Command::new("/opt/homebrew/bin/mysql")
                .args([
                    "-u", "tpch", "-ptpch", "-D", "tpch_test",
                    "--local-infile=1",
                    "-e", &format!(
                        "LOAD DATA LOCAL INFILE '{:?}' INTO TABLE {} \
                         FIELDS TERMINATED BY '|' LINES TERMINATED BY '\\n';",
                        tbl_path, table
                    ),
                ])
                .output()
                .map_err(|e| format!("load {}: {}", table, e))?,
            Engine::PostgreSql => Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")
                .args([
                    "-U", "liying", "-d", "tpch_test",
                    "-c", &format!(
                        "COPY {} FROM '{:?}' WITH (FORMAT csv, DELIMITER '|');",
                        table, tbl_path
                    ),
                ])
                .output()
                .map_err(|e| format!("load {}: {}", table, e))?,
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

/// TPC-H data dir location (env: TPCH_DATA_DIR or default /tmp/tpch_sf01).
pub fn default_data_dir() -> PathBuf {
    std::env::var("TPCH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/tpch_sf01"))
}
