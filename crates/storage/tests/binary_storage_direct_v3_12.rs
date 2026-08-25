//! V312-12 coverage improvement tests for `sqlrustgo_storage::BinaryTableStorage`
//! direct public API (Issue #4431 followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Target: improve `crates/storage/src/binary_storage.rs` coverage
//! (currently ~55% lines, 45% functions) by exercising its many public
//! utility methods (save, load, persist_table, new, etc.) and the
//! StorageEngine trait impl that uses them.

use sqlrustgo_storage::binary_storage::BinaryTableStorage;
use sqlrustgo_storage::engine::{ColumnDefinition, TableData, TableInfo};
use sqlrustgo_storage::{Record, StorageEngine, Value as SqlValue};
use tempfile::TempDir;

fn make_info() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "a".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
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
    }
}

fn make_data() -> TableData {
    TableData {
        info: make_info(),
        rows: vec![],
    }
}

fn make_dir() -> (TempDir, BinaryTableStorage) {
    let temp_dir = TempDir::new().unwrap();
    let storage = BinaryTableStorage::new(temp_dir.path().to_path_buf()).unwrap();
    (temp_dir, storage)
}

// --------------------------------------------------------------------------
// Construction
// --------------------------------------------------------------------------

#[test]
fn cov_binary_storage_new() {
    let (_t, _s) = make_dir();
}

#[test]
fn cov_binary_storage_new_with_data() {
    let temp_dir = TempDir::new().unwrap();
    let _s = BinaryTableStorage::new_with_data(temp_dir.path().to_path_buf()).unwrap();
}

// --------------------------------------------------------------------------
// Exists
// --------------------------------------------------------------------------

#[test]
fn cov_binary_storage_exists_false() {
    let (_t, mut s) = make_dir();
    assert!(!s.exists("t"));
}

#[test]
fn cov_binary_storage_exists_true_after_save() {
    let (temp_dir, s) = make_dir();
    s.save("t", &make_data()).unwrap();
    assert!(s.exists("t"));
    // Verify on disk
    let path = temp_dir.path().join("t.bin");
    assert!(path.exists());
}

#[test]
fn cov_binary_storage_exists_multiple() {
    let (_t, mut s) = make_dir();
    s.save("a", &make_data()).unwrap();
    s.save("b", &make_data()).unwrap();
    s.save("c", &make_data()).unwrap();
    assert!(s.exists("a"));
    assert!(s.exists("b"));
    assert!(s.exists("c"));
}

// --------------------------------------------------------------------------
// Save / Load roundtrip
// --------------------------------------------------------------------------

#[test]
fn cov_binary_storage_save_load_empty() {
    let (_t, mut s) = make_dir();
    s.save("t", &make_data()).unwrap();
    let loaded = s.load("t").unwrap();
    assert_eq!(loaded.info.name, "t");
    assert!(loaded.rows.is_empty());
}

#[test]
fn cov_binary_storage_save_load_with_rows() {
    let (_t, mut s) = make_dir();
    let mut data = make_data();
    data.rows.push(vec![SqlValue::Integer(1)]);
    data.rows.push(vec![SqlValue::Integer(2)]);
    data.rows.push(vec![SqlValue::Integer(3)]);
    s.save("t", &data).unwrap();
    let loaded = s.load("t").unwrap();
    assert_eq!(loaded.rows.len(), 3);
}

#[test]
fn cov_binary_storage_save_load_text() {
    let (temp_dir, s) = make_dir();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "s".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            primary_key: false,
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
    };
    let mut data = TableData {
        info: info,
        rows: vec![],
    };
    data.rows.push(vec![SqlValue::Text("hello".to_string())]);
    s.save("t", &data).unwrap();
    let loaded = s.load("t").unwrap();
    assert_eq!(loaded.rows.len(), 1);
    let _ = temp_dir;
}

#[test]
fn cov_binary_storage_save_load_float() {
    let (_t, mut s) = make_dir();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "v".to_string(),
            data_type: "FLOAT".to_string(),
            nullable: false,
            primary_key: false,
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
    };
    let mut data = TableData {
        info: info,
        rows: vec![],
    };
    data.rows.push(vec![SqlValue::Float(1.5)]);
    s.save("t", &data).unwrap();
    let _ = s.load("t").unwrap();
}

#[test]
fn cov_binary_storage_save_load_bool() {
    let (_t, mut s) = make_dir();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "b".to_string(),
            data_type: "BOOLEAN".to_string(),
            nullable: false,
            primary_key: false,
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
    };
    let mut data = TableData {
        info: info,
        rows: vec![],
    };
    data.rows.push(vec![SqlValue::Boolean(true)]);
    s.save("t", &data).unwrap();
    let _ = s.load("t").unwrap();
}

#[test]
fn cov_binary_storage_save_load_null() {
    let (_t, mut s) = make_dir();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "a".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: true,
            primary_key: false,
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
    };
    let mut data = TableData {
        info: info,
        rows: vec![],
    };
    data.rows.push(vec![SqlValue::Null]);
    s.save("t", &data).unwrap();
    let _ = s.load("t").unwrap();
}

#[test]
fn cov_binary_storage_save_load_multi_column() {
    let (_t, mut s) = make_dir();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "a".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "b".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
    };
    let mut data = TableData {
        info: info,
        rows: vec![],
    };
    data.rows.push(vec![
        SqlValue::Integer(1),
        SqlValue::Text("foo".to_string()),
    ]);
    s.save("t", &data).unwrap();
    let _ = s.load("t").unwrap();
}

#[test]
fn cov_binary_storage_save_overwrite() {
    let (_t, mut s) = make_dir();
    s.save("t", &make_data()).unwrap();
    s.save("t", &make_data()).unwrap();
}

#[test]
fn cov_binary_storage_save_load_roundtrip_many() {
    let (_t, mut s) = make_dir();
    let mut data = make_data();
    for i in 0..100i64 {
        data.rows.push(vec![SqlValue::Integer(i)]);
    }
    s.save("t", &data).unwrap();
    let loaded = s.load("t").unwrap();
    assert_eq!(loaded.rows.len(), 100);
}

// --------------------------------------------------------------------------
// persist_table
// --------------------------------------------------------------------------

#[test]
fn cov_binary_storage_persist_table_missing() {
    let (_t, mut s) = make_dir();
    s.persist_table("nonexistent").unwrap(); // no-op for missing
}

#[test]
fn cov_binary_storage_persist_table_existing() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.persist_table("t").unwrap();
}

#[test]
fn cov_binary_storage_persist_table_after_insert() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    s.persist_table("t").unwrap();
}

// --------------------------------------------------------------------------
// StorageEngine trait
// --------------------------------------------------------------------------

#[test]
fn cov_binary_storage_engine_create_table() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
}

#[test]
fn cov_binary_storage_engine_insert_and_scan() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_binary_storage_engine_insert_many() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    for i in 0..50i64 {
        s.insert("t", vec![vec![SqlValue::Integer(i)]]).unwrap();
    }
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 50);
}

#[test]
fn cov_binary_storage_engine_drop_table() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.drop_table("t").unwrap();
}

#[test]
fn cov_binary_storage_engine_flush() {
    let (_t, mut s) = make_dir();
    s.flush().unwrap();
}

#[test]
fn cov_binary_storage_engine_has_table() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    assert!(s.has_table("t"));
    assert!(!s.has_table("missing"));
}

#[test]
fn cov_binary_storage_engine_update() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let _ = s.update("t", &[], &[(0, SqlValue::Integer(99))]);
}

#[test]
fn cov_binary_storage_engine_delete() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let _ = s.delete("t", &[]);
}

#[test]
fn cov_binary_storage_engine_scan_empty() {
    let (_t, mut s) = make_dir();
    let rows = s.scan("t").unwrap();
    assert!(rows.is_empty());
}

#[test]
fn cov_binary_storage_engine_scan_after_create() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    let _ = s.scan("t").unwrap();
}

#[test]
fn cov_binary_storage_engine_in_tx_false() {
    let (_t, mut s) = make_dir();
    assert!(!s.in_transaction());
}

#[test]
fn cov_binary_storage_engine_persist_inserted() {
    let (_t, mut s) = make_dir();
    s.create_table(&make_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let _ = s.persist_table("t");
}
