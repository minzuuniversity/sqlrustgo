//! L2: Write N rows -> flush -> verify root.bin row count + segment file exists.

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::bin_index::read_root_index_file;
use sqlrustgo_storage::engine::Value;
use sqlrustgo_storage::ColumnDefinition;
use tempfile::tempdir;

#[test]
fn test_basic_io_100k_rows() {
    let dir = tempdir().expect("tempdir");
    let mut storage = BinaryTableStorageV2::new(dir.path().to_path_buf())
        .expect("BinaryTableStorageV2::new");

    // BIGINT (8 bytes) matches Value::Integer (i64) encoding width - INTEGER (4) does not.
    let mut id_col = ColumnDefinition::new("id", "BIGINT");
    id_col.primary_key = true;

    let mut data_col = ColumnDefinition::new("data", "VARCHAR(255)");
    data_col.nullable = true;

    let schema = vec![id_col, data_col];
    storage.create_table("t1", schema).expect("create_table");

    // Record = Vec<Value>
    let records: Vec<Vec<Value>> = (0..100_000i64)
        .map(|i| vec![Value::Integer(i), Value::Text(format!("row_{}", i))])
        .collect();

    storage.insert_streaming("t1", records).expect("insert_streaming");
    storage.flush().expect("flush");

    // Verify on-disk root.bin reports correct row count (Option A - exercises format)
    let root_path = dir.path().join("t1.root.bin");
    let idx = read_root_index_file(&root_path).expect("read_root_index_file");
    assert_eq!(idx.total_rows, 100_000, "root.bin total_rows mismatch");
    assert!(!idx.segments.is_empty(), "root.bin must list >=1 segment");
    let seg_count: u32 = idx.segments.iter().map(|s| s.row_count).sum();
    assert_eq!(seg_count as u64, 100_000, "segments sum != 100_000");

    // Verify at least one segment file exists on disk
    let segments_on_disk: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let binding = e.file_name();
            let name = binding.to_string_lossy();
            name.contains("_seg_") && name.ends_with(".bin")
        })
        .collect();
    assert!(!segments_on_disk.is_empty(), "no segment files on disk");
}
