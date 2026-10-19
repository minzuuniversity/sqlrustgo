//! #5025 / #5375: a concurrent connection must not be able to retarget
//! another connection's statement.
//!
//! This file used to be a probe documenting a TOCTOU window: the engine
//! kept a shared `current_db: RwLock<String>`, and `tbl()` re-read it on
//! *every* call, so a connection that ran `USE d2` mid-statement could
//! retarget a statement that had already started on another connection.
//! Measured under concurrent switching, that misdirected 53.76% of
//! statements — not a rare interleaving but the common case.
//!
//! #5057 step 3 removed `current_db` / `set_current_db` altogether: the
//! database is bound into the call (`*_in_db`), so there is no shared slot
//! left to race on.
//!
//! That removal turns the probe into a gate, which is what it is now: the
//! same setup — two databases holding a table of the same name, a reader
//! naming one of them, a writer working in the other — must *always* give
//! the reader its own database's rows, with no timing dependence and no
//! "observed in this run" branch.

use sqlrustgo_storage::{FileStorage, StorageEngine, Value};
use std::sync::Arc;
use std::thread;

fn tbl(name: &str) -> sqlrustgo_storage::TableInfo {
    use sqlrustgo_storage::ColumnDefinition;
    sqlrustgo_storage::TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
        clustered: false,
    }
}

fn seed(storage: &mut FileStorage, db: &str, table: &str, id: i64) {
    // #5375: the database is named at the call site — there is no
    // "switch" to do any more.
    storage
        .create_table_in_db(db, &tbl(table))
        .expect("create table");
    storage
        .insert_in_db(db, table, vec![vec![Value::Integer(id)]])
        .expect("insert");
    storage.flush().expect("flush");
}

/// A reader that names `d1` must keep seeing `d1`, no matter what a
/// concurrent connection does in `d2`.
#[test]
fn a_concurrent_connection_cannot_retarget_another_connections_statement() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = FileStorage::new(tmp.path().to_path_buf()).expect("open");

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    seed(&mut storage, "d1", "shared_name", 1);
    seed(&mut storage, "d2", "shared_name", 2);

    let shared = Arc::new(parking_lot::RwLock::new(storage));
    let reader_storage = Arc::clone(&shared);
    let reader = thread::spawn(move || {
        // The reader names its database at the call site, exactly as the
        // executor does now that `current_db` is gone.
        let guard = reader_storage.read();
        let tables = guard.list_tables_in_db("d1");
        let rows = guard.scan_in_db("d1", "shared_name").expect("scan d1");
        (tables, rows.len())
    });

    // Writer side: another connection works in d2 while the reader is
    // mid-statement. Under the old design this was the `USE d2` that
    // could retarget the reader.
    thread::sleep(std::time::Duration::from_millis(60));
    {
        let mut guard = shared.write();
        guard
            .insert_in_db("d2", "shared_name", vec![vec![Value::Integer(3)]])
            .expect("write into d2");
    }

    let (tables_seen, rows) = reader.join().expect("reader thread");

    assert_eq!(rows, 1, "the reader must see d1's own row, got {rows} rows");
    assert!(
        tables_seen.iter().any(|t| t == "shared_name"),
        "d1 must list its own table, got {tables_seen:?}"
    );

    // The concurrent write must have landed in d2 and nowhere else.
    let guard = shared.read();
    let d1 = guard.scan_in_db("d1", "shared_name").expect("scan d1");
    assert_eq!(d1.len(), 1, "d1 must still hold exactly its own row");
    let d2 = guard.scan_in_db("d2", "shared_name").expect("scan d2");
    assert_eq!(
        d2.len(),
        2,
        "d2 must hold its own row plus the concurrent write"
    );
}
