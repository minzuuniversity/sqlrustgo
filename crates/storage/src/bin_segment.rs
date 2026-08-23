//! BINT v3 binary segment file format.
//!
//! Each segment is a 16 KB page-aligned file containing:
//!   - Segment header (16 KB page 0): magic + version + schema + CRC32C
//!   - Data region: rows in fixed+variable encoding
//!   - Page footer (last 16 KB): row_count + segment_size + CRC32C

use crate::engine::ColumnDefinition;
use std::path::PathBuf;

/// BINT v3 segment file magic bytes.
pub const SEGMENT_MAGIC: &[u8; 8] = b"BINTv3\0\0";

/// BINT v3 format version.
pub const SEGMENT_VERSION: u32 = 3;

/// Default data region start offset (16 KB page).
pub const DATA_START_OFFSET: u32 = 0x4000;

/// Default segment size cap (64 MB).
pub const DEFAULT_SEGMENT_SIZE_CAP: usize = 64 * 1024 * 1024;

/// Row header size (16 bytes: row_size + var_field_offset + row_id + null_bitmap + reserved).
pub const ROW_HEADER_SIZE: usize = 16;

/// Row footer size (4 bytes: CRC32C).
pub const ROW_FOOTER_SIZE: usize = 4;

/// A 16 KB page-aligned segment file writer.
pub struct SegmentWriter {
    path: PathBuf,
    schema: Vec<ColumnDefinition>,
    bytes_written: u32,
    rows_in_segment: u32,
    max_segment_size: usize,
}
