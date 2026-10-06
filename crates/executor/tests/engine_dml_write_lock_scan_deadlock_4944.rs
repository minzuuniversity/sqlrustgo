//! #4944: self-deadlock when a table scan is requested from inside the
//! write critical section.
//!
//! `ExecutionEngine::storage_read()` acquires a **read** lock. It tries
//! `try_read()` first and falls back to a blocking `read()` when that
//! fails — which it always does for the thread already holding the **write**
//! lock. `parking_lot::RwLock` is not reentrant, so such a call blocks on
//! itself forever.
//!
//! Two call sites in `src/engine_dml.rs` did exactly that:
//!
//! | site | statement | symptom |
//! |---|---|---|
//! | AUTOINCREMENT `MAX(id)` scan | `INSERT` into a table with an `AUTOINCREMENT` column | executor never returns |
//! | REPLACE conflict scan | `REPLACE INTO ...` | executor never returns |
//!
//! Both already had a guard-taking sibling nearby
//! (`scan_for_reader_with(&storage, ..)`), which exists for exactly this
//! reason and is documented as such in `execution_engine_methods.rs`. These
//! two call sites simply used the wrong variant.
//!
//! The tests below hang (rather than fail) if the deadlock returns, so they
//! are run under `cargo test`'s default per-test timeout rather than being
//! written as `#[ignore]`d GAP markers — which is what the WP-C matrix
//! matrix did for the AUTOINCREMENT half before the fix.

use parking_lot::RwLock;
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::{MemoryStorage, Value};
use std::sync::Arc;

fn engine() -> MemoryExecutionEngine {
    MemoryExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn as_i64(v: &Value) -> i64 {
    v.as_integer()
        .unwrap_or_else(|| panic!("expected integer, got {v:?}"))
}

fn as_str(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn insert_into_autoincrement_table_returns() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO t(name) VALUES ('alice'),('bob'),('carol')")
        .unwrap();
    let r = e.execute("SELECT id, name FROM t ORDER BY id").unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(as_i64(&r.rows[0][0]), 1);
    assert_eq!(as_i64(&r.rows[1][0]), 2);
    assert_eq!(as_i64(&r.rows[2][0]), 3);
    assert_eq!(as_str(&r.rows[0][1]), "alice");
}

#[test]
fn autoincrement_continues_after_delete() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, v INT)")
        .unwrap();
    e.execute("INSERT INTO t(v) VALUES (10),(20),(30)").unwrap();
    e.execute("DELETE FROM t WHERE id = 2").unwrap();
    let r = e.execute("SELECT id FROM t ORDER BY id").unwrap();
    assert_eq!(
        r.rows.iter().map(|x| as_i64(&x[0])).collect::<Vec<_>>(),
        vec![1, 3],
        "the surviving rows keep their ids"
    );
    e.execute("INSERT INTO t(v) VALUES (40)").unwrap();
    let r = e.execute("SELECT MAX(id) FROM t").unwrap();
    assert_eq!(
        as_i64(&r.rows[0][0]),
        4,
        "a reused id must not be handed out again"
    );
}

#[test]
fn replace_into_existing_row_replaces_it() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, v VARCHAR(50))")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1,'apple')").unwrap();
    let r = e.execute("REPLACE INTO t VALUES (1,'banana')").unwrap();
    assert_eq!(r.affected_rows, 1);
    let rows = e.execute("SELECT v FROM t WHERE id = 1").unwrap().rows;
    assert_eq!(
        rows.len(),
        1,
        "REPLACE must not leave a duplicate row: {rows:?}"
    );
    assert_eq!(as_str(&rows[0][0]), "banana");
}

#[test]
fn replace_into_new_key_inserts() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, v VARCHAR(50))")
        .unwrap();
    let r = e.execute("REPLACE INTO t VALUES (1,'apple')").unwrap();
    assert_eq!(r.affected_rows, 1);
    let rows = e.execute("SELECT v FROM t WHERE id = 1").unwrap().rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(as_str(&rows[0][0]), "apple");
}

/// Both deadlocks were in the INSERT path, so a mixed statement must work
/// end to end: an AUTOINCREMENT insert, a REPLACE against it, and a read.
#[test]
fn autoincrement_and_replace_together() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, v VARCHAR(50))")
        .unwrap();
    e.execute("INSERT INTO t(v) VALUES ('a'),('b')").unwrap();
    e.execute("REPLACE INTO t VALUES (2,'b2')").unwrap();
    let rows = e.execute("SELECT id, v FROM t ORDER BY id").unwrap().rows;
    assert_eq!(rows.len(), 2, "REPLACE must not add a third row: {rows:?}");
    assert_eq!(as_i64(&rows[0][0]), 1);
    assert_eq!(as_i64(&rows[1][0]), 2);
    assert_eq!(as_str(&rows[1][1]), "b2");
}
