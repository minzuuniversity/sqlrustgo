// V313-103 / Issue #4692 + #4671 end-to-end smoke.
// Both issues are closed in prior commits. The two positive tests
// in this file (single-expression UDF, MATERIALIZED VIEW persistence)
// demonstrate the parts that are genuinely resolved end-to-end.
// The three remaining sub-problems (writable CTE body execution,
// BEGIN ... END with DECLARE statements) require v3.13-level
// executor work and are tracked separately.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn verify_4692_materialized_view_persists() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE base(id INT)").unwrap();
    x.execute("INSERT INTO base VALUES (1), (2), (3)").unwrap();
    x.execute("CREATE MATERIALIZED VIEW mv AS SELECT id FROM base")
        .expect("CREATE MATERIALIZED VIEW must parse and execute");
    let r = x
        .execute("SELECT id FROM mv")
        .expect("SELECT from the materialized view must resolve");
    let n: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(v) => *v,
            other => panic!("expected integer, got {:?}", other),
        })
        .collect();
    assert_eq!(n, vec![1, 2, 3], "view must reflect base data");
}

#[test]
fn verify_4671_udf_single_expr_body() {
    let mut x = fresh_mem();
    x.execute("CREATE FUNCTION f1(x INT) RETURNS INT RETURN x * 2")
        .expect("single-expression UDF must register");
    let r = x
        .execute("SELECT f1(10)")
        .expect("calling the single-expression UDF must work");
    let got: i64 = match &r.rows[0][0] {
        Value::Integer(v) => *v,
        other => panic!("expected integer, got {:?}", other),
    };
    assert_eq!(got, 20);
}

#[test]
fn verify_4671_udf_begin_end_declare() {
    // V313-103 / Issue #4671 sub-2: BEGIN ... END body with
    // DECLARE / SET / RETURN. The issue-body example is
    // `f2(x INT) RETURNS INT BEGIN DECLARE r INT; SET r = x + 100;
    // RETURN r; END`, expecting f2(5) = 105.
    let mut x = fresh_mem();
    x.execute(
        "CREATE FUNCTION f2(x INT) RETURNS INT \
         BEGIN DECLARE r INT; SET r = x + 100; RETURN r; END",
    )
    .expect("multi-statement UDF with DECLARE+SET+RETURN must register");
    let r = x
        .execute("SELECT f2(5)")
        .expect("calling the multi-statement UDF must work");
    let got: i64 = match &r.rows[0][0] {
        Value::Integer(v) => *v,
        other => panic!("expected integer, got {:?}", other),
    };
    assert_eq!(got, 105, "f2(5) must compute 5 + 100 = 105");
}
