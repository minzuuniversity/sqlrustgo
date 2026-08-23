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

/// Row header size (16 bytes: row_size + var_field_offset + row_id).
pub const ROW_HEADER_SIZE: usize = 16;

/// Row footer size (4 bytes: CRC32C).
pub const ROW_FOOTER_SIZE: usize = 4;

/// 16-byte row header at the start of every BINT v3 row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowHeader {
    /// Total row size including header, fixed fields, var fields, and footer.
    pub row_size: u32,
    /// Byte offset (relative to row start) where variable-length fields begin.
    pub var_field_offset: u32,
    /// Logical row id / LSN.
    pub row_id: u64,
}

/// Encode row header to 16 bytes (little-endian).
pub fn encode_row_header(h: &RowHeader) -> [u8; 16] {
    let mut buf = [0u8; 16];
    buf[0..4].copy_from_slice(&h.row_size.to_le_bytes());
    buf[4..8].copy_from_slice(&h.var_field_offset.to_le_bytes());
    buf[8..16].copy_from_slice(&h.row_id.to_le_bytes());
    buf
}

/// Decode 16 bytes back into a row header.
pub fn decode_row_header(buf: &[u8; 16]) -> RowHeader {
    RowHeader {
        row_size: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        var_field_offset: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
        row_id: u64::from_le_bytes([buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15]]),
    }
}

use crc32c::Crc32cHasher;
use std::hash::Hasher;

/// Compute CRC32C over row bytes (excluding the trailing 4-byte footer).
pub fn compute_row_crc(row_bytes: &[u8]) -> u32 {
    let mut hasher = Crc32cHasher::default();
    hasher.write(row_bytes);
    hasher.finish() as u32
}

/// Verify that the given CRC32C matches the row bytes.
pub fn verify_row_crc(row_bytes: &[u8], expected: u32) -> bool {
    compute_row_crc(row_bytes) == expected
}

/// A 16 KB page-aligned segment file writer.
pub struct SegmentWriter {
    path: PathBuf,
    schema: Vec<ColumnDefinition>,
    bytes_written: u32,
    rows_in_segment: u32,
    max_segment_size: usize,
}

/// Returns the fixed byte width of a column, or `None` for variable-length types.
pub fn column_width(col: &ColumnDefinition) -> Option<usize> {
    let dt = col.data_type.to_uppercase();
    let base_type = dt.split('(').next().unwrap_or(&dt).trim();

    match base_type {
        "BIGINT" | "BIGINT UNSIGNED" => Some(8),
        "INT" | "INTEGER" | "INT UNSIGNED" | "INTEGER UNSIGNED" => Some(4),
        "SMALLINT" | "SMALLINT UNSIGNED" => Some(2),
        "TINYINT" | "TINYINT UNSIGNED" => Some(1),
        "FLOAT" | "REAL" => Some(4),
        "DOUBLE" | "DOUBLE PRECISION" => Some(8),
        "BOOL" | "BOOLEAN" => Some(1),
        "DATE" => Some(4),
        "TIMESTAMP" | "DATETIME" => Some(8),
        "CHAR" => col.char_max_length,
        "DECIMAL" | "NUMERIC" => Some(16),
        "VARCHAR" | "TEXT" | "BLOB" | "JSON" | "JSONB" | "VARBINARY" => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ColumnDefinition;

    fn make_col(name: &str, dt: &str, char_len: Option<usize>) -> ColumnDefinition {
        ColumnDefinition {
            name: name.to_string(),
            data_type: dt.to_string(),
            nullable: true,
            primary_key: false,
            default_value: None,
            auto_increment: false,
            char_max_length: char_len,
            collation: None,
        }
    }

    #[test]
    fn test_column_width_fixed_types() {
        assert_eq!(column_width(&make_col("a", "BIGINT", None)), Some(8));
        assert_eq!(column_width(&make_col("a", "INT", None)), Some(4));
        assert_eq!(column_width(&make_col("a", "INTEGER", None)), Some(4));
        assert_eq!(column_width(&make_col("a", "SMALLINT", None)), Some(2));
        assert_eq!(column_width(&make_col("a", "FLOAT", None)), Some(4));
        assert_eq!(column_width(&make_col("a", "REAL", None)), Some(4));
        assert_eq!(column_width(&make_col("a", "DOUBLE", None)), Some(8));
        assert_eq!(column_width(&make_col("a", "BOOL", None)), Some(1));
        assert_eq!(column_width(&make_col("a", "BOOLEAN", None)), Some(1));
        assert_eq!(column_width(&make_col("a", "DATE", None)), Some(4));
        assert_eq!(column_width(&make_col("a", "TIMESTAMP", None)), Some(8));
        assert_eq!(column_width(&make_col("a", "DATETIME", None)), Some(8));
        assert_eq!(column_width(&make_col("a", "DECIMAL", None)), Some(16));
    }

    #[test]
    fn test_column_width_char() {
        let col = make_col("c", "CHAR", Some(20));
        assert_eq!(column_width(&col), Some(20));
    }

    #[test]
    fn test_column_width_variable_types() {
        assert_eq!(column_width(&make_col("v", "VARCHAR(255)", Some(255))), None);
        assert_eq!(column_width(&make_col("t", "TEXT", None)), None);
        assert_eq!(column_width(&make_col("b", "BLOB", None)), None);
        assert_eq!(column_width(&make_col("j", "JSON", None)), None);
    }

    #[test]
    fn test_row_header_size_constant() {
        use std::mem::size_of;
        assert_eq!(size_of::<RowHeader>(), ROW_HEADER_SIZE);
        assert_eq!(size_of::<RowHeader>(), 16);
    }

    #[test]
    fn test_row_header_roundtrip() {
        let h = RowHeader {
            row_size: 256,
            var_field_offset: 64,
            row_id: 0xDEADBEEFCAFEBABE,
        };
        let buf = encode_row_header(&h);
        assert_eq!(buf.len(), 16);
        let h2 = decode_row_header(&buf);
        assert_eq!(h.row_size, h2.row_size);
        assert_eq!(h.var_field_offset, h2.var_field_offset);
        assert_eq!(h.row_id, h2.row_id);
    }

    #[test]
    fn test_row_crc_deterministic() {
        let row = b"hello world";
        let crc1 = compute_row_crc(row);
        let crc2 = compute_row_crc(row);
        assert_eq!(crc1, crc2);
        assert_ne!(crc1, 0);
    }

    #[test]
    fn test_row_crc_detects_corruption() {
        let row = b"hello world";
        let crc = compute_row_crc(row);
        let mut tampered = row.to_vec();
        tampered[0] ^= 0xFF;
        assert!(!verify_row_crc(&tampered, crc));
        assert!(verify_row_crc(row, crc));
    }
}
