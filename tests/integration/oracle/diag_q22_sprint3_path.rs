//! V312-58 Sprint 3 Phase 2 — Q22 path diagnostic
//!
//! Q22 has TWO correlated subqueries:
//! 1. `c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE ...)`
//!    → handled by try_scalar_agg_index_lookup OR scalar_subq_cache
//!    fallback (depends on whether inner WHERE has equality)
//! 2. `NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)`
//!    → anti-semi-join (Phase 3 HashAntiSemiJoin target)
//!
//! Run:
//!   cargo test --test diag_q22_sprint3_path --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp";

// SQLite oracle for Q22 on 1000 customer / 6000 order subset.
const Q22_MINI_ORACLE: &[(&str, i64, f64)] = &[
    ("13", 16, 119189.58),
    ("17", 12, 93336.45),
    ("18", 16, 126528.67),
    ("23", 13, 97852.70),
    ("29", 18, 134837.46),
    ("30", 24, 182999.02),
    ("31", 13, 100587.77),
];

// SQLite oracle for Q22 on 150K customer / 1.5M order subset (full SF=0.01).
const Q22_100K_ORACLE: &[(&str, i64, f64)] = &[
    ("13", 888, 6737713.99),
    ("17", 861, 6460573.72),
    ("18", 964, 7236687.40),
    ("23", 892, 6701457.95),
    ("29", 948, 7158866.63),
    ("30", 909, 6808436.13),
    ("31", 922, 6806670.18),
];
const Q22_MINI_NUMCUST_TOL: i64 = 0; // exact integer match required
const Q22_MINI_TOTACCT_TOL: f64 = 0.01; // sum tolerance

const Q22_SQL: &str = "SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM (SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale GROUP BY cntrycode ORDER BY cntrycode";

fn run_q22(customer_tbl: &str, orders_tbl: &str) -> (f64, Vec<(String, i64, f64)>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    let t_create = Instant::now();
    engine.execute("CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)").unwrap();
    engine.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)").unwrap();
    let t_create_elapsed = t_create.elapsed().as_secs_f64();
    let t_bulk = Instant::now();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("customer", &format!("{}/{}", DATA_DIR, customer_tbl))
            .unwrap();
        st.bulk_load_tbl_file("orders", &format!("{}/{}", DATA_DIR, orders_tbl))
            .unwrap();
    }
    let t_bulk_elapsed = t_bulk.elapsed().as_secs_f64();
    eprintln!(
        "  setup: CREATE TABLE {:.3}s + bulk_load {:.3}s",
        t_create_elapsed, t_bulk_elapsed
    );

    reset_v312_58_sprint3_diag();
    let start = Instant::now();
    let r = engine
        .execute(Q22_SQL)
        .unwrap_or_else(|e| panic!("Q22 failed: {}", e));
    let elapsed = start.elapsed().as_secs_f64();

    let rows: Vec<(String, i64, f64)> = r
        .rows
        .iter()
        .map(|row| {
            let cntrycode = match &row[0] {
                Value::Text(s) => s.clone(),
                Value::Integer(i) => format!("{:02}", i),
                other => panic!("unexpected cntrycode type: {:?}", other),
            };
            let numcust = match &row[1] {
                Value::Integer(i) => *i,
                other => panic!("unexpected numcust type: {:?}", other),
            };
            let totacctbal = match &row[2] {
                Value::Float(f) => *f,
                Value::Integer(i) => *i as f64,
                Value::Text(s) => s.parse::<f64>().unwrap_or(0.0),
                other => panic!("unexpected totacctbal type: {:?}", other),
            };
            (cntrycode, numcust, totacctbal)
        })
        .collect();

    (elapsed, rows)
}

#[test]
#[ignore]
fn diag_q22_mini_path() {
    eprintln!("=== Q22 mini path diagnostic (1000 customer / 6000 orders) ===");
    let (elapsed, rows) = run_q22("q22_customer_mini.tbl", "q22_orders_mini.tbl");
    eprintln!("Q22 mini: elapsed {:.3}s, {} cntrycode groups", elapsed, rows.len());
    for (cc, nc, ta) in &rows {
        eprintln!("  {}: numcust={}, totacctbal={}", cc, nc, ta);
    }
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());

    // Oracle compare
    assert_eq!(rows.len(), Q22_MINI_ORACLE.len(), "Q22 row count mismatch");
    for (i, (got, exp)) in rows.iter().zip(Q22_MINI_ORACLE.iter()).enumerate() {
        assert_eq!(
            got.0, exp.0,
            "Q22 row {}: cntrycode mismatch got={} expect={}",
            i, got.0, exp.0
        );
        let nc_diff = (got.1 - exp.1).abs();
        assert!(
            nc_diff <= Q22_MINI_NUMCUST_TOL,
            "Q22 row {}: numcust diff {} > tol {}",
            i,
            nc_diff,
            Q22_MINI_NUMCUST_TOL
        );
        let ta_diff = (got.2 - exp.2).abs();
        assert!(
            ta_diff < Q22_MINI_TOTACCT_TOL,
            "Q22 row {} ({}): totacctbal diff {} > tol {} (got={} expect={})",
            i,
            got.0,
            ta_diff,
            Q22_MINI_TOTACCT_TOL,
            got.2,
            exp.2
        );
    }
    eprintln!("✓ Q22 mini oracle MATCH ({} rows)", rows.len());
}

fn assert_q22_oracle(test_name: &str, rows: &[(String, i64, f64)], oracle: &[(&str, i64, f64)]) {
    assert_eq!(rows.len(), oracle.len(), "{} row count mismatch", test_name);
    for (i, (got, exp)) in rows.iter().zip(oracle.iter()).enumerate() {
        assert_eq!(got.0, exp.0, "{} row {}: cntrycode mismatch", test_name, i);
        assert_eq!(
            got.1, exp.1,
            "{} row {} ({}): numcust mismatch got={} expect={}",
            test_name, i, got.0, got.1, exp.1
        );
        let ta_diff = (got.2 - exp.2).abs();
        assert!(
            ta_diff < Q22_MINI_TOTACCT_TOL,
            "{} row {} ({}): totacctbal diff {} > tol {} (got={} expect={})",
            test_name,
            i,
            got.0,
            ta_diff,
            Q22_MINI_TOTACCT_TOL,
            got.2,
            exp.2
        );
    }
}

#[test]
#[ignore]
fn diag_q22_30k_path() {
    eprintln!("=== Q22 30K customer / 300K orders path diagnostic ===");
    let (elapsed, rows) = run_q22("q22_customer_20k.tbl", "q22_orders_200k.tbl");
    eprintln!(
        "Q22 30K: elapsed {:.2}s, {} cntrycode groups",
        elapsed,
        rows.len()
    );
    for (cc, nc, ta) in &rows {
        eprintln!("  {}: numcust={}, totacctbal={}", cc, nc, ta);
    }
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());
}

#[test]
#[ignore]
fn diag_q22_60k_path() {
    eprintln!("=== Q22 60K customer / 600K orders path diagnostic ===");
    let (elapsed, rows) = run_q22("q22_customer_60k.tbl", "q22_orders_600k.tbl");
    eprintln!(
        "Q22 60K: elapsed {:.2}s, {} cntrycode groups",
        elapsed,
        rows.len()
    );
    for (cc, nc, ta) in &rows {
        eprintln!("  {}: numcust={}, totacctbal={}", cc, nc, ta);
    }
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());
}

#[test]
#[ignore]
fn diag_q22_100k_path() {
    eprintln!("=== Q22 100K customer / 1.5M orders path diagnostic ===");
    let (elapsed, rows) = run_q22("customer_clean.tbl", "orders_clean.tbl");
    eprintln!(
        "Q22 100K: elapsed {:.2}s, {} cntrycode groups",
        elapsed,
        rows.len()
    );
    for (cc, nc, ta) in &rows {
        eprintln!("  {}: numcust={}, totacctbal={}", cc, nc, ta);
    }
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());
    assert_q22_oracle("Q22 100K", &rows, Q22_100K_ORACLE);
    eprintln!("✓ Q22 100K oracle MATCH ({} rows)", rows.len());
}
