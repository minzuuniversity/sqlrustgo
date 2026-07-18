//! Tests for the I/O trait abstraction in [`sqlrustgo_tools::traits`].

use sqlrustgo_tools::traits::{MockIo, RealIo, SqlRustGoIo};
use std::io;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// RealIo tests
// ---------------------------------------------------------------------------

/// Verify [`RealIo`] reads a file it just wrote.
#[test]
fn test_real_io_read_write() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.txt");

    let io = RealIo;
    io.write(&path, b"hello world").unwrap();
    let data = io.read(&path).unwrap();
    assert_eq!(data, b"hello world");
}

/// Verify [`RealIo::exists`] returns true for existing paths and false for missing.
#[test]
fn test_real_io_exists() {
    let io = RealIo;
    let tmp = TempDir::new().unwrap();
    assert!(io.exists(tmp.path()));
    assert!(!io.exists(tmp.path().join("nonexistent").as_path()));
}

/// Verify [`RealIo::create_dir_all`] creates a nested directory.
#[test]
fn test_real_io_create_dir_all() {
    let io = RealIo;
    let tmp = TempDir::new().unwrap();
    let nested = tmp.path().join("a").join("b").join("c");
    io.create_dir_all(&nested).unwrap();
    assert!(nested.is_dir());
}

/// Verify [`RealIo::remove_file`] deletes a file.
#[test]
fn test_real_io_remove_file() {
    let io = RealIo;
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("to_delete.txt");
    std::fs::write(&file, b"delete me").unwrap();
    assert!(file.exists());
    io.remove_file(&file).unwrap();
    assert!(!file.exists());
}

/// Verify [`RealIo::remove_dir_all`] deletes a directory recursively.
#[test]
fn test_real_io_remove_dir_all() {
    let io = RealIo;
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("to_delete");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("file.txt"), b"hello").unwrap();
    assert!(dir.exists());
    io.remove_dir_all(&dir).unwrap();
    assert!(!dir.exists());
}

/// Verify [`RealIo::read`] returns an error for missing files.
#[test]
fn test_real_io_read_missing() {
    let io = RealIo;
    let tmp = TempDir::new().unwrap();
    let result = io.read(&tmp.path().join("does_not_exist.txt"));
    assert!(result.is_err());
}

/// Verify [`RealIo::write`] can overwrite an existing file.
#[test]
fn test_real_io_write_overwrite() {
    let io = RealIo;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("overwrite.txt");
    io.write(&path, b"first").unwrap();
    io.write(&path, b"second").unwrap();
    assert_eq!(io.read(&path).unwrap(), b"second");
}

// ---------------------------------------------------------------------------
// MockIo tests
// ---------------------------------------------------------------------------

/// Verify [`MockIo::happy`] exists returns true by default.
#[test]
fn test_mock_io_happy_exists() {
    let io = MockIo::happy();
    assert!(io.exists(std::path::Path::new("/anything")));
    assert!(io.exists(std::path::Path::new("/nothing")));
}

/// Verify [`MockIo::exists`] with `with_missing` returns false.
#[test]
fn test_mock_io_with_missing() {
    let io = MockIo::happy().with_missing();
    assert!(!io.exists(std::path::Path::new("/any")));
    assert!(!io.exists(std::path::Path::new("/another")));
}

/// Verify [`MockIo::exists`] default returns true.
#[test]
fn test_mock_io_exists_default_true() {
    let io = MockIo::happy();
    assert!(io.exists(std::path::Path::new("/any")));
}

/// Verify [`MockIo`] with injected read error.
#[test]
fn test_mock_io_read_error() {
    let io = MockIo::happy().with_read_err(io::ErrorKind::PermissionDenied);
    let result = io.read(std::path::Path::new("/any/path"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

/// Verify [`MockIo`] with injected create_dir_all error.
#[test]
fn test_mock_io_create_dir_error() {
    let io = MockIo::happy().with_create_dir_err(io::ErrorKind::ReadOnlyFilesystem);
    let result = io.create_dir_all(std::path::Path::new("/any/path"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::ReadOnlyFilesystem);
}

/// Verify [`MockIo`] with injected write error via direct field.
#[test]
fn test_mock_io_write_error() {
    let mut io = MockIo::happy();
    *io.write_err.lock() = Some(io::Error::new(io::ErrorKind::Other, "mock write error"));
    let result = io.write(std::path::Path::new("/any/path"), b"data");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
}

/// Verify [`MockIo`] with injected remove_dir_all error via direct field.
#[test]
fn test_mock_io_remove_dir_error() {
    let mut io = MockIo::happy();
    *io.remove_dir_all_err.lock() = Some(io::Error::new(io::ErrorKind::Other, "mock dir error"));
    let result = io.remove_dir_all(std::path::Path::new("/any/path"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
}

/// Verify [`MockIo`] with injected remove_file error via direct field.
#[test]
fn test_mock_io_remove_file_error() {
    let mut io = MockIo::happy();
    *io.remove_file_err.lock() = Some(io::Error::new(io::ErrorKind::Other, "mock file error"));
    let result = io.remove_file(std::path::Path::new("/any/path"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
}

// ============================================================================
// Additional MockIo and RealIo tests
// ============================================================================

#[test]
fn test_mock_io_remove_dir_err_via_lock() {
    use sqlrustgo_tools::traits::{MockIo, SqlRustGoIo};
    let io = MockIo::happy();
    *io.remove_dir_all_err.lock() = Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "locked",
    ));
    let result = io.remove_dir_all(std::path::Path::new("/tmp/test"));
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn test_mock_io_happy_default() {
    use sqlrustgo_tools::traits::MockIo;
    let io = MockIo::happy();
    assert!(io.exists_return);
    assert!(io.read_err.lock().is_none());
    assert!(io.write_err.lock().is_none());
    assert!(io.create_dir_all_err.lock().is_none());
    assert!(io.remove_dir_all_err.lock().is_none());
    assert!(io.remove_file_err.lock().is_none());
}

#[test]
fn test_real_io_exists_false() {
    use sqlrustgo_tools::traits::{RealIo, SqlRustGoIo};
    let io = RealIo;
    let result = io.exists(std::path::Path::new("/nonexistent_path_12345"));
    assert!(!result);
}
