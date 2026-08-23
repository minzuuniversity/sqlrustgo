//! root.bin index file for BINT v3 tables.
//!
//! Format:
//!   u32: version
//!   u16: segment count
//!   reserved: 10 bytes
//!   per-segment: u32 id, u32 name_len, name_bytes, u32 row_count, u64 byte_size
//!   u64: schema_hash
//!   u64: total_rows
//!   u32: index_crc (over all preceding bytes)

use std::path::PathBuf;
use crc32c::Crc32cHasher;
use std::hash::Hasher;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentInfo {
    pub segment_id: u32,
    pub file_name: String,
    pub row_count: u32,
    pub byte_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootIndex {
    pub version: u32,
    pub segments: Vec<SegmentInfo>,
    pub schema_hash: u64,
    pub total_rows: u64,
    pub index_crc: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("buffer too short")]
    TooShort,
    #[error("CRC mismatch")]
    CrcMismatch,
    #[error("invalid version: {0}")]
    InvalidVersion(u32),
}

pub fn encode_root_index(idx: &RootIndex) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&idx.version.to_le_bytes());
    buf.extend_from_slice(&(idx.segments.len() as u16).to_le_bytes());
    buf.extend_from_slice(&[0u8; 10]); // reserved
    for s in &idx.segments {
        buf.extend_from_slice(&s.segment_id.to_le_bytes());
        let name_bytes = s.file_name.as_bytes();
        buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&s.row_count.to_le_bytes());
        buf.extend_from_slice(&s.byte_size.to_le_bytes());
    }
    buf.extend_from_slice(&idx.schema_hash.to_le_bytes());
    buf.extend_from_slice(&idx.total_rows.to_le_bytes());
    // Compute CRC over all preceding bytes
    let mut hasher = Crc32cHasher::default();
    hasher.write(&buf);
    let crc = hasher.finish() as u32;
    buf.extend_from_slice(&crc.to_le_bytes());
    buf
}

pub fn decode_root_index(buf: &[u8]) -> Result<RootIndex, IndexError> {
    if buf.len() < 4 + 2 + 10 + 8 + 8 + 4 {
        return Err(IndexError::TooShort);
    }
    let mut pos = 0;
    let version = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap()); pos += 4;
    if version != 3 {
        return Err(IndexError::InvalidVersion(version));
    }
    let seg_count = u16::from_le_bytes(buf[pos..pos+2].try_into().unwrap()) as usize; pos += 2;
    pos += 10; // reserved
    let mut segments = Vec::with_capacity(seg_count);
    for _ in 0..seg_count {
        if pos + 4 + 4 > buf.len() {
            return Err(IndexError::TooShort);
        }
        let segment_id = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap()); pos += 4;
        let name_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap()) as usize; pos += 4;
        if pos + name_len + 4 + 8 > buf.len() {
            return Err(IndexError::TooShort);
        }
        let file_name = String::from_utf8(buf[pos..pos+name_len].to_vec())
            .map_err(|_| IndexError::TooShort)?; pos += name_len;
        let row_count = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap()); pos += 4;
        let byte_size = u64::from_le_bytes(buf[pos..pos+8].try_into().unwrap()); pos += 8;
        segments.push(SegmentInfo { segment_id, file_name, row_count, byte_size });
    }
    if pos + 8 + 8 + 4 > buf.len() {
        return Err(IndexError::TooShort);
    }
    let schema_hash = u64::from_le_bytes(buf[pos..pos+8].try_into().unwrap()); pos += 8;
    let total_rows = u64::from_le_bytes(buf[pos..pos+8].try_into().unwrap()); pos += 8;
    let index_crc = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap());
    // Verify CRC
    let mut hasher = Crc32cHasher::default();
    hasher.write(&buf[..pos]);
    let computed = hasher.finish() as u32;
    if computed != index_crc {
        return Err(IndexError::CrcMismatch);
    }
    Ok(RootIndex { version, segments, schema_hash, total_rows, index_crc })
}

pub fn read_root_index_file(path: &PathBuf) -> Result<RootIndex, IndexError> {
    let buf = std::fs::read(path).map_err(|_| IndexError::TooShort)?;
    decode_root_index(&buf)
}

pub fn write_root_index_file(path: &PathBuf, idx: &RootIndex) -> std::io::Result<()> {
    let buf = encode_root_index(idx);
    std::fs::write(path, buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_index_roundtrip() {
        let idx = RootIndex {
            version: 3,
            segments: vec![
                SegmentInfo { segment_id: 0, file_name: "seg_000.bin".into(), row_count: 1000, byte_size: 65536 },
                SegmentInfo { segment_id: 1, file_name: "seg_001.bin".into(), row_count: 800, byte_size: 50000 },
            ],
            schema_hash: 0xCAFEBABEDEADBEEF,
            total_rows: 1800,
            index_crc: 0,
        };
        let buf = encode_root_index(&idx);
        assert!(buf.len() < 4096);
        let idx2 = decode_root_index(&buf).unwrap();
        assert_eq!(idx2.version, 3);
        assert_eq!(idx2.segments.len(), 2);
        assert_eq!(idx2.total_rows, 1800);
        assert_eq!(idx2.schema_hash, 0xCAFEBABEDEADBEEF);
    }

    #[test]
    fn test_root_index_detects_corruption() {
        let idx = RootIndex {
            version: 3,
            segments: vec![],
            schema_hash: 0,
            total_rows: 0,
            index_crc: 0,
        };
        let mut buf = encode_root_index(&idx);
        buf[5] ^= 0xFF;
        assert!(decode_root_index(&buf).is_err());
    }
}
