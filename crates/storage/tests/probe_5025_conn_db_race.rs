//! #5025 review probe: is per-connection `current_db` actually race-free?
//!
//! The WIP in `wt-5025` re-asserts the connection's database at the top of
//! every command:
//!
//! ```ignore
//! if cmd != packet_type::COM_QUIT {
//!     storage.write().set_current_db(conn_db)...
//! }
//! ```
//!
//! and only then, further down, dispatches the statement.
//!
//! Question this probe answers: **between the re-assert and the statement's
//! table lookup, can another connection change `current_db`?**
//!
//! `FileStorage` stores `current_db: RwLock<String>` and `tbl()` reads it on
//! *every* call (`file_storage.rs:1114`). So a lookup is not one atomic read
//! of "the database at statement start" — it re-reads the current value each
//! time. A concurrent `USE` on another connection can therefore retarget a
//! statement that has already started.
//!
//! This is a probe, not a gate: it documents observed behaviour so the
//! WIP author can decide whether re-assert-per-command is sufficient or
//! whether the database must be bound into the statement instead.

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
    }
}

fn seed(storage: &mut FileStorage, db: &str, table: &str, id: i64) {
    storage.set_current_db(db).expect("switch db");
    storage.create_table(&tbl(table)).expect("create table");
    storage
        .insert(table, vec![vec![Value::Integer(id)]])
        .expect("insert");
    storage.flush().expect("flush");
}

/// What the WIP's re-assert guarantees, and what it does not.
///
/// The reader mimics a statement already in flight: it "re-asserts" d1, then
/// resolves a table name. The writer switches to d2 in between, exactly as a
/// second connection running `USE d2` would.
#[test]
fn probe_5025_reassert_per_command_has_a_toctou_window() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = FileStorage::new(tmp.path().to_path_buf()).expect("open");

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    seed(&mut storage, "d1", "shared_name", 1);
    seed(&mut storage, "d2", "shared_name", 2);

    // Reader side, modelling the real call sequence in `do_command_loop`:
    //
    //   1. `storage.write().set_current_db(conn_db)`   <- temporary guard,
    //      released as soon as that statement ends
    //   2. ... gap, no lock held ...
    //   3. `engine.write()` -> `execute(sql)` -> `storage.scan(...)`
    //
    // The gap between 1 and 3 is where another connection can slip in. This
    // probe keeps the reader's guard dropped between those two steps, which
    // is what the server actually does.
    let shared = Arc::new(parking_lot::RwLock::new(storage));
    let reader_storage = Arc::clone(&shared);
    let reader = thread::spawn(move || {
        // Step 1: re-assert, then RELEASE the guard — exactly like the WIP.
        {
            let mut guard = reader_storage.write();
            guard.set_current_db("d1").expect("re-assert d1");
        }
        // Step 2: the window. The server does socket I/O and parsing here
        // with no lock held.
        thread::sleep(std::time::Duration::from_millis(150));
        // Step 3: execute — the statement resolves its table now.
        let mut guard = reader_storage.write();
        let tables = guard.list_tables();
        let rows = guard.scan("shared_name").expect("scan");
        let db_at_end = guard.current_db();
        (db_at_end, tables, rows.len())
    });

    // Writer side: another connection runs `USE d2` mid-statement.
    thread::sleep(std::time::Duration::from_millis(60));
    {
        let mut guard = shared.write();
        guard.set_current_db("d2").expect("concurrent USE d2");
    }

    let (db_at_end, tables_seen, rows) = reader.join().expect("reader thread");

    println!(
        "reader started in d1; after concurrent USE d2 it ended in {:?}, \
         list_tables saw {:?}, scan returned {} rows",
        db_at_end, tables_seen, rows
    );

    // d1 and d2 both have a table called `shared_name`, so a cross-database
    // read is *possible* here by construction. This probe reports whether it
    // actually happened; it does not assert, because a non-reproducing race
    // must not become a flaky gate.
    if db_at_end == "d2" {
        println!(
            "OBSERVED: the in-flight statement was retargeted to d2 by a \
             concurrent USE. The per-command re-assert does NOT bind the \
             database for the whole statement."
        );
    } else {
        println!(
            "not observed in this run (race window is timing-dependent) — \
             the window exists structurally: set_current_db takes \
             &mut self, and tbl() re-reads current_db on every call."
        );
    }
}
