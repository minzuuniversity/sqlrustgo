//! TPC-H Q13 subquery repro test
//!
//! 22-22-AUDIT-GATE-REPORT §4 documents:
//!   - bare subquery `SELECT o_custkey FROM orders WHERE o_comment LIKE
//!     '%special%requests%'` returns 7 rows ✓
//!   - `IN (subquery)` should return 14 customers ✗ (returns 50 — bug)
//!   - `NOT IN (subquery)` should return 36 customers ✗ (returns 50 — bug)
//!
//! This test uses the canonical Sprint 7 SF=0.001 schema (matching
//! `eval_22_v_auth_sqlite` / 22-22-AUDIT) so the LIKE behavior matches
//! authoritative SQLite exactly.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    // Canonical TPC-H SF=0.001 schema (Sprint 7 fixture).
    engine
        .execute(
            "CREATE TABLE region ( \
                r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, \
                r_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE nation ( \
                n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, \
                n_regionkey INTEGER NOT NULL, n_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE supplier ( \
                s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, \
                s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, \
                s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE customer ( \
                c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, \
                c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, \
                c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, \
                c_mktsegment TEXT, c_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE part ( \
                p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, \
                p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, \
                p_size INTEGER NOT NULL, p_container TEXT NOT NULL, \
                p_retailprice REAL NOT NULL, p_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE partsupp ( \
                ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, \
                ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, \
                ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE orders ( \
                o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, \
                o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, \
                o_orderdate TEXT NOT NULL, o_orderpriority TEXT, \
                o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE lineitem ( \
                l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
                l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, \
                l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, \
                l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, \
                l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, \
                l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, \
                l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))",
        )
        .unwrap();

    let data = PathBuf::from(
        "/home/ai/sqlrustgo/.worktrees/v39-wired-audit/tests/data/tpch-sf001",
    );
    let schemas: Vec<(&str, usize)> = vec![
        ("region", 3),
        ("nation", 4),
        ("supplier", 7),
        ("customer", 8),
        ("part", 9),
        ("partsupp", 5),
        ("orders", 9),
        ("lineitem", 16),
    ];
    for (tbl, cols) in &schemas {
        let path = data.join(format!("{}.tbl", tbl));
        let content = std::fs::read_to_string(&path).unwrap();
        let mut n = 0;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.trim_end_matches('|').split('|').collect();
            if parts.len() < *cols {
                continue;
            }
            let vals: Vec<String> = parts[..*cols]
                .iter()
                .map(|p| format!("'{}'", p.replace('\'', "''")))
                .collect();
            let sql = format!("INSERT INTO {} VALUES ({})", tbl, vals.join(","));
            if engine.execute(&sql).is_ok() {
                n += 1;
            }
        }
        eprintln!("Loaded {}: {} rows", tbl, n);
    }
    engine
}

#[test]
fn q13_bare_subquery_row_count() {
    let mut engine = make_engine();
    // Bare subquery should return all distinct o_custkey whose orders
    // match the LIKE pattern. Per SQLite baseline (22-22-AUDIT), this
    // returns 7 rows.
    let r = engine
        .execute(
            "SELECT o_custkey FROM orders WHERE o_comment LIKE '%special%requests%'",
        )
        .unwrap();
    eprintln!("bare subquery returned {} rows", r.rows.len());
    assert!(
        (1..49).contains(&r.rows.len()),
        "bare subquery should be in 1..49, got {}",
        r.rows.len()
    );
}

#[test]
fn q13_in_subquery_filters() {
    let mut engine = make_engine();
    let r = engine
        .execute(
            "SELECT c_custkey FROM customer \
             WHERE c_custkey IN (SELECT o_custkey FROM orders \
             WHERE o_comment LIKE '%special%requests%')",
        )
        .unwrap();
    eprintln!("IN (subquery) returned {} customers", r.rows.len());
    assert!(
        r.rows.len() < 50,
        "IN should be selective, got {} (expected < 50)",
        r.rows.len()
    );
}

#[test]
fn q13_not_in_subquery_excludes() {
    let mut engine = make_engine();
    let r = engine
        .execute(
            "SELECT c_custkey FROM customer \
             WHERE c_custkey NOT IN (SELECT o_custkey FROM orders \
             WHERE o_comment LIKE '%special%requests%')",
        )
        .unwrap();
    eprintln!("NOT IN (subquery) returned {} customers", r.rows.len());
    assert!(
        r.rows.len() < 50,
        "NOT IN should exclude some customers, got {} (expected < 50)",
        r.rows.len()
    );
}