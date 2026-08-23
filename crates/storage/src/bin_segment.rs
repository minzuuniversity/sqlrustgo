//! BINT v3 binary segment file format.
//!
//! Each segment is a 16 KB page-aligned file containing:
//!   - Segment header (16 KB page 0): magic + version + schema + CRC32C
//!   - Data region: rows in fixed+variable encoding
//!   - Page footer (last 16 KB): row_count + segment_size + CRC32C

use crate::engine::ColumnDefinition;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use thiserror::Error;

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
        row_id: u64::from_le_bytes([
            buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
        ]),
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
    file: BufWriter<File>,
    bytes_written: u32,
    rows_in_segment: u32,
    max_segment_size: usize,
    next_row_id: u64,
    sealed: bool,
}

impl SegmentWriter {
    pub fn new(path: PathBuf, schema: Vec<ColumnDefinition>) -> std::io::Result<Self> {
        Self::with_size_cap(path, schema, DEFAULT_SEGMENT_SIZE_CAP)
    }

    pub fn with_size_cap(
        path: PathBuf,
        schema: Vec<ColumnDefinition>,
        max_segment_size: usize,
    ) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = File::create(&path)?;
        let mut writer = BufWriter::with_capacity(1 << 20, file); // 1 MB BufWriter
                                                                  // Write placeholder header (row_count=0; will not rewrite on seal in this task)
        let _ = encode_segment_header(&SegmentHeader {
            magic: *SEGMENT_MAGIC,
            version: SEGMENT_VERSION,
            flags: 0,
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
            col_count: schema.len() as u16,
            row_count: 0,
            _reserved1: 0,
            schema_offset: 0x0020,
            data_start: DATA_START_OFFSET,
            _reserved2: 0,
            header_crc: 0,
        });
        // We must persist the 16 KB header page on disk. write_all via BufWriter flushes on drop.
        let header_page = encode_segment_header(&SegmentHeader {
            magic: *SEGMENT_MAGIC,
            version: SEGMENT_VERSION,
            flags: 0,
            ts: 0,
            col_count: schema.len() as u16,
            row_count: 0,
            _reserved1: 0,
            schema_offset: 0x0020,
            data_start: DATA_START_OFFSET,
            _reserved2: 0,
            header_crc: 0,
        });
        writer.write_all(&header_page)?;
        // Pad from end of header (44 bytes) to DATA_START_OFFSET (16 KB)
        let pad = DATA_START_OFFSET as usize - header_page.len();
        if pad > 0 {
            writer.write_all(&vec![0u8; pad])?;
        }
        Ok(Self {
            path,
            schema,
            file: writer,
            bytes_written: DATA_START_OFFSET,
            rows_in_segment: 0,
            max_segment_size,
            next_row_id: 0,
            sealed: false,
        })
    }

    pub fn append(&mut self, values: &[Option<Vec<u8>>]) -> Result<u64, RowDecodeError> {
        if self.sealed {
            return Err(RowDecodeError::InvalidVarLength(
                "cannot append to sealed segment".into(),
            ));
        }
        let row = encode_row(&self.schema, values, self.next_row_id);
        if self.bytes_written as usize + row.len() + SEGMENT_FOOTER_SIZE > self.max_segment_size {
            return Err(RowDecodeError::InvalidVarLength(
                "segment size cap exceeded".into(),
            ));
        }
        self.file
            .write_all(&row)
            .map_err(|e| RowDecodeError::InvalidVarLength(format!("write error: {}", e)))?;
        self.bytes_written += row.len() as u32;
        self.rows_in_segment += 1;
        let id = self.next_row_id;
        self.next_row_id += 1;
        Ok(id)
    }

    pub fn seal(&mut self) -> Result<(), RowDecodeError> {
        if self.sealed {
            return Ok(());
        }
        // Pad to next 16 KB boundary; footer occupies its own 16 KB page
        let cur = self.bytes_written as usize;
        let footer_start = ((cur + 16383) / 16384) * 16384;
        let pad = footer_start - cur;
        if pad > 0 {
            self.file
                .write_all(&vec![0u8; pad])
                .map_err(|e| RowDecodeError::InvalidVarLength(format!("pad error: {}", e)))?;
            self.bytes_written += pad as u32;
        }
        let total_size = footer_start as u64 + SEGMENT_FOOTER_SIZE as u64;
        let footer = encode_segment_footer(&SegmentFooter {
            row_count: self.rows_in_segment,
            segment_size: total_size as u32,
            next_offset: 0,
            footer_crc: 0,
        });
        // Footer is 16 bytes; pad to fill 16 KB page
        self.file
            .write_all(&footer)
            .map_err(|e| RowDecodeError::InvalidVarLength(format!("footer error: {}", e)))?;
        let footer_page_pad = 16384 - footer.len();
        if footer_page_pad > 0 {
            self.file
                .write_all(&vec![0u8; footer_page_pad])
                .map_err(|e| {
                    RowDecodeError::InvalidVarLength(format!("footer pad error: {}", e))
                })?;
        }
        self.bytes_written += 16384;
        self.file
            .flush()
            .map_err(|e| RowDecodeError::InvalidVarLength(format!("flush error: {}", e)))?;
        self.sealed = true;
        Ok(())
    }

    pub fn bytes_written(&self) -> u32 {
        self.bytes_written
    }

    pub fn rows_in_segment(&self) -> u32 {
        self.rows_in_segment
    }
}

impl Drop for SegmentWriter {
    fn drop(&mut self) {
        if !self.sealed {
            let _ = self.seal();
        }
    }
}

// ============================================================
// Task 1.8: SegmentReader round-trip
// ============================================================

/// Reads rows from a sealed BINT v3 segment file.
pub struct SegmentReader {
    #[allow(dead_code)]
    path: PathBuf,
    #[allow(dead_code)]
    schema: Vec<ColumnDefinition>,
    header: SegmentHeader,
    footer: SegmentFooter,
    data_region: Vec<u8>,
}

impl SegmentReader {
    pub fn open(
        path: &std::path::Path,
        schema: Vec<ColumnDefinition>,
    ) -> Result<Self, RowDecodeError> {
        let bytes = std::fs::read(path)
            .map_err(|e| RowDecodeError::InvalidVarLength(format!("read error: {}", e)))?;
        if bytes.len() < 16384 * 2 {
            return Err(RowDecodeError::TooShort {
                expected: 16384 * 2,
                actual: bytes.len(),
            });
        }
        let header_arr: [u8; 16384] = bytes[0..16384].try_into().unwrap();
        let header = decode_segment_header(&header_arr)?;
        // Footer is the last 16 bytes; the final 16 KB page is padding around it
        let footer_start = bytes.len() - 16384;
        let footer_arr: [u8; 16] = bytes[footer_start..footer_start + 16].try_into().unwrap();
        let footer = decode_segment_footer(&footer_arr)?;
        let data_region = bytes[header.data_start as usize..footer_start].to_vec();
        Ok(Self {
            path: path.to_path_buf(),
            schema,
            header,
            footer,
            data_region,
        })
    }

    /// Iterates over all rows in the data region, yielding decode results.
    /// Corrupted rows produce an `Err` in the iterator output.
    pub fn iter_rows(
        &self,
    ) -> impl Iterator<Item = Result<Vec<Option<Vec<u8>>>, RowDecodeError>> + '_ {
        let mut pos = 0usize;
        let data = &self.data_region;
        let schema = &self.schema;
        std::iter::from_fn(move || {
            if pos + ROW_HEADER_SIZE + 2 + ROW_FOOTER_SIZE > data.len() {
                return None;
            }
            let row_size = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            if row_size < ROW_HEADER_SIZE + ROW_FOOTER_SIZE || pos + row_size > data.len() {
                return None;
            }
            let row_bytes = &data[pos..pos + row_size];
            let result = decode_row(schema, row_bytes);
            pos += row_size;
            Some(result)
        })
    }

    #[allow(dead_code)]
    pub fn row_count(&self) -> u32 {
        self.footer.row_count
    }

    #[allow(dead_code)]
    pub fn segment_size(&self) -> u32 {
        self.footer.segment_size
    }
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
pub fn encode_row(schema: &[ColumnDefinition], values: &[Option<Vec<u8>>], row_id: u64) -> Vec<u8> {
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
            assert_eq!(
                val.len(),
                width,
                "fixed column {} byte width mismatch",
                col.name
            );
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
    let expected_crc = u32::from_le_bytes(row_bytes[row_bytes.len() - 4..].try_into().unwrap());
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
                row_bytes[var_cur],
                row_bytes[var_cur + 1],
                row_bytes[var_cur + 2],
                row_bytes[var_cur + 3],
            ]) as usize;
            var_cur += 4;
            if var_cur + len > row_bytes.len() - ROW_FOOTER_SIZE {
                return Err(RowDecodeError::InvalidVarLength(format!(
                    "declared length {} exceeds row body",
                    len
                )));
            }
            values[i] = Some(row_bytes[var_cur..var_cur + len].to_vec());
            var_cur += len;
        }
    }
    Ok(values)
}

// ============================================================
// Task 1.6: Segment header (16 KB page) + segment footer (16 B)
// ============================================================

/// Padding added after the 44-byte fixed header to reach 16 KB page size.
pub const SEGMENT_HEADER_PADDING_SIZE: usize = 16384 - 0x2C;

/// Size of segment footer in bytes (4 + 8 + 4 = 16).
pub const SEGMENT_FOOTER_SIZE: usize = 16;

/// 44-byte fixed segment header (followed by padding to fill 16 KB).
#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct SegmentHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub flags: u32,
    pub ts: u64,
    pub col_count: u16,
    pub row_count: u32,
    pub _reserved1: u16, // bytes 30..32 (alignment padding)
    pub schema_offset: u16,
    pub data_start: u32,
    pub _reserved2: u16, // bytes 38..40 (alignment padding)
    pub header_crc: u32,
}

impl PartialEq for SegmentHeader {
    fn eq(&self, other: &Self) -> bool {
        self.magic == other.magic
            && self.version == other.version
            && self.flags == other.flags
            && self.ts == other.ts
            && self.col_count == other.col_count
            && self.row_count == other.row_count
            && self._reserved1 == other._reserved1
            && self.schema_offset == other.schema_offset
            && self.data_start == other.data_start
            && self._reserved2 == other._reserved2
            && self.header_crc == other.header_crc
    }
}

impl Eq for SegmentHeader {}

/// 16-byte segment footer at end of file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentFooter {
    pub row_count: u32,
    pub segment_size: u32,
    pub next_offset: u32,
    pub footer_crc: u32,
}

/// Encode segment header into a 16 KB page with CRC32C.
pub fn encode_segment_header(h: &SegmentHeader) -> Vec<u8> {
    let mut buf = vec![0u8; 16384];
    buf[0..8].copy_from_slice(&h.magic);
    buf[8..12].copy_from_slice(&h.version.to_le_bytes());
    buf[12..16].copy_from_slice(&h.flags.to_le_bytes());
    buf[16..24].copy_from_slice(&h.ts.to_le_bytes());
    buf[24..26].copy_from_slice(&h.col_count.to_le_bytes());
    buf[26..30].copy_from_slice(&h.row_count.to_le_bytes());
    // bytes 30..32: _reserved1 (alignment)
    buf[32..34].copy_from_slice(&h.schema_offset.to_le_bytes());
    buf[34..38].copy_from_slice(&h.data_start.to_le_bytes());
    // bytes 38..40: _reserved2 (alignment)
    // Compute CRC over bytes [0..0x28] then write at [0x28..0x2C]
    let crc = compute_row_crc(&buf[0..0x28]);
    buf[0x28..0x2C].copy_from_slice(&crc.to_le_bytes());
    buf
}

/// Decode segment header from a 16 KB page; verifies CRC32C.
pub fn decode_segment_header(buf: &[u8; 16384]) -> Result<SegmentHeader, RowDecodeError> {
    let expected_crc = u32::from_le_bytes(buf[0x28..0x2C].try_into().unwrap());
    if !verify_row_crc(&buf[0..0x28], expected_crc) {
        let computed = compute_row_crc(&buf[0..0x28]);
        return Err(RowDecodeError::CrcMismatch {
            computed,
            expected: expected_crc,
        });
    }
    let magic: [u8; 8] = buf[0..8].try_into().unwrap();
    Ok(SegmentHeader {
        magic,
        version: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        flags: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        ts: u64::from_le_bytes(buf[16..24].try_into().unwrap()),
        col_count: u16::from_le_bytes(buf[24..26].try_into().unwrap()),
        row_count: u32::from_le_bytes(buf[26..30].try_into().unwrap()),
        _reserved1: u16::from_le_bytes(buf[30..32].try_into().unwrap()),
        schema_offset: u16::from_le_bytes(buf[32..34].try_into().unwrap()),
        data_start: u32::from_le_bytes(buf[34..38].try_into().unwrap()),
        _reserved2: u16::from_le_bytes(buf[38..40].try_into().unwrap()),
        header_crc: expected_crc,
    })
}

/// Encode segment footer (16 bytes with CRC32C).
pub fn encode_segment_footer(f: &SegmentFooter) -> [u8; 16] {
    let mut buf = [0u8; 16];
    buf[0..4].copy_from_slice(&f.row_count.to_le_bytes());
    buf[4..8].copy_from_slice(&f.segment_size.to_le_bytes());
    buf[8..12].copy_from_slice(&f.next_offset.to_le_bytes());
    // bytes 12..16 hold CRC32C; compute over [0..12] then write
    let crc = compute_row_crc(&buf[0..12]);
    buf[12..16].copy_from_slice(&crc.to_le_bytes());
    buf
}

/// Decode segment footer; verifies CRC32C.
pub fn decode_segment_footer(buf: &[u8; 16]) -> Result<SegmentFooter, RowDecodeError> {
    let expected_crc = u32::from_le_bytes(buf[12..16].try_into().unwrap());
    if !verify_row_crc(&buf[0..12], expected_crc) {
        let computed = compute_row_crc(&buf[0..12]);
        return Err(RowDecodeError::CrcMismatch {
            computed,
            expected: expected_crc,
        });
    }
    Ok(SegmentFooter {
        row_count: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
        segment_size: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        next_offset: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        footer_crc: expected_crc,
    })
}

/// Top-level error type for BINT v3 segment operations.
///
/// Wraps [`RowDecodeError`] (low-level decode failures) and adds higher-level
/// corruption variants for caller diagnostics.
#[derive(Debug, Error)]
pub enum BinSegmentError {
    #[error(
        "row {} CRC32C mismatch: computed {computed:#x}, expected {expected:#x}",
        row_id
    )]
    RowChecksumMismatch {
        row_id: u64,
        computed: u32,
        expected: u32,
    },
    #[error("data corruption at {location}: {detail}")]
    DataCorruption { location: String, detail: String },
    #[error("index corruption in {index_file}: {detail}")]
    IndexCorruption { index_file: String, detail: String },
    #[error("row decode failed: {0}")]
    Decode(#[from] RowDecodeError),
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
        assert_eq!(
            column_width(&make_col("v", "VARCHAR(255)", Some(255))),
            None
        );
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
        let values = vec![Some(42i32.to_le_bytes().to_vec()), Some(b"alice".to_vec())];
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
        let schema = vec![make_col("a", "INT", None), make_col("b", "TEXT", None)];
        let values = vec![Some(7i32.to_le_bytes().to_vec()), None];
        let row = encode_row(&schema, &values, 1);
        let decoded = decode_row(&schema, &row).unwrap();
        assert!(decoded[1].is_none());
    }

    // ============================================================
    // Task 1.6: Segment header + footer tests
    // ============================================================

    #[test]
    fn test_segment_header_size_constant() {
        use std::mem::size_of;
        assert_eq!(size_of::<SegmentHeader>(), 0x2C); // 44 bytes (CRC32C at 0x28..0x2C)
    }

    #[test]
    fn test_segment_header_roundtrip() {
        let h = SegmentHeader {
            magic: *SEGMENT_MAGIC,
            version: SEGMENT_VERSION,
            flags: 0,
            ts: 1234567890,
            col_count: 5,
            row_count: 100,
            _reserved1: 0,
            schema_offset: 0x0020,
            data_start: DATA_START_OFFSET,
            _reserved2: 0,
            header_crc: 0, // filled by encoder
        };
        let buf = encode_segment_header(&h);
        assert_eq!(buf.len(), 16384);
        let arr: [u8; 16384] = buf[..16384].try_into().unwrap();
        let h2 = decode_segment_header(&arr).unwrap();
        // Copy fields to avoid unaligned reference errors with packed struct
        let version = h2.version;
        let col_count = h2.col_count;
        let row_count = h2.row_count;
        assert_eq!(version, SEGMENT_VERSION);
        assert_eq!(col_count, 5);
        assert_eq!(row_count, 100);
    }

    #[test]
    fn test_segment_header_detects_corruption() {
        let h = SegmentHeader {
            magic: *SEGMENT_MAGIC,
            version: SEGMENT_VERSION,
            flags: 0,
            ts: 0,
            col_count: 1,
            row_count: 0,
            _reserved1: 0,
            schema_offset: 0x0020,
            data_start: DATA_START_OFFSET,
            _reserved2: 0,
            header_crc: 0,
        };
        let mut buf = encode_segment_header(&h);
        buf[10] ^= 0xFF; // corrupt version field
        let arr: [u8; 16384] = buf[..16384].try_into().unwrap();
        let result = decode_segment_header(&arr);
        assert!(matches!(result, Err(RowDecodeError::CrcMismatch { .. })));
    }

    #[test]
    fn test_segment_footer_roundtrip() {
        let f = SegmentFooter {
            row_count: 42,
            segment_size: 0xDEADBEEFu32,
            next_offset: 0xCAFEBABEu32,
            footer_crc: 0,
        };
        let buf = encode_segment_footer(&f);
        assert_eq!(buf.len(), 16);
        let f2 = decode_segment_footer(&buf).unwrap();
        assert_eq!(f2.row_count, 42);
        assert_eq!(f2.segment_size, 0xDEADBEEFu32);
        assert_eq!(f2.next_offset, 0xCAFEBABEu32);
    }

    // ============================================================
    // Task 1.7: SegmentWriter streaming append + seal
    // ============================================================

    #[test]
    fn test_segment_writer_append_and_seal() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin");
        let schema = vec![
            make_col("id", "INT", None),
            make_col("name", "VARCHAR(255)", None),
        ];
        let mut w = SegmentWriter::new(path.clone(), schema.clone()).unwrap();
        for i in 0..100 {
            let values = vec![
                Some((i as i32).to_le_bytes().to_vec()),
                Some(format!("name_{}", i).into_bytes()),
            ];
            let row_id = w.append(&values).unwrap();
            assert_eq!(row_id, i as u64);
        }
        w.seal().unwrap();
        // File should exist and be page-aligned (16384 multiple)
        let meta = std::fs::metadata(&path).unwrap();
        assert_eq!(meta.len() % 16384, 0);
    }

    #[test]
    fn test_segment_writer_respects_size_cap() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("cap.bin");
        let schema = vec![make_col("data", "TEXT", None)];
        let mut w = SegmentWriter::with_size_cap(path.clone(), schema, 32 * 1024).unwrap();
        let big = vec![b'x'; 4096];
        // Append 4 KB rows; cap is 32 KB → ~7 should fit, 8th should fail
        let mut success = 0;
        for _ in 0..20 {
            match w.append(&[Some(big.clone())]) {
                Ok(_) => success += 1,
                Err(_) => break,
            }
        }
        assert!(success > 0 && success < 20);
    }

    // ============================================================
    // Task 1.8: SegmentReader round-trip + corruption skip
    // ============================================================

    #[test]
    fn test_segment_writer_reader_roundtrip() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("rt.bin");
        let schema = vec![
            make_col("id", "INT", None),
            make_col("name", "VARCHAR(255)", None),
        ];
        let mut w = SegmentWriter::new(path.clone(), schema.clone()).unwrap();
        let n = 1000;
        for i in 0..n {
            let values = vec![
                Some((i as i32).to_le_bytes().to_vec()),
                Some(format!("name_{}", i).into_bytes()),
            ];
            w.append(&values).unwrap();
        }
        w.seal().unwrap();
        let reader = SegmentReader::open(&path, schema).unwrap();
        let rows: Vec<_> = reader.iter_rows().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), n as usize);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row[0].as_ref().unwrap(), &(i as i32).to_le_bytes().to_vec());
            let expected_name = format!("name_{}", i);
            assert_eq!(row[1].as_ref().unwrap(), &expected_name.into_bytes());
        }
    }

    #[test]
    fn test_segment_reader_skips_corrupted_rows() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("corrupt.bin");
        let schema = vec![make_col("x", "INT", None)];
        let mut w = SegmentWriter::new(path.clone(), schema.clone()).unwrap();
        for i in 0..10 {
            w.append(&[Some((i as i32).to_le_bytes().to_vec())])
                .unwrap();
        }
        w.seal().unwrap();
        // Tamper with row 5's CRC byte (last 4 bytes of the row at known offset)
        let mut bytes = std::fs::read(&path).unwrap();
        // Find the 5th row: read first row's row_size header to advance, OR just iterate.
        // Simpler: data starts at DATA_START_OFFSET; each row has 16-byte header showing row_size.
        // First row: header[0..4]=row_size (LE), so first row_size is at DATA_START_OFFSET.
        let first_row_size = u32::from_le_bytes([
            bytes[DATA_START_OFFSET as usize],
            bytes[DATA_START_OFFSET as usize + 1],
            bytes[DATA_START_OFFSET as usize + 2],
            bytes[DATA_START_OFFSET as usize + 3],
        ]) as usize;
        let row5_start = DATA_START_OFFSET as usize + 5 * first_row_size;
        if row5_start + first_row_size <= bytes.len() {
            bytes[row5_start + first_row_size - 1] ^= 0xFF; // corrupt last byte (CRC)
        }
        std::fs::write(&path, &bytes).unwrap();
        let reader = SegmentReader::open(&path, schema).unwrap();
        let rows: Vec<_> = reader.iter_rows().filter_map(|r| r.ok()).collect();
        assert!(rows.len() < 10); // at least one row was skipped
        assert!(rows.len() >= 9); // but most were valid
    }

    // ============================================================
    // Task 1.9: BinSegmentError
    // ============================================================

    #[test]
    fn test_bin_segment_error_display() {
        let e = BinSegmentError::RowChecksumMismatch {
            row_id: 42,
            computed: 0xDEAD,
            expected: 0xBEEF,
        };
        let s = format!("{}", e);
        assert!(s.contains("row 42"));
        assert!(s.contains("dead"));
        assert!(s.contains("beef"));

        let e2 = BinSegmentError::DataCorruption {
            location: "seg_000.bin".into(),
            detail: "invalid magic".into(),
        };
        let s2 = format!("{}", e2);
        assert!(s2.contains("seg_000.bin"));
        assert!(s2.contains("invalid magic"));

        let e3 = BinSegmentError::IndexCorruption {
            index_file: "root.bin".into(),
            detail: "CRC mismatch".into(),
        };
        assert!(format!("{}", e3).contains("root.bin"));
    }

    #[test]
    fn test_from_row_decode_error() {
        let inner = RowDecodeError::CrcMismatch {
            computed: 0xDEAD,
            expected: 0xBEEF,
        };
        let wrapped: BinSegmentError = inner.into();
        match wrapped {
            BinSegmentError::Decode(RowDecodeError::CrcMismatch { .. }) => {}
            other => panic!("expected Decode(CrcMismatch), got {:?}", other),
        }
    }
}
