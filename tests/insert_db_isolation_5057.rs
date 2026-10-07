//! #5057: the INSERT/UPDATE/DELETE paths must resolve tables against the
//! database the statement belongs to, not the one another connection last
//! selected.
//!
//! The storage-layer `*_in_db` methods arrived with #5081/#5090, but the
//! engine never called them — `execute_insert`/`_update`/`_delete` went
//! through `storage.insert`/`update`/`delete`, which read the shared
//! `current_db`. These tests go through the engine, which is the path that
//! was missing.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::path::PathBuf;
use std::sync::Arc;

fn engine() -> (ExecutionEngine<FileStorage>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("ins_iso_5057_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let e = ExecutionEngine::new(Arc::new(RwLock::new(
        FileStorage::new(dir.clone()).unwrap(),
    )));
    (e, dir)
}

fn seed(x: &mut ExecutionEngine<FileStorage>) {
    x.execute("CREATE DATABASE d1").unwrap();
    x.execute("CREATE DATABASE d2").unwrap();
    for db in ["d1", "d2"] {
        x.execute(&format!("USE {db}")).unwrap();
        x.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
    }
}

#[test]
fn insert_resolves_against_the_selected_database() {
    let (mut x, dir) = engine();
    seed(&mut x);

    x.execute("USE d1").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("USE d2").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();

    x.execute("USE d1").unwrap();
    let d1 = x.execute("SELECT COUNT(*) FROM t").unwrap().rows;
    x.execute("USE d2").unwrap();
    let d2 = x.execute("SELECT COUNT(*) FROM t").unwrap().rows;
    assert_eq!(d1[0][0], sqlrustgo_types::Value::Integer(1));
    assert_eq!(d2[0][0], sqlrustgo_types::Value::Integer(1));
}

#[test]
fn update_touches_only_the_selected_database() {
    let (mut x, dir) = engine();
    seed(&mut x);
    for (db, id) in [("d1", 1), ("d2", 2)] {
        x.execute(&format!("USE {db}")).unwrap();
        x.execute(&format!("INSERT INTO t VALUES ({id})")).unwrap();
    }

    x.execute("USE d1").unwrap();
    x.execute("UPDATE t SET id = 99").unwrap();

    x.execute("USE d1").unwrap();
    let d1 = x.execute("SELECT id FROM t").unwrap().rows;
    x.execute("USE d2").unwrap();
    let d2 = x.execute("SELECT id FROM t").unwrap().rows;
    assert_eq!(d1[0][0], sqlrustgo_types::Value::Integer(99));
    assert_eq!(
        d2[0][0],
        sqlrustgo_types::Value::Integer(2),
        "the update must not reach into the other database"
    );
}

#[test]
fn delete_touches_only_the_selected_database() {
    let (mut x, dir) = engine();
    seed(&mut x);
    for (db, id) in [("d1", 1), ("d2", 2)] {
        x.execute(&format!("USE {db}")).unwrap();
        x.execute(&format!("INSERT INTO t VALUES ({id})")).unwrap();
    }

    x.execute("USE d1").unwrap();
    x.execute("DELETE FROM t WHERE id = 1").unwrap();

    let left1 = x.execute("SELECT COUNT(*) FROM t").unwrap().rows;
    x.execute("USE d2").unwrap();
    let left2 = x.execute("SELECT COUNT(*) FROM t").unwrap().rows;
    assert_eq!(left1[0][0], sqlrustgo_types::Value::Integer(0));
    assert_eq!(
        left2[0][0],
        sqlrustgo_types::Value::Integer(1),
        "d2's row must survive a delete issued in d1"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
