//! #5057: `FileStorage` must route table operations to the database the
//! caller named, not to whichever database the shared storage last held.
//!
//! # Why these tests exist
//!
//! `FileStorage` used to keep a `current_db` field and its write helpers
//! read it from the inside (`scoped_key(&self.current_db.read().unwrap(), ...)`).
//! The storage is shared by every connection, so that value answered "whoever
//! ran `USE` last", not "whoever is asking" — and `tbl()` re-read it on every
//! call, so a statement already in flight could be retargeted by another
//! connection mid-flight.
//!
//! #5057 threads an explicit `db` through the write chain instead:
//! `insert_in_db` / `force_insert_in_db` reach `insert_at`, which passes the
//! database down through `insert_direct` / `insert_buffered` /
//! `update_pk_index*` / `flush_buffer`.
//!
//! # What changed since the first version of this file
//!
//! The storage-level `current_db` / `set_current_db` pair has since been
//! **removed from the `StorageEngine` trait**. The session's database now
//! lives in `ExecutionEngine::session_db` and reaches the storage as an
//! explicit `db` argument on every `*_in_db` method. The database-less
//! methods no longer "follow `current_db`" — they resolve to
//! `DEFAULT_DATABASE` deterministically (`FileStorage::insert` is
//! `insert_at(DEFAULT_DATABASE, ...)`).
//!
//! So the two families are still distinct, but the second one now has a
//! *fixed* target instead of a moving one. That is a stronger guarantee
//! than before, and this file pins it:
//!
//! 1. Naming a database routes there. A caller who named `d2` must never
//!    have the row land anywhere else.
//! 2. Omitting the database lands in `default` — not in "whatever the last
//!    `USE` selected". Two unnamed inserts from two connections can no
//!    longer bleed into each other.
//! 3. The whole chain honours the database, not just the entry point. A
//!    `tbl_in` at the method entry that quietly fell back to something else
//!    inside a helper would pass (1) and lose the row in (2).
//!
//! The mutation notes in `SESSION_CONTEXT_5057_RECON_2026-10-07.md` §8 record
//! what is deliberately NOT covered here yet: `flush_all_buffers` still
//! flushes only the active database, because `dirty_tables` is keyed by a
//! mixture of bare and scoped table names. Fixing that needs the set scoped
//! first; doing it here would write files under the wrong directory.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};

const DEFAULT_DB: &str = "default";

fn tbl(name: &str) -> TableInfo {
    TableInfo {
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

fn fresh() -> (tempfile::TempDir, FileStorage) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let storage = FileStorage::new(tmp.path().to_path_buf()).expect("open");
    (tmp, storage)
}

/// Count rows in `(db, table)`.
///
/// No flush here. `flush()` persists dirty tables, and a flush cannot bring
/// a *named* database's rows to disk — that limitation is `dirty_tables`'
/// unscoped keys, tracked in `SESSION_CONTEXT_5057_RECON_2026-10-07.md` §8.
/// Reading through `scan_in_db`, which merges the write buffer, sidesteps it
/// and asserts what these tests are about: which database the rows landed in.
fn count(storage: &FileStorage, db: &str, table: &str) -> usize {
    storage.scan_in_db(db, table).expect("scan").len()
}

/// Naming a database must route the rows there.
#[test]
fn issue_5057_insert_in_db_routes_to_the_named_database() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &tbl("shared"))
        .expect("create in d1");
    s.create_table_in_db("d2", &tbl("shared"))
        .expect("create in d2");

    s.insert_in_db("d2", "shared", vec![vec![Value::Integer(1)]])
        .expect("insert naming d2");

    assert_eq!(
        count(&s, "d2", "shared"),
        1,
        "the row named for d2 must land in d2"
    );
    assert_eq!(
        count(&s, "d1", "shared"),
        0,
        "naming d2 must not write into d1"
    );
}

/// The counterpart: omitting the database lands in `default`.
///
/// This used to be "still follows `current_db`". With the storage-level
/// `current_db` gone, the database-less methods resolve to
/// `DEFAULT_DATABASE` — a fixed target, so the row cannot be retargeted by
/// another connection's `USE` no matter what.
#[test]
fn issue_5057_insert_without_a_database_lands_in_default() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &tbl("shared"))
        .expect("create in d1");
    s.create_table_in_db(DEFAULT_DB, &tbl("shared"))
        .expect("create in default");
    s.create_table_in_db("d2", &tbl("shared"))
        .expect("create in d2");

    s.insert("shared", vec![vec![Value::Integer(1)]])
        .expect("unnamed insert");

    assert_eq!(count(&s, DEFAULT_DB, "shared"), 1);
    assert_eq!(count(&s, "d1", "shared"), 0);
    assert_eq!(count(&s, "d2", "shared"), 0);
}

/// The two families must not bleed into each other: an insert that names a
/// database followed by one that does not must land in two different places.
#[test]
fn issue_5057_named_and_unnamed_inserts_do_not_bleed() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &tbl("shared"))
        .expect("create in d1");
    s.create_table_in_db(DEFAULT_DB, &tbl("shared"))
        .expect("create in default");
    s.create_table_in_db("d2", &tbl("shared"))
        .expect("create in d2");

    s.insert("shared", vec![vec![Value::Integer(10)]])
        .expect("unnamed insert -> default");
    s.insert_in_db("d2", "shared", vec![vec![Value::Integer(20)]])
        .expect("named insert -> d2");

    assert_eq!(
        count(&s, DEFAULT_DB, "shared"),
        1,
        "unnamed insert goes to default"
    );
    assert_eq!(count(&s, "d2", "shared"), 1, "named insert goes to d2");
    assert_eq!(count(&s, "d1", "shared"), 0, "neither insert may reach d1");
}

/// `force_insert` bypasses the write buffer, so it has its own chain. It
/// must honour a named database for the same reason.
#[test]
fn issue_5057_force_insert_in_db_routes_to_the_named_database() {
    let (_tmp, mut s) = fresh();
    s.create_database("d1").expect("create d1");
    s.create_database("d2").expect("create d2");

    s.create_table_in_db("d1", &tbl("shared"))
        .expect("create in d1");
    s.create_table_in_db("d2", &tbl("shared"))
        .expect("create in d2");

    s.force_insert_in_db("d2", "shared", vec![Value::Integer(99)])
        .expect("force insert into d2");

    assert_eq!(count(&s, "d2", "shared"), 1, "force_insert must reach d2");
    assert_eq!(
        count(&s, "d1", "shared"),
        0,
        "force_insert naming d2 must not touch d1"
    );
}
