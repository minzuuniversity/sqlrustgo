//! V312-90 / P3-MATH-005 regression integration test:
//! `SIGN(x)` scalar function — was completely missing from
//! `eval_fn` dispatch (oversight when PR #4772 added the rest of
//! the math functions in the Issue #4698 batch). `SELECT SIGN(x)`
//! returned NULL for every input, positive or negative.
//!
//! After this fix, SIGN returns Integer(-1 / 0 / 1), matching
//! MySQL and SQLite canonical semantics. NULL input → NULL, NaN
//! → NULL, non-numeric text → NULL.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn col_i(row: &[Value], i: usize) -> i64 {
    match &row[i] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer at col {}, got {:?}", i, other),
    }
}

#[test]
fn v312_90_sign_integer_inputs() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (n INT)").unwrap();
    x.execute("INSERT INTO t VALUES (-5), (-1), (0), (1), (5)")
        .unwrap();
    let res = x
        .execute("SELECT n, SIGN(n) FROM t ORDER BY n")
        .unwrap();
    assert_eq!(res.rows.len(), 5);
    // n=-5  → -1
    assert_eq!(col_i(&res.rows[0], 0), -5);
    assert_eq!(col_i(&res.rows[0], 1), -1);
    // n=-1  → -1
    assert_eq!(col_i(&res.rows[1], 1), -1);
    // n=0   → 0
    assert_eq!(col_i(&res.rows[2], 1), 0);
    // n=1   → 1
    assert_eq!(col_i(&res.rows[3], 1), 1);
    // n=5   → 1
    assert_eq!(col_i(&res.rows[4], 1), 1);
}

#[test]
fn v312_90_sign_float_inputs() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (x FLOAT)").unwrap();
    x.execute("INSERT INTO t VALUES (-1.5), (0.0), (1.5)")
        .unwrap();
    let res = x.execute("SELECT x, SIGN(x) FROM t ORDER BY x").unwrap();
    assert_eq!(res.rows.len(), 3);
    // Float input → Integer result (canonical type), matching
    // MySQL SIGN(1.5) = 1 and SQLite SIGN(1.5) = 1.
    assert_eq!(col_i(&res.rows[0], 1), -1); // x=-1.5
    assert_eq!(col_i(&res.rows[1], 1), 0); // x=0.0
    assert_eq!(col_i(&res.rows[2], 1), 1); // x=1.5
}

#[test]
fn v312_90_sign_null_input_returns_null() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (n INT)").unwrap();
    x.execute("INSERT INTO t VALUES (NULL), (3)").unwrap();
    let res = x.execute("SELECT n, SIGN(n) FROM t ORDER BY n").unwrap();
    assert_eq!(res.rows.len(), 2);
    // NULL → NULL
    assert_eq!(res.rows[0][0], Value::Null);
    assert_eq!(res.rows[0][1], Value::Null);
    // 3 → 1
    assert_eq!(col_i(&res.rows[1], 1), 1);
}

#[test]
fn v312_90_sign_literal_constants() {
    let mut x = fresh();
    let r = x.execute("SELECT SIGN(0), SIGN(7), SIGN(-7)").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(0));
    assert_eq!(r.rows[0][1], Value::Integer(1));
    assert_eq!(r.rows[0][2], Value::Integer(-1));
}
