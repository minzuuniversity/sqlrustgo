//! V312-77 / Issue #4847 regression integration tests:
//! Explicit transaction semantics: ROLLBACK must undo DML inside BEGIN...ROLLBACK.
//!
//! Root causes fixed:
//! - Path A (CLI): `-- comment\nBEGIN` fragment was not stripped before
//!   `starts_with("BEGIN")` check, so the #4626 pre-COMMIT workaround never
//!   fired and the engine received BEGIN with a lingering implicit tx.
//! - Path B/C (engine): `commit_transaction` / `rollback_transaction`
//!   returned errors when no tx was active, which aborts the whole batch
//!   instead of no-op'ing.
//! - Path D (engine): `commit_implicit_dml_tx` unconditionally consumed the
//!   current tx even when inside an explicit BEGIN, so DML auto-committed
//!   and ROLLBACK had nothing to undo. Fixed by adding
//!   `is_explicit_transaction` flag that prevents implicit commits inside
//!   explicit transactions.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Test A: basic BEGIN + INSERT + ROLLBACK — ROLLBACK must undo the INSERT.
#[test]
fn v312_77_rollback_undoes_insert() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("BEGIN").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    x.execute("ROLLBACK").unwrap();
    // After ROLLBACK, only row with a=1 should remain.
    let res = x.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    let count = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(count, 1, "ROLLBACK must undo the second INSERT");
}

/// Test B: BEGIN + UPDATE + ROLLBACK — ROLLBACK must restore the old value.
/// This is the key test for #4847: UPDATE inside BEGIN...ROLLBACK.
#[test]
fn v312_77_rollback_undoes_update() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY, b INT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    x.execute("BEGIN").unwrap();
    x.execute("UPDATE t SET b = 99 WHERE a = 1").unwrap();
    x.execute("ROLLBACK").unwrap();
    // After ROLLBACK, b should still be 10.
    let res = x.execute("SELECT b FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    let b = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(b, 10, "ROLLBACK must restore b to 10, not leave it at 99");
}

/// Test C: COMMIT must persist the UPDATE.
#[test]
fn v312_77_commit_persists_update() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY, b INT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    x.execute("BEGIN").unwrap();
    x.execute("UPDATE t SET b = 99 WHERE a = 1").unwrap();
    x.execute("COMMIT").unwrap();
    let res = x.execute("SELECT b FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    let b = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(b, 99, "COMMIT must persist the UPDATE");
}

/// Test D: ROLLBACK when no transaction is active must be a no-op
/// (Path B/C — pre-fix engine returned "transaction already aborted").
/// Note: MySQL auto-commits each DML statement outside an explicit BEGIN,
/// so the INSERT has already been committed before ROLLBACK fires.
/// The test verifies that ROLLBACK itself does not error and that the
/// committed data is not affected (not that it gets rolled back).
#[test]
fn v312_77_rollback_no_active_tx_is_noop() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    // ROLLBACK with no active tx — must not error.
    let result = x.execute("ROLLBACK");
    assert!(
        result.is_ok(),
        "ROLLBACK with no active tx must not error, got {:?}",
        result
    );
    // The INSERT was auto-committed. ROLLBACK is a no-op so committed data persists.
    let res = x.execute("SELECT COUNT(*) FROM t").unwrap();
    let count = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(
        count, 1,
        "auto-committed INSERT must persist after ROLLBACK no-op"
    );
}

/// Test E: COMMIT when no transaction is active must be a no-op
/// (Path B — pre-fix engine returned "transaction already committed").
#[test]
fn v312_77_commit_no_active_tx_is_noop() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    // COMMIT with no active tx — must not error.
    let result = x.execute("COMMIT");
    assert!(
        result.is_ok(),
        "COMMIT with no active tx must not error, got {:?}",
        result
    );
    // Table must still have the row.
    let res = x.execute("SELECT COUNT(*) FROM t").unwrap();
    let count = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(count, 1, "COMMIT no-op must not affect data");
}

/// Test F: BEGIN + DELETE + ROLLBACK — ROLLBACK must restore the deleted row.
#[test]
fn v312_77_rollback_undoes_delete() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY, b INT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    x.execute("BEGIN").unwrap();
    x.execute("DELETE FROM t WHERE a = 2").unwrap();
    x.execute("ROLLBACK").unwrap();
    let res = x.execute("SELECT COUNT(*) FROM t").unwrap();
    let count = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(count, 2, "ROLLBACK must restore the deleted row");
}

/// Test G: INSERT + BEGIN + UPDATE + ROLLBACK — the #4847 original scenario
/// (DML before BEGIN, then UPDATE inside BEGIN, then ROLLBACK).
#[test]
fn v312_77_rollback_after_prior_implicit_insert() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(a INT PRIMARY KEY, b INT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    // The INSERT above starts an implicit tx. Now BEGIN should not error
    // (Path A fix: pre-COMMIT clears it). Then UPDATE inside BEGIN, ROLLBACK.
    x.execute("BEGIN").unwrap();
    x.execute("UPDATE t SET b = 99 WHERE a = 1").unwrap();
    x.execute("ROLLBACK").unwrap();
    let res = x.execute("SELECT b FROM t").unwrap();
    let b = match &res.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer, got {:?}", other),
    };
    assert_eq!(b, 10, "UPDATE inside BEGIN...ROLLBACK must be undone");
}
