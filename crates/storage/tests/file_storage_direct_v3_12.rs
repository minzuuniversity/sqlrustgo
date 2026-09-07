//! V312-12 coverage improvement tests for `sqlrustgo_storage::FileStorage`
//! direct public API (Issue #4419 followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Target: improve `crates/storage/src/file_storage.rs` coverage (currently
//! 68.5% lines, 55.98% functions) by exercising its many public utility
//! methods (table management, index management, persistence, etc.).

use sqlrustgo_storage::engine::ColumnDefinition;
use sqlrustgo_storage::engine::TableInfo;
use sqlrustgo_storage::{FileStorage, StorageEngine, Value as SqlValue};
use tempfile::TempDir;

fn make_storage() -> (TempDir, FileStorage) {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileStorage::new(temp_dir.path().to_path_buf()).unwrap();
    (temp_dir, storage)
}

fn table_info() -> TableInfo {
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
        original_sql: String::new(),
    }
}

fn table_info_multi() -> TableInfo {
    TableInfo {
        name: "wide".to_string(),
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
            ColumnDefinition {
                name: "c".to_string(),
                data_type: "DECIMAL".to_string(),
                nullable: false,
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
        original_sql: String::new(),
    }
}

// --------------------------------------------------------------------------
// Construction
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_new() {
    let (_t, _s) = make_storage();
}

#[test]
fn cov_file_storage_new_with_buffer_config() {
    let temp_dir = TempDir::new().unwrap();
    let _s =
        FileStorage::new_with_buffer_config(temp_dir.path().to_path_buf(), 1024, true).unwrap();
}

// --------------------------------------------------------------------------
// Table management
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_create_table() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
}

#[test]
fn cov_file_storage_create_table_multi_columns() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info_multi()).unwrap();
}

#[test]
fn cov_file_storage_table_names_empty() {
    let (_t, mut s) = make_storage();
    assert!(s.table_names().is_empty());
}

#[test]
fn cov_file_storage_table_names_after_create() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    let names = s.table_names();
    assert!(names.contains(&"t".to_string()));
}

#[test]
fn cov_file_storage_contains_table() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    assert!(s.contains_table("t"));
    assert!(!s.contains_table("missing"));
}

#[test]
fn cov_file_storage_get_table() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    let _t_data = s.get_table("t");
}

#[test]
fn cov_file_storage_get_table_missing() {
    let (_t, mut s) = make_storage();
    let _t_data = s.get_table("missing");
}

#[test]
fn cov_file_storage_get_table_mut() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    let _t_data = s.get_table_mut("t");
}

#[test]
fn cov_file_storage_drop_table() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.drop_table("t").unwrap();
    assert!(!s.contains_table("t"));
}

#[test]
fn cov_file_storage_insert_table() {
    let (_t, mut s) = make_storage();
    let t_data = sqlrustgo_storage::engine::TableData {
        info: table_info(),
        rows: vec![],
    };
    s.insert_table("t".to_string(), t_data).unwrap();
    assert!(s.contains_table("t"));
}

#[test]
fn cov_file_storage_clear_all_tables() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.clear_all_tables();
}

// --------------------------------------------------------------------------
// Database management
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_create_database() {
    let (_t, mut s) = make_storage();
    s.create_database("testdb").unwrap();
}

#[test]
fn cov_file_storage_drop_database() {
    let (_t, mut s) = make_storage();
    s.create_database("testdb").unwrap();
    s.drop_database("testdb").unwrap();
}

#[test]
fn cov_file_storage_flush() {
    let (_t, mut s) = make_storage();
    s.flush().unwrap();
}

#[test]
fn cov_file_storage_persist_table() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.persist_table("t").unwrap();
}

// --------------------------------------------------------------------------
// StorageEngine trait methods
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_scan_empty() {
    let (_t, mut s) = make_storage();
    let rows = s.scan("t").unwrap();
    assert!(rows.is_empty());
}

#[test]
fn cov_file_storage_scan_after_create() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    let _ = s.scan("t").unwrap();
}

#[test]
fn cov_file_storage_insert_one() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_file_storage_insert_many() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    for i in 0..100i64 {
        s.insert("t", vec![vec![SqlValue::Integer(i)]]).unwrap();
    }
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 100);
}

#[test]
fn cov_file_storage_insert_many_via_vec() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    let records: Vec<Vec<SqlValue>> = (0..50i64).map(|i| vec![SqlValue::Integer(i)]).collect();
    s.insert("t", records).unwrap();
    let _ = s.scan("t").unwrap();
}

#[test]
fn cov_file_storage_insert_text() {
    let (_t, mut s) = make_storage();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "s".to_string(),
            data_type: "TEXT".to_string(),
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
        original_sql: String::new(),
    };
    s.create_table(&info).unwrap();
    s.insert("t", vec![vec![SqlValue::Text("hello".to_string())]])
        .unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_file_storage_insert_float() {
    let (_t, mut s) = make_storage();
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
        original_sql: String::new(),
    };
    s.create_table(&info).unwrap();
    s.insert("t", vec![vec![SqlValue::Float(1.5)]]).unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_file_storage_insert_null() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Null]]).unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_file_storage_insert_bool() {
    let (_t, mut s) = make_storage();
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
        original_sql: String::new(),
    };
    s.create_table(&info).unwrap();
    s.insert("t", vec![vec![SqlValue::Boolean(true)]]).unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_file_storage_insert_with_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    s.insert_with_index("t", "a", 1, 0).unwrap();
    let _ = s.scan("t").unwrap();
}

#[test]
fn cov_file_storage_insert_with_index_multi() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    for i in 0..50i64 {
        s.insert_with_index("t", "a", i, 0).unwrap();
    }
    let _ = s.scan("t").unwrap();
}

#[test]
fn cov_file_storage_update() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let _ = s.update("t", &[], &[(0, SqlValue::Integer(99))]);
}

#[test]
fn cov_file_storage_update_with_filter() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(2)]]).unwrap();
    let _ = s.update("t", &[SqlValue::Integer(1)], &[(0, SqlValue::Integer(100))]);
}

#[test]
fn cov_file_storage_delete() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let _ = s.delete("t", &[]);
}

#[test]
fn cov_file_storage_delete_with_filter() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(2)]]).unwrap();
    let _ = s.delete("t", &[SqlValue::Integer(1)]);
}

// --------------------------------------------------------------------------
// Index management
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_create_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
}

#[test]
fn cov_file_storage_has_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    assert!(s.has_index("t", "a"));
    assert!(!s.has_index("t", "missing"));
}

#[test]
fn cov_file_storage_get_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    let _ = s.get_index("t", "a");
}

#[test]
fn cov_file_storage_search_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    s.insert_with_index("t", "a", 1, 0).unwrap();
    s.insert_with_index("t", "a", 2, 0).unwrap();
    let r = s.search_index("t", "a", 1i64);
    let _ = r;
}

#[test]
fn cov_file_storage_range_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    for i in 0..10i64 {
        s.insert_with_index("t", "a", i, 0).unwrap();
    }
    let r = s.range_index("t", "a", 2i64, 7i64);
    let _ = r;
}

#[test]
fn cov_file_storage_drop_index() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    s.drop_index("t", "a").unwrap();
    assert!(!s.has_index("t", "a"));
}

#[test]
fn cov_file_storage_flush_indexes() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.create_index("t", "a", 0).unwrap();
    s.flush_indexes().unwrap();
}

#[test]
fn cov_file_storage_flush_all_buffers() {
    let (_t, mut s) = make_storage();
    s.flush_all_buffers().unwrap();
}

// --------------------------------------------------------------------------
// Partition operations
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_partition_rows() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    for i in 0..100i64 {
        s.insert("t", vec![vec![SqlValue::Integer(i)]]).unwrap();
    }
    let parts = s.partition_rows("t", 4);
    let _ = parts;
}

#[test]
fn cov_file_storage_partition_rows_empty() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    let parts = s.partition_rows("t", 4);
    let _ = parts;
}

#[test]
fn cov_file_storage_partition_rows_single_partition() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
    let parts = s.partition_rows("t", 1);
    let _ = parts;
}

// --------------------------------------------------------------------------
// Parallel flush
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_flush_parallel() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    s.flush_parallel().unwrap();
}

#[test]
fn cov_file_storage_discard_all_buffers() {
    let (_t, mut s) = make_storage();
    s.discard_all_buffers();
}

// --------------------------------------------------------------------------
// Edge cases
// --------------------------------------------------------------------------

#[test]
fn cov_file_storage_scan_with_null_columns() {
    let (_t, mut s) = make_storage();
    let info = TableInfo {
        name: "t".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "a".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: true,
                primary_key: false,
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
        original_sql: String::new(),
    };
    s.create_table(&info).unwrap();
    s.insert("t", vec![vec![SqlValue::Null, SqlValue::Null]])
        .unwrap();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn cov_file_storage_repeat_insert_scan() {
    let (_t, mut s) = make_storage();
    s.create_table(&table_info()).unwrap();
    for _ in 0..3 {
        s.insert("t", vec![vec![SqlValue::Integer(1)]]).unwrap();
        s.insert("t", vec![vec![SqlValue::Integer(2)]]).unwrap();
        s.flush().unwrap();
    }
    let rows = s.scan("t").unwrap();
    assert!(!rows.is_empty());
}
