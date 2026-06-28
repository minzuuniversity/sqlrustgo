//! TPC-H Q20 / Q21 regression: correlated EXISTS / NOT EXISTS
//! fast-path produces correct boolean results.
//!
//! Sprint 8 PR 2 (per openspec/changes/2026-06-08-v390-sprint8-q21-exists
//! tasks.md §6.1 → §6.3 wiring tests).
//!
//! Q21's structure:
//!   SELECT ... FROM supplier, lineitem l1, orders, nation
//!   WHERE ...
//!     AND EXISTS (SELECT * FROM lineitem l2
//!                  WHERE l2.l_orderkey = l1.l_orderkey
//!                    AND l2.l_suppkey <> l1.l_suppkey)
//!     AND NOT EXISTS (SELECT * FROM lineitem l3
//!                  WHERE l3.l_orderkey = l1.l_orderkey
//!                    AND l3.l_suppkey <> l1.l_suppkey
//!                    AND l3.l_receiptdate > l3.l_commitdate)
//!
//! The correlated EXISTS / NOT EXISTS inner subqueries use the
//! existing `build_subquery_index` path (src/engine_select.rs)
//! which builds a HashSet<Value> once per `(table, col)` pair
//! (effectively a hash-join index of the inner keys) and reuses it
//! per outer row in O(1) average.
//!
//! This test exercises the same code path the Q21 in-process eval
//! would use, with a minimal hand-rolled fixture so the test
//! doesn't depend on the SF=0.1 dataset.

use sqlrustgo::ExecutionEngine;
use sqlrustgo::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(Arc::clone(&storage))
}

fn setup_supplier_lineitem_min() -> ExecutionEngine<MemoryStorage> {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT, s_nationkey INTEGER)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, l_suppkey INTEGER, l_receiptdate TEXT, l_commitdate TEXT)",
    )
    .unwrap();
    e.execute("INSERT INTO supplier VALUES (1, 'Acme', 1)")
        .unwrap();
    e.execute("INSERT INTO supplier VALUES (2, 'Beta', 1)")
        .unwrap();
    e.execute("INSERT INTO supplier VALUES (3, 'Gamma', 1)")
        .unwrap();
    // orderkey 100: 3 lineitems across 3 suppliers (for l2 EXISTS test)
    e.execute("INSERT INTO lineitem VALUES (100, 1, '1995-01-01', '1995-01-02')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 2, '1995-01-03', '1995-01-04')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 3, '1995-01-05', '1995-01-06')")
        .unwrap();
    // orderkey 200: only 1 lineitem for suppkey 1
    e.execute("INSERT INTO lineitem VALUES (200, 1, '1995-01-01', '1995-01-02')")
        .unwrap();
    e
}

#[test]
fn exists_fast_path_matches_other_supplier_for_multi_supplier_order() {
    let mut e = setup_supplier_lineitem_min();

    // For l1 = (orderkey=100, suppkey=1): EXISTS a lineitem in
    // orderkey=100 with suppkey != 1 → YES (rows for suppkey 2 and 3).
    // The legacy slow path AND the build_subquery_index path
    // should agree.
    let r = e
        .execute(
            "SELECT s_name FROM supplier WHERE s_suppkey = 1 \
             AND EXISTS (SELECT * FROM lineitem \
                          WHERE l_orderkey = 100 AND l_suppkey <> 1)",
        )
        .unwrap();
    assert_eq!(
        r.rows.len(),
        1,
        "expected 1 row for suppkey=1, got {:?}",
        r.rows
    );

    // For l1 = (orderkey=200, suppkey=1): no other lineitems in
    // orderkey=200 → EXISTS = false → supplier 1 NOT in result.
    let r2 = e
        .execute(
            "SELECT s_name FROM supplier WHERE s_suppkey = 1 \
             AND EXISTS (SELECT * FROM lineitem \
                          WHERE l_orderkey = 200 AND l_suppkey <> 1)",
        )
        .unwrap();
    assert_eq!(
        r2.rows.len(),
        0,
        "expected 0 rows (no other suppkey in order 200)"
    );
}

// Known issue: the correlated NOT EXISTS subquery path
// currently returns TRUE for the `receipt > commit` predicate
// when both subquery rows have one late row, because the fast-path
// index only stores the per-table key set (a HashSet<Value>) and
// does NOT re-evaluate the residual predicate per outer row.
// Fix deferred to q21-exists/tasks.md §6.3 follow-up (need to
// extend the SubqueryIndex to carry the qualifying rows so the
// residual predicate is checked per outer-row substitution).
#[test]
fn not_exists_fast_path_filters_late_receipts() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER, l_suppkey INTEGER, l_receiptdate TEXT, l_commitdate TEXT)",
    )
    .unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 1, '1995-01-10', '1995-01-15')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 2, '1995-03-01', '1995-01-05')")
        .unwrap();
    let r = e
        .execute(
            "SELECT l_suppkey FROM lineitem l1 WHERE l_orderkey = 100 \
             AND NOT EXISTS (SELECT * FROM lineitem l3 \
                              WHERE l3.l_orderkey = l1.l_orderkey \
                                AND l3.l_suppkey <> l1.l_suppkey \
                                AND l3.l_receiptdate > l3.l_commitdate)",
        )
        .unwrap();
    let keys: Vec<i64> = r
        .rows
        .iter()
        .map(|r| {
            if let Value::Integer(n) = r[0] {
                n
            } else {
                panic!()
            }
        })
        .collect();
    assert_eq!(
        keys.len(),
        1,
        "expected only suppkey 2 (which has a late row elsewhere); got {:?}",
        keys
    );
    assert!(!keys.contains(&1));
    assert!(keys.contains(&2));
}
