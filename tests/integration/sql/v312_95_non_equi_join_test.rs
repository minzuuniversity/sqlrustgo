//! V312-95 / P3-JOIN-001 regression integration test:
//! `JOIN ... ON <non-equi>` — the join executor previously extracted
//! only the equi-key columns from the ON predicate and silently
//! dropped the rest. A non-equi join like `a.x < b.y/10` would
//! match only rows where x == y (treating `<` as `=`), missing
//! the actual filter intent and returning cartesian-product rows
//! with no filtering.
//!
//! The fix: `find_join_key_index` returns `JoinKey::All` for
//! non-equi operators, forcing cartesian + ON post-filter; the
//! post-filter runs BEFORE the LEFT/RIGHT/FULL padding so the
//! outer-join semantic is preserved.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_95_join_non_equi_less_than_filters() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    x.execute("CREATE TABLE b (y INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1), (5)").unwrap();
    x.execute("INSERT INTO b VALUES (1), (5)").unwrap();
    // Pre-fix: returned all 4 cartesian rows.
    // Post-fix: only the 1 row where 1 < 5.
    let res = x
        .execute("SELECT a.x, b.y FROM a JOIN b ON a.x < b.y")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(5));
}

#[test]
fn v312_95_join_non_equi_with_arithmetic() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    x.execute("CREATE TABLE b (y INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1), (2)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (20)").unwrap();
    // V312-95 / P3-JOIN-001: non-equi join + arithmetic in the
    // predicate. The parser requires parens around the right-hand
    // arithmetic to keep the tree shape `BinaryOp(Identifier,
    // "<", BinaryOp(Identifier, "/", Literal))` so the post-filter
    // can evaluate `b.y/10` correctly.
    // a.x < (b.y/10) for (a=1..2, b=10|20):
    //   (1,10): 1 < 1 = false
    //   (1,20): 1 < 2 = true
    //   (2,10): 2 < 1 = false
    //   (2,20): 2 < 2 = false
    // Only (1,20) matches.
    let res = x
        .execute("SELECT a.x, b.y FROM a JOIN b ON a.x < (b.y/10)")
        .unwrap();
    let rows: Vec<(i64, i64)> = res
        .rows
        .iter()
        .map(|r| (match &r[0] { Value::Integer(n) => *n, _ => 0 },
                  match &r[1] { Value::Integer(n) => *n, _ => 0 }))
        .collect();
    assert_eq!(rows, vec![(1, 20)]);
}

#[test]
fn v312_95_join_mixed_equi_and_non_equi() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT, m INT)").unwrap();
    x.execute("CREATE TABLE b (y INT, n INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1, 10), (2, 20)").unwrap();
    x.execute("INSERT INTO b VALUES (1, 100), (2, 50)").unwrap();
    // a.x = b.y AND a.m < b.n.
    // (1,10)-(1,100): 1=1 & 10<100 → match.
    // (2,20)-(2,50):  2=2 & 20<50  → match.
    // (cross): a.x != b.y → fail the = arm.
    let res = x
        .execute("SELECT a.x, a.m, b.y, b.n FROM a JOIN b ON a.x = b.y AND a.m < b.n")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
}

#[test]
fn v312_95_join_equi_still_works_regression() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    x.execute("CREATE TABLE b (y INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1), (2), (3)").unwrap();
    x.execute("INSERT INTO b VALUES (1), (3)").unwrap();
    // Equi-join: pre-existing hash-join path must still work.
    let res = x
        .execute("SELECT a.x, b.y FROM a JOIN b ON a.x = b.y ORDER BY a.x")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(1));
    assert_eq!(res.rows[1][0], Value::Integer(3));
    assert_eq!(res.rows[1][1], Value::Integer(3));
}

#[test]
fn v312_95_join_right_join_padding_preserved() {
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (id INT, name TEXT)").unwrap();
    x.execute("CREATE TABLE t2 (id INT, val INT)").unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .unwrap();
    x.execute("INSERT INTO t2 VALUES (1, 10), (2, 20), (4, 40)")
        .unwrap();
    // RIGHT JOIN: t2 has id=4 unmatched → padded with NULLs.
    // The ON post-filter must NOT drop the padded row (otherwise the
    // outer-join semantic is broken).
    let res = x
        .execute("SELECT t1.id, t1.name, t2.val FROM t1 RIGHT JOIN t2 ON t1.id = t2.id ORDER BY t2.val")
        .unwrap();
    assert_eq!(res.rows.len(), 3);
    // First two rows: matched.
    assert_eq!(res.rows[0][2], Value::Integer(10));
    assert_eq!(res.rows[1][2], Value::Integer(20));
    // Third row: padded (t1.id=4 has no match in t2).
    assert_eq!(res.rows[2][0], Value::Null);
    assert_eq!(res.rows[2][1], Value::Null);
    assert_eq!(res.rows[2][2], Value::Integer(40));
}

#[test]
fn v312_95_join_cross_join_still_works() {
    // CROSS JOIN is a separate syntactic form. Pre-fix it also went
    // through the JoinKey::All path. Make sure the post-filter doesn't
    // break it (the cartesian marker `Literal("true")` is a no-op).
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    x.execute("CREATE TABLE b (y INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1), (2)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (20)").unwrap();
    let res = x
        .execute("SELECT a.x, b.y FROM a CROSS JOIN b ORDER BY a.x, b.y")
        .unwrap();
    assert_eq!(res.rows.len(), 4);
}

#[test]
fn v312_95_parser_comparison_rhs_handles_div_arithmetic() {
    // V313-107 / P3-JOIN-007 regression: the parser used to call
    // parse_primary_expression for the right-hand side of comparison
    // operators (<, >, =, ...), which only handled primary expressions
    // (literals, identifiers, parentheses, function calls) but NOT
    // arithmetic expressions like `b.y / 10`. After the comparison
    // operator was consumed, the parser would stop at the division
    // operator and exit the join-chain loop, breaking 3-table non-equi
    // joins like `a JOIN b ON a.x < b.y/10 JOIN c ON b.y < c.z`.
    // Fix: call parse_additive_expression so the right-hand side of
    // a comparison can contain a multiplicative subexpression.
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    x.execute("CREATE TABLE b (y INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1), (2)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (20)").unwrap();
    // 2-table: a.x < b.y/10 → a.x < 1 for b.y=10 (false), a.x < 2 for b.y=20 (true).
    let res = x
        .execute("SELECT a.x, b.y FROM a JOIN b ON a.x < b.y/10 ORDER BY b.y")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(20));
}

#[test]
fn v312_95_parser_select_expression_with_division() {
    // The parser precedence fix also covers SELECT-list expressions:
    // `SELECT a < b/10 FROM ...` used to fail with "Expected expression"
    // because the comparison parser called parse_primary_expression for
    // the right-hand side. Now it correctly parses `b/10` as a
    // multiplicative subexpression.
    let mut x = fresh();
    x.execute("CREATE TABLE t (a INT, b INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 10), (5, 50)").unwrap();
    let res = x
        .execute("SELECT a < b/10 AS cmp FROM t ORDER BY a")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    // 1 < 10/10 = 1 < 1 = false (0).
    assert_eq!(res.rows[0][0], Value::Boolean(false));
    // 5 < 50/10 = 5 < 5 = false (0).
    assert_eq!(res.rows[1][0], Value::Boolean(false));
}
