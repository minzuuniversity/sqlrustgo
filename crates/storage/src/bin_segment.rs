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

/// Errors that can occur while decoding a row.
#[derive(Debug, thiserror::Error)]
pub enum RowDecodeError {
    #[error("row bytes too short: expected at least {expected}, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("row CRC32C mismatch: computed {computed:#x}, expected {expected:#x}")]
    CrcMismatch { computed: u32, expected: u32 },
    #[error("invalid var-length field: {0}")]
    InvalidVarLength(String),
}

/// Encode one row using fixed+variable layout.
///
/// `values[i]` is `None` if column `i` is NULL, else `Some(raw_bytes)` where
/// raw_bytes must already be in the column's on-disk encoding
/// (i32 LE for Int, UTF-8 for VarChar, etc.).
pub fn encode_row(
    schema: &[ColumnDefinition],
    values: &[Option<Vec<u8>>],
    row_id: u64,
) -> Vec<u8> {
    // Compute null bitmap
    let mut null_bitmap: u16 = 0;
    for (i, v) in values.iter().enumerate() {
        if v.is_none() {
            null_bitmap |= 1 << i;
        }
    }
    // Compute fixed field total size (excluding NULLs)
    let fixed_size: usize = schema
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if (null_bitmap >> i) & 1 == 1 {
                None
            } else {
                column_width(c)
            }
        })
        .sum();
    // Layout: header(16) | fixed... | null_bitmap(2) | var fields | footer(4)
    let var_field_offset = (ROW_HEADER_SIZE + fixed_size) as u32;
    let header = RowHeader {
        row_size: 0, // filled below
        var_field_offset,
        row_id,
    };
    let header_bytes = encode_row_header(&header);
    let mut buf = Vec::with_capacity(header_bytes.len() + fixed_size + 64);
    buf.extend_from_slice(&header_bytes);
    // Write fixed-length fields (in column order, skipping NULLs)
    for (i, col) in schema.iter().enumerate() {
        if (null_bitmap >> i) & 1 == 1 {
            continue;
        }
        if let Some(width) = column_width(col) {
            let val = values[i].as_ref().expect("non-null column must have value");
            assert_eq!(val.len(), width, "fixed column {} byte width mismatch", col.name);
            buf.extend_from_slice(val);
        }
    }
    // Write null_bitmap at start of var region (2 bytes LE)
    buf.extend_from_slice(&null_bitmap.to_le_bytes());
    // Write variable-length fields: each = u32 length + bytes
    for (i, col) in schema.iter().enumerate() {
        if (null_bitmap >> i) & 1 == 1 {
            continue;
        }
        if column_width(col).is_none() {
            let val = values[i].as_ref().unwrap();
            buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
            buf.extend_from_slice(val);
        }
    }
    // Patch row_size in header (now that we know total)
    let row_size = (buf.len() + ROW_FOOTER_SIZE) as u32;
    buf[0..4].copy_from_slice(&row_size.to_le_bytes());
    // Append CRC32C footer over everything except footer itself
    let crc = compute_row_crc(&buf);
    buf.extend_from_slice(&crc.to_le_bytes());
    buf
}

/// Decode a row back into columnar values (NULL = None).
pub fn decode_row(
    schema: &[ColumnDefinition],
    row_bytes: &[u8],
) -> Result<Vec<Option<Vec<u8>>>, RowDecodeError> {
    if row_bytes.len() < ROW_HEADER_SIZE + 2 + ROW_FOOTER_SIZE {
        return Err(RowDecodeError::TooShort {
            expected: ROW_HEADER_SIZE + 2 + ROW_FOOTER_SIZE,
            actual: row_bytes.len(),
        });
    }
    let header_arr: [u8; 16] = row_bytes[0..16].try_into().unwrap();
    let header = decode_row_header(&header_arr);
    let body = &row_bytes[..row_bytes.len() - ROW_FOOTER_SIZE];
    let expected_crc =
        u32::from_le_bytes(row_bytes[row_bytes.len() - 4..].try_into().unwrap());
    if !verify_row_crc(body, expected_crc) {
        let computed = compute_row_crc(body);
        return Err(RowDecodeError::CrcMismatch {
            computed,
            expected: expected_crc,
        });
    }
    // Read null_bitmap at start of var region
    let var_pos = header.var_field_offset as usize;
    let null_bitmap = u16::from_le_bytes([body[var_pos], body[var_pos + 1]]);
    let var_data_start = var_pos + 2;
    let mut values: Vec<Option<Vec<u8>>> = vec![None; schema.len()];
    let mut fixed_pos = ROW_HEADER_SIZE;
    let mut var_cur = var_data_start;
    for (i, col) in schema.iter().enumerate() {
        if (null_bitmap >> i) & 1 == 1 {
            continue;
        }
        if let Some(width) = column_width(col) {
            values[i] = Some(row_bytes[fixed_pos..fixed_pos + width].to_vec());
            fixed_pos += width;
        } else {
            if var_cur + 4 > row_bytes.len() - ROW_FOOTER_SIZE {
                return Err(RowDecodeError::InvalidVarLength(
                    "truncated var-length header".into(),
                ));
            }
            let len = u32::from_le_bytes([
                row_bytes[var_cur], row_bytes[var_cur + 1],
                row_bytes[var_cur + 2], row_bytes[var_cur + 3],
            ]) as usize;
            var_cur += 4;
            if var_cur + len > row_bytes.len() - ROW_FOOTER_SIZE {
                return Err(RowDecodeError::InvalidVarLength(
                    format!("declared length {} exceeds row body", len),
                ));
            }
            values[i] = Some(row_bytes[var_cur..var_cur + len].to_vec());
            var_cur += len;
        }
    }
    Ok(values)
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

    #[test]
    fn test_encode_row_int_and_text() {
        let schema = vec![
            make_col("id", "INT", None),
            make_col("name", "VARCHAR(255)", None),
        ];
        let values = vec![
            Some(42i32.to_le_bytes().to_vec()),
            Some(b"alice".to_vec()),
        ];
        let row = encode_row(&schema, &values, 1);
        // header(16) + int(4) + null_bitmap(2) + var_len(4) + name(5) + crc(4) = 35
        assert!(row.len() >= 35);
        let decoded = decode_row(&schema, &row).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].as_ref().unwrap(), &42i32.to_le_bytes().to_vec());
        assert_eq!(decoded[1].as_ref().unwrap(), b"alice");
    }

    #[test]
    fn test_encode_row_with_nulls() {
        let schema = vec![
            make_col("a", "INT", None),
            make_col("b", "TEXT", None),
        ];
        let values = vec![Some(7i32.to_le_bytes().to_vec()), None];
        let row = encode_row(&schema, &values, 1);
        let decoded = decode_row(&schema, &row).unwrap();
        assert!(decoded[1].is_none());
    }
}
