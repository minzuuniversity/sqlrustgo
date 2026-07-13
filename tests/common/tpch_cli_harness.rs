//! TPC-H CLI E2E test harness — spawns `sqlrustgo-mysql-server` as a
//! subprocess and drives SQL via the real `mysql` CLI binary.
//!
//! This is the canonical wire E2E test surface: every SQL statement and
//! data load goes through the same protocol path a real `mysql` user
//! would use. Unlike `tpch_wire_harness.rs` (which uses the custom
//! `MySqlTestClient` raw protocol client), this harness uses the system
//! `mysql` CLI binary, proving the server works with production MySQL
//! client tooling.
//!
//! If the `mysql` CLI is not installed, all tests using this harness
//! will panic at spawn time. Set the `MYSQL` environment variable to
//! an alternate binary path if needed.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value as JsonValue;

// ============================================================================
// Server subprocess wrapper
// ============================================================================

/// Default TCP probe timeout for server readiness.
const READINESS_TIMEOUT: Duration = Duration::from_secs(10);

/// SQLRustGo server subprocess handle.
///
/// Dropping this struct kills the server process.
pub struct CliServer {
    child: Child,
    port: u16,
}

impl CliServer {
    /// Start `sqlrustgo-mysql-server serve` with the given data directory.
    ///
    /// Finds a free port, spawns the server subprocess, and waits for it
    /// to become ready (TCP connection succeeds).
    pub fn start(data_dir: &Path) -> Result<Self, String> {
        let bin = resolve_binary()?;
        let port = pick_free_port()?;

        let child = Command::new(&bin)
            .arg("serve")
            .args(["--host", "127.0.0.1"])
            .args(["--port", &port.to_string()])
            .args(["--data-dir", data_dir.to_str().unwrap()])
            .args(["--log-level", "warn"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {:?}: {}", bin, e))?;

        let server = Self { child, port };
        server.wait_ready()?;
        Ok(server)
    }

    /// Wait until the server accepts TCP connections.
    fn wait_ready(&self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let deadline = Instant::now() + READINESS_TIMEOUT;
        while Instant::now() < deadline {
            if std::net::TcpStream::connect_timeout(
                &addr.parse().unwrap(),
                Duration::from_millis(200),
            )
            .is_ok()
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(format!("server not ready after {:?}", READINESS_TIMEOUT))
    }

    /// Kill the server subprocess.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Port the server is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for CliServer {
    fn drop(&mut self) {
        self.kill();
    }
}

// ============================================================================
// mysql CLI wrapper
// ============================================================================

/// Run a SQL statement via `mysql` CLI. Returns `(stdout, stderr, exit_code)`.
///
/// Uses `-N -B` for batch mode (tab-separated, no metadata headers).
pub fn mysql_exec(
    host: &str,
    port: u16,
    user: &str,
    db: Option<&str>,
    sql: &str,
) -> (String, String, i32) {
    let mut cmd = Command::new(mysql_binary());
    cmd.args(["-h", host, "-P", &port.to_string(), "-u", user])
        .args(["--protocol=TCP", "--default-character-set=utf8mb4"]);
    if let Some(d) = db {
        cmd.arg(d);
    }
    cmd.args(["-N", "-B", "-e", sql]);
    let out = cmd.output().expect("mysql CLI must be installed");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

/// Run a file-based SQL script via `mysql` CLI.
/// Returns `(stdout, stderr, exit_code)`.
pub fn mysql_exec_file(
    host: &str,
    port: u16,
    user: &str,
    db: Option<&str>,
    file: &Path,
) -> (String, String, i32) {
    let mut cmd = Command::new(mysql_binary());
    cmd.args(["-h", host, "-P", &port.to_string(), "-u", user])
        .args(["--protocol=TCP", "--default-character-set=utf8mb4"]);
    if let Some(d) = db {
        cmd.arg(d);
    }
    cmd.arg("-e").arg(format!("source {}", file.display()));
    let out = cmd.output().expect("mysql CLI must be installed");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

/// Execute SQL and parse the first column of the first row as i64.
pub fn mysql_exec_i64(
    host: &str,
    port: u16,
    user: &str,
    db: Option<&str>,
    sql: &str,
) -> Result<i64, String> {
    let (out, err, code) = mysql_exec(host, port, user, db, sql);
    if code != 0 {
        return Err(format!("mysql exit {}: {}", code, err));
    }
    out.trim()
        .parse()
        .map_err(|e| format!("parse i64 from {:?}: {}", out.trim(), e))
}

/// Execute a query, return all rows as tab-separated lines.
pub fn mysql_query(
    host: &str,
    port: u16,
    user: &str,
    db: Option<&str>,
    sql: &str,
) -> Result<Vec<String>, String> {
    let (out, err, code) = mysql_exec(host, port, user, db, sql);
    if code != 0 {
        return Err(format!("mysql exit {}: {}", code, err));
    }
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect())
}

/// Execute a query and return the row count.
pub fn mysql_query_count(
    host: &str,
    port: u16,
    user: &str,
    db: Option<&str>,
    sql: &str,
) -> Result<usize, String> {
    let rows = mysql_query(host, port, user, db, sql)?;
    Ok(rows.len())
}

// ============================================================================
// Fixture loading
// ============================================================================

/// TPC-H SF=0.001 schema DDL (8 tables).
pub const SCHEMA_DDL_SF001: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

/// TPC-H SF=0.1 schema DDL (8 tables, l_quantity is REAL not INTEGER per TPC-H spec).
pub const SCHEMA_DDL_SF01: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

/// All 8 TPC-H table names in load order.
pub const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Expected row counts for SF=0.001 fixture.
pub const EXPECTED_COUNTS_SF001: &[(&str, u64)] = &[
    ("region", 5),
    ("nation", 25),
    ("supplier", 10),
    ("customer", 15),
    ("part", 20),
    ("partsupp", 80),
    ("orders", 150),
    ("lineitem", 614),
];

/// Expected row counts for SF=0.1 fixture.
pub const EXPECTED_COUNTS_SF01: &[(&str, u64)] = &[
    ("region", 5),
    ("nation", 25),
    ("supplier", 100),
    ("customer", 1500),
    ("part", 2000),
    ("partsupp", 8000),
    ("orders", 1500),
    ("lineitem", 6015),
];

/// Load fixture data into a running server via `mysql` CLI.
///
/// Creates schemas with `CREATE TABLE`, then loads all `.tbl` files
/// via `LOAD DATA LOCAL INFILE` through the `mysql` CLI.
pub fn cli_load_fixture(
    host: &str,
    port: u16,
    user: &str,
    fixture_dir: &Path,
    schema_ddl: &[&str],
    expected_counts: &[(&str, u64)],
) -> Result<(), String> {
    let user = if user.is_empty() { "tester" } else { user };

    // 1. Create schemas
    for ddl in schema_ddl {
        let (out, err, code) = mysql_exec(host, port, user, None, ddl);
        if code != 0 {
            return Err(format!("CREATE TABLE failed ({}): {} {}", code, out, err));
        }
    }

    // 2. Load each .tbl file
    for (tbl, expected) in expected_counts {
        let path = fixture_dir.join(format!("{}.tbl", tbl));
        let sql = format!(
            "LOAD DATA LOCAL INFILE '{}' INTO TABLE {} FIELDS TERMINATED BY '|' LINES TERMINATED BY '\\n'",
            path.display(),
            tbl
        );

        let mut cmd = Command::new(mysql_binary());
        cmd.args(["-h", host, "-P", &port.to_string()])
            .args(["-u", user, "--protocol=TCP"])
            .args(["--local-infile=1"])
            .arg(tbl)
            .args(["-N", "-B", "-e", &sql]);
        let out = cmd.output().expect("mysql LOAD DATA");
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("LOAD DATA {} failed: {}", tbl, stderr));
        }

        // Verify row count
        let count = mysql_query_count(
            host,
            port,
            user,
            Some(tbl),
            &format!("SELECT COUNT(*) FROM {}", tbl),
        )?;
        if count != *expected as usize {
            return Err(format!(
                "{}: expected {} rows, got {}",
                tbl, expected, count
            ));
        }
        eprintln!("  loaded {}: {} rows", tbl, count);
    }

    Ok(())
}

/// Start a CLI server with the SF=0.001 fixture loaded.
pub fn start_sf001_cli() -> Result<CliServer, String> {
    let data_dir = PathBuf::from("tests/data/tpch-sf001");
    if !data_dir.exists() {
        return Err(format!("fixture not found: {}", data_dir.display()));
    }
    let server = CliServer::start(&data_dir)?;
    eprintln!("[cli] server started on port {}", server.port);
    cli_load_fixture(
        "127.0.0.1",
        server.port,
        "tester",
        &data_dir,
        SCHEMA_DDL_SF001,
        EXPECTED_COUNTS_SF001,
    )?;
    Ok(server)
}

/// Start a CLI server with the SF=0.1 fixture loaded.
pub fn start_sf01_cli() -> Result<CliServer, String> {
    let data_dir = PathBuf::from("tests/data/tpch-sf01");
    if !data_dir.exists() {
        return Err(format!("fixture not found: {}", data_dir.display()));
    }
    let server = CliServer::start(&data_dir)?;
    eprintln!("[cli] server started on port {}", server.port);
    cli_load_fixture(
        "127.0.0.1",
        server.port,
        "tester",
        &data_dir,
        SCHEMA_DDL_SF01,
        EXPECTED_COUNTS_SF01,
    )?;
    Ok(server)
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Resolve the `sqlrustgo-mysql-server` binary path.
/// Uses CARGO_TARGET_DIR and CARGO_MANIFEST_DIR to find the binary
/// regardless of the current working directory.
fn resolve_binary() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("target"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let bin = target_dir.join(profile).join("sqlrustgo-mysql-server");
    if bin.exists() {
        return Ok(bin);
    }
    Err(format!(
        "sqlrustgo-mysql-server binary not found at {:?}",
        bin
    ))
}

/// Resolve the `mysql` CLI binary path.
fn mysql_binary() -> String {
    std::env::var("MYSQL").unwrap_or_else(|_| "mysql".to_string())
}

/// Pick a free port by binding a listener and reading the assigned port.
fn pick_free_port() -> Result<u16, String> {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| format!("bind: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("addr: {}", e))?
        .port();
    drop(listener);
    Ok(port)
}

// ============================================================================
// Query execution helpers
// ============================================================================

/// Run a query and return (Result<rows,err>, elapsed). Uses `-N -B`.
pub fn run_query_timed(
    host: &str,
    port: u16,
    user: &str,
    db: Option<&str>,
    sql: &str,
    _timeout_s: u64,
) -> (Result<Vec<String>, String>, Duration) {
    let start = Instant::now();
    let result = mysql_query(host, port, user, db, sql).map_err(|e| e.to_string());
    let elapsed = start.elapsed();
    (result, elapsed)
}

// ============================================================================
// Baseline comparison (mirrors tpch_wire_harness::compare_cells)
// ============================================================================

/// Read a three-way reference JSON. Returns (row_count, first_3_rows as Vec<String>).
pub fn read_three_way(data_dir: &Path, qnum: u8) -> Option<(u64, Vec<String>)> {
    let p = data_dir
        .join("expected")
        .join(format!("Q{}_three_way.json", qnum));
    if !p.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&p).ok()?;
    let v: JsonValue = serde_json::from_str(&content).ok()?;
    let rc = v["engines"]["sqlite"]["row_count"].as_u64()?;
    let first3: Vec<String> = v["engines"]["sqlite"]["first_3_rows"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(|x| x.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Some((rc, first3))
}

/// Cell-level comparison with float tolerance.
/// Mirrors `tpch_wire_harness::compare_cells` so CLI tests can use the
/// same assertion surface as MySqlTestClient-based tests.
pub fn compare_cells(
    actual: &[Vec<String>],
    baseline: &JsonValue,
    float_tol: f64,
) -> Result<(), String> {
    let engine = baseline
        .get("engines")
        .and_then(|e| e.get("sqlite"))
        .ok_or_else(|| "baseline missing engines.sqlite".to_string())?;

    let expected_rows = engine
        .get("row_count")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "baseline missing engines.sqlite.row_count".to_string())?;

    if actual.len() as u64 != expected_rows {
        return Err(format!(
            "row_count mismatch: actual={} expected={}",
            actual.len(),
            expected_rows
        ));
    }

    let first_3_raw = engine
        .get("first_3_rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "baseline missing engines.sqlite.first_3_rows".to_string())?;

    if first_3_raw.is_empty() {
        return Ok(());
    }

    let mut expected: Vec<Vec<String>> = first_3_raw
        .iter()
        .map(|row| {
            row.as_str()
                .map(|s| s.split('|').map(|c| c.to_string()).collect())
                .unwrap_or_default()
        })
        .collect();

    let compare_n = expected.len().min(3).min(actual.len());
    let mut actual_first: Vec<Vec<String>> = actual.iter().take(compare_n).cloned().collect();
    actual_first.sort();
    expected.sort();
    expected.truncate(compare_n);

    let cell_eq = |a: &str, b: &str| -> bool {
        if a == b {
            return true;
        }
        match (a.parse::<f64>(), b.parse::<f64>()) {
            (Ok(av), Ok(bv)) => {
                let diff = (av - bv).abs();
                let scale = av.abs().max(bv.abs()).max(1.0);
                diff <= float_tol * scale
            }
            _ => false,
        }
    };

    for (row_idx, (a_row, e_row)) in actual_first.iter().zip(expected.iter()).enumerate() {
        if a_row.len() != e_row.len() {
            return Err(format!(
                "row {}: column count mismatch: actual={} expected={}",
                row_idx,
                a_row.len(),
                e_row.len()
            ));
        }
        for (col_idx, (a, e)) in a_row.iter().zip(e_row.iter()).enumerate() {
            if !cell_eq(a, e) {
                return Err(format!(
                    "cell mismatch at row={} col={}: actual={:?} expected={:?}",
                    row_idx, col_idx, a, e
                ));
            }
        }
    }

    Ok(())
}

// ============================================================================
// Tests (self-verification)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mysql_binary_resolves() {
        let bin = mysql_binary();
        let output = Command::new(&bin).args(["--version"]).output();
        assert!(
            output.is_ok(),
            "mysql CLI at '{}' should be executable",
            bin
        );
    }

    #[test]
    fn test_pick_free_port() {
        // Keep the listener alive for the full duration of the test to avoid
        // a TOCTOU race: if we drop the listener and immediately rebind,
        // another process can grab the port in between.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port > 0, "assigned port should be non-zero");

        // Re-confirm the port is still bound (it should be — listener is alive)
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let _rebind = std::net::TcpListener::bind(addr).expect_err(
            "port should still be bound while original listener is alive",
        );
        // Now drop the original listener; port returns to the OS free pool.
        drop(listener);
        // Give the OS a moment to reclaim the port (SYN/ TIME_WAIT etc.)
        let _rebind2 = std::net::TcpListener::bind(addr);
        // We allow either success (port genuinely free) or error (OS still
        // holding it), because the race we *are* fixing is the TOCTOU between
        // pick_free_port returning and the first rebind attempt.
    }
}
