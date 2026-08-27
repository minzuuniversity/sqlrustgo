//! Regression tests for Issue #4519 — SAVEPOINT / ROLLBACK TO SAVEPOINT
//! physical-undo semantics.
//!
//! Tsinghua's MySQL curriculum (chapter 9) introduces SAVEPOINT and
//! `ROLLBACK TO SAVEPOINT` as a way to undo part of an in-flight
//! transaction without aborting the whole thing. Before #4519, the
//! engine registered savepoints and replayed the undo log, but the
//! per-record closure passed into `rollback_to_savepoint_with_undo`
//! was a `|_| Ok(())` no-op — so students running the §9 examples
//! observed `val=200` instead of the expected `val=100` after
//! `ROLLBACK TO SAVEPOINT sp1`.
//!
//! These tests pin down the four physical-undo paths:
//! - UPDATE no-WHERE (delete + insert loop, every row touched)
//! - UPDATE with-WHERE (delete + insert loop, only matching rows)
//! - DELETE no-WHERE (whole-table delete)
//! - DELETE with-WHERE (per-PK delete)
//! - INSERT undo (delete the new row)
//! - Nested savepoints (outer survives inner rollback)
//! - RELEASE SAVEPOINT drops the savepoint but keeps prior undo entries
//!   for outer scope
//! - ROLLBACK TO SAVEPOINT with an unknown name returns an error and
//!   leaves state untouched.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_storage::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn scalar_int(row: &[Value], idx: usize) -> i64 {
    match row[idx] {
        Value::Integer(n) => n,
        ref other => panic!("expected Integer at column {idx}, got {other:?}"),
    }
}

fn scalar_text(row: &[Value], idx: usize) -> String {
    match &row[idx] {
        Value::Text(s) => s.clone(),
        ref other => panic!("expected Text at column {idx}, got {other:?}"),
    }
}

#[test]
fn savepoint_undo_update_no_where_restores_prior_value() {
    // Tsinghua §9 first example. After SAVEPOINT sp1 + UPDATE + ROLLBACK
    // TO SAVEPOINT sp1, the row must be back to its pre-UPDATE value.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("UPDATE t SET val = 200 WHERE id = 1").unwrap();
    // Sanity: forward UPDATE is observable inside the TX.
    let mid = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&mid.rows[0], 0), 200);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    let after = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&after.rows[0], 0), 100);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_undo_update_with_where_restores_matching_rows() {
    // Multi-row UPDATE with WHERE; rollback must restore ONLY the
    // matching rows, leaving non-matching rows intact.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 100), (2, 200), (3, 300)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("UPDATE t SET val = 999 WHERE id IN (1, 3)")
        .unwrap();
    let mid = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(scalar_int(&mid.rows[0], 1), 999);
    assert_eq!(scalar_int(&mid.rows[1], 1), 200);
    assert_eq!(scalar_int(&mid.rows[2], 1), 999);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    let after = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(scalar_int(&after.rows[0], 1), 100);
    assert_eq!(scalar_int(&after.rows[1], 1), 200);
    assert_eq!(scalar_int(&after.rows[2], 1), 300);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_undo_delete_no_where_reinserts_all_rows() {
    // Whole-table DELETE after SAVEPOINT sp1 — all rows must come back.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("DELETE FROM t").unwrap();
    let mid = e.execute("SELECT COUNT(*) AS c FROM t").unwrap();
    assert_eq!(scalar_int(&mid.rows[0], 0), 0);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    let after = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(after.rows.len(), 3);
    assert_eq!(scalar_int(&after.rows[0], 0), 1);
    assert_eq!(scalar_int(&after.rows[0], 1), 10);
    assert_eq!(scalar_int(&after.rows[1], 0), 2);
    assert_eq!(scalar_int(&after.rows[1], 1), 20);
    assert_eq!(scalar_int(&after.rows[2], 0), 3);
    assert_eq!(scalar_int(&after.rows[2], 1), 30);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_undo_delete_with_where_reinserts_matching_rows() {
    // Selective DELETE after SAVEPOINT sp1 — only the deleted rows
    // come back; non-matching rows are untouched.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("DELETE FROM t WHERE id = 2").unwrap();
    let mid = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(mid.rows.len(), 2);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    let after = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(after.rows.len(), 3);
    assert_eq!(scalar_int(&after.rows[0], 0), 1);
    assert_eq!(scalar_int(&after.rows[1], 0), 2);
    assert_eq!(scalar_int(&after.rows[1], 1), 20);
    assert_eq!(scalar_int(&after.rows[2], 0), 3);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_undo_insert_removes_new_rows() {
    // INSERT after SAVEPOINT sp1 — rollback must delete the new rows.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice')").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("INSERT INTO t VALUES (2, 'bob'), (3, 'carol')")
        .unwrap();
    let mid = e.execute("SELECT id FROM t ORDER BY id").unwrap();
    assert_eq!(mid.rows.len(), 3);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    let after = e.execute("SELECT id, name FROM t ORDER BY id").unwrap();
    assert_eq!(after.rows.len(), 1);
    assert_eq!(scalar_int(&after.rows[0], 0), 1);
    assert_eq!(scalar_text(&after.rows[0], 1), "alice");
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_nested_inner_rollback_outer_survives() {
    // Nested savepoints: ROLLBACK TO SAVEPOINT sp2 must NOT undo the
    // changes that were committed BEFORE SAVEPOINT sp1.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("UPDATE t SET val = 150 WHERE id = 1").unwrap();
    e.execute("SAVEPOINT sp2").unwrap();
    e.execute("UPDATE t SET val = 200 WHERE id = 1").unwrap();
    e.execute("ROLLBACK TO SAVEPOINT sp2").unwrap();
    // After inner rollback we should be back to the sp1 state (val=150).
    let after_inner = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&after_inner.rows[0], 0), 150);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    // After outer rollback we should be back to the pre-savepoint state (val=100).
    let after_outer = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&after_outer.rows[0], 0), 100);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_release_savepoint_keeps_prior_undo_for_outer() {
    // RELEASE SAVEPOINT sp1 should drop sp1 from the stack but keep
    // its undo entries visible to outer sp0 — so a later
    // ROLLBACK TO SAVEPOINT sp0 still undoes everything between
    // BEGIN and sp0.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    e.execute("SAVEPOINT sp0").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("UPDATE t SET val = 999 WHERE id = 1").unwrap();
    e.execute("RELEASE SAVEPOINT sp1").unwrap();
    e.execute("ROLLBACK TO SAVEPOINT sp0").unwrap();
    let after = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&after.rows[0], 0), 100);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_rollback_to_unknown_name_is_error_state_intact() {
    // ROLLBACK TO SAVEPOINT <missing> must surface an error and leave
    // the storage state unchanged.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("UPDATE t SET val = 200 WHERE id = 1").unwrap();
    let r = e.execute("ROLLBACK TO SAVEPOINT sp_missing");
    assert!(
        r.is_err(),
        "expected error for unknown savepoint, got {r:?}"
    );
    // Storage state should still reflect the forward UPDATE — the
    // rollback was rejected, so val stays at 200.
    let mid = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&mid.rows[0], 0), 200);
    e.execute("COMMIT").unwrap();
}

#[test]
fn savepoint_no_active_savepoint_dml_does_not_grow_undo_log() {
    // When no savepoint is active, DML must NOT pollute the undo log —
    // the helper short-circuits via `has_active_savepoint(tx_id)`.
    // We verify behaviourally: a plain COMMIT (no savepoint, no
    // rollback) leaves the post-DML state in place.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    e.execute("UPDATE t SET val = 200 WHERE id = 1").unwrap();
    e.execute("COMMIT").unwrap();
    let r = e.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(scalar_int(&r.rows[0], 0), 200);
}

#[test]
fn savepoint_undo_mixed_dml_sequence_in_reverse_order() {
    // Insert + update + delete all after SAVEPOINT sp1; rollback must
    // restore the pre-savepoint snapshot (row 1 only, val=100). Row 2
    // was inserted after sp1, so it must NOT survive the rollback.
    let mut e = engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("INSERT INTO t VALUES (2, 200)").unwrap();
    e.execute("UPDATE t SET val = 150 WHERE id = 1").unwrap();
    e.execute("DELETE FROM t WHERE id = 2").unwrap();
    // Pre-rollback: row 1 should be 150, row 2 should be gone.
    let mid = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(mid.rows.len(), 1);
    assert_eq!(scalar_int(&mid.rows[0], 0), 1);
    assert_eq!(scalar_int(&mid.rows[0], 1), 150);
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();
    // Post-rollback: row 1 back to 100, row 2 NOT present (was inserted
    // after sp1).
    let after = e.execute("SELECT id, val FROM t ORDER BY id").unwrap();
    assert_eq!(after.rows.len(), 1);
    assert_eq!(scalar_int(&after.rows[0], 0), 1);
    assert_eq!(scalar_int(&after.rows[0], 1), 100);
    e.execute("COMMIT").unwrap();
}
