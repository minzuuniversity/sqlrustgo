//! Q20 mini bisect diagnostic — verify each subquery layer works
//!
//! Run:
//!   cargo test --test diag_q20_mini_bisect --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp";

fn make_engine() -> ExecutionEngine<MemoryStorage> {
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
    engine
}

fn run(sql: &str, label: &str) -> (f64, usize) {
    let mut engine = make_engine();
    reset_v312_58_sprint3_diag();
    let start = Instant::now();
    let r = engine.execute(sql);
    let elapsed = start.elapsed().as_secs_f64();
    match r {
        Ok(rs) => {
            eprintln!(
                "[{}] elapsed {:.3}s, {} rows",
                label,
                elapsed,
                rs.rows.len()
            );
            (elapsed, rs.rows.len())
        }
        Err(e) => {
            eprintln!("[{}] ERROR: {}", label, e);
            (elapsed, 0)
        }
    }
}

#[test]
#[ignore]
fn diag_q20_bisect() {
    eprintln!("=== Q20 mini bisect diagnostic ===");
    eprintln!("Data: 30 GERMANY suppliers, 306 forest parts, 42 partsupp, 50 lineitem(1994)");
    eprintln!();

    // L0: full Q20
    run(
        "SELECT s_name, s_address FROM supplier, nation \
         WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                        AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem \
                                            WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey \
                                              AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) \
         ORDER BY s_name",
        "L0 full Q20",
    );

    // L1: just supplier join nation
    run(
        "SELECT s_name, s_address FROM supplier, nation \
         WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' \
         ORDER BY s_name",
        "L1 supplier JOIN nation",
    );

    // L2: supplier with EXISTS partsupp (no nested IN or SUM)
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey) \
         ORDER BY s_name",
        "L2 supplier EXISTS partsupp",
    );

    // L3: partsupp with middle IN-subquery
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')) \
         ORDER BY s_name",
        "L3 EXISTS + IN forest%",
    );

    // L4: partsupp with full nested (no SUM)
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                        AND ps_availqty > 100) \
         ORDER BY s_name",
        "L4 EXISTS + IN + ps_availqty>100",
    );

    // L5: full Q20 minus s_nationkey filter (use EXISTS subquery only)
    run(
        "SELECT s_name FROM supplier WHERE s_suppkey <= 100 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                        AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem \
                                            WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey \
                                              AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) \
         ORDER BY s_name",
        "L5 full correlated SUM EXISTS (no nation filter)",
    );

    // L6: L2 + ps_availqty > 100 (literal)
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_availqty > 100) \
         ORDER BY s_name",
        "L6 EXISTS + ps_availqty>100 (literal)",
    );

    // L7: L2 + ps_partkey = 7936 (literal, no IN subquery)
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey = 7936) \
         ORDER BY s_name",
        "L7 EXISTS + ps_partkey=7936 (literal)",
    );

    // L8: L2 + ps_partkey = 7936 + ps_availqty > 100
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey = 7936 AND ps_availqty > 100) \
         ORDER BY s_name",
        "L8 EXISTS + partkey=7936 + availqty>100",
    );

    // L9: L4 (full nested with literal availqty) — confirm broken
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                        AND ps_availqty > 100) \
         ORDER BY s_name",
        "L9 L3 + availqty>100 (literal, full)",
    );

    // L10: comma-join + simple EXISTS (L0 outer shell, L2 inner) — see if comma breaks
    run(
        "SELECT s_name FROM supplier, nation \
         WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey) \
         ORDER BY s_name",
        "L10 comma-join + simple EXISTS",
    );

    // L11: comma-join + EXISTS with forest IN
    run(
        "SELECT s_name FROM supplier, nation \
         WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')) \
         ORDER BY s_name",
        "L11 comma-join + EXISTS + IN forest",
    );

    // ===== Subquery-side bisect: bypass EXISTS to test partsupp direct =====
    eprintln!("\n--- Direct partsupp scans (no outer correlation) ---");

    // L12: ps_partkey IN forest% only
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')",
        "L12 partsupp IN forest% only",
    );

    // L13: ps_availqty > 100 only
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_availqty > 100",
        "L13 partsupp availqty>100 only",
    );

    // L14: ps_availqty > 100 AND IN forest% (the failing combo without outer correlation)
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_availqty > 100 AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')",
        "L14 availqty>100 AND IN forest%",
    );

    // L15: invert order — IN forest% AND availqty > 100
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') AND ps_availqty > 100",
        "L15 IN forest% AND availqty>100",
    );

    // L16: with correlated key + IN (no availqty)
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_suppkey = 33 AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')",
        "L16 ps_suppkey=33 AND IN forest%",
    );

    // L17: with correlated key + availqty (no IN)
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_suppkey = 33 AND ps_availqty > 100",
        "L17 ps_suppkey=33 AND availqty>100",
    );

    // L18: full correlated: ps_suppkey=33 AND availqty>100 AND IN forest%
    run(
        "SELECT ps_partkey FROM partsupp WHERE ps_suppkey = 33 AND ps_availqty > 100 AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')",
        "L18 ps_suppkey=33 AND availqty>100 AND IN forest%",
    );

    // ===== Wrap literal equivalents in EXISTS to isolate EXISTS wrapper =====
    eprintln!("\n--- EXISTS wrapper with literal subquery key ---");

    // L19: EXISTS with literal ps_suppkey=33 + IN (the failing combo, but as literal)
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = 33 \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                        AND ps_availqty > 100) \
         ORDER BY s_name",
        "L19 EXISTS literal-key + IN + availqty",
    );

    // L20: EXISTS with literal ps_suppkey=33 + IN (no availqty) — should match at least 1
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = 33 \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')) \
         ORDER BY s_name",
        "L20 EXISTS literal-key + IN (no availqty)",
    );

    // L21: EXISTS correlated + IN (no availqty) — should match 30
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')) \
         ORDER BY s_name",
        "L21 EXISTS correlated + IN (no availqty)",
    );

    // L22: EXISTS correlated + availqty (no IN) — should match 30
    run(
        "SELECT s_name FROM supplier WHERE s_nationkey = 7 \
           AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                        AND ps_availqty > 100) \
         ORDER BY s_name",
        "L22 EXISTS correlated + availqty",
    );

    eprintln!("\n{}", dump_v312_58_sprint3_diag());
}
