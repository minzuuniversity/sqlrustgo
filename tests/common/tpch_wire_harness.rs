//! TPC-H wire-protocol test harness — shared helper for 13 in-process
//! → wire protocol migration tests/tpch_*.rs files.

use super::MySqlTestClient;
use serde_json::Value as JsonValue;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SF001_DIR: &str = "tests/data/tpch-sf001";
pub const SF01_DIR: &str = "tests/data/tpch-sf01";

pub const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

pub const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Start ephemeral server + load SF=0.001 fixture
pub fn start_sf001() -> MySqlTestClient {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral for sf001");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle for sf001");
    load_fixture(&mut client, SF001_DIR);
    client
}

/// Start ephemeral server + load SF=0.1 fixture (with 60s timeouts for larger data)
pub fn start_sf01() -> MySqlTestClient {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral for sf01");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle for sf01");
    client
        .set_timeouts(Duration::from_secs(60), Duration::from_secs(60))
        .expect("set_timeouts");
    load_fixture(&mut client, SF01_DIR);
    client
}

/// Load all 8 .tbl files via LOAD DATA LOCAL INFILE
pub fn load_fixture(client: &mut MySqlTestClient, fixture_dir: &str) {
    for ddl in SCHEMA_DDL {
        client
            .exec(ddl)
            .unwrap_or_else(|e| panic!("exec DDL `{ddl}`: {e}"));
    }
    let dir = PathBuf::from(fixture_dir);
    for tbl in TABLES {
        let path = dir.join(format!("{tbl}.tbl"));
        if !path.exists() {
            panic!("fixture not found: {}", path.display());
        }
        let n = client
            .load_local_infile(&path, tbl)
            .unwrap_or_else(|e| panic!("load_local_infile {}: {}", path.display(), e));
        eprintln!("  loaded {tbl}: {n} rows");
    }
}

/// Run a single query with timing, return (Result<rows>, elapsed)
pub fn run_query_timed(
    client: &mut MySqlTestClient,
    sql: &str,
    timeout_s: u64,
) -> (Result<Vec<Vec<String>>, String>, Duration) {
    client
        .set_timeouts(
            Duration::from_secs(timeout_s),
            Duration::from_secs(timeout_s),
        )
        .expect("set_timeouts");
    let start = Instant::now();
    let result = client.query_rows(sql);
    let elapsed = start.elapsed();
    (result.map_err(|e| e.to_string()), elapsed)
}

/// Read SQLite/MariaDB/PG baseline JSON
pub fn read_baseline(path: &Path) -> JsonValue {
    let body =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("parse JSON {}: {}", path.display(), e))
}

/// Cell-level comparison with float tolerance
pub fn compare_cells(
    actual: &[Vec<String>],
    baseline: &JsonValue,
    _float_tol: f64,
) -> Result<(), String> {
    let expected_rows = baseline
        .get("row_count")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "baseline missing row_count".to_string())?;
    if actual.len() as u64 != expected_rows {
        return Err(format!(
            "row_count mismatch: actual={} expected={}",
            actual.len(),
            expected_rows
        ));
    }
    Ok(())
}
