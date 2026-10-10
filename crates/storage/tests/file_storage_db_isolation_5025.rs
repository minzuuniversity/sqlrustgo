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

use sqlrustgo_storage::{FileStorage, SessionContext, StorageEngine};
use sqlrustgo_types::Value;
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

    let ctx1 = SessionContext::new("d1", 0);
    let ctx2 = SessionContext::new("d2", 0);

    // The same table name in both databases must not clash.
    storage
        .create_table_in_db(&ctx1, &t1_table())
        .expect("create t1 in d1");
    storage
        .insert_in_db(&ctx1, "t1", vec![vec![Value::Integer(1)]])
        .expect("insert into d1");
    storage.flush().expect("flush");
    storage
        .create_table_in_db(&ctx2, &t1_table())
        .expect("same table name in d2 must not clash with d1's");

    // d2's t1 is a different table: empty.
    let rows = storage.scan_in_db(&ctx2, "t1").expect("scan d2's t1");
    assert!(
        rows.is_empty(),
        "d2's t1 must not contain d1's row (shared namespace), got {} rows",
        rows.len()
    );

    // d1's t1 still has its row.
    let rows = storage.scan_in_db(&ctx1, "t1").expect("scan d1's t1");
    assert_eq!(rows.len(), 1, "d1's t1 must still hold exactly its own row");
}

/// `list_tables` must not leak another database's tables.
#[test]
fn issue_5025_file_storage_list_tables_is_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = make_db(tmp.path());

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    let ctx1 = SessionContext::new("d1", 0);
    let ctx2 = SessionContext::new("d2", 0);
    storage
        .create_table_in_db(&ctx1, &tbl("only_in_d1"))
        .expect("create d1 table");
    storage
        .create_table_in_db(&ctx2, &tbl("only_in_d2"))
        .expect("create d2 table");
    let mut tables = storage.list_tables_in_db(&ctx1);
    tables.sort();
    assert_eq!(
        tables,
        vec!["only_in_d1".to_string()],
        "d1 must list only its own table"
    );
    let mut tables = storage.list_tables_in_db(&ctx2);
    tables.sort();
    assert_eq!(
        tables,
        vec!["only_in_d2".to_string()],
        "d2 must list only its own table"
    );
}

/// Switching to a database that does not exist must fail and must not
/// change the active database.

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

/// `list_tables_in_db` must enumerate a stated database's tables without
/// flipping the engine-wide `current_db` — the read lock held by
/// `SHOW TABLES FROM` cannot do that (`set_current_db` needs `&mut`).
#[test]
fn issue_5025_list_tables_in_db_returns_target_database_tables() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = make_db(tmp.path());

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    let ctx1 = SessionContext::new("d1", 0);
    let ctx2 = SessionContext::new("d2", 0);
    storage
        .create_table_in_db(&ctx1, &tbl("t1"))
        .expect("create t1 in d1");
    storage
        .create_table_in_db(&ctx2, &tbl("t1"))
        .expect("create t1 in d2");
    storage
        .create_table_in_db(&ctx2, &tbl("t2"))
        .expect("create t2 in d2");

    // current_db stays d2 throughout — the answer must not depend on it.
    let mut d1_tables = storage.list_tables_in_db(&SessionContext::new("d1", 0));
    d1_tables.sort();
    assert_eq!(
        d1_tables,
        vec!["t1".to_string()],
        "list_tables_in_db(d1) must list only d1's tables while current_db=d2"
    );

    let mut d2_tables = storage.list_tables_in_db(&SessionContext::new("d2", 0));
    d2_tables.sort();
    assert_eq!(
        d2_tables,
        vec!["t1".to_string(), "t2".to_string()],
        "list_tables_in_db(d2) must list d2's tables"
    );
}

/// While `current_db` is the implicit default, a named database's tables
/// must not leak into `list_tables()`. The `db_dir() == None` branch in
/// `table_names()` lists every cache entry unfiltered, but every cache
/// key is scoped (`tbl()` always applies `scoped_key`) — so the unscoped
/// "list everything" shortcut reports named databases' tables under
/// `default`.
#[test]
fn issue_5025_file_storage_default_database_does_not_leak_named_database_tables() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut storage = make_db(tmp.path());

    storage.create_database("d1").expect("create d1");
    let ctx1 = SessionContext::new("d1", 0);
    storage
        .create_table_in_db(&ctx1, &tbl("only_in_d1"))
        .expect("create table in d1");
    let mut tables = storage.list_tables_in_db(&SessionContext::new("default", 0));
    tables.sort();
    assert_eq!(
        tables,
        Vec::<String>::new(),
        "default's list_tables must NOT leak d1's table"
    );
}
