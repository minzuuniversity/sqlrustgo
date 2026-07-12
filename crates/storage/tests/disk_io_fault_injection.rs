//! Integration tests for Disk I/O delay fault injection (T-19).
//!
//! These tests verify [`IoFaultInjector`] behavior with real file I/O
//! to confirm that delay, corruption, and dropout interact correctly
//! with the filesystem.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tempfile::TempDir;

use sqlrustgo_storage::{IoDelayConfig, IoFaultInjector};

/// Helper: create a temp file with known content.
fn setup_temp_file(dir: &Path, name: &str, content: &[u8]) -> std::io::Result<File> {
    let path = dir.join(name);
    let mut file = File::create(&path)?;
    file.write_all(content)?;
    file.sync_all()?;
    // Re-open for reading
    File::open(&path)
}

// -----------------------------------------------------------------------
// Delay integration tests
// -----------------------------------------------------------------------

#[test]
fn test_integration_read_delay() {
    let dir = TempDir::new().unwrap();
    let content = b"disk io delay test data";
    let mut file = setup_temp_file(dir.path(), "delay_read.bin", content).unwrap();

    let config = IoDelayConfig {
        delay_ms: 30,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let mut buf = vec![0u8; content.len()];
    let start = std::time::Instant::now();
    let result = injector.apply_read(&mut buf, |b| file.read_exact(b));
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "read should succeed: {:?}", result);
    assert_eq!(buf.as_slice(), content);
    assert!(
        elapsed.as_millis() >= 30,
        "read should be delayed >=30ms, got {:?}",
        elapsed
    );
}

#[test]
fn test_integration_write_delay() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("delay_write.bin");
    let mut file = File::create(&path).unwrap();

    let config = IoDelayConfig {
        delay_ms: 25,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let data = b"write delay integration test";
    let start = std::time::Instant::now();
    let result = injector.apply_write(data.as_slice(), |d| file.write_all(d));
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "write should succeed: {:?}", result);
    // Verify content was written
    let mut readback = Vec::new();
    File::open(&path)
        .unwrap()
        .read_to_end(&mut readback)
        .unwrap();
    assert_eq!(readback, data);
    assert!(
        elapsed.as_millis() >= 25,
        "write should be delayed >=25ms, got {:?}",
        elapsed
    );
}

// -----------------------------------------------------------------------
// Dropout integration tests
// -----------------------------------------------------------------------

#[test]
fn test_integration_dropout_all_reads_fail() {
    let dir = TempDir::new().unwrap();
    let mut file = setup_temp_file(dir.path(), "dropout_read.bin", b"data").unwrap();

    let config = IoDelayConfig {
        dropout_rate: 1.0,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let mut buf = [0u8; 4];
    let result: std::io::Result<()> = injector.apply_read(&mut buf, |b| file.read_exact(b));
    assert!(
        result.is_err(),
        "all reads should fail with dropout_rate=1.0"
    );
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::ConnectionReset);
}

#[test]
fn test_integration_dropout_all_writes_fail() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("dropout_write.bin");
    let mut file = File::create(&path).unwrap();

    let config = IoDelayConfig {
        dropout_rate: 1.0,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let result: std::io::Result<()> = injector.apply_write(b"data", |d| file.write_all(d));
    assert!(
        result.is_err(),
        "all writes should fail with dropout_rate=1.0"
    );
}

// -----------------------------------------------------------------------
// Corruption integration tests
// -----------------------------------------------------------------------

#[test]
fn test_integration_corruption_detected() {
    let dir = TempDir::new().unwrap();
    let original = b"0123456789ABCDEFGH";
    let mut file = setup_temp_file(dir.path(), "corrupt.bin", original).unwrap();

    let config = IoDelayConfig {
        corruption_rate: 1.0,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let mut buf = [0u8; 18];
    let _ = injector.apply_read(&mut buf, |b| file.read_exact(b));

    // With corruption_rate=1.0, data should be different
    assert_ne!(buf, *original, "data should be corrupted");
}

// -----------------------------------------------------------------------
// Combined: delay + dropout + corruption
// -----------------------------------------------------------------------

#[test]
fn test_integration_delay_then_dropout() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("combined.bin");
    let mut file = File::create(&path).unwrap();

    // delay + dropout: first operation gets delayed, then fails
    let config = IoDelayConfig {
        delay_ms: 10,
        dropout_rate: 1.0,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let start = std::time::Instant::now();
    let result: std::io::Result<()> =
        injector.apply_read(&mut [0u8; 4], |_b| file.read_exact(&mut [0u8; 4]));
    let elapsed = start.elapsed();

    assert!(result.is_err(), "should fail due to dropout");
    assert!(
        elapsed.as_millis() >= 10,
        "delay should still apply even when dropout triggers, got {:?}",
        elapsed
    );
}

// -----------------------------------------------------------------------
// No-fault pass-through
// -----------------------------------------------------------------------

#[test]
fn test_integration_no_fault() {
    let dir = TempDir::new().unwrap();
    let mut file = setup_temp_file(dir.path(), "nofault.bin", b"hello world").unwrap();

    let mut injector = IoFaultInjector::new(IoDelayConfig::default());
    let mut buf = [0u8; 11];
    let result = injector.apply_read(&mut buf, |b| file.read_exact(b));

    assert!(result.is_ok());
    assert_eq!(&buf, b"hello world");
}

// -----------------------------------------------------------------------
// Write-then-read round-trip with delay
// -----------------------------------------------------------------------

#[test]
fn test_integration_write_read_roundtrip_with_delay() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("roundtrip.bin");

    // Write with delay
    let mut file = File::create(&path).unwrap();
    let config = IoDelayConfig {
        delay_ms: 15,
        ..Default::default()
    };
    let mut injector = IoFaultInjector::new(config);

    let payload = b"roundtrip test payload!";
    injector
        .apply_write(payload.as_slice(), |d| file.write_all(d))
        .unwrap();
    drop(file);

    // Read back with delay
    let mut file = File::open(&path).unwrap();
    let mut buf = vec![0u8; payload.len()];
    injector
        .apply_read(&mut buf, |b| file.read_exact(b))
        .unwrap();

    assert_eq!(buf, payload, "roundtrip data should match");
}
