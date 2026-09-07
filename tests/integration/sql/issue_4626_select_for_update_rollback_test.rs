//! Regression tests for Issue #4626 (V312-RC-GA PR-A7) —
//! `SELECT ... FOR UPDATE` followed by `ROLLBACK` must not silently
//! abort the surrounding transaction.
//!
//! Reproducer from issue #4626 body (CLI batch context, where INSERT
//! before BEGIN does NOT leave stale implicit-tx state):
//!
//! ```sql
//! CREATE TABLE t(id int, val int);
//! INSERT INTO t VALUES (1, 10);
//! BEGIN;
//! SELECT * FROM t WHERE id = 1 FOR UPDATE;
//! ROLLBACK;
//! ```
//!
//! Expected: `ROLLBACK` succeeds without "transaction already aborted".
//! Actual (before fix): ROLLBACK errors with that message because
//! SELECT FOR UPDATE internally aborts the surrounding tx.
//!
//! These tests exercise the same path via `engine.execute()` without
//! the INSERT implicit-tx leakage (which is a separate concern);
//! the core SELECT FOR UPDATE → ROLLBACK failure mode is what we lock
//! down for v3.12.0 GA.

use sqlrustgo::{ExecutionEngine, MemoryStorage};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(
        MemoryStorage::new(),
    )))
}

#[test]
fn select_for_update_then_rollback_succeeds() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, val INTEGER)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    let sel = e
        .execute("SELECT * FROM t WHERE id = 1 FOR UPDATE")
        .unwrap();
    assert_eq!(sel.rows.len(), 0, "empty table returns 0 rows");
    let rb = e.execute("ROLLBACK");
    assert!(
        rb.is_ok(),
        "ROLLBACK after SELECT FOR UPDATE must succeed; got: {:?}",
        rb.err().map(|e| e.to_string())
    );
}

#[test]
fn select_for_update_then_commit_succeeds() {
    // Positive control: COMMIT after SELECT FOR UPDATE should also succeed
    // when current_tx_id is still set (i.e., SELECT FOR UPDATE did NOT
    // silently abort the tx).
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, val INTEGER)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SELECT * FROM t WHERE id = 1 FOR UPDATE")
        .unwrap();
    let commit = e.execute("COMMIT");
    assert!(
        commit.is_ok(),
        "COMMIT after SELECT FOR UPDATE must succeed; got: {:?}",
        commit.err().map(|e| e.to_string())
    );
}

#[test]
fn select_for_update_then_begin_again_succeeds() {
    // Anti-regression: after ROLLBACK (post SELECT FOR UPDATE), a fresh
    // BEGIN should be possible. This guards against `current_tx_id` being
    // stuck at None after FOR UPDATE.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, val INTEGER)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SELECT * FROM t WHERE id = 1 FOR UPDATE")
        .unwrap();
    e.execute("ROLLBACK").unwrap();
    let begin2 = e.execute("BEGIN");
    assert!(
        begin2.is_ok(),
        "BEGIN after ROLLBACK (post SELECT FOR UPDATE) must succeed; got: {:?}",
        begin2.err().map(|e| e.to_string())
    );
    let commit2 = e.execute("COMMIT");
    assert!(
        commit2.is_ok(),
        "second COMMIT must succeed; got: {:?}",
        commit2.err().map(|e| e.to_string())
    );
}
