//! TPC-H 22 audit on **SF=0.01** canonical fixture (in-process).
//! Compare sqlrustgo row counts to SQLite ground truth.
//!
//! ## Fixture path refresh (v3.9.0, branch fix/q13-not-in-subquery)
//!
//! Originally this audit pointed at `/tmp/tpch_3way_sf001/clean` — a
//! one-shot Sprint 5 v8 staging directory that has since been cleaned up.
//! The replacement lives in the worktree:
//!
//! - `tests/data/tpch-sf01/` — 8 `.tbl` files for region/nation/supplier/
//!   customer/part/partsupp/orders/lineitem (lineitem ≈ 60 000 rows,
//!   SF=0.01 canonical — see commit `044338ff`).
//!
//! The companion SQLite ground-truth DB at `/tmp/tpch_3way_sf001.db` was
//! likewise rebuilt from the same `.tbl` files (Python loader) and is
//! reused as `/tmp/tpch_sf01_audit.db`.
//!
//! To regenerate the SQLite baseline if it is missing:
//!
//! ```text
//! python3 scripts/dev/build_tpch_sf01_sqlite.py
//! ```
//!
//! ## Q9 hang fix (commit 3e7d5e56)
//!
//! The 6-table join in Q9 used to hang for 30+ minutes at SF=0.01 because
//! RIGHT/FULL join bookkeeping did an O(N) `Vec::position(...)` per match
//! in `execute_single_join`. The fix stores the row index alongside the
//! `&Vec<Value>` in the right-side hashmap (see `src/engine_select.rs`
//! around line 1346), turning the bookkeeping into O(1) per match and
//! letting Q9 complete in <1 s on SF=0.01 (lineitem=60K).

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock};

/// SF=0.01 canonical fixture directory (60 000 lineitem rows).
///
/// Resolved relative to the manifest dir at test time so this test is
/// independent of the developer machine layout.
const FIXTURE_DIR_RELATIVE: &str = "tests/data/tpch-sf01";

/// SQLite ground-truth database rebuilt from the same `.tbl` files.
const SQLITE_BASELINE_DB: &str = "/tmp/tpch_sf01_audit.db";

const QUERIES_DIR: &str = "/home/ai/sqlrustgo/queries";

fn fixture_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR points at the worktree root for integration tests
    // (tests/*.rs), which is where `tests/data/tpch-sf01/` lives.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_DIR_RELATIVE)
}

const SCHEMA_SQL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

fn load_tbl_file(
    storage: &Arc<RwLock<MemoryStorage>>,
    tbl_name: &str,
    tbl_path: &PathBuf,
    columns: usize,
) -> Result<usize, String> {
    let content = fs::read_to_string(tbl_path)
        .map_err(|e| format!("Cannot read {}: {}", tbl_path.display(), e))?;
    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let line = line.trim_end_matches('|');
        let values: Vec<&str> = line.split('|').collect();
        if values.len() != columns {
            continue;
        }
        let record: Vec<SqlValue> = values
            .iter()
            .map(|v| {
                let s = v.trim();
                if s.is_empty() {
                    SqlValue::Null
                } else if let Ok(i) = s.parse::<i64>() {
                    SqlValue::Integer(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    SqlValue::Float(f)
                } else {
                    SqlValue::Text(s.to_string())
                }
            })
            .collect();
        batch.push(record);
        if batch.len() >= BATCH_SIZE {
            let mut storage = storage.write().map_err(|e| format!("Lock error: {}", e))?;
            storage
                .insert(tbl_name, batch.clone())
                .map_err(|e| format!("Insert error: {}", e))?;
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut storage = storage.write().map_err(|e| format!("Lock error: {}", e))?;
        storage
            .insert(tbl_name, batch.clone())
            .map_err(|e| format!("Insert error: {}", e))?;
        count += batch.len();
    }
    Ok(count)
}

#[test]
fn test_tpch_22_inprocess_sf01() {
    eprintln!("\n=== TPC-H 22 in-process audit (SF=0.01, AFTER Q9 O(1) hash fix) ===\n");
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    for ddl in SCHEMA_SQL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {ddl} - {e}"));
    }
    eprintln!("  Schema created ({} tables)", SCHEMA_SQL.len());

    let base = fixture_dir();
    let mut total_rows = 0usize;
    for tbl in TABLES {
        let tbl_path = base.join(format!("{}.tbl", tbl));
        let content = fs::read_to_string(&tbl_path).expect("read");
        let columns = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim_end_matches('|').split('|').count())
            .unwrap_or(0);
        let n = load_tbl_file(&storage, tbl, &tbl_path, columns)
            .unwrap_or_else(|e| panic!("Failed to load {tbl}: {e}"));
        eprintln!("  {tbl}: {n} rows");
        total_rows += n;
    }
    eprintln!("  total: {total_rows} rows");

    let mut matched = 0;
    let mut mismatched = 0;
    let mut err_count = 0;
    for q in 1..=22 {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, q);
        let sql = fs::read_to_string(&sql_path).unwrap();
        let sql = sql.trim().trim_end_matches(';');

        // SQLite ground truth
// Strip the trailing `ORDER BY ...` clause from the SQL before
// wrapping in `SELECT COUNT(*) FROM (...) sub`. ORDER BY in a
// subquery is a no-op for row count anyway, but SQLite's parser
// rejects nested `(SELECT ... ORDER BY) sub` with a syntax
// error. Engine wrapper tolerates ORDER BY, but we want the
// SQLite baseline to succeed.
 let sql_no_order = sql
 .rsplit_once("ORDER BY")
 .map(|(head, _)| head.trim().trim_end_matches(';'))
 .unwrap_or(sql);
// `EXTRACT(YEAR FROM X) [AS Y]` → `CAST(strftime('%Y', X) AS INTEGER) [AS Y]`.
// This handles both the SELECT projection (with alias) and any
// GROUP BY (often without alias — SQLite doesn't accept EXTRACT
// at all, so we must rewrite it everywhere it appears).
 let sqlite_sql = if q ==7 || q ==8 || q ==9 {
 let re_extract = regex::Regex::new(r"EXTRACT\(YEAR FROM ([^)]+)\)(?: AS (\w+))?").unwrap();
 re_extract
 .replace_all(&sql_no_order, |caps: &regex::Captures| {
 let col = caps.get(1).unwrap().as_str();
 match caps.get(2) {
 Some(alias) => format!("CAST(strftime('%Y', {}) AS INTEGER) AS {}", col, alias.as_str()),
 None => format!("CAST(strftime('%Y', {}) AS INTEGER)", col),
 }
 })
 .to_string()
 } else {
 sql_no_order.to_string()
 };
 let sqlite_out = Command::new("sqlite3")
 .args([
 SQLITE_BASELINE_DB,
 &format!("SELECT COUNT(*) FROM ({}) sub", sqlite_sql),
 ])
 .output()
 .ok();
        let s_rc: Option<i64> = sqlite_out
            .as_ref()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok());

        // Engine result: wrap in COUNT(*) so we get the row count of
        // the result set, not a result column.
        let wrap = format!("SELECT COUNT(*) AS c FROM ({}) sub", sql);
        let engine_count: Option<i64> = match engine.execute(&wrap) {
            Ok(r) => r
                .rows
                .first()
                .and_then(|row| row.first())
                .and_then(|v| match v {
                    SqlValue::Integer(i) => Some(*i),
                    SqlValue::Float(f) => Some(*f as i64),
                    _ => None,
                }),
            Err(e) => {
                eprintln!("Engine ERR Q{q}: {e}");
                None
            }
        };

        let cat = match (engine_count, s_rc) {
            (Some(e), Some(s)) if e == s => {
                matched += 1;
                "MATCH"
            }
            (Some(_), Some(_)) => {
                mismatched += 1;
                "MISMATCH"
            }
            (None, _) => {
                err_count += 1;
                "ERR"
            }
            (Some(_), None) => {
                mismatched += 1;
                "MISMATCH (no baseline)"
            }
        };
        eprintln!(
            "Q{:>2}: engine={:>6?} sqlite={:>6?}  {}",
            q, engine_count, s_rc, cat
        );
    }
    eprintln!("\n=== Totals ({} rows loaded) ===", total_rows);
    eprintln!("MATCHED   : {matched}/22");
    eprintln!("MISMATCHED: {mismatched}/22");
    eprintln!("ERR       : {err_count}/22");
    assert!(
        mismatched == 0 && err_count == 0,
        "TPC-H gate failure: {mismatched} mismatched, {err_count} errors"
    );
}
