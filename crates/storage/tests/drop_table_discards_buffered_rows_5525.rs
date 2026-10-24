//! #5525 — dropping a table must drop the rows that are still buffered.
//!
//! `insert_buffer` is a second copy of table state (see
//! `WriteState::insert_buffer` in `file_storage.rs`). `FileStorage::drop_table_in`
//! used to remove only the `tables` entry, so the buffered rows of a dropped
//! table survived and came back the moment a table of the same name was
//! created again.
//!
//! The logical backup/restore hit this directly: `restore --drop-first`
//! landed twice as many rows as the backup held, because the first
//! restore's buffered rows were still in the buffer when the table was
//! dropped and recreated.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};
use tempfile::TempDir;

fn table_info(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// A dropped table's buffered rows must not come back.
#[test]
fn test_drop_table_discards_the_buffered_rows() {
    let dir = TempDir::new().unwrap();
    let mut storage = FileStorage::new(dir.path().to_path_buf()).unwrap();

    storage
        .create_table_in_db("default", &table_info("t"))
        .unwrap();
    storage
        .insert_in_db("default", "t", vec![vec![Value::Integer(1)]])
        .unwrap();

    // The row is visible while the table exists.
    assert_eq!(storage.scan_in_db("default", "t").unwrap().len(), 1);

    storage.drop_table_in_db("default", "t").unwrap();
    assert!(!storage.has_table_in("default", "t"));

    // Recreating the table must give an **empty** table, not the dropped
    // table's rows.
    storage
        .create_table_in_db("default", &table_info("t"))
        .unwrap();
    assert_eq!(
        storage.scan_in_db("default", "t").unwrap().len(),
        0,
        "the dropped table's buffered rows came back"
    );

    storage
        .insert_in_db("default", "t", vec![vec![Value::Integer(2)]])
        .unwrap();
    let rows = storage.scan_in_db("default", "t").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Integer(2));
}

/// The same, in a named database.
#[test]
fn test_drop_table_in_db_discards_the_buffered_rows() {
    let dir = TempDir::new().unwrap();
    let mut storage = FileStorage::new(dir.path().to_path_buf()).unwrap();

    storage.create_table_in_db("d1", &table_info("t")).unwrap();
    storage
        .insert_in_db("d1", "t", vec![vec![Value::Integer(1)]])
        .unwrap();

    storage.drop_table_in_db("d1", "t").unwrap();
    storage.create_table_in_db("d1", &table_info("t")).unwrap();

    assert_eq!(
        storage.scan_in_db("d1", "t").unwrap().len(),
        0,
        "the dropped table's buffered rows came back"
    );
}

/// Dropping one table must not disturb another table's buffer.
#[test]
fn test_drop_table_keeps_the_other_tables_buffer() {
    let dir = TempDir::new().unwrap();
    let mut storage = FileStorage::new(dir.path().to_path_buf()).unwrap();

    storage
        .create_table_in_db("default", &table_info("a"))
        .unwrap();
    storage
        .create_table_in_db("default", &table_info("b"))
        .unwrap();
    storage
        .insert_in_db("default", "a", vec![vec![Value::Integer(1)]])
        .unwrap();
    storage
        .insert_in_db("default", "b", vec![vec![Value::Integer(2)]])
        .unwrap();

    storage.drop_table_in_db("default", "a").unwrap();

    assert_eq!(storage.scan_in_db("default", "b").unwrap().len(), 1);
}
