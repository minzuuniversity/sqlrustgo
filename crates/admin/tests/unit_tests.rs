//! Unit tests for sqlrustgo-admin public API.

use sqlrustgo_admin::{
    backup::{tar_extract_all, tar_extract_one, BackupError, BackupResult},
    manifest::{sha256_bytes, sha256_file, walk_files, FileEntry, Manifest},
    pitr::{parse_target_time, PitrResult},
    restore::RestoreResult,
    verify::{verify_extracted, VerifyError, VerifyErrorKind, VerifyResult},
    wire_client::{LogicalBackupResult, StatusReport, WireError},
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================================
// manifest module
// ============================================================================

#[test]
fn test_sha256_bytes_empty() {
    assert_eq!(
        sha256_bytes(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_sha256_bytes_hello() {
    assert_eq!(
        sha256_bytes(b"hello world"),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn test_sha256_bytes_a() {
    assert_eq!(
        sha256_bytes(b"a"),
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    );
}

#[test]
fn test_sha256_file_basic() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("test.txt");
    fs::write(&file_path, "hello").unwrap();
    assert_eq!(sha256_file(&file_path).unwrap(), sha256_bytes(b"hello"));
}

#[test]
fn test_sha256_file_not_found() {
    assert!(sha256_file(Path::new("/nonexistent")).is_err());
}

#[test]
fn test_walk_files_empty() {
    let tmp = TempDir::new().unwrap();
    assert!(walk_files(tmp.path()).unwrap().is_empty());
}

#[test]
fn test_walk_files_nested() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("a/b")).unwrap();
    fs::write(tmp.path().join("a/file1.txt"), "1").unwrap();
    fs::write(tmp.path().join("a/b/file2.txt"), "2").unwrap();
    let files = walk_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn test_file_entry_fields() {
    let entry = FileEntry {
        path: "/data/t1.bin".to_string(),
        size: 1024,
        sha256: "abc123".to_string(),
    };
    assert_eq!(entry.path, "/data/t1.bin");
    assert_eq!(entry.size, 1024);
}

#[test]
fn test_manifest_new_empty() {
    let manifest = Manifest::new();
    assert_eq!(manifest.version, 1);
    assert!(manifest.data_files.is_empty());
    assert!(manifest.wal_file.is_none());
}

#[test]
fn test_manifest_with_entries() {
    let mut manifest = Manifest::new();
    manifest.data_files.push(FileEntry {
        path: "/t1.bin".to_string(),
        size: 100,
        sha256: "h1".to_string(),
    });
    manifest.data_files.push(FileEntry {
        path: "/t2.bin".to_string(),
        size: 200,
        sha256: "h2".to_string(),
    });
    manifest.total_size_bytes = 300;
    assert_eq!(manifest.data_files.len(), 2);
    assert_eq!(manifest.total_size_bytes, 300);
}

#[test]
fn test_manifest_to_from_json() {
    let mut manifest = Manifest::new();
    manifest.data_files.push(FileEntry {
        path: "/t1.bin".to_string(),
        size: 100,
        sha256: "h".to_string(),
    });
    let json = manifest.to_json().unwrap();
    let parsed = Manifest::from_json(&json).unwrap();
    assert_eq!(parsed.version, manifest.version);
    assert_eq!(parsed.data_files.len(), manifest.data_files.len());
}

#[test]
fn test_manifest_from_json_invalid() {
    assert!(Manifest::from_json("not json").is_err());
}

// ============================================================================
// backup module
// ============================================================================

#[test]
fn test_tar_extract_all_empty() {
    let tmp = TempDir::new().unwrap();
    let tar_path = tmp.path().join("empty.tar");
    fs::write(&tar_path, vec![0u8; 1024]).unwrap();
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    assert!(tar_extract_all(&tar_path, &out).is_ok());
}

#[test]
fn test_tar_extract_one_not_found() {
    let tmp = TempDir::new().unwrap();
    let tar_path = tmp.path().join("test.tar");
    fs::write(&tar_path, vec![0u8; 1024]).unwrap();
    assert!(tar_extract_one(&tar_path, "nonexistent").is_err());
}

#[test]
fn test_backup_error_io_display() {
    let err = BackupError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let display = format!("{}", err);
    assert!(display.contains("file not found") || display.contains("IO"));
}

#[test]
fn test_backup_result_fields() {
    let manifest = Manifest::new();
    let result = BackupResult {
        manifest,
        output_path: PathBuf::from("/tmp/bak"),
        output_size_bytes: 1024,
        manifest_sha256: "abc123".to_string(),
    };
    assert_eq!(result.output_size_bytes, 1024);
}

// ============================================================================
// pitr module
// ============================================================================

#[test]
fn test_parse_target_time_valid() {
    assert_eq!(parse_target_time("1700000000").unwrap(), 1700000000);
}

#[test]
fn test_parse_target_time_invalid() {
    assert!(parse_target_time("").is_err());
    assert!(parse_target_time("abc").is_err());
    assert!(parse_target_time("-1").is_err());
}

#[test]
fn test_pitr_result_fields() {
    let result = PitrResult {
        target_time: 1699999999,
        entries_scanned: 100,
        entries_applied: 10,
        entries_skipped: 5,
        transactions_committed: 3,
        transactions_aborted: 1,
        active_transactions_at_target: 0,
    };
    assert_eq!(result.target_time, 1699999999);
    assert_eq!(result.entries_applied, 10);
    assert_eq!(result.transactions_committed, 3);
}

// ============================================================================
// verify module
// ============================================================================

#[test]
fn test_verify_error_kind_checksum() {
    let err = VerifyErrorKind::ChecksumMismatch {
        expected: "abc".to_string(),
        actual: "def".to_string(),
    };
    let display = format!("{:?}", err);
    assert!(display.contains("ChecksumMismatch"));
}

#[test]
fn test_verify_error_kind_file_missing() {
    let err = VerifyErrorKind::FileMissing;
    let display = format!("{:?}", err);
    assert!(display.contains("FileMissing"));
}

#[test]
fn test_verify_error_kind_wal_missing() {
    let err = VerifyErrorKind::WalMissing;
    let display = format!("{:?}", err);
    assert!(display.contains("WalMissing"));
}

#[test]
fn test_verify_error_display() {
    let err = VerifyError {
        kind: VerifyErrorKind::FileMissing,
        path: "/backup/data.bin".to_string(),
    };
    let display = format!("{}", err);
    assert!(display.contains("data.bin"));
}

#[test]
fn test_verify_result_no_errors() {
    let manifest = Manifest::new();
    let result = VerifyResult {
        manifest,
        errors: vec![],
        verified_files: 5,
    };
    assert!(result.errors.is_empty());
    assert_eq!(result.verified_files, 5);
}

#[test]
fn test_verify_result_with_errors() {
    let manifest = Manifest::new();
    let result = VerifyResult {
        manifest,
        errors: vec![VerifyError {
            kind: VerifyErrorKind::FileMissing,
            path: "/t1.bin".to_string(),
        }],
        verified_files: 10,
    };
    assert!(!result.errors.is_empty());
    assert_eq!(result.errors[0].path, "/t1.bin");
}

// ============================================================================
// restore module
// ============================================================================

#[test]
fn test_restore_result_fields() {
    let manifest = Manifest::new();
    let result = RestoreResult {
        manifest,
        restored_data_files: 3,
        restored_wal: true,
    };
    assert_eq!(result.restored_data_files, 3);
    assert!(result.restored_wal);
}

// ============================================================================
// wire_client module
// ============================================================================

#[test]
fn test_wire_error_connect() {
    let err = WireError::Connect("refused".to_string());
    assert!(format!("{}", err).contains("refused"));
}

#[test]
fn test_wire_error_query() {
    let err = WireError::Query("syntax error".to_string());
    assert!(format!("{}", err).contains("syntax error"));
}

#[test]
fn test_wire_error_io() {
    let err = WireError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
    assert!(format!("{}", err).contains("gone"));
}

#[test]
fn test_wire_error_protocol() {
    let err = WireError::Protocol("bad shape".to_string());
    assert!(format!("{}", err).contains("bad shape"));
}

#[test]
fn test_status_report_fields() {
    let report = StatusReport {
        server_version: "8.0.30".to_string(),
        total_queries: 1000,
        slow_queries: 10,
        uptime_seconds: 3600,
        active_connections: 5,
    };
    assert_eq!(report.server_version, "8.0.30");
    assert_eq!(report.uptime_seconds, 3600);
    assert_eq!(report.active_connections, 5);
}

#[test]
fn test_logical_backup_result_fields() {
    let result = LogicalBackupResult {
        tables: vec!["t1".to_string(), "t2".to_string()],
        row_counts: std::collections::HashMap::from_iter([
            ("t1".to_string(), 100),
            ("t2".to_string(), 200),
        ]),
        output_path: "/backup".to_string(),
        output_size_bytes: 1024,
    };
    assert_eq!(result.tables.len(), 2);
    assert_eq!(result.tables[0], "t1");
    assert_eq!(result.row_counts.get("t1"), Some(&100));
}
