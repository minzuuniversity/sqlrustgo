//! #5140: views are per-database objects, like tables.
//!
//! `ExecutionEngine::views` keyed by the bare view name while storage keyed its
//! own view registry by `scoped_key(db, name)` (#5025). `CREATE VIEW` writes
//! both, and they drifted:
//!
//! ```text
//! d1: CREATE VIEW v1 AS SELECT * FROM t
//! d2: SHOW TABLES       -> v1 listed          (d1's view leaked)
//! d2: SELECT * FROM v1  -> "Table not found: t"
//! d2: CREATE VIEW shared AS ... -> "already exists"   (d2 has no such view)
//! ```
//!
//! The second one is the worst: the defining query came from `d1`, but the
//! table inside it resolved against `d2` — the view crossed databases from the
//! inside out.
//!
//! The in-memory key is now `"{db}\0{view}"`, matching the storage-side
//! scoping. The NUL separator cannot occur in a database or view name.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, StorageEngine};
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine(tag: &str) -> ExecutionEngine<FileStorage> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("view_db_{}_{}_{}", tag, std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(
        FileStorage::new(dir).expect("open"),
    )))
}

fn names(rows: &[Vec<Value>]) -> Vec<String> {
    rows.iter().map(|r| r[0].to_string()).collect()
}

/// A view created in d1 must not be listed from d2.
#[test]
fn show_tables_does_not_leak_another_databases_view() {
    let mut e = engine("leak");
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("CREATE DATABASE d2").expect("d2");

    e.execute("USE d1").expect("d1");
    e.execute("CREATE TABLE t (v INT)").expect("t");
    e.execute("INSERT INTO t VALUES (1)").expect("row");
    e.execute("CREATE VIEW v1 AS SELECT * FROM t")
        .expect("view");

    e.execute("USE d2").expect("d2");
    let rows = e.execute("SHOW TABLES").expect("show");
    assert!(
        !names(&rows.rows).iter().any(|n| n.contains("v1")),
        "d1's view leaked into d2's listing: {:?}",
        rows.rows
    );

    // And it is still listed from its own database.
    e.execute("USE d1").expect("back to d1");
    let rows = e.execute("SHOW TABLES").expect("show");
    assert!(
        names(&rows.rows).iter().any(|n| n.contains("v1")),
        "the view must remain visible in the database that owns it: {:?}",
        rows.rows
    );
}

/// A view must not be selectable from a database that does not own it.
///
/// Before the fix, `SELECT * FROM v` in `d2` found `d1`'s view definition and
/// then failed inside it with "Table not found: t" — the view crossed
/// databases from the inside out. Now the name simply is not there, which is
/// the same answer MySQL gives.
#[test]
fn a_view_is_not_selectable_from_another_database() {
    let mut e = engine("resolve");
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("CREATE DATABASE d2").expect("d2");

    e.execute("USE d1").expect("d1");
    e.execute("CREATE TABLE t (v INT)").expect("t in d1");
    e.execute("INSERT INTO t VALUES (42)").expect("d1 row");
    e.execute("CREATE VIEW v AS SELECT * FROM t")
        .expect("view in d1");

    e.execute("USE d2").expect("d2");
    let err = e
        .execute("SELECT * FROM v")
        .expect_err("d2 has no view `v`");
    assert!(
        matches!(&err, sqlrustgo::SqlError::TableNotFound(n) if n == "v"),
        "expected TableNotFound(v) for a view owned by d1, got {err:?}"
    );

    // The owning database still reads it.
    e.execute("USE d1").expect("back to d1");
    let rows = e
        .execute("SELECT * FROM v")
        .expect("select through the view");
    assert_eq!(rows.rows, vec![vec![Value::Integer(42)]]);
}

/// Two databases may each hold a view of the same name.
#[test]
fn the_same_view_name_can_exist_in_two_databases() {
    let mut e = engine("same");
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("CREATE DATABASE d2").expect("d2");

    e.execute("USE d1").expect("d1");
    e.execute("CREATE TABLE t (v INT)").expect("t d1");
    e.execute("INSERT INTO t VALUES (111)").expect("d1 row");
    e.execute("CREATE VIEW shared AS SELECT * FROM t")
        .expect("view d1");

    e.execute("USE d2").expect("d2");
    e.execute("CREATE TABLE t (v INT)").expect("t d2");
    e.execute("INSERT INTO t VALUES (222)").expect("d2 row");
    // Before the fix this was rejected: "View 'shared' already exists".
    e.execute("CREATE VIEW shared AS SELECT * FROM t")
        .expect("a same-named view in another database is not a conflict");

    e.execute("USE d1").expect("back to d1");
    let a = e.execute("SELECT * FROM shared").expect("d1's view");
    assert_eq!(a.rows, vec![vec![Value::Integer(111)]]);

    e.execute("USE d2").expect("d2");
    let b = e.execute("SELECT * FROM shared").expect("d2's view");
    assert_eq!(
        b.rows,
        vec![vec![Value::Integer(222)]],
        "each database resolves its own `shared`"
    );
}

/// `DROP VIEW` in one database must not remove the other's same-named view.
#[test]
fn drop_view_is_scoped_to_its_database() {
    let mut e = engine("drop");
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("CREATE DATABASE d2").expect("d2");

    e.execute("USE d1").expect("d1");
    e.execute("CREATE TABLE t (v INT)").expect("t d1");
    e.execute("CREATE VIEW v AS SELECT * FROM t")
        .expect("view d1");

    e.execute("USE d2").expect("d2");
    e.execute("CREATE TABLE t (v INT)").expect("t d2");
    e.execute("CREATE VIEW v AS SELECT * FROM t")
        .expect("view d2");

    e.execute("DROP VIEW v").expect("drop in d2");

    e.execute("USE d1").expect("d1");
    let a = e.execute("SELECT * FROM v");
    assert!(
        a.is_ok(),
        "dropping d2's view must not remove d1's: {:?}",
        a.err()
    );
}

/// `SHOW FULL TABLES` labels views as VIEW / BASE TABLE per database.
#[test]
fn show_full_tables_labels_only_its_own_views() {
    let mut e = engine("full");
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("CREATE DATABASE d2").expect("d2");

    e.execute("USE d1").expect("d1");
    e.execute("CREATE TABLE t (v INT)").expect("t d1");
    e.execute("CREATE VIEW v AS SELECT * FROM t")
        .expect("view d1");

    e.execute("USE d2").expect("d2");
    e.execute("CREATE TABLE t (v INT)").expect("t d2");
    let rows = e.execute("SHOW FULL TABLES").expect("show full");
    let types: Vec<String> = rows
        .rows
        .iter()
        .map(|r| format!("{} {}", r[0], r[1]))
        .collect();
    assert!(
        !types.iter().any(|t| t.contains("v") && t.contains("VIEW")),
        "d1's view must not be labelled in d2's listing: {types:?}"
    );
}
