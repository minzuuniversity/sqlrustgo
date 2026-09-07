//! V312-85 / Issue #4752: CHECK constraint error uses Display, not raw AST.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn check_violation_error_is_human_readable() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(val int, CONSTRAINT chk_pos CHECK (val > 0 AND val < 100))")
        .unwrap();
    x.execute("INSERT INTO t VALUES (5)").unwrap();
    let r = x.execute("INSERT INTO t VALUES (200)");
    assert!(r.is_err(), "expected error, got {:?}", r);
    let err = format!("{}", r.unwrap_err());
    println!("ERR: {}", err);
    // Should contain human-readable expression, NOT raw AST
    assert!(!err.contains("BinaryOp("), "got raw AST leak: {}", err);
    assert!(err.contains("val"), "missing column: {}", err);
    assert!(err.contains("chk_pos"), "missing constraint name: {}", err);
}

#[test]
fn check_violation_or_clause() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE u(val int, CHECK (val > 100 OR val < -100))")
        .unwrap();
    let r = x.execute("INSERT INTO u VALUES (50)");
    assert!(r.is_err());
    let err = format!("{}", r.unwrap_err());
    println!("ERR: {}", err);
    assert!(!err.contains("BinaryOp("), "got raw AST leak: {}", err);
}

#[test]
fn check_between_clause() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE v(val int, CHECK (val BETWEEN 10 AND 50))")
        .unwrap();
    let r = x.execute("INSERT INTO v VALUES (100)");
    assert!(r.is_err());
    let err = format!("{}", r.unwrap_err());
    println!("ERR: {}", err);
    assert!(!err.contains("BinaryOp("), "got raw AST leak: {}", err);
}

// ============================================================================
// V312-95 v2 / Issue #4815 — BustubX-EDU P3-CONST-001 false-alarm guard
//
// Issue #4815 was filed suspecting CHECK constraint silent failure
// (`val >= 0` allowing -5). Manual + differential verification confirmed
// enforcement IS correct: `INSERT ... VALUES (2, -5)` is rejected with
// `Execution error: CHECK constraint 'unnamed' violated: (val >= 0)`.
//
// The differential_test.py classifier treated this as a parse error (because
// the error string contains "error:") when it is actually a runtime error.
// This test pins the issue-body anchor sequence to lock in correct behavior.
// ============================================================================

#[test]
fn check_negative_value_rejected_4815() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t (id INT, val INT CHECK (val >= 0))")
        .unwrap();

    // Valid value must succeed.
    x.execute("INSERT INTO t VALUES (1, 10)")
        .expect("val=10 must be accepted");

    // Boundary value (0) must succeed — the predicate is `>=`, not `>`.
    x.execute("INSERT INTO t VALUES (3, 0)")
        .expect("val=0 boundary must be accepted");

    // The issue anchor case: val=-5 must be rejected.
    let r = x.execute("INSERT INTO t VALUES (2, -5)");
    assert!(
        r.is_err(),
        "INSERT val=-5 must violate CHECK (val >= 0); got Ok (constraint silent failure)"
    );
    let err = format!("{}", r.unwrap_err());
    assert!(
        err.contains("CHECK") && err.contains("val >= 0"),
        "error must mention CHECK + the predicate; got: {}",
        err
    );

    // Final state: only the two valid rows landed.
    let r = x.execute("SELECT * FROM t ORDER BY id").unwrap();
    assert_eq!(r.rows.len(), 2, "only valid rows should be persisted");
    assert_eq!(r.rows[0][1], Value::Integer(10));
    assert_eq!(r.rows[1][1], Value::Integer(0));
}
