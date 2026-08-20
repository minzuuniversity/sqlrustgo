// F-27: Table Compression (LZ4/zstd)
// Issue: #2828
// Change: openspec/changes/f-27-table-compression
//
// Real storage integration. Uses sqlrustgo_common::compression.

use sqlrustgo_common::compression::{CompressionAlgorithm, TableCompressor};

#[test]
fn test_compress_basic() {
    let mut comp = TableCompressor::new();
    let data = vec![b'x'; 1000]; // highly compressible
    let size = comp.compress("t1", &data);
    assert!(size < data.len(), "should compress to smaller size");
}

#[test]
fn test_decompress_roundtrip() {
    let mut comp = TableCompressor::new();
    let original = b"hello world hello world hello world".to_vec();
    comp.compress("t1", &original);
    let result = comp.decompress("t1").unwrap();
    assert_eq!(result.as_slice(), original.as_slice());
}

#[test]
fn test_compression_ratio() {
    let mut comp = TableCompressor::new();
    let data = vec![b'x'; 1000];
    comp.compress("t1", &data);
    let ratio = comp.ratio("t1");
    assert!(ratio > 1.0, "ratio should be > 1 for repetitive data");
}

#[test]
fn test_multiple_tables() {
    let mut comp = TableCompressor::new();
    comp.compress("users", b"alice bob carol");
    comp.compress("orders", b"order1 order2 order3 order4");
    assert!(comp.decompress("users").is_some());
    assert!(comp.decompress("orders").is_some());
    assert!(comp.decompress("unknown").is_none());
}

#[test]
fn test_empty_data() {
    let mut comp = TableCompressor::new();
    comp.compress("t", b"");
    let result = comp.decompress("t").unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_lz4_algorithm() {
    let mut comp = TableCompressor::with_algorithm(CompressionAlgorithm::Lz4);
    let data = vec![b'y'; 500];
    comp.compress("t1", &data);
    assert_eq!(comp.algorithm("t1"), Some(CompressionAlgorithm::Lz4));
    let ratio = comp.ratio("t1");
    assert!(ratio > 1.0, "LZ4 should compress repetitive data");
}

#[test]
fn test_random_data_poor_compression() {
    let mut comp = TableCompressor::new();
    // Random-ish data doesn't compress well
    let data: Vec<u8> = (0..100).map(|i| i as u8).collect();
    let original_size = data.len();
    comp.compress("t1", &data);
    // Decompression should still work
    assert!(comp.decompress("t1").unwrap().len() == original_size);
}

#[test]
fn test_compression_stats() {
    let mut comp = TableCompressor::new();
    comp.compress("t1", &[b'z'; 200]);
    assert_eq!(comp.original_size("t1"), 200);
    assert!(comp.compressed_size("t1") > 0);
    assert!(comp.compressed_size("t1") < 200);
    let stats = comp.stats("t1").unwrap();
    assert_eq!(stats.original_size, 200);
}
