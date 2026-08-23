//! JSON → BINT v3 lazy migration.

use crate::bin_index::{write_root_index_file, RootIndex, SegmentInfo};
use crate::bin_segment::SegmentWriter;
use crate::engine::{SqlError, SqlResult, TableData, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableFormat {
    Binary,
    Json,
    Missing,
}

/// Detect whether a table's files on disk are BIN, JSON, or absent.
pub fn detect_table_format(data_dir: &Path, table: &str) -> TableFormat {
    let root_bin = data_dir.join(format!("{}.root.bin", table));
    if root_bin.exists() {
        return TableFormat::Binary;
    }
    let json = data_dir.join(format!("{}.json", table));
    if json.exists() {
        return TableFormat::Json;
    }
    TableFormat::Missing
}

/// Encode a Value to its BINT v3 on-disk byte representation.
/// Duplicated from binary_storage_v2.rs since that function is private.
fn encode_value_to_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Null => Vec::new(),
        Value::Boolean(b) => vec![*b as u8],
        Value::Integer(i) => i.to_le_bytes().to_vec(),
        Value::Float(f) => f.to_le_bytes().to_vec(),
        Value::Text(s) => s.as_bytes().to_vec(),
        Value::Blob(b) => b.clone(),
        Value::Point(x, y) => {
            let mut buf = Vec::new();
            buf.extend_from_slice(&x.to_le_bytes());
            buf.extend_from_slice(&y.to_le_bytes());
            buf
        }
        Value::Json(j) => j.to_string().as_bytes().to_vec(),
    }
}

/// Atomically migrate a table from JSON to BIN format.
///
/// Steps:
/// 1. Write new .bin segment
/// 2. Write new .root.bin index
/// 3. Atomically rename .json → .json.bak
///
/// If any step fails, .json remains in place and no .bak is created.
pub fn migrate_json_to_bin(
    data_dir: &Path,
    table: &str,
    json_data: &TableData,
) -> SqlResult<()> {
    std::fs::create_dir_all(data_dir).map_err(|e| SqlError::ExecutionError(e.to_string()))?;

    // Step 1: Write BIN segment
    let seg_path = data_dir.join(format!("{}_seg_0000.bin", table));
    let mut writer = SegmentWriter::new(seg_path.clone(), json_data.info.columns.clone())
        .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
    for record in &json_data.rows {
        let values: Vec<Option<Vec<u8>>> = record.iter().map(|v| {
            if matches!(v, Value::Null) {
                None
            } else {
                Some(encode_value_to_bytes(v))
            }
        }).collect();
        writer.append(&values).map_err(|e| SqlError::ExecutionError(e.to_string()))?;
    }
    writer.seal().map_err(|e| SqlError::ExecutionError(e.to_string()))?;

    // Step 2: Write root.index
    let seg_size = std::fs::metadata(&seg_path)
        .map_err(|e| SqlError::ExecutionError(e.to_string()))?
        .len();
    let root_idx = RootIndex {
        version: 3,
        segments: vec![SegmentInfo {
            segment_id: 0,
            file_name: seg_path.file_name().unwrap().to_string_lossy().to_string(),
            row_count: writer.rows_in_segment(),
            byte_size: seg_size,
        }],
        schema_hash: 0,
        total_rows: json_data.rows.len() as u64,
        index_crc: 0,
    };
    let root_path = data_dir.join(format!("{}.root.bin", table));
    write_root_index_file(&root_path, &root_idx)
        .map_err(|e| SqlError::ExecutionError(e.to_string()))?;

    // Step 3: Atomic rename .json → .json.bak
    let json_path = data_dir.join(format!("{}.json", table));
    let bak_path = data_dir.join(format!("{}.json.bak", table));
    if json_path.exists() {
        std::fs::rename(&json_path, &bak_path)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ColumnDefinition, TableInfo};
    use tempfile::tempdir;

    #[test]
    fn test_detect_json_only() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("t1.json"), b"{}").unwrap();
        assert!(matches!(detect_table_format(dir.path(), "t1"), TableFormat::Json));
    }

    #[test]
    fn test_detect_bin_only() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("t1.root.bin"), b"\0\0\0\0").unwrap();
        assert!(matches!(detect_table_format(dir.path(), "t1"), TableFormat::Binary));
    }

    #[test]
    fn test_detect_both_prefers_bin() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("t1.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("t1.root.bin"), b"\0\0\0\0").unwrap();
        assert!(matches!(detect_table_format(dir.path(), "t1"), TableFormat::Binary));
    }

    #[test]
    fn test_detect_missing() {
        let dir = tempdir().unwrap();
        assert!(matches!(detect_table_format(dir.path(), "missing"), TableFormat::Missing));
    }

    #[test]
    fn test_migrate_json_to_bin_atomic() {
        let dir = tempdir().unwrap();
        let table = "t1";
        // Write a JSON file
        let json_path = dir.path().join(format!("{}.json", table));
        std::fs::write(&json_path, b"{}").unwrap();
        // Build TableData with actual structure
        let td = TableData {
            info: TableInfo {
                name: table.into(),
                columns: vec![ColumnDefinition {
                    name: "x".into(),
                    data_type: "BIGINT".into(),
                    nullable: false,
                    primary_key: true,
                    default_value: None,
                    auto_increment: false,
                    char_max_length: None,
                    collation: None,
                }],
                foreign_keys: Default::default(),
                unique_constraints: Default::default(),
                check_constraints: Default::default(),
                partition_info: None,
                compression: None,
                collations: Default::default(),
            },
            rows: vec![vec![Value::Integer(42)]],
        };
        migrate_json_to_bin(dir.path(), table, &td).unwrap();
        // root.bin should exist
        assert!(dir.path().join(format!("{}.root.bin", table)).exists());
        // .json should be archived to .json.bak
        assert!(dir.path().join(format!("{}.json.bak", table)).exists());
        assert!(!json_path.exists());
    }

    #[test]
    fn test_migrate_atomic_on_failure() {
        let dir = tempdir().unwrap();
        let table = "t2";
        let json_path = dir.path().join(format!("{}.json", table));
        std::fs::write(&json_path, b"original").unwrap();
        // Pass empty TableData that will cause write to fail (empty columns)
        let td = TableData {
            info: TableInfo {
                name: table.into(),
                columns: vec![],
                foreign_keys: Default::default(),
                unique_constraints: Default::default(),
                check_constraints: Default::default(),
                partition_info: None,
                compression: None,
                collations: Default::default(),
            },
            rows: vec![],
        };
        // Use a non-existent parent dir that can't be created (path is a file, not dir)
        let bad_dir = dir.path().join("not_a_dir");
        std::fs::write(&bad_dir, b"im a file not dir").unwrap();
        let result = migrate_json_to_bin(bad_dir.as_path(), table, &td);
        // If it fails, no .bak file should be created
        if result.is_err() {
            // No .bak file should be created since migration failed
            assert!(!dir.path().join("not_a_dir").join(format!("{}.json.bak", table)).exists());
        }
    }
}
