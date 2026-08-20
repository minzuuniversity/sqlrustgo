//! Regression test for V312-48 issue #4274: canonical TPC-H Q8 8-way comma-join
//! returns 7 rows (1992-1998) instead of 2 rows (1995-1996) because the
//! date-range predicate `o_orderdate >= '1995-01-01' AND o_orderdate < '1996-12-31'`
//! is silently dropped after the comma-join hash-chain fast path runs.
//!
//! Root cause: `try_comma_join_hash_chain` only consumes `=` equi-join
//! predicates between joined tables. The thread-local flag
//! `COMMA_JOIN_WHERE_CONSUMED` was then set to `true` whenever the
//! chain succeeded, which caused the post-join `eval_predicate` step to
//! skip the entire WHERE clause — including range predicates like Q8's
//! date filter.
//!
//! Fix: a new helper `where_expr_has_unhandled_residual` detects any
//! non-`=` predicate in WHERE (range, LIKE, BETWEEN, IN-list, etc.).
//! `where_fully_consumed` now requires the WHERE to contain NO residual
//! predicate, so the date filter survives.
//!
//! Expected baseline (SQLite, sf1): 2 rows (years 1995 and 1996).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::Arc;

const FIXTURE: &str = "/home/openclaw/sqlrustgo_work/tests/data/tpch-sf01";

fn load(storage: &Arc<RwLock<MemoryStorage>>, tbl: &str, ncols: usize) -> usize {
    let path = format!("{}/{}.tbl", FIXTURE, tbl);
    let content = fs::read_to_string(&path).expect("read");
    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split('|').collect();
        if values.len() < ncols {
            continue;
        }
        let record: Vec<SqlValue> = values[..ncols]
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
            let mut s = storage.write();
            let _ = s.insert(tbl, batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write();
        let _ = s.insert(tbl, batch.clone());
        count += batch.len();
    }
    count
}

#[test]
#[ignore] // Heavy: requires ~6M lineitem rows; run with --ignored
fn q8_canonical_8way_date_range() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute(
            "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, \
         c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, \
         c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, \
         o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, \
         o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, \
         o_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
         l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, \
         l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
         l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, \
         l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, \
         l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, \
         s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, \
         s_acctbal REAL NOT NULL, s_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, \
         n_regionkey INTEGER NOT NULL, n_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, \
         r_comment TEXT NOT NULL)",
        )
        .unwrap();

    load(&storage, "customer", 8);
    load(&storage, "orders", 9);
    load(&storage, "lineitem", 16);
    load(&storage, "supplier", 7);
    load(&storage, "nation", 4);
    load(&storage, "region", 3);

    let sql = "SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year, \
                      SUM(CASE WHEN n2.n_name = 'GERMANY' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) \
                      / SUM(l_extendedprice * (1 - l_discount)) AS mkt_share \
               FROM customer, orders, lineitem, supplier, nation n1, nation n2, region \
               WHERE c_custkey = o_custkey \
                 AND l_orderkey = o_orderkey \
                 AND l_suppkey = s_suppkey \
                 AND c_nationkey = n1.n_nationkey \
                 AND s_nationkey = n1.n_nationkey \
                 AND s_nationkey = n2.n_nationkey \
                 AND n1.n_regionkey = r_regionkey \
                 AND r_name = 'EUROPE' \
                 AND n2.n_name = 'GERMANY' \
                 AND o_orderdate >= '1995-01-01' \
                 AND o_orderdate <  '1996-12-31' \
               GROUP BY EXTRACT(YEAR FROM o_orderdate) \
               ORDER BY o_year";

    let r = engine.execute(sql).unwrap();
    // SQLite sf1 baseline: 2 rows (o_year=1995 and o_year=1996).
    assert_eq!(
        r.rows.len(),
        2,
        "Q8 row count must match SQLite baseline (2 years: 1995, 1996); got {}",
        r.rows.len()
    );

    // Sanity-check the years themselves.
    let year_1995 = r
        .rows
        .iter()
        .any(|row| matches!(row.first(), Some(SqlValue::Integer(1995))));
    let year_1996 = r
        .rows
        .iter()
        .any(|row| matches!(row.first(), Some(SqlValue::Integer(1996))));
    assert!(year_1995, "missing 1995 row");
    assert!(year_1996, "missing 1996 row");
}
