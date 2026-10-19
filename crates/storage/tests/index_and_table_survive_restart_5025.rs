//! #5025: a named database's tables and indexes must survive a restart.
//!
//! #5025 moved each database's files into `data/<db>/` and keyed the
//! in-memory caches by *scoped* name. It updated the write side and the
//! read helpers, but left the two startup loaders reading only `data_dir`
//! and — for indexes — filing results under the *bare* `(table, column)`
//! tuple that `has_index`/`get_index` can no longer look up. Three
//! separate failures fell out of that one omission:
//!
//!   1. a table created in a named database was written to
//!      `data/<db>/t.json` and then absent from the cache on the next
//!      open, so `scan` reported zero rows against a file on disk;
//!   2. the same for that database's index files;
//!   3. even the *default* database's indexes never loaded, because the
//!      bare key they were stored under could not match the scoped key
//!      `has_index` asks for.
//!
//! (3) is what the pre-existing `test_e2e_index_survives_restart` was
//! failing on; (1) and (2) had no coverage at all.
//!
//! #5375: `current_db` / `set_current_db` were removed from the engine in
//! #5057 step 3. The database-less helpers (`insert_table`, `insert`,
//! `create_index`, `has_index`, `get_table`, `scan`) all resolve against
//! `DEFAULT_DATABASE` now, so the database is named at the call site:
//! `create_table_in_db` / `insert_in_db` for the write side, and
//! `get_table_info_in` / `scan_in_db` / `scan_with_index_in_db` for the
//! read side.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};
use std::path::{Path, PathBuf};

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

fn fresh_data_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("5025_restart_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    data
}

/// Create the table in `db`, write one row into it, and flush.
///
/// `create_table_in_db` also pre-creates the primary-key index under the
/// named database's own scoped key and writes it into that database's
/// directory — which is what the restart tests below read back.
fn write_one(data: &Path, db: &str, table: &str, id: i64) {
    let mut fs = FileStorage::new(data.to_path_buf()).unwrap();
    if db != "default" {
        fs.create_database(db).unwrap();
    }
    fs.create_table_in_db(db, &tbl(table)).unwrap();
    fs.insert_in_db(db, table, vec![vec![Value::Integer(id)]])
        .unwrap();
    fs.flush().unwrap();
}

#[test]
fn table_in_a_named_database_survives_restart() {
    let data = fresh_data_dir("tbl");
    write_one(&data, "shop", "t", 1);

    // The row really is on disk, under the database's own directory —
    // otherwise this test could pass for the wrong reason.
    assert!(
        data.join("shop").join("t.json").exists(),
        "precondition: shop/t.json must exist on disk"
    );

    let fs = FileStorage::new(data.clone()).unwrap();
    assert!(
        fs.get_table_info_in("shop", "t").is_ok(),
        "table must be back in the cache after reopening"
    );
    let rows = fs.scan_in_db("shop", "t").unwrap();
    assert_eq!(
        rows.len(),
        1,
        "the committed row must be visible, got {rows:?}"
    );
    assert_eq!(rows[0][0], Value::Integer(1));
}

#[test]
fn index_in_a_named_database_survives_restart() {
    let data = fresh_data_dir("idx");
    write_one(&data, "shop", "t", 1);
    assert!(data.join("shop").join("t_idx_id.json").exists());

    let fs = FileStorage::new(data.clone()).unwrap();
    // The index must come back under the *named* database's scoped key.
    // `scan_with_index_in_db` is the database-scoped form of the lookup
    // (`has_index` is database-less and therefore only ever sees
    // `default`), so it is what proves the scoped key was used.
    assert!(
        fs.scan_with_index_in_db("shop", "t", "id", &Value::Integer(1))
            .is_ok(),
        "the index must load back under the scoped key"
    );
}

#[test]
fn index_in_the_default_database_survives_restart() {
    let data = fresh_data_dir("defidx");
    write_one(&data, "default", "t", 1);

    let fs = FileStorage::new(data.clone()).unwrap();
    assert!(
        fs.has_index("t", "id"),
        "default-database indexes used to be filed under a bare key that has_index could not match"
    );
}

#[test]
fn same_table_name_in_two_databases_stays_separate_across_restart() {
    let data = fresh_data_dir("iso");
    write_one(&data, "d1", "shared", 11);
    write_one(&data, "d2", "shared", 22);

    let fs = FileStorage::new(data.clone()).unwrap();
    let a = fs.scan_in_db("d1", "shared").unwrap();
    let b = fs.scan_in_db("d2", "shared").unwrap();

    assert_eq!(a.len(), 1, "d1 must see exactly its own row, got {a:?}");
    assert_eq!(b.len(), 1, "d2 must see exactly its own row, got {b:?}");
    assert_eq!(a[0][0], Value::Integer(11));
    assert_eq!(b[0][0], Value::Integer(22));
    // Both databases' indexes must load back under their own scoped keys.
    assert!(
        fs.scan_with_index_in_db("d1", "shared", "id", &Value::Integer(11))
            .is_ok(),
        "d1's index must load back under its own scoped key"
    );
    assert!(
        fs.scan_with_index_in_db("d2", "shared", "id", &Value::Integer(22))
            .is_ok(),
        "d2's index must load back under its own scoped key"
    );
}

/// NOTE: this test does **not** pin the `_idx_` skip in `load_all_tables`.
/// Mutation M3 deleted that skip and all five tests still passed — a
/// serialised `BPlusTree` already fails to parse as a `StoredTableData`,
/// so the `if let Ok(..)` below the skip swallows it either way. The skip
/// is kept because it states the intent outright instead of relying on a
/// serde mismatch, but it is *not* covered evidence for the restart fix.
/// The tests that do pin that fix are the four above.
#[test]
fn index_files_are_not_mistaken_for_tables() {
    // `t_idx_id.json` sits in the same directory as `t.json`. The table
    // loader has to skip it, or it would try to parse a serialised
    // BPlusTree as a StoredTableData and — worse — cache a bogus "table".
    let data = fresh_data_dir("notbl");
    write_one(&data, "default", "t", 1);

    let fs = FileStorage::new(data.clone()).unwrap();
    let names = fs.list_tables();
    assert!(
        !names.iter().any(|n| n.contains("_idx_")),
        "index files must not surface as tables, got {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "t"),
        "the real table must still be listed, got {names:?}"
    );
}
