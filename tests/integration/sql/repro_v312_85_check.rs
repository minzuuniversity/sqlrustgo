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
    x.execute(
        "CREATE TABLE t(val int, CONSTRAINT chk_pos CHECK (val > 0 AND val < 100))",
    )
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
