//! F2: A row with a corrupted CRC32C must be skipped during scan; remaining
//! rows must still decode successfully.
//!
//! This proves per-row corruption isolation: one bad row doesn't poison
//! the whole segment.

use sqlrustgo_storage::bin_segment::{SegmentReader, SegmentWriter};
use sqlrustgo_storage::engine::ColumnDefinition;
use tempfile::TempDir;

#[test]
fn test_corrupted_row_is_skipped() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path().join("crc.bin");

    // BIGINT schema (NOT INTEGER) — same trap as T6.1.
    let mut id_col = ColumnDefinition::new("x", "BIGINT");
    id_col.primary_key = true;
    let schema = vec![id_col];

    let mut writer = SegmentWriter::new(path.clone(), schema.clone()).expect("SegmentWriter::new");
    for i in 0..10i64 {
        writer
            .append(&[Some(i.to_le_bytes().to_vec())])
            .expect("append");
    }
    writer.seal().expect("seal");

    // Row layout for BIGINT-only: 16-byte header + 2-byte null_bitmap +
    // 8-byte bigint + 4-byte crc32c = 30 bytes per row. Data region starts
    // at offset 16384 (after the segment header page).
    //
    // Flip a byte at row 5's CRC footer byte:
    //   row 5 starts at 16384 + 5 * 30 = 16534
    //   CRC occupies the trailing 4 bytes of the row, i.e. offsets 26..30
    //   Flip the first CRC byte at offset 16534 + 26 = 16560.
    let mut bytes = std::fs::read(&path).expect("read segment");
    assert!(bytes.len() > 16564, "segment must contain row 5 entirely");
    bytes[16560] ^= 0xFF;
    std::fs::write(&path, &bytes).expect("write corrupted");

    // Reader must open successfully (file is large enough; header + footer pages intact)
    // and yield exactly 9 valid rows (the corrupted row 5 returns Err).
    let reader = SegmentReader::open(&path, schema).expect("open");
    let valid_rows: Vec<_> = reader.iter_rows().filter_map(|r| r.ok()).collect();
    assert_eq!(
        valid_rows.len(),
        9,
        "exactly one row corrupted; 9 valid rows expected"
    );
}
