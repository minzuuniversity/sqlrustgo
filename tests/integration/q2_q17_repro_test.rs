use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

// TPC-H SF=1 regression for V312-35 / #4182:
// correlated scalar subqueries (Q2 MIN-over-join, Q17 AVG threshold).
// Row counts / values must match the SQLite oracle:
//   Q2  = 44 rows   (full query incl. ORDER BY)
//   Q17 = 23512.7528571429
//
// data/tpch-tiny is a symlink to data/tpch-sf01 in the repo, so this
// works regardless of which constant name is used.
const DATA_DIR: &str = "data/tpch-sf01";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

const TBL_FILES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for s in SCHEMAS {
        engine.execute(s).unwrap();
    }
    {
        let mut st = storage.write();
        for t in TBL_FILES {
            let path = format!("{}/{}.tbl", DATA_DIR, t);
            st.bulk_load_tbl_file(t, &path).unwrap();
        }
    }
    engine
}

fn scalar_float(rows: &[Vec<sqlrustgo::Value>]) -> f64 {
    match rows[0].get(0) {
        Some(sqlrustgo::Value::Float(f)) => *f,
        _ => panic!("expected Float scalar, got {:?}", rows[0].get(0)),
    }
}

#[test]
fn q2_correlated_min_over_join() {
    // V312-35 / #4182: `ps_supplycost = (SELECT MIN(ps_supplycost) FROM
    // partsupp, supplier, nation, region WHERE p_partkey = ps_partkey ...)`
    // The subquery's own columns (ps_partkey, s_suppkey, n_*, r_*) must NOT
    // be substituted with outer-row values, while the outer `p_partkey`
    // must be. A wrong substitution yields 0 rows; the oracle is 44.
    let mut engine = setup();
    let q2 = "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM supplier, nation, region, part, partsupp WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND p_size = 15 AND p_type LIKE '%BRASS' AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE' AND ps_supplycost = (SELECT MIN(ps_supplycost) FROM partsupp, supplier, nation, region WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE') ORDER BY s_acctbal DESC, n_name, s_name, p_partkey LIMIT 100";
    let res = engine
        .execute(q2)
        .unwrap_or_else(|e| panic!("Q2 failed: {}", e));
    assert_eq!(
        res.rows.len(),
        44,
        "Q2 expected 44 rows (SQLite oracle), got {}",
        res.rows.len()
    );
}

#[test]
fn q17_avg_threshold_subquery() {
    // V312-35 / #4182: `l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM
    // lineitem WHERE l_partkey = p_partkey)`. The comma-join hash chain
    // consumes only equality predicates; a correlated subquery residual
    // must be re-evaluated per row in the post-join filter. If the WHERE
    // is wrongly marked consumed, the result equals the no-filter sum
    // (3087989897.177311), far from the oracle 23512.7528571429.
    let mut engine = setup();
    let q17_nof = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'MED BOX'";
    let r_nof = engine
        .execute(q17_nof)
        .unwrap_or_else(|e| panic!("Q17-NOF failed: {}", e));
    let nof = scalar_float(&r_nof.rows);
    let q17 = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'MED BOX' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)";
    let res = engine
        .execute(q17)
        .unwrap_or_else(|e| panic!("Q17 failed: {}", e));
    let got = scalar_float(&res.rows);
    assert!(
        (got - 23512.7528571429).abs() < 1e-6,
        "Q17 expected 23512.7528571429 (SQLite oracle), got {}",
        got
    );
    assert!(
        (got - nof).abs() > 1.0,
        "Q17 must differ from the no-subquery-filter sum ({}); subquery filter not applied",
        nof
    );
}
