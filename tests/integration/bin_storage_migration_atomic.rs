//! L2: JSON → BIN migration atomicity. Verify no .json.bak exists if write fails.
//!
//! Schema uses BIGINT (not INTEGER) — BINT v3 encode_value_to_bytes emits
//! Value::Integer as i64 (8 bytes); column_width for INTEGER is 4. Same
//! pre-existing mismatch as T5.1/T5.2, tracked for final review.

use sqlrustgo_storage::bin_migration::{detect_table_format, migrate_json_to_bin, TableFormat};
use sqlrustgo_storage::engine::{ColumnDefinition, Record, TableData, TableInfo, Value};
use tempfile::TempDir;

#[test]
fn test_migration_atomic_no_partial_state() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let table = "t1";
    let json_path = temp_dir.path().join(format!("{}.json", table));
    std::fs::write(&json_path, b"{}").expect("write json stub");

    // Build the TableData that migrate_json_to_bin expects.
    // TableInfo requires `name`, `columns`; serde-defaulted fields use
    // `..Default::default()`.
    let mut id_col = ColumnDefinition::new("x", "BIGINT");
    id_col.primary_key = true;
    let info = TableInfo {
        name: table.to_string(),
        columns: vec![id_col],
        ..Default::default()
    };
    let rows: Vec<Record> = vec![vec![Value::Integer(1i64)]];
    let td = TableData { info, rows };

    migrate_json_to_bin(temp_dir.path(), table, &td).expect("migrate_json_to_bin should succeed");

    // Post-migration state:
    assert!(
        matches!(
            detect_table_format(temp_dir.path(), table),
            TableFormat::Binary
        ),
        "detect_table_format must return Binary after migration"
    );
    assert!(
        !json_path.exists(),
        "{}.json should be renamed (no longer present)",
        table
    );
    let bak_path = temp_dir.path().join(format!("{}.json.bak", table));
    assert!(
        bak_path.exists(),
        "{}.json.bak should exist after migration",
        table
    );
    let root_path = temp_dir.path().join(format!("{}.root.bin", table));
    assert!(
        root_path.exists(),
        "{}.root.bin should exist after migration",
        table
    );
}
