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

use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, StorageEngine, TableData, TableInfo, Value,
};
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
    }
}

fn fresh_data_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("5025_restart_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    data
}

/// Create `db`, write one row and one index into `table`, then flush.
fn write_one(data: &Path, db: &str, table: &str, id: i64) {
    let mut fs = FileStorage::new(data.to_path_buf()).unwrap();
    if db != "default" {
        fs.create_database(db).unwrap();
    }
    fs.set_current_db(db).unwrap();
    fs.insert_table(
        table.into(),
        TableData {
            info: tbl(table),
            rows: vec![],
        },
    )
    .unwrap();
    fs.insert(table, vec![vec![Value::Integer(id)]]).unwrap();
    fs.create_index(table, "id", 0).unwrap();
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

    let mut fs = FileStorage::new(data.clone()).unwrap();
    fs.set_current_db("shop").unwrap();
    assert!(
        fs.get_table("t").is_some(),
        "table must be back in the cache after reopening"
    );
    let rows = fs.scan("t").unwrap();
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

    let mut fs = FileStorage::new(data.clone()).unwrap();
    fs.set_current_db("shop").unwrap();
    assert!(
        fs.has_index("t", "id"),
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

    let mut fs = FileStorage::new(data.clone()).unwrap();
    fs.set_current_db("d1").unwrap();
    let a = fs.scan("shared").unwrap();
    fs.set_current_db("d2").unwrap();
    let b = fs.scan("shared").unwrap();

    assert_eq!(a.len(), 1, "d1 must see exactly its own row, got {a:?}");
    assert_eq!(b.len(), 1, "d2 must see exactly its own row, got {b:?}");
    assert_eq!(a[0][0], Value::Integer(11));
    assert_eq!(b[0][0], Value::Integer(22));
    assert!(fs.has_index("shared", "id"), "d2's index must load too");
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
