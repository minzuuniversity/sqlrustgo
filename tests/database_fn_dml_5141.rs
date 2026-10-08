//! #5141: `DATABASE()` / `SCHEMA()` must resolve to the connection's database
//! in every statement, not only SELECT.
//!
//! The substitution was applied inside `execute_select` only, so DML fell
//! through to `eval_fn`'s hard-coded `"default"`:
//!
//! ```text
//! USE d1;  -- t (v INT, tag TEXT)
//! UPDATE t SET tag = DATABASE() WHERE v = 1   ->  tag = "default"   (silent)
//! INSERT INTO t VALUES (2, DATABASE())          ->  tag = NULL        (silent)
//! ```
//!
//! Two paths, two different wrong answers for one function, no error anywhere.
//!
//! The fix rewrites the whole statement in `execute` before dispatch, so a
//! future statement type inherits the behaviour instead of regressing.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, StorageEngine};
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine(tag: &str) -> ExecutionEngine<FileStorage> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("db_fn_{}_{}_{}", tag, std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(
        FileStorage::new(dir).expect("open"),
    )))
}

fn seeded(tag: &str) -> ExecutionEngine<FileStorage> {
    let mut e = engine(tag);
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("USE d1").expect("use d1");
    e.execute("CREATE TABLE t (v INT, tag TEXT)").expect("t");
    e.execute("INSERT INTO t VALUES (1, 'a')").expect("seed");
    e
}

/// The regression that mattered: UPDATE stored the fallback literal.
#[test]
fn update_set_database_stores_the_connection_database() {
    let mut e = seeded("upd");
    e.execute("UPDATE t SET tag = DATABASE() WHERE v = 1")
        .expect("update");

    let rows = e.execute("SELECT tag FROM t").expect("read");
    assert_eq!(
        rows.rows[0][0],
        Value::Text("d1".to_string()),
        "DATABASE() in UPDATE SET must store the connection's database"
    );
}

/// INSERT stored NULL rather than the literal — a second, different wrong
/// answer for the same function.
#[test]
fn insert_values_database_stores_the_connection_database() {
    let mut e = seeded("ins");
    e.execute("INSERT INTO t VALUES (2, DATABASE())")
        .expect("insert");

    let rows = e.execute("SELECT tag FROM t WHERE v = 2").expect("read");
    assert_eq!(
        rows.rows[0][0],
        Value::Text("d1".to_string()),
        "DATABASE() in INSERT VALUES must store the connection's database, not NULL"
    );
}

/// A different connection's database must not leak in.
#[test]
fn database_reflects_the_running_connection_not_the_last_use() {
    let mut e = engine("leak");
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("CREATE DATABASE d2").expect("d2");
    e.execute("USE d1").expect("d1");
    e.execute("CREATE TABLE t (tag TEXT)").expect("t");
    e.execute("USE d2").expect("d2");
    e.execute("CREATE TABLE t (tag TEXT)").expect("t in d2");

    // On d2 now; the table is d2's.
    e.execute("INSERT INTO t VALUES (DATABASE())")
        .expect("insert");
    let rows = e.execute("SELECT tag FROM t").expect("read");
    assert_eq!(rows.rows[0][0], Value::Text("d2".to_string()));

    e.execute("USE d1").expect("back to d1");
    e.execute("INSERT INTO t VALUES (DATABASE())")
        .expect("insert");
    let rows = e.execute("SELECT tag FROM t").expect("read");
    assert_eq!(
        rows.rows[0][0],
        Value::Text("d1".to_string()),
        "each database records its own name"
    );
}

/// `SCHEMA()` is the documented MySQL synonym and travels the same path.
#[test]
fn schema_is_substituted_alongside_database() {
    let mut e = seeded("sch");
    e.execute("INSERT INTO t VALUES (3, SCHEMA())")
        .expect("insert");
    let rows = e.execute("SELECT tag FROM t WHERE v = 3").expect("read");
    assert_eq!(rows.rows[0][0], Value::Text("d1".to_string()));
}

/// SELECT still works — the fix must not regress the path that already worked.
#[test]
fn select_still_reports_the_connection_database() {
    let mut e = seeded("sel");
    let rows = e.execute("SELECT DATABASE()").expect("select");
    assert_eq!(rows.rows[0][0], Value::Text("d1".to_string()));
}

/// A predicate on `DATABASE()` in DML must see the same value.
#[test]
fn where_clause_predicate_on_database_matches() {
    let mut e = seeded("whr");
    // No row should match: the table has v = 1, not a row whose v is 'd1'.
    let none = e
        .execute("UPDATE t SET tag = 'x' WHERE DATABASE() = 'nope'")
        .expect("update");
    assert_eq!(
        none.affected_rows, 0,
        "predicate must see the real database"
    );

    let all = e
        .execute("UPDATE t SET tag = 'y' WHERE DATABASE() = 'd1'")
        .expect("update");
    assert_eq!(
        all.affected_rows, 1,
        "predicate must match on the real database"
    );
}
