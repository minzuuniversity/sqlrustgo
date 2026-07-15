//! Compression module for SQLRustGo
//!
//! Provides LZ4, zstd, and zlib (flate2) compression algorithms.
//! Used for table compression (F-27) and page compression.

use std::collections::HashMap;
use std::io::Write;

/// Compression algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// LZ4 - fast compression, good for hot data
    Lz4,
    /// Zstd - balanced speed/ratio, good default
    Zstd,
    /// zlib/DEFLATE - widely compatible
    Zlib,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        CompressionAlgorithm::Zstd
    }
}

/// Table compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub ratio: f64,
    pub algorithm: CompressionAlgorithm,
}

impl CompressionStats {
    /// Calculate compression ratio
    pub fn ratio(&self) -> f64 {
        if self.compressed_size == 0 {
            0.0
        } else {
            self.original_size as f64 / self.compressed_size as f64
        }
    }
}

/// Table compressor supporting multiple algorithms
pub struct TableCompressor {
    compressed: HashMap<String, Vec<u8>>,
    stats: HashMap<String, CompressionStats>,
    default_algorithm: CompressionAlgorithm,
}

impl TableCompressor {
    /// Create a new TableCompressor with default algorithm (zstd)
    pub fn new() -> Self {
        Self {
            compressed: HashMap::new(),
            stats: HashMap::new(),
            default_algorithm: CompressionAlgorithm::Zstd,
        }
    }

    /// Create with specific default algorithm
    pub fn with_algorithm(algorithm: CompressionAlgorithm) -> Self {
        Self {
            compressed: HashMap::new(),
            stats: HashMap::new(),
            default_algorithm: algorithm,
        }
    }

    /// Compress data using the specified algorithm
    pub fn compress_with(&mut self, table: &str, data: &[u8], algorithm: CompressionAlgorithm) -> usize {
        let original_size = data.len();
        let compressed = match algorithm {
            CompressionAlgorithm::Lz4 => compress_lz4(data),
            CompressionAlgorithm::Zstd => compress_zstd(data),
            CompressionAlgorithm::Zlib => compress_zlib(data),
        };
        let compressed_size = compressed.len();
        
        self.compressed.insert(table.to_string(), compressed);
        self.stats.insert(table.to_string(), CompressionStats {
            original_size,
            compressed_size,
            ratio: if compressed_size > 0 {
                original_size as f64 / compressed_size as f64
            } else {
                0.0
            },
            algorithm,
        });
        
        compressed_size
    }

    /// Compress data using the default algorithm (zstd)
    pub fn compress(&mut self, table: &str, data: &[u8]) -> usize {
        self.compress_with(table, data, self.default_algorithm)
    }

    /// Decompress data for a table
    pub fn decompress(&self, table: &str) -> Option<Vec<u8>> {
        let data = self.compressed.get(table)?;
        let stats = self.stats.get(table)?;
        
        let decompressed = match stats.algorithm {
            CompressionAlgorithm::Lz4 => decompress_lz4(data).ok()?,
            CompressionAlgorithm::Zstd => decompress_zstd(data).ok()?,
            CompressionAlgorithm::Zlib => decompress_zlib(data).ok()?,
        };
        
        Some(decompressed)
    }

    /// Get compression stats for a table
    pub fn stats(&self, table: &str) -> Option<&CompressionStats> {
        self.stats.get(table)
    }

    /// Get compression ratio for a table
    pub fn ratio(&self, table: &str) -> f64 {
        self.stats.get(table).map(|s| s.ratio()).unwrap_or(0.0)
    }

    /// Get compressed size
    pub fn compressed_size(&self, table: &str) -> usize {
        self.stats.get(table).map(|s| s.compressed_size).unwrap_or(0)
    }

    /// Get original size
    pub fn original_size(&self, table: &str) -> usize {
        self.stats.get(table).map(|s| s.original_size).unwrap_or(0)
    }

    /// Get algorithm used for a table
    pub fn algorithm(&self, table: &str) -> Option<CompressionAlgorithm> {
        self.stats.get(table).map(|s| s.algorithm)
    }

    /// List all compressed tables
    pub fn tables(&self) -> Vec<String> {
        self.compressed.keys().cloned().collect()
    }

    /// Clear all compressed data
    pub fn clear(&mut self) {
        self.compressed.clear();
        self.stats.clear();
    }
}

impl Default for TableCompressor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LZ4 compression
// ============================================================================

/// Compress data using LZ4
pub fn compress_lz4(data: &[u8]) -> Vec<u8> {
    lz4_flex::compress_prepend_size(data)
}

/// Decompress LZ4 data
pub fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    lz4_flex::decompress_size_prepended(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// ============================================================================
// Zstd compression
// ============================================================================

/// Compress data using zstd
pub fn compress_zstd(data: &[u8]) -> Vec<u8> {
    zstd::encode_all(data, 0).expect("zstd encode failed")
}

/// Decompress zstd data
pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    zstd::decode_all(std::io::Cursor::new(data))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// ============================================================================
// zlib/DEFLATE compression (using flate2)
// ============================================================================

/// Compress data using zlib/DEFLATE
pub fn compress_zlib(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).expect("zlib write failed");
    encoder.finish().expect("zlib finish failed")
}

/// Decompress zlib data
pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    
    let mut decoder = ZlibDecoder::new(data);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

// ============================================================================
// Convenience functions
// ============================================================================

/// Compress bytes using the best available algorithm (zstd)
pub fn compress(data: &[u8]) -> Vec<u8> {
    compress_zstd(data)
}

/// Decompress bytes compressed with compress()
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    decompress_zstd(data)
}

/// Get the name of an algorithm
pub fn algorithm_name(algo: CompressionAlgorithm) -> &'static str {
    match algo {
        CompressionAlgorithm::Lz4 => "LZ4",
        CompressionAlgorithm::Zstd => "Zstd",
        CompressionAlgorithm::Zlib => "zlib",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_roundtrip() {
        let data = b"hello world hello world hello world";
        let compressed = compress_lz4(data);
        let decompressed = decompress_lz4(&compressed).unwrap();
        assert_eq!(decompressed.as_slice(), data.as_slice());
    }

    #[test]
    fn test_zstd_roundtrip() {
        let data = b"hello world hello world hello world";
        let compressed = compress_zstd(data);
        let decompressed = decompress_zstd(&compressed).unwrap();
        assert_eq!(decompressed.as_slice(), data.as_slice());
    }

    #[test]
    fn test_zlib_roundtrip() {
        let data = b"hello world hello world hello world";
        let compressed = compress_zlib(data);
        let decompressed = decompress_zlib(&compressed).unwrap();
        assert_eq!(decompressed.as_slice(), data.as_slice());
    }

    #[test]
    fn test_table_compressor() {
        let mut comp = TableCompressor::new();
        let data = vec![b'x'; 1000]; // highly compressible
        comp.compress("t1", &data);
        
        let decompressed = comp.decompress("t1").unwrap();
        assert_eq!(decompressed.as_slice(), data.as_slice());
        assert!(comp.ratio("t1") > 1.0, "ratio should be > 1 for repetitive data");
    }

    #[test]
    fn test_table_compressor_multiple_algorithms() {
        let mut comp = TableCompressor::new();
        let data = b"test data for compression";
        
        comp.compress_with("t1", data, CompressionAlgorithm::Lz4);
        comp.compress_with("t2", data, CompressionAlgorithm::Zstd);
        comp.compress_with("t3", data, CompressionAlgorithm::Zlib);
        
        assert_eq!(comp.decompress("t1").unwrap(), data);
        assert_eq!(comp.decompress("t2").unwrap(), data);
        assert_eq!(comp.decompress("t3").unwrap(), data);
        
        assert_eq!(comp.algorithm("t1"), Some(CompressionAlgorithm::Lz4));
        assert_eq!(comp.algorithm("t2"), Some(CompressionAlgorithm::Zstd));
        assert_eq!(comp.algorithm("t3"), Some(CompressionAlgorithm::Zlib));
    }

    #[test]
    fn test_compression_ratio() {
        let mut comp = TableCompressor::new();
        let data = vec![b'x'; 1000]; // highly repetitive
        comp.compress("t1", &data);
        
        assert!(comp.ratio("t1") > 1.0, "ratio should be > 1 for repetitive data");
        assert_eq!(comp.original_size("t1"), 1000);
        assert!(comp.compressed_size("t1") < 1000);
    }

    #[test]
    fn test_algorithm_name() {
        assert_eq!(algorithm_name(CompressionAlgorithm::Lz4), "LZ4");
        assert_eq!(algorithm_name(CompressionAlgorithm::Zstd), "Zstd");
        assert_eq!(algorithm_name(CompressionAlgorithm::Zlib), "zlib");
    }
}
