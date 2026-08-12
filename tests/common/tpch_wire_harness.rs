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

/// Start ephemeral server + load fixture dir via LOAD DATA LOCAL INFILE.
///
/// `timeout_s`: Some(s) → set read/write timeouts to s seconds; None → use
/// the `MySqlTestClient` default (5s for read, 5s for write).
fn start_with_fixture(fixture_dir: &str, timeout_s: Option<u64>) -> MySqlTestClient {
    let data_dir = PathBuf::from(fixture_dir);
    let config = EphemeralConfig {
        data_dir: Some(data_dir),
        bootstrap_tables: false,
        bootstrap_users: true,        metrics_port: None,

        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle");
    if let Some(t) = timeout_s {
        client
            .set_timeouts(Duration::from_secs(t), Duration::from_secs(t))
            .expect("set_timeouts");
    }
    load_fixture(&mut client, fixture_dir);
    client
}

/// Start ephemeral server + load SF=0.001 fixture (5s default timeouts).
pub fn start_sf001() -> MySqlTestClient {
    start_with_fixture(SF001_DIR, None)
}

/// Start ephemeral server + load SF=0.1 fixture (60s timeouts for larger data).
pub fn start_sf01() -> MySqlTestClient {
    start_with_fixture(SF01_DIR, Some(60))
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

/// Cell-level comparison with float tolerance.
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
                "row {}: column count mismatch: actual={} expected={} (actual={:?}, expected={:?})",
                row_idx,
                a_row.len(),
                e_row.len(),
                a_row,
                e_row
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

#[cfg(test)]
mod tests {
    use super::{compare_cells, JsonValue};
    use serde_json::json;

    fn baseline(row_count: u64, first_3_rows: &[&str]) -> JsonValue {
        json!({
            "engines": {
                "sqlite": {
                    "row_count": row_count,
                    "first_3_rows": first_3_rows,
                }
            }
        })
    }

    #[test]
    fn all_cells_match_returns_ok() {
        let actual = vec![
            vec!["A".into(), "1".into(), "1.5".into()],
            vec!["B".into(), "2".into(), "2.5".into()],
        ];
        let b = baseline(2, &["A|1|1.5", "B|2|2.5"]);
        assert!(compare_cells(&actual, &b, 1e-3).is_ok());
    }

    #[test]
    fn row_count_mismatch_returns_err_with_count_message() {
        let actual = vec![vec!["A".into()]];
        let b = baseline(2, &["A|x"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(
            err.contains("row_count mismatch"),
            "expected row_count message, got: {err}"
        );
        assert!(err.contains("actual=1"), "got: {err}");
        assert!(err.contains("expected=2"), "got: {err}");
    }

    #[test]
    fn single_cell_mismatch_returns_err_with_position() {
        let actual = vec![
            vec!["A".into(), "1".into(), "1.5".into()],
            vec!["B".into(), "WRONG".into(), "2.5".into()],
        ];
        let b = baseline(2, &["A|1|1.5", "B|2|2.5"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(
            err.contains("row=1") && err.contains("col=1"),
            "expected cell position row=1 col=1, got: {err}"
        );
        assert!(err.contains("WRONG"), "actual value missing in: {err}");
        assert!(err.contains("\"2\""), "expected value missing in: {err}");
    }

    #[test]
    fn float_tolerance_accepts_close_values() {
        let actual = vec![vec!["1.0001".into()]];
        let b = baseline(1, &["1.0"]);
        assert!(compare_cells(&actual, &b, 1e-3).is_ok());
    }

    #[test]
    fn float_tolerance_rejects_far_values() {
        let actual = vec![vec!["1.5".into()]];
        let b = baseline(1, &["1.0"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(
            err.contains("cell mismatch"),
            "expected cell mismatch, got: {err}"
        );
    }

    #[test]
    fn empty_first_3_rows_skips_cell_check() {
        let actual: Vec<Vec<String>> = vec![];
        let b = baseline(0, &[]);
        assert!(compare_cells(&actual, &b, 1e-3).is_ok());
    }

    #[test]
    fn missing_engines_sqlite_returns_err() {
        let bad: JsonValue = json!({});
        let actual: Vec<Vec<String>> = vec![];
        let err = compare_cells(&actual, &bad, 1e-3).unwrap_err();
        assert!(err.contains("engines.sqlite"), "got: {err}");
    }

    #[test]
    fn column_count_mismatch_returns_err() {
        let actual = vec![vec!["A".into(), "1".into()]];
        let b = baseline(1, &["A|1|extra"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(err.contains("column count mismatch"), "got: {err}");
    }
}
