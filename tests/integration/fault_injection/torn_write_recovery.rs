//! F1: Truncated segment file must be detected and rejected or yield a
//! reduced row count.
//!
//! Torn-write protection: when a 16 KB page-aligned segment is half-written
//! (process killed mid-flush, fsync lost power, etc.), SegmentReader::open
//! must either:
//!   (a) fail open() with RowDecodeError::TooShort (file < 32 KB = header
//!       page + final page missing), OR
//!   (b) open successfully but expose row_count() < the number of rows
//!       actually written before the crash.

use sqlrustgo_storage::bin_segment::{SegmentReader, SegmentWriter};
use sqlrustgo_storage::engine::ColumnDefinition;
use tempfile::TempDir;

#[test]
fn test_truncated_segment_is_rejected() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path().join("seg.bin");

    // BIGINT schema (NOT INTEGER) — Value::Integer writes 8 bytes; column_width
    // for "INTEGER" returns 4 which causes RowDecodeError::TooShort mid-row.
    let mut id_col = ColumnDefinition::new("x", "BIGINT");
    id_col.primary_key = true;
    let schema = vec![id_col];

    let mut writer = SegmentWriter::new(path.clone(), schema.clone()).expect("SegmentWriter::new");
    for i in 0..100i64 {
        writer
            .append(&[Some(i.to_le_bytes().to_vec())])
            .expect("append");
    }
    writer.seal().expect("seal");

    let original_size = std::fs::metadata(&path).expect("metadata").len();
    assert!(
        original_size >= 32_768,
        "sealed segment must be >= header+footer pages"
    );

    // Truncate to half size — guaranteed below 32 KB threshold.
    let half_size = (original_size / 2) as usize;
    let truncated_bytes = std::fs::read(&path).expect("read")[..half_size].to_vec();
    std::fs::write(&path, &truncated_bytes).expect("write truncated");

    let result = SegmentReader::open(&path, schema);

    // Either open fails (TooShort) or row_count is below the 100 rows we wrote.
    let ok = match result {
        Err(_) => true, // SegmentReader::open rejected the truncated file
        Ok(reader) => reader.row_count() < 100,
    };
    assert!(
        ok,
        "truncated segment must be rejected (open Err or row_count < 100)"
    );
}
