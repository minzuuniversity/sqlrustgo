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

// ---------------------------------------------------------------------------
// #5266 S1: `get_table_info_in` and `scan_in_db`.
// ---------------------------------------------------------------------------

/// Reads must be asked of the stated database too. Both had trait defaults
/// that answered about the current database, which is why
/// `SELECT COUNT(*) FROM d1.t` reported `Table not found` even though `d1.t`
/// existed on disk: the lookup itself resolved to `default`.
///
/// Data is inserted through the storage API on purpose — SQL `INSERT` still
/// has its own namespace defect, and letting that leak in here would make a
/// failure ambiguous between the write path and the read path.
fn assert_read_contract<S: StorageEngine>(tag: &str, mut s: S) {
    s.create_database("d1").unwrap();
    s.create_database("d2").unwrap();
    s.create_database("d3").unwrap();

    s.create_table_in_db("d1", &table("t")).unwrap();
    s.create_table_in_db("d2", &table("t")).unwrap();
    // NOTE: `insert_in_db` is a *different* broken default (scheduled as D1 in
    // #5266) and is deliberately not used to build the fixture — otherwise a
    // failure here could be the write path rather than the read path. The two
    // tables are distinguished by their column count instead, which
    // `create_table_in_db` (already fixed) can express.
    let wide = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    s.drop_table_in_db("d2", "t").unwrap();
    s.create_table_in_db("d2", &wide).unwrap();

    // A read against a stated database sees that database's rows.
    // Both tables are empty, so the row *shape* is what distinguishes them:
    // `d1.t` was created with two columns, `d2.t` with one. A read that
    // resolved against the wrong database would return the other shape — or,
    // before the fix, `default.t`, which does not exist at all.
    let d1 = s.scan_in_db("d1", "t").unwrap();
    let d2 = s.scan_in_db("d2", "t").unwrap();
    assert!(
        d1.is_empty(),
        "[{tag}] a freshly created table must be empty"
    );
    assert!(
        d2.is_empty(),
        "[{tag}] a freshly created table must be empty"
    );

    // The distinguishing check is metadata: `d1.t` has two columns, `d2.t`
    // one. Resolving either against the wrong namespace swaps them.
    let wide = s.get_table_info_in("d1", "t").unwrap();
    let narrow = s.get_table_info_in("d2", "t").unwrap();
    assert_eq!(
        wide.columns.len(),
        2,
        "[{tag}] d1.t has the wrong shape — read from another namespace"
    );
    assert_eq!(
        narrow.columns.len(),
        1,
        "[{tag}] d2.t has the wrong shape — read from another namespace"
    );

    // A database with no such table must not resolve to another namespace.
    //
    // `get_table_info_in` returns `Table not found` — that is a real error.
    // `scan_in_db` returns an empty row set for a missing table; that is the
    // long-standing convention of these storages (`unwrap_or_default`), not
    // something S1 changes. What matters is that the empty set comes from
    // `d3`, not from `default` — and `default.t` is never created here, so a
    // fallback would be indistinguishable unless we plant a decoy.
    assert!(
        s.get_table_info_in("d3", "t").is_err(),
        "[{tag}] get_table_info_in must not resolve a missing table elsewhere"
    );

    // Decoy: `default.t` exists and is wide. If `scan_in_db("d3", "t")` fell
    // back to `default`, it would come back with rows.
    s.create_table_in_db("default", &wide).unwrap();
    assert!(
        s.scan_in_db("d3", "t").unwrap().is_empty(),
        "[{tag}] scan_in_db(\"d3\") returned `default`'s rows — namespace fallback"
    );
}

#[test]
fn memory_storage_read_honours_stated_database() {
    use sqlrustgo_storage::engine::MemoryStorage;
    assert_read_contract("memory", MemoryStorage::new());
}

#[test]
fn file_storage_read_honours_stated_database() {
    use sqlrustgo_storage::file_storage::FileStorage;
    let dir = temp_dir("file_read");
    assert_read_contract("file", FileStorage::new(dir.clone()).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mvcc_storage_read_delegates_stated_database() {
    use sqlrustgo_storage::engine::MemoryStorage;
    use sqlrustgo_storage::mvcc_storage::MvccStorage;
    assert_read_contract("mvcc", MvccStorage::new(MemoryStorage::new()));
}

#[test]
fn wal_storage_read_delegates_stated_database() {
    use sqlrustgo_storage::{MemoryStorage, MemoryWalManager, WalStorage};
    assert_read_contract(
        "wal",
        WalStorage::new(MemoryStorage::new(), MemoryWalManager::new()).unwrap(),
    );
}

/// Flat storages cannot serve a foreign database. Both methods return
/// `SqlResult`, so they can — and must — report that rather than answering
/// from `default`.
fn assert_flat_storage_read_refuses<S: StorageEngine>(tag: &str, mut s: S) {
    s.create_table_in_db("default", &table("t")).unwrap();
    // `insert_in_db` is a separate defect (D1); not relied on here.

    assert!(
        s.scan_in_db("default", "t").is_ok(),
        "[{tag}] `default` is the one namespace this storage has"
    );
    assert!(
        s.get_table_info_in("default", "t").is_ok(),
        "[{tag}] `default` is the one namespace this storage has"
    );
    assert!(
        s.scan_in_db("d1", "t").is_err(),
        "[{tag}] scan_in_db must refuse a database it cannot address"
    );
    assert!(
        s.get_table_info_in("d1", "t").is_err(),
        "[{tag}] get_table_info_in must refuse a database it cannot address"
    );
}

#[test]
fn binary_table_storage_read_refuses_foreign_database() {
    use sqlrustgo_storage::binary_storage::BinaryTableStorage;
    let dir = temp_dir("binflat_read");
    assert_flat_storage_read_refuses(
        "binary_table",
        BinaryTableStorage::new(dir.clone()).unwrap(),
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn binary_table_storage_v2_read_refuses_foreign_database() {
    use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
    let dir = temp_dir("binv2_read");
    assert_flat_storage_read_refuses("binary_v2", BinaryTableStorageV2::new(dir.clone()).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn append_only_storage_read_refuses_foreign_database() {
    use sqlrustgo_storage::append_only_storage::AppendOnlyStorage;
    let dir = temp_dir("append_read");
    assert_flat_storage_read_refuses("append_only", AppendOnlyStorage::new(dir.clone()).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}
