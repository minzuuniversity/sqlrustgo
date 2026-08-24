//! V312-58 Sprint 3 Phase 3 — Q20 path diagnostic
//!
//! Q20 has 3 layers of correlated subqueries:
//! 1. Outer EXISTS correlated to outer supplier.s_suppkey
//! 2. Middle IN-semi-join (ps_partkey IN forest% parts)
//! 3. Inner correlated scalar SUM aggregate
//!
//! Expected path on mini subset:
//!   - try_decorrelate on outer EXISTS → Semi join
//!   - middle IN-subquery is static (no outer ref), folds to filter
//!   - inner SUM aggregate per (ps_partkey, ps_suppkey, 1994 date range)
//!
//! Run:
//!   cargo test --test diag_q20_sprint3_path --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp";

const Q20_SQL: &str = "SELECT s_name, s_address FROM supplier, nation \
                       WHERE s_nationkey = n_nationkey \
                         AND n_name = 'GERMANY' \
                         AND EXISTS (SELECT * FROM partsupp \
                                      WHERE ps_suppkey = s_suppkey \
                                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                                        AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) \
                                                             FROM lineitem \
                                                             WHERE l_partkey = ps_partkey \
                                                               AND l_suppkey = ps_suppkey \
                                                               AND l_shipdate >= '1994-01-01' \
                                                               AND l_shipdate < '1995-01-01')) \
                       ORDER BY s_name";

fn run_q20() -> (f64, Vec<(String, String)>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)").unwrap();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("nation", &format!("{}/nation_clean.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("supplier", &format!("{}/supplier_q20_mini.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("part", &format!("{}/part_q20_mini.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("partsupp", &format!("{}/partsupp_q20_mini.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("lineitem", &format!("{}/lineitem_q20_mini.tbl", DATA_DIR))
            .unwrap();
    }

    reset_v312_58_sprint3_diag();
    let start = Instant::now();
    let r = engine
        .execute(Q20_SQL)
        .unwrap_or_else(|e| panic!("Q20 failed: {}", e));
    let elapsed = start.elapsed().as_secs_f64();

    let rows: Vec<(String, String)> = r
        .rows
        .iter()
        .map(|row| {
            let name = match &row[0] {
                Value::Text(s) => s.clone(),
                other => panic!("unexpected s_name type: {:?}", other),
            };
            let addr = match &row[1] {
                Value::Text(s) => s.clone(),
                other => panic!("unexpected s_address type: {:?}", other),
            };
            (name, addr)
        })
        .collect();

    (elapsed, rows)
}

#[test]
#[ignore]
fn diag_q20_mini_path() {
    eprintln!(
        "=== Q20 mini path diagnostic (332 suppliers / 100 parts / 332 partsupp / 20 lineitem) ==="
    );
    let (elapsed, rows) = run_q20();
    eprintln!(
        "Q20 mini: elapsed {:.3}s, {} supplier rows",
        elapsed,
        rows.len()
    );
    for (name, addr) in &rows {
        eprintln!("  {} | {}", name, addr);
    }
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());
}
