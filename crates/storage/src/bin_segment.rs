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
}
