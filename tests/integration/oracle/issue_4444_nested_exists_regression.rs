//! V312-58 / Issue #4444 (Phase 3): integration regression for the
//! Q20-style nested-EXISTS pattern.  The full TPC-H Q20 looks like:
//!
//!   SELECT s_name, s_address
//!   FROM supplier, nation, partsupp
//!   WHERE s_suppkey = ps_suppkey
//!     AND s_nationkey = n_nationkey
//!     AND n_name = 'CANADA'
//!     AND EXISTS (
//!       SELECT * FROM partsupp ps2
//!       WHERE ps2.ps_partkey = partsupp.ps_partkey
//!         AND ps2.ps_suppkey = partsupp.ps_suppkey
//!         AND ps2.ps_availqty > (
//!           SELECT 0.5 * SUM(l_quantity)
//!           FROM lineitem
//!           WHERE l_partkey = ps2.ps_partkey
//!             AND l_suppkey = ps2.ps_suppkey
//!             AND l_shipdate >= '1994-01-01'
//!             AND l_shipdate <  '1995-01-01'
//!         )
//!     )
//!
//! Phase 1 (#4442) detects the inner scalar-aggregate-in-WHERE pattern.
//! Phase 2 (#4443) materializes the scalar aggregate so the EXISTS body's
//! `ps_availqty > <value>` becomes a static predicate.  Phase 3 (#4444,
//! this PR) wires the outer EXISTS into a HashSemiJoin (acceptance
//! criterion #1).
//!
//! This test verifies the end-to-end behavior on an in-memory fixture:
//! only suppliers whose partsupp has `ps_availqty > 0.5 * in-window
//! SUM(l_quantity)` appear in the output.  The query is small enough
//! that the SF=1 evidence requirement (~6M lineitem rows) is out of
//! scope here — see `q20_potential_part_promotion_perf` for that.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Acceptance #1 + #2 + #3 (V312-58 / Issue #4444): a Q20-shape query
/// runs end-to-end and returns exactly the suppliers whose partsupp
/// passes the per-(partkey, suppkey) SUM(l_quantity) threshold.
#[test]
fn q20_nested_exists_returns_correct_suppliers() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE supplier (\
         s_suppkey INTEGER PRIMARY KEY, s_name TEXT, s_address TEXT, \
         s_nationkey INTEGER)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE partsupp (\
         ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (\
         l_partkey INTEGER, l_suppkey INTEGER, l_quantity INTEGER, l_shipdate TEXT)",
    )
    .unwrap();

    // Two suppliers: s_suppkey=1 (CANADA) and s_suppkey=2 (GERMANY).
    e.execute("INSERT INTO supplier VALUES (1, 'A', 'addr-1', 10)").unwrap();
    e.execute("INSERT INTO supplier VALUES (2, 'B', 'addr-2', 20)").unwrap();
    e.execute("INSERT INTO nation VALUES (10, 'CANADA')").unwrap();
    e.execute("INSERT INTO nation VALUES (20, 'GERMANY')").unwrap();

    // Two partsupp rows under CANADA supplier s_suppkey=1:
    //   (part=10, supp=1, avail=1000)  →  in-window SUM for (10,1)=130
    //                                     0.5*130=65; 1000 > 65 → KEEP
    //   (part=20, supp=1, avail=10)    →  in-window SUM for (20,1)=1000
    //                                     0.5*1000=500; 10 < 500  → DROP
    e.execute("INSERT INTO partsupp VALUES (10, 1, 1000)").unwrap();
    e.execute("INSERT INTO partsupp VALUES (20, 1, 10)").unwrap();

    // lineitem for (part=10, supp=1): in-window sum = 10+20+100 = 130
    for q in [10, 20, 100] {
        e.execute(&format!(
            "INSERT INTO lineitem VALUES (10, 1, {q}, '1994-06-15')"
        ))
        .unwrap();
    }
    // lineitem for (part=20, supp=1): in-window sum = 100+200+700 = 1000
    for q in [100, 200, 700] {
        e.execute(&format!(
            "INSERT INTO lineitem VALUES (20, 1, {q}, '1994-06-15')"
        ))
        .unwrap();
    }
    // Out-of-window rows (shipdate >= 1995-01-01) — must be excluded by
    // the residual filter on `l_shipdate`.
    e.execute("INSERT INTO lineitem VALUES (10, 1, 999999, '1995-06-15')").unwrap();
    e.execute("INSERT INTO lineitem VALUES (20, 1, 999999, '1995-06-15')").unwrap();

    let r = e.execute(
        "SELECT DISTINCT s_suppkey \
         FROM supplier, nation, partsupp \
         WHERE s_suppkey = ps_suppkey \
           AND s_nationkey = n_nationkey \
           AND n_name = 'CANADA' \
           AND EXISTS ( \
             SELECT 1 FROM partsupp ps2 \
             WHERE ps2.ps_partkey = partsupp.ps_partkey \
               AND ps2.ps_suppkey = partsupp.ps_suppkey \
               AND ps2.ps_availqty > ( \
                 SELECT 0.5 * SUM(l_quantity) FROM lineitem \
                 WHERE l_partkey = ps2.ps_partkey \
                   AND l_suppkey = ps2.ps_suppkey \
                   AND l_shipdate >= '1994-01-01' \
                   AND l_shipdate <  '1995-01-01' \
               ) \
           ) \
         ORDER BY s_suppkey",
    )
    .unwrap();
    // Only (part=10, supp=1) passes; supplier s_suppkey=1 still appears
    // because at least one of its partsupp rows passes the threshold.
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(1)]],
        "Q20 in-memory: only s_suppkey=1 should appear (its (part=10) \
         partsupp row passes ps_availqty=1000 > 0.5*130=65)"
    );
}

/// Acceptance #5 (regression guard): the HashSemiJoin `from_select`
/// constructor and the prewarm path do not regress a simpler Q4-shape
/// nested-EXISTS query (no scalar aggregate inside).
#[test]
fn q4_shape_existence_filter_still_works() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_orderdate TEXT, o_totalprice REAL)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, l_commitdate TEXT, l_receiptdate TEXT)",
    )
    .unwrap();
    // Two orders; one has a lineitem with commitdate < receiptdate.
    e.execute("INSERT INTO orders VALUES (1, '1994-01-01', 100.0)").unwrap();
    e.execute("INSERT INTO orders VALUES (2, '1994-01-01', 200.0)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, '1994-02-01', '1994-03-01')")
        .unwrap();
    // No lineitem for order 2 → EXISTS fails → order 2 excluded.

    let r = e.execute(
        "SELECT o_orderkey FROM orders \
         WHERE EXISTS ( \
           SELECT 1 FROM lineitem \
           WHERE l_orderkey = o_orderkey \
             AND l_commitdate < l_receiptdate \
         )",
    )
    .unwrap();
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(1)]],
        "Q4-shape EXISTS: only order 1 passes"
    );
}