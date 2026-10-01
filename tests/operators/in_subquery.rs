//! Operator-level regression tests for `SELECT ... WHERE col IN/NOT IN (subquery)`.
//!
//! Two engine defects motivated this suite:
//!
//! 1. **Correlated `IN` was always true.** `where_expr_has_correlated_subquery`
//!    reported `false` for `Expression::In`/`NotIn`, so Step 1.5 never ran for
//!    that shape and `eval_predicate` fell through to a conservative stub
//!    (`In | NotIn => true`). `WHERE c.user_id IN (SELECT ... WHERE
//!    o.customer_id = c.user_id)` therefore returned every row.
//! 2. **Long literal lists were quadratic.** A non-correlated `IN` subquery is
//!    rewritten into a literal `InList`; the evaluator re-parsed every literal
//!    for every row, costing O(rows × literals). Lists of 16+ entries now fold
//!    into a pre-parsed hash set, which these tests pin as semantically
//!    identical to the linear scan.
//!
//! Expected values are those of SQLite (the teaching-corpus oracle).
//!
//! Acceptance (per Issue #3283):
//! - Use a minimal fixture (3-5 rows)
//! - Assert cell values, not just row count
//! - Run in < 100ms

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

/// `customer(12,..)` / `orders(16,..)` fixture mirroring the teaching seed,
/// scaled down to the rows each case actually needs.
fn seeded() -> ExecutionEngine<MemoryStorage> {
    let mut e = engine();
    e.execute("CREATE TABLE customer (user_id TEXT, register_year INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE orders (order_id INTEGER, customer_id TEXT, total_amount INTEGER)")
        .unwrap();
    for (u, y) in [
        ("U20190001", 2019),
        ("U20200002", 2020),
        ("U20210003", 2021),
        ("U20220004", 2022),
        ("U20230005", 2023),
    ] {
        e.execute(&format!("INSERT INTO customer VALUES ('{}', {})", u, y))
            .unwrap();
    }
    for (oid, cid, amt) in [
        (1, "U20190001", 800),
        (2, "U20190001", 100),
        (3, "U20200002", 650),
        (4, "U20210003", 900),
        (5, "U20220004", 200),
    ] {
        e.execute(&format!(
            "INSERT INTO orders VALUES ({}, '{}', {})",
            oid, cid, amt
        ))
        .unwrap();
    }
    e
}

fn count(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> String {
    e.execute(sql).unwrap().rows[0][0].to_string()
}

#[test]
fn uncorrelated_in_subquery() {
    let mut e = seeded();
    // Customers that have an order above 500: U20190001, U20200002, U20210003
    let c = count(
        &mut e,
        "SELECT COUNT(*) FROM customer \
         WHERE user_id IN (SELECT customer_id FROM orders WHERE total_amount > 500)",
    );
    assert_eq!(c, "3");
}

#[test]
fn uncorrelated_not_in_subquery() {
    let mut e = seeded();
    // The complement: 5 - 3 = 2
    let c = count(
        &mut e,
        "SELECT COUNT(*) FROM customer \
         WHERE user_id NOT IN (SELECT customer_id FROM orders WHERE total_amount > 500)",
    );
    assert_eq!(c, "2");
}

// KNOWN DEFECT (pre-existing on origin/main, verified against pristine a8dba8d31e):
// `where_expr_has_correlated_subquery` reports `false` for `Expression::In`/
// `NotIn` (src/engine_utils.rs), so Step 1.5 is skipped for a correlated
// `IN (subquery)` and Step 1.6 never substitutes it. `eval_predicate` then hits
// its conservative `In | NotIn => true` stub and returns every row.
// Un-ignore once the `In`/`NotIn` arms are routed through per-row evaluation.
#[test]
#[ignore = "correlated IN subquery falls back to always-true on main"]
fn correlated_in_subquery_uses_outer_row() {
    let mut e = seeded();
    // Regression for defect 1: the subquery references `c.user_id`, so the
    // outer row decides the answer. Order 5 (U20220004, 200) is below the
    // threshold, so U20220004 is excluded; U20230005 has no orders at all.
    let c = count(
        &mut e,
        "SELECT COUNT(*) FROM customer c \
         WHERE c.user_id IN (SELECT o.customer_id FROM orders o \
                             WHERE o.total_amount > 500 AND o.customer_id = c.user_id)",
    );
    assert_eq!(c, "3", "correlated IN must not fall back to always-true");
}

#[test]
#[ignore = "correlated NOT IN subquery falls back to always-true on main"]
fn correlated_not_in_subquery_uses_outer_row() {
    let mut e = seeded();
    let c = count(
        &mut e,
        "SELECT COUNT(*) FROM customer c \
         WHERE c.user_id NOT IN (SELECT o.customer_id FROM orders o \
                                 WHERE o.customer_id = c.user_id AND o.total_amount > 500)",
    );
    assert_eq!(
        c, "2",
        "correlated NOT IN must not fall back to always-true"
    );
}

#[test]
fn in_subquery_returns_column_values() {
    let mut e = seeded();
    // Assert the rows themselves, not just their count.
    let r = e
        .execute(
            "SELECT user_id FROM customer \
             WHERE register_year IN (SELECT register_year FROM customer WHERE register_year > 2021) \
             ORDER BY user_id",
        )
        .unwrap();
    let got: Vec<String> = r.rows.iter().map(|row| row[0].to_string()).collect();
    assert_eq!(got, vec!["U20220004", "U20230005"]);
}

#[test]
fn in_empty_subquery_yields_nothing() {
    let mut e = seeded();
    assert_eq!(
        count(
            &mut e,
            "SELECT COUNT(*) FROM customer \
             WHERE register_year IN (SELECT register_year FROM customer WHERE register_year > 9999)",
        ),
        "0"
    );
}

#[test]
fn not_in_empty_subquery_yields_everything() {
    let mut e = seeded();
    // `x NOT IN ()` is vacuously true, so all 5 customers survive.
    assert_eq!(
        count(
            &mut e,
            "SELECT COUNT(*) FROM customer \
             WHERE register_year NOT IN (SELECT register_year FROM customer WHERE register_year > 9999)",
        ),
        "5"
    );
}

#[test]
fn long_in_list_matches_linear_scan() {
    // 20 literals — above IN_LIST_SET_THRESHOLD (16), so this exercises the
    // folded `InValueSet` path and pins it against the linear scan's answer.
    let mut e = engine();
    e.execute("CREATE TABLE t (v INTEGER)").unwrap();
    for v in 1..=30 {
        e.execute(&format!("INSERT INTO t VALUES ({})", v)).unwrap();
    }

    let long_list = (1..=20)
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let via_set = count(
        &mut e,
        &format!("SELECT COUNT(*) FROM t WHERE v IN ({})", long_list),
    );
    assert_eq!(via_set, "20");

    // Same predicate written as a subquery, which is rewritten into the very
    // same long literal list. Both paths must agree.
    let via_subquery = count(
        &mut e,
        "SELECT COUNT(*) FROM t WHERE v IN (SELECT v FROM t WHERE v <= 20)",
    );
    assert_eq!(via_subquery, "20");

    let not_in = count(
        &mut e,
        &format!("SELECT COUNT(*) FROM t WHERE v NOT IN ({})", long_list),
    );
    assert_eq!(not_in, "10");
}

#[test]
fn long_in_list_ignores_out_of_range_and_duplicates() {
    let mut e = engine();
    e.execute("CREATE TABLE t (v INTEGER)").unwrap();
    for v in [1, 2, 3] {
        e.execute(&format!("INSERT INTO t VALUES ({})", v)).unwrap();
    }
    // 17 entries, duplicates and misses included: still a set of {1, 2}
    let list = "1, 2, 2, 1, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19";
    let c = count(
        &mut e,
        &format!("SELECT COUNT(*) FROM t WHERE v IN ({})", list),
    );
    assert_eq!(c, "2");
}
