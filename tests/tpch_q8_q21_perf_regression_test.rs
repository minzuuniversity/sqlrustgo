//! TPC-H Q8 + Q21 perf regression test (Sprint 6 v3.9.0).
//!
//! Before Sprint 6 fixes, both queries timed out (>30s) at SF=0.1:
//!
//! - Q8 30s+ timeout → < 1s (PR #3341, 150x speedup)
//! - Q21 30s+ timeout → < 3s (PR #3336+#3341, predicate pushdown)
//!
//! Root causes (both fixed in `src/engine_select.rs`):
//!
//! - **Q8**: `pre_filter_cartesian_right_table` did column lookup
//!   with `c.name == col_name`, but `execute_single_join` renames
//!   aliased right-table columns to `<alias>.<col>` (e.g. `n2.n_name`).
//!   Filter silently failed for n2/region joins, leaving all 25
//!   nations unfiltered → 60K × 25 cartesian product.
//!
//! - **Q21**: `pre_eval_exists_subquery_fast` called `storage.scan`
//!   with the `table|alias` encoded name (`"lineitem|l2"`) — storage
//!   doesn't recognize the alias suffix → fast path returns None →
//!   slow `execute_select` fallback scans 60K lineitems per row.
//!
//! This test pins the performance budget to ensure these regressions
//! cannot silently return.

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

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

fn data_dir() -> PathBuf {
    env::var("TPCH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/tpch_sf01_v2"))
}

fn load_tbl(path: &PathBuf, n_cols: usize) -> Vec<Vec<String>> {
    let f = File::open(path).expect("open .tbl");
    let r = BufReader::new(f);
    let mut rows = Vec::new();
    for line in r.lines() {
        let line = line.unwrap();
        let line = line.trim_end_matches('\n');
        let parts: Vec<&str> = line.split('|').collect();
        let parts: Vec<String> = if parts.last() == Some(&"") {
            parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            parts.iter().map(|s| s.to_string()).collect()
        };
        if parts.len() < n_cols {
            continue;
        }
        rows.push(parts);
    }
    rows
}

fn make_engine() -> Option<ExecutionEngine<MemoryStorage>> {
    let dir = data_dir();
    if !dir.exists() {
        eprintln!("Data dir not found: {}", dir.display());
        return None;
    }
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for ddl in SCHEMA_SQL {
        if let Err(e) = engine.execute(ddl) {
            eprintln!("DDL failed: {e}");
            return None;
        }
    }
    for (tbl, n) in [
        ("region", 3),
        ("nation", 4),
        ("supplier", 7),
        ("customer", 8),
        ("part", 9),
        ("partsupp", 5),
        ("orders", 9),
        ("lineitem", 16),
    ] {
        let path = dir.join(format!("{tbl}.tbl"));
        if !path.exists() {
            eprintln!("Missing: {}", path.display());
            return None;
        }
        let rows = load_tbl(&path, n);
        let n_rows = rows.len();
        if let Err(e) = engine.execute(&format!(
            "INSERT INTO {tbl} VALUES {}",
            rows.iter()
                .map(|r| format!(
                    "({})",
                    r.iter()
                        .map(|v| format!("'{}'", v.replace('\'', "''")))
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .collect::<Vec<_>>()
                .join(",")
        )) {
            eprintln!("INSERT {tbl} failed: {e}");
            return None;
        }
        eprintln!("  loaded {tbl}: {n_rows} rows");
    }
    Some(engine)
}

#[test]
fn q8_alias_prefilter_perf() {
    // TPC-H Q8: 7-table comma-join. Before fix: 30s+ timeout.
    // After fix (PR #3341): < 1s on SF=0.1 (60K lineitems).
    let Some(mut engine) = make_engine() else {
        eprintln!("SKIP: data not available");
        return;
    };
    let sql = std::fs::read_to_string("queries/q8.sql").expect("queries/q8.sql");
    let start = Instant::now();
    let r = engine.execute(&sql).expect("Q8 execute");
    let elapsed = start.elapsed();
    eprintln!("Q8: {} rows, elapsed={:?}", r.rows.len(), elapsed);
    assert!(
        elapsed.as_secs() < 5,
        "Q8 took {elapsed:?} (budget 5s) — alias pre-filter regressed"
    );
}

#[test]
fn q21_exists_alias_perf() {
    // TPC-H Q21: 4-table join with 2 EXISTS correlated subqueries.
    // Before fix (PR #3336+#3341, predicate pushdown):
    //   30s+ timeout (storage.scan with "lineitem|l2" fails → slow path).
    // After fix: < 3s on SF=0.1.
    let Some(mut engine) = make_engine() else {
        eprintln!("SKIP: data not available");
        return;
    };
    let sql = std::fs::read_to_string("queries/q21.sql").expect("queries/q21.sql");
    let start = Instant::now();
    let r = engine.execute(&sql).expect("Q21 execute");
    let elapsed = start.elapsed();
    eprintln!("Q21: {} rows, elapsed={:?}", r.rows.len(), elapsed);
    assert!(
        elapsed.as_secs() < 5,
        "Q21 took {elapsed:?} (budget 5s) — subquery fast-path regressed"
    );
}
