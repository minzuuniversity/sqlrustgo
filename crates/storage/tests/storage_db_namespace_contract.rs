//! #5250: the `StorageEngine` database-namespace contract.
//!
//! `create_table_in_db` / `drop_table_in_db` used to have trait default bodies
//! that read `let _ = db;` and then operated on the *current* database. Every
//! caller that named a database got the current one instead, with no error —
//! `CREATE TABLE d1.t` succeeded and put the table somewhere else entirely.
//!
//! The defaults are gone, so the compiler now forces every implementation to
//! say what it does with `db`. That is the compile-time half of the guarantee.
//! The compiler cannot tell whether an implementation *uses* `db`, though — an
//! implementation can still satisfy the signature and ignore the argument. So
//! the same matrix below runs against each storage that supports namespaces.
//!
//! Storages with no namespace at all (`AppendOnlyStorage`,
//! `BinaryTableStorage`, `BinaryTableStorageV2`, `TableLevelStorage`) are
//! covered separately: `default` must work and any other database must be an
//! error. Refusing is the contract — silently relocating the table is not.

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo, Value};

fn table(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                primary_key: true,
                ..Default::default()
            },
            ColumnDefinition {
                name: "v".to_string(),
                data_type: "INTEGER".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "srg_ns_{}_{}_{:?}",
        tag,
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The shared matrix. `storage` must support database namespaces.
fn assert_namespace_contract<S: StorageEngine>(tag: &str, mut s: S) {
    s.create_database("d1").unwrap();
    s.create_database("d2").unwrap();

    // A table created against a named database lands in *that* database.
    s.create_table_in_db("d1", &table("t")).unwrap();
    s.create_table_in_db("d2", &table("t")).unwrap();
    assert!(s.has_table_in("d1", "t"), "[{tag}] d1.t missing");
    assert!(s.has_table_in("d2", "t"), "[{tag}] d2.t missing");
    assert!(
        !s.has_table_in("default", "t"),
        "[{tag}] create_table_in_db leaked the table into `default`"
    );

    // Dropping in one database leaves the other one alone.
    s.drop_table_in_db("d1", "t").unwrap();
    assert!(!s.has_table_in("d1", "t"), "[{tag}] d1.t survived drop");
    assert!(
        s.has_table_in("d2", "t"),
        "[{tag}] dropping d1.t also removed d2.t"
    );

    // `default` is an ordinary database for these storages.
    s.create_table_in_db("default", &table("t")).unwrap();
    assert!(s.has_table_in("default", "t"), "[{tag}] default.t missing");
    s.drop_table_in_db("default", "t").unwrap();
    assert!(
        !s.has_table_in("default", "t"),
        "[{tag}] default.t survived"
    );
}

/// Same-named tables in different databases must stay independent.
///
/// This is the case a namespace fallback hides best: if `create_table_in_db`
/// quietly redirects to the current database, then creating `d1.t` after
/// `default.t` either collides or lands on top of the existing table. Checking
/// only "does the table exist" misses that, so this one also writes rows.
fn assert_same_name_tables_are_independent<S: StorageEngine>(tag: &str, mut s: S) {
    s.create_database("d1").unwrap();

    s.create_table_in_db("default", &table("t")).unwrap();
    s.create_table_in_db("d1", &table("t")).unwrap();

    s.insert_in_db(
        "default",
        "t",
        vec![vec![Value::Integer(1), Value::Integer(10)]],
    )
    .unwrap();
    s.insert_in_db("d1", "t", vec![vec![Value::Integer(2), Value::Integer(20)]])
        .unwrap();

    let d_rows = s.scan_in_db("default", "t").unwrap();
    let n_rows = s.scan_in_db("d1", "t").unwrap();
    assert_eq!(d_rows.len(), 1, "[{tag}] default.t row count");
    assert_eq!(n_rows.len(), 1, "[{tag}] d1.t row count");
    assert_eq!(
        d_rows[0][0],
        Value::Integer(1),
        "[{tag}] default.t holds the wrong row — rows crossed databases"
    );
    assert_eq!(
        n_rows[0][0],
        Value::Integer(2),
        "[{tag}] d1.t holds the wrong row — rows crossed databases"
    );

    // Dropping the named copy must not disturb `default.t`.
    s.drop_table_in_db("d1", "t").unwrap();
    assert!(!s.has_table_in("d1", "t"), "[{tag}] d1.t survived drop");
    assert!(
        s.has_table_in("default", "t"),
        "[{tag}] default.t was dropped"
    );
    let d_rows = s.scan_in_db("default", "t").unwrap();
    assert_eq!(
        d_rows.len(),
        1,
        "[{tag}] dropping d1.t changed default.t's contents"
    );
    assert_eq!(d_rows[0][0], Value::Integer(1));
}

#[test]
fn memory_storage_honours_stated_database() {
    use sqlrustgo_storage::engine::MemoryStorage;
    assert_namespace_contract("memory", MemoryStorage::new());
    assert_same_name_tables_are_independent("memory", MemoryStorage::new());
}

#[test]
fn file_storage_honours_stated_database() {
    use sqlrustgo_storage::file_storage::FileStorage;
    let dir = temp_dir("file");
    let s = FileStorage::new(dir.clone()).unwrap();
    assert_namespace_contract("file", s);
    assert_same_name_tables_are_independent(
        "file",
        FileStorage::new(
            dir.with_file_name(dir.file_name().unwrap().to_string_lossy().to_string() + "_2"),
        )
        .unwrap(),
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mvcc_storage_delegates_stated_database() {
    use sqlrustgo_storage::engine::MemoryStorage;
    use sqlrustgo_storage::mvcc_storage::MvccStorage;
    assert_namespace_contract("mvcc", MvccStorage::new(MemoryStorage::new()));
    assert_same_name_tables_are_independent("mvcc", MvccStorage::new(MemoryStorage::new()));
}

#[test]
fn wal_storage_delegates_stated_database() {
    use sqlrustgo_storage::{MemoryStorage, MemoryWalManager, WalStorage};
    assert_namespace_contract(
        "wal",
        WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).unwrap(),
    );
    assert_same_name_tables_are_independent(
        "wal",
        WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).unwrap(),
    );
}

/// Storages with a single flat namespace must refuse a database they cannot
/// honour, and must still work for the one namespace they actually have.
fn assert_flat_storage_refuses<S: StorageEngine>(tag: &str, mut s: S) {
    s.create_table_in_db("default", &table("t")).unwrap();
    assert!(
        s.has_table_in("default", "t"),
        "[{tag}] the storage should still support `default`"
    );

    let err = s.create_table_in_db("d1", &table("t")).expect_err(&format!(
        "[{tag}] create_table_in_db(\"d1\") must not succeed silently"
    ));
    assert!(
        !err.to_string().is_empty(),
        "[{tag}] the error must name the problem"
    );
    assert!(
        !s.has_table_in("d1", "t"),
        "[{tag}] a refused create must not leave a table behind"
    );
    assert!(
        s.has_table_in("default", "t"),
        "[{tag}] a refused create must not disturb `default`"
    );

    let _ = s.drop_table_in_db("d1", "t");
    assert!(
        s.has_table_in("default", "t"),
        "[{tag}] refusing a drop must not remove the default table"
    );
}

#[test]
fn binary_table_storage_refuses_foreign_database() {
    use sqlrustgo_storage::binary_storage::BinaryTableStorage;
    let dir = temp_dir("binflat");
    let s = BinaryTableStorage::new(dir.clone()).unwrap();
    assert_flat_storage_refuses("binary_table", s);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn binary_table_storage_v2_refuses_foreign_database() {
    use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
    let dir = temp_dir("binv2");
    let s = BinaryTableStorageV2::new(dir.clone()).unwrap();
    assert_flat_storage_refuses("binary_v2", s);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn append_only_storage_refuses_foreign_database() {
    use sqlrustgo_storage::append_only_storage::AppendOnlyStorage;
    let dir = temp_dir("append");
    let s = AppendOnlyStorage::new(dir.clone()).unwrap();
    assert_flat_storage_refuses("append_only", s);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn table_level_storage_refuses_foreign_database() {
    use sqlrustgo_storage::table_level_storage::TableLevelStorage;
    use sqlrustgo_storage::MemoryWalManager;

    // `TableLevelStorage::create_table` is not wired up yet — it answers
    // "create_table not yet integrated". That is pre-existing and unrelated to
    // the namespace contract, so this test only pins what *is* specified:
    // the namespace-aware methods must not silently answer about `default`.
    let mut s: TableLevelStorage = TableLevelStorage::new(Box::new(MemoryWalManager::new()));

    // Asking about a foreign database must be `false` even though `t` exists
    // in the only namespace this storage has.
    assert!(
        s.create_table_in_db("d1", &table("t")).is_err(),
        "[table_level] a foreign database must be refused"
    );
    assert!(
        s.drop_table_in_db("d1", "t").is_err(),
        "[table_level] dropping in a foreign database must be refused"
    );
    assert!(
        !s.has_table_in("d1", "t"),
        "[table_level] must not answer a foreign-database query from `default`"
    );
}
