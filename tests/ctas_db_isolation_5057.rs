//! #5057: `CREATE TABLE … AS SELECT` must land the new table in the
//! **connection's** database.
//!
//! `execute_create_table` (src/engine_create.rs:28) built its table with
//! `storage.create_table(&info)` and filled it with `storage.insert(&name, …)` —
//! neither takes a database, so both resolved against the storage's one shared
//! `current_db`. The file carried no `session_db` reference at all, while
//! `execute_select` had been fixed in the first #5057 step
//! (engine_select.rs:778, 3526, 3582).
//!
//! The consequence: connection B ran `USE d2`, then CTAS, and the table landed
//! in whichever database the *last* `USE` wrote — not B's.
//!
//! The #5057 isolation tests cover SELECT / INSERT / UPDATE / DELETE but never
//! CTAS, so this file pins it.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

type FS = FileStorage;

/// Two engines over one storage — two connections on one server.
fn two_connections() -> (
    ExecutionEngine<FS>,
    ExecutionEngine<FS>,
    Arc<parking_lot::RwLock<FS>>,
    PathBuf,
) {
    // Per-call counter: these tests run in parallel and each wipes its
    // directory, so a shared name lets them delete each other's data.
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ctas_db_5057_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.clone();
    let storage = Arc::new(parking_lot::RwLock::new(
        FS::new(path.clone()).expect("open"),
    ));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(Arc::clone(&storage)),
        storage,
        path,
    )
}

fn count(x: &mut ExecutionEngine<FS>, sql: &str) -> Option<usize> {
    x.execute(sql).ok().map(|r| r.rows.len())
}

/// `CREATE TABLE` on a connection whose session differs from the storage's
/// shared `current_db`.
///
/// `execute_create_table` (src/engine_create.rs) built its table with
/// `storage.create_table(&info)` — no database argument, so it resolved against
/// whichever database the last `USE` wrote. `execute_select` had already been
/// fixed in the first #5057 step (engine_select.rs:778); the write side was not.
#[test]
fn plain_create_table_lands_in_the_running_connections_database() {
    let (mut a, mut b, _s, _p) = two_connections();

    a.execute("CREATE DATABASE d1").expect("d1");
    a.execute("CREATE DATABASE d2").expect("d2");

    b.execute("USE d2").expect("b -> d2");
    a.execute("USE d1")
        .expect("a -> d1 — shared current_db now d1");

    b.execute("CREATE TABLE plain (id INT PRIMARY KEY)")
        .expect("create on b");
    b.execute("INSERT INTO plain VALUES (5)")
        .expect("insert on b");

    assert_eq!(
        count(&mut b, "SELECT id FROM plain"),
        Some(1),
        "the table belongs to B's d2"
    );
    assert_eq!(
        count(&mut a, "SELECT id FROM plain"),
        None,
        "the table must not appear in d1"
    );
}

/// Same table name in both databases: each connection must keep seeing its own
/// after the other one creates a table.
#[test]
fn same_name_in_two_databases_stays_separate() {
    let (mut a, mut b, _s, _p) = two_connections();

    a.execute("CREATE DATABASE d1").expect("d1");
    a.execute("CREATE DATABASE d2").expect("d2");

    a.execute("USE d1").expect("a -> d1");
    a.execute("CREATE TABLE t (v INT)").expect("t in d1");
    a.execute("INSERT INTO t VALUES (1)").expect("d1 row");

    b.execute("USE d2").expect("b -> d2");
    b.execute("CREATE TABLE t (v INT)").expect("t in d2");
    b.execute("INSERT INTO t VALUES (2)").expect("d2 row");
    a.execute("USE d1")
        .expect("a -> d1 again, stealing current_db");

    let a_rows = a.execute("SELECT v FROM t").expect("read a");
    assert_eq!(a_rows.rows[0][0], Value::Integer(1), "A keeps d1's row");
    let b_rows = b.execute("SELECT v FROM t").expect("read b");
    assert_eq!(b_rows.rows[0][0], Value::Integer(2), "B keeps d2's row");
}

/// A table created on B stays invisible to A after A switches back and forth —
/// the shared `current_db` must not resurrect it in d1.
#[test]
fn a_table_created_on_one_connection_never_leaks_into_the_other() {
    let (mut a, mut b, _s, _p) = two_connections();

    a.execute("CREATE DATABASE d1").expect("d1");
    a.execute("CREATE DATABASE d2").expect("d2");

    a.execute("USE d1").expect("a -> d1");
    b.execute("USE d2").expect("b -> d2");
    b.execute("CREATE TABLE only_b (id INT PRIMARY KEY)")
        .expect("create on b");

    for _ in 0..3 {
        a.execute("USE d1").unwrap();
        assert_eq!(
            count(&mut a, "SELECT id FROM only_b"),
            None,
            "B's table must not appear in d1, however often A switches"
        );
        b.execute("USE d2").unwrap();
        assert_eq!(count(&mut b, "SELECT id FROM only_b"), Some(0));
    }
}
