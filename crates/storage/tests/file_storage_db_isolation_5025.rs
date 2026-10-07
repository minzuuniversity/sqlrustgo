//! #5025: `FileStorage` table-space isolation by database.
//!
//! `tests/e2e/multi_db_isolation_5025.rs` drives `MemoryExecutionEngine`,
//! so it cannot reach the production path. The real server opens a
//! `FileStorage` (via `BoxStorageEngine`), and before this change
//! `FileStorage` did not implement `current_db` / `set_current_db` at all —
//! it inherited the trait defaults, which return `Ok(())` and `"default"`.
//! Storage-layer isolation was therefore unreachable in production while
//! being fully green in tests.
//!
//! These tests exercise `FileStorage` directly against a real temp dir.
//!
//! ## A note on flushing
//!
//! `scan` reads only committed rows; recent writes sit in the insert buffer.
//! Seed data must be flushed before asserting, or a backup/export-style
//! read comes back empty and the test "proves" the wrong thing. (Same trap
//! as #4938 and #4995.)

use sqlrustgo_storage::{FileStorage, StorageEngine, Value};
use std::path::Path;

fn make_db(dir: &Path) -> FileStorage {
    FileStorage::new(dir.to_path_buf()).expect("open FileStorage")
}

fn insert_and_flush(storage: &mut FileStorage, table: &str, vals: Vec<Vec<Value>>) {
    storage
        .insert(table, vals)
        .unwrap_or_else(|e| panic!("insert into {table}: {e:?}"));
    // Without this the row is still in the insert buffer and `scan` cannot
    // see it.
    storage.flush().expect("flush");
}

/// Two databases must have independent table namespaces on disk.
#[test]
fn issue_5025_file_storage_tables_are_scoped_per_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = make_db(tmp.path());

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");

    // The same table name in both databases must not clash — that clash
    // ("Table 't1' already exists") is the exact symptom #5025 reported.
    storage.set_current_db("d1").expect("switch to d1");
    storage.create_table(&t1_table()).expect("create t1 in d1");
    insert_and_flush(&mut storage, "t1", vec![vec![Value::Integer(1)]]);

    storage.set_current_db("d2").expect("switch to d2");
    storage
        .create_table(&t1_table())
        .expect("same table name in d2 must not clash with d1's");

    // d2's t1 is a different table: empty.
    let rows = storage.scan("t1").expect("scan d2's t1");
    assert!(
        rows.is_empty(),
        "d2's t1 must not contain d1's row (shared namespace), got {} rows",
        rows.len()
    );

    // d1's t1 still has its row.
    storage.set_current_db("d1").expect("switch back to d1");
    let rows = storage.scan("t1").expect("scan d1's t1");
    assert_eq!(rows.len(), 1, "d1's t1 must still hold exactly its own row");
}

/// `list_tables` must not leak another database's tables.
#[test]
fn issue_5025_file_storage_list_tables_is_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = make_db(tmp.path());

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");

    storage.set_current_db("d1").expect("d1");
    storage
        .create_table(&tbl("only_in_d1"))
        .expect("create d1 table");

    storage.set_current_db("d2").expect("d2");
    storage
        .create_table(&tbl("only_in_d2"))
        .expect("create d2 table");

    storage.set_current_db("d1").expect("back to d1");
    let tables = storage.list_tables();
    assert!(
        tables.iter().any(|t| t == "only_in_d1"),
        "d1 must list its own table, got {tables:?}"
    );
    assert!(
        !tables.iter().any(|t| t == "only_in_d2"),
        "d1 must NOT list d2's table (shared namespace), got {tables:?}"
    );
    // The internal scoping key must never reach the caller. MemoryStorage
    // uses `db\u{1}table`; if FileStorage ever adopts that, the raw key
    // would leak into SHOW TABLES output.
    assert!(
        !tables.iter().any(|t| t.contains('\u{1}')),
        "internal scoping separator leaked into list_tables: {tables:?}"
    );
}

/// Switching to a database that does not exist must fail and must not
/// change the active database.
#[test]
fn issue_5025_file_storage_unknown_database_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = make_db(tmp.path());
    storage.create_database("d1").expect("create d1");

    let before = storage.current_db();
    let r = storage.set_current_db("no_such_db");
    assert!(
        r.is_err(),
        "switching to an unknown database must error, got {:?}",
        r.map(|_| "Ok")
    );
    assert_eq!(
        storage.current_db(),
        before,
        "a rejected switch must leave the active database unchanged"
    );
}

fn t1_table() -> sqlrustgo_storage::TableInfo {
    tbl("t1")
}

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
