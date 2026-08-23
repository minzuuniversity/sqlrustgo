//! L2: multi-segment → single snapshot → row count preserved.
//!
//! TDD red phase: BinCompactor is not yet implemented (T5.5). This test
//! will not compile until `crates/storage/src/bin_compactor.rs` is added
//! and exported in `crates/storage/src/lib.rs`. After T5.5, it must pass.
//!
//! Schema uses BIGINT — Value::Integer encodes as i64 (8 bytes), and
//! column_width for INTEGER is 4 (known pre-existing mismatch tracked
//! for final review). BIGINT matches.

use sqlrustgo_storage::bin_compactor::{BinCompactor, CompactorConfig};
use sqlrustgo_storage::bin_index::read_root_index_file;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, Value};
use sqlrustgo_storage::BinaryTableStorageV2;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_compaction_preserves_data() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let data_dir: PathBuf = temp_dir.path().to_path_buf();
    let mut storage = BinaryTableStorageV2::new(data_dir.clone()).expect("V2 init");

    let mut id_col = ColumnDefinition::new("id", "BIGINT");
    id_col.primary_key = true;
    let schema = vec![id_col];
    storage.create_table("t1", schema).expect("create_table");

    // Three batches of 100k rows → many segments after flush.
    let batch_size: i64 = 100_000;
    for batch_idx in 0..3 {
        let start = batch_idx * batch_size;
        let end = start + batch_size;
        let records: Vec<Record> = (start..end).map(|i| vec![Value::Integer(i)]).collect();
        storage
            .insert_streaming("t1", records)
            .expect("insert_streaming");
    }
    storage.flush().expect("flush");

    // V2 has no get_table_row_count(); read from the on-disk root.bin.
    let root_path = data_dir.join("t1.root.bin");
    let pre_idx = read_root_index_file(&root_path).expect("read pre root.bin");
    let pre_count = pre_idx.total_rows;
    assert_eq!(pre_count, 300_000, "pre-compaction total_rows");

    let compactor = BinCompactor::new(CompactorConfig {
        max_segment_count: 1,
        oom_safe: true,
    });
    compactor.run(&data_dir, "t1").expect("compactor.run");

    let post_idx = read_root_index_file(&root_path).expect("read post root.bin");
    let post_count = post_idx.total_rows;
    assert_eq!(post_count, 300_000, "post-compaction total_rows");
}
