//! V312-58 Sprint 4 Task #88 — Q20 nested Subquery-in-BinaryOp diagnostic
//!
//! Self-contained test using INSERT statements (no .tbl file dependency).
//! 10 suppliers × 3 forest parts × 10 partsupp × 60 lineitem.
//!
//! Each supplier has at least one partsupp that has 0.5*SUM > ps_availqty.
//!
//! Run:
//!   cargo test --release --test diag_q20_sprint4_inline -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage,
};
use std::sync::Arc;
use std::time::Instant;

/// Setup 10 suppliers, 3 forest parts + 2 other parts, 10 partsupp, 60 lineitem.
/// All 10 suppliers are GERMANY (n_nationkey=7).
/// All partsupp rows have ps_availqty=100.
fn make_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute(
            "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, \
             n_regionkey INTEGER NOT NULL, n_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, \
             s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, \
             s_acctbal REAL NOT NULL, s_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, \
             p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, \
             p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, \
             p_comment TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, \
             ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, \
             PRIMARY KEY (ps_partkey, ps_suppkey))",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
             l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, \
             l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
             l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, \
             l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, \
             l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
        )
        .unwrap();

    // nation: 1 row for GERMANY (n_nationkey=7)
    engine
        .execute("INSERT INTO nation VALUES (7, 'GERMANY', 1, 'comment')")
        .unwrap();

    // supplier: 10 rows, all GERMANY
    for i in 1..=10 {
        engine
            .execute(&format!(
                "INSERT INTO supplier VALUES ({}, 'Supplier#{}', 'Addr{}', 7, '800-{}-0001', 100.0, 'comment')",
                i, i, i, i
            ))
            .unwrap();
    }

    // part: 3 forest parts + 2 other parts (forest parts must match LIKE 'forest%')
    engine
        .execute("INSERT INTO part VALUES (1, 'forest green widget', 'M#1', 'Brand#1', 'TYPE', 10, 'BOX', 100.0, 'c')")
        .unwrap();
    engine
        .execute("INSERT INTO part VALUES (2, 'forest blue gadget', 'M#1', 'Brand#2', 'TYPE', 20, 'BOX', 200.0, 'c')")
        .unwrap();
    engine
        .execute("INSERT INTO part VALUES (3, 'forest red thingamajig', 'M#1', 'Brand#3', 'TYPE', 30, 'BOX', 300.0, 'c')")
        .unwrap();

    // partsupp: 10 rows. Each supplier has 1 row for partkey=1 (forest), all with ps_availqty=200.
    // 0.5*SUM for (partkey=1, suppkey=S) = 0.5 * (6 lineitem rows * 10 each) = 30
    // So ps_availqty=200 > 30 → matches the Q20 filter (excess inventory).
    // L4 uses literal `ps_availqty > 100` → 200 > 100 = TRUE.
    for s in 1..=10 {
        engine
            .execute(&format!(
                "INSERT INTO partsupp VALUES (1, {}, 200, 10.0, 'c')",
                s
            ))
            .unwrap();
    }

    // lineitem: 60 rows. For each (partkey=1, suppkey=s), 6 rows in 1994 with qty=10.
    // Total per (partkey, suppkey) = 6*10 = 60; 0.5*SUM = 30; ps_availqty=100 > 30.
    let mut orderkey = 1;
    for s in 1..=10 {
        for _ in 0..6 {
            engine
                .execute(&format!(
                    "INSERT INTO lineitem VALUES ({}, 1, {}, 1, 10, 1000.0, 0.05, 0.01, \
                     'N', 'O', '1994-06-15', '1994-06-20', '1994-06-25', 'NONE', 'TRUCK', 'comment')",
                    orderkey, s
                ))
                .unwrap();
            orderkey += 1;
        }
    }

    engine
}

/// Run a single SQL, return (elapsed_secs, row_count, dump).
fn run(label: &str, sql: &str) -> (f64, usize, String) {
    let mut engine = make_engine();
    reset_v312_58_sprint3_diag();
    let start = Instant::now();
    let r = engine.execute(sql);
    let elapsed = start.elapsed().as_secs_f64();
    let dump = dump_v312_58_sprint3_diag();
    match r {
        Ok(rs) => {
            eprintln!(
                "[{}] elapsed {:.3}s, {} rows",
                label,
                elapsed,
                rs.rows.len()
            );
            eprintln!("  DIAG: {}", dump.lines().collect::<Vec<_>>().join(" | "));
            for (i, row) in rs.rows.iter().take(5).enumerate() {
                eprintln!("  row[{}] = {:?}", i, row);
            }
            (elapsed, rs.rows.len(), dump)
        }
        Err(e) => {
            eprintln!("[{}] ERROR: {} | DIAG: {}", label, e, dump);
            (elapsed, 0, dump)
        }
    }
}

#[test]
#[ignore]
fn diag_q20_l0_inline() {
    eprintln!("=== Q20 L0 (full Q20, no nation filter via embedded nation in s_nationkey) ===");
    // Use a simpler version without nation join: just s_suppkey <= 10.
    let sql = "SELECT s_name, s_address FROM supplier \
               WHERE s_suppkey <= 10 \
                 AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                              AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                              AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem \
                                                  WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey \
                                                    AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) \
               ORDER BY s_name";
    let (_elapsed, rows, _dump) = run("L0-inline", sql);
    // Expect 10 suppliers: each has a partsupp with ps_availqty=100 < 0.5*SUM=600
    assert_eq!(rows, 10, "Q20 L0 should return 10 supplier rows");
}

#[test]
#[ignore]
fn diag_q20_l4_inline() {
    eprintln!("=== Q20 L4 (literal availqty, no SUM) ===");
    let sql = "SELECT s_name FROM supplier WHERE s_suppkey <= 10 \
               AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                            AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                            AND ps_availqty > 100) \
               ORDER BY s_name";
    let (_elapsed, rows, _dump) = run("L4-inline", sql);
    // Expect 10 suppliers
    assert_eq!(rows, 10, "Q20 L4 should return 10 supplier rows");
}

#[test]
#[ignore]
fn diag_q20_l5_inline() {
    eprintln!("=== Q20 L5 (full SUM correlated EXISTS, no nation) ===");
    let sql = "SELECT s_name FROM supplier WHERE s_suppkey <= 10 \
               AND EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey \
                            AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                            AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem \
                                                WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey \
                                                  AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')) \
               ORDER BY s_name";
    let (_elapsed, rows, _dump) = run("L5-inline", sql);
    // Expect 10 suppliers
    assert_eq!(rows, 10, "Q20 L5 should return 10 supplier rows");
}
