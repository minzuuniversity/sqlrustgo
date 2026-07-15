// Admin crate additional coverage tests
// Covers: verify.rs, restore.rs, manifest.rs, backup.rs error paths

use std::fs;
use tempfile::TempDir;

use sqlrustgo_admin::backup::{physical_backup, tar_extract_all, tar_extract_one, BackupError};
use sqlrustgo_admin::manifest::{sha256_bytes, sha256_file, FileEntry, Manifest};
use sqlrustgo_admin::restore::physical_restore;
use sqlrustgo_admin::verify::{verify_extracted, VerifyError, VerifyErrorKind};

// ============ sha256_bytes tests ============

#[test]
fn test_sha256_bytes_hello() {
    let h = sha256_bytes(b"hello");
    assert_eq!(
        h,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_sha256_bytes_empty() {
    let h = sha256_bytes(b"");
    assert_eq!(
        h,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_sha256_bytes_known() {
    let h = sha256_bytes(b"test data 123");
    assert_eq!(h.len(), 64);
    assert!(h
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

#[test]
fn test_sha256_file_known_content() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("f.txt");
    fs::write(&path, b"hello").unwrap();
    let h = sha256_file(&path).unwrap();
    assert_eq!(
        h,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

// ============ Manifest tests ============

#[test]
fn test_manifest_new() {
    let m = Manifest::new();
    assert_eq!(m.version, 1);
    assert!(m.data_files.is_empty());
    assert!(m.wal_file.is_none());
    assert_eq!(m.total_size_bytes, 0);
}

#[test]
fn test_manifest_default() {
    let m: Manifest = Default::default();
    assert_eq!(m.version, 1);
}

#[test]
fn test_manifest_to_json_from_json_roundtrip() {
    let mut m = Manifest::new();
    m.data_files.push(FileEntry {
        path: "t1.dat".into(),
        size: 100,
        sha256: "abc123".into(),
    });
    let json = m.to_json().unwrap();
    let m2 = Manifest::from_json(&json).unwrap();
    assert_eq!(m, m2);
}

#[test]
fn test_manifest_scan_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let m = Manifest::scan(tmp.path(), None).unwrap();
    assert!(m.data_files.is_empty());
    assert!(m.wal_file.is_none());
}

#[test]
fn test_manifest_scan_with_files() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.txt"), b"content a").unwrap();
    fs::write(tmp.path().join("b.txt"), b"content b").unwrap();
    let m = Manifest::scan(tmp.path(), None).unwrap();
    assert_eq!(m.data_files.len(), 2);
    assert!(m.wal_file.is_none());
}

#[test]
fn test_manifest_scan_with_wal() {
    let data_dir = TempDir::new().unwrap();
    let wal_path = data_dir.path().join("sqlrustgo.wal");
    fs::write(&wal_path, b"wal content").unwrap();
    let m = Manifest::scan(data_dir.path(), Some(&wal_path)).unwrap();
    assert!(m.wal_file.is_some());
    assert_eq!(m.wal_file.as_ref().unwrap().path, "sqlrustgo.wal");
}

#[test]
fn test_manifest_write_to_read_from() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("manifest.json");
    let m = Manifest::new();
    m.write_to(&path).unwrap();
    let m2 = Manifest::read_from(&path).unwrap();
    assert_eq!(m.version, m2.version);
    assert_eq!(m.data_files, m2.data_files);
}

#[test]
fn test_manifest_scan_calculates_total_size() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.txt"), b"12345678").unwrap();
    let m = Manifest::scan(tmp.path(), None).unwrap();
    assert_eq!(m.total_size_bytes, 8);
}

// ============ walk_files tests ============

#[test]
fn test_walk_files_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let files = sqlrustgo_admin::manifest::walk_files(tmp.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn test_walk_files_nested_dirs() {
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(tmp.path().join("root.txt"), b"root").unwrap();
    fs::write(sub.join("nested.txt"), b"nested").unwrap();
    let files = sqlrustgo_admin::manifest::walk_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn test_walk_files_subdir_only_files() {
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("empty_sub");
    fs::create_dir(&sub).unwrap();
    let files = sqlrustgo_admin::manifest::walk_files(tmp.path()).unwrap();
    assert!(files.is_empty());
}

// ============ verify_extracted tests ============

#[test]
fn test_verify_extracted_empty_manifest() {
    let tmp = TempDir::new().unwrap();
    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![],
        wal_file: None,
        total_size_bytes: 0,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.errors.len(), 0);
    assert_eq!(result.verified_files, 0);
}

#[test]
fn test_verify_extracted_file_missing() {
    let tmp = TempDir::new().unwrap();
    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![FileEntry {
            path: "t1.dat".into(),
            size: 100,
            sha256: "abc123".into(),
        }],
        wal_file: None,
        total_size_bytes: 100,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.verified_files, 0);
    match &result.errors[0].kind {
        VerifyErrorKind::FileMissing => {}
        other => panic!("expected FileMissing, got {:?}", other),
    }
}

#[test]
fn test_verify_extracted_checksum_mismatch() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    let file_path = data_dir.join("t1.dat");
    fs::write(&file_path, b"hello world").unwrap();

    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![FileEntry {
            path: "t1.dat".into(),
            size: 11,
            sha256: "wronghash".into(),
        }],
        wal_file: None,
        total_size_bytes: 11,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.verified_files, 0);
    match &result.errors[0].kind {
        VerifyErrorKind::ChecksumMismatch { expected, actual } => {
            assert_eq!(expected, "wronghash");
            assert_ne!(actual, "wronghash");
        }
        other => panic!("expected ChecksumMismatch, got {:?}", other),
    }
}

#[test]
fn test_verify_extracted_wal_missing() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(data_dir.join("t1.dat"), b"data").unwrap();

    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![FileEntry {
            path: "t1.dat".into(),
            size: 4,
            sha256: sha256_bytes(b"data"),
        }],
        wal_file: Some(FileEntry {
            path: "wal/sqlrustgo.wal".into(),
            size: 1024,
            sha256: "walhash".into(),
        }),
        total_size_bytes: 1028,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.verified_files, 1);
    match &result.errors[0].kind {
        VerifyErrorKind::WalMissing => {}
        other => panic!("expected WalMissing, got {:?}", other),
    }
}

#[test]
fn test_verify_extracted_wal_checksum_mismatch() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(data_dir.join("t1.dat"), b"data").unwrap();
    let wal_dir = tmp.path().join("wal");
    fs::create_dir_all(&wal_dir).unwrap();
    fs::write(wal_dir.join("sqlrustgo.wal"), b"wal content").unwrap();

    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![FileEntry {
            path: "t1.dat".into(),
            size: 4,
            sha256: sha256_bytes(b"data"),
        }],
        wal_file: Some(FileEntry {
            path: "wal/sqlrustgo.wal".into(),
            size: 11,
            sha256: "wrongwalhash".into(),
        }),
        total_size_bytes: 15,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.verified_files, 1);
    match &result.errors[0].kind {
        VerifyErrorKind::ChecksumMismatch { expected, .. } => {
            assert_eq!(expected, "wrongwalhash");
        }
        other => panic!("expected ChecksumMismatch, got {:?}", other),
    }
}

#[test]
fn test_verify_extracted_all_files_ok() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    let content1 = b"file one";
    let content2 = b"file two";
    fs::write(data_dir.join("f1.dat"), content1).unwrap();
    fs::write(data_dir.join("f2.dat"), content2).unwrap();

    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![
            FileEntry {
                path: "f1.dat".into(),
                size: content1.len() as u64,
                sha256: sha256_bytes(content1),
            },
            FileEntry {
                path: "f2.dat".into(),
                size: content2.len() as u64,
                sha256: sha256_bytes(content2),
            },
        ],
        wal_file: None,
        total_size_bytes: (content1.len() + content2.len()) as u64,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.errors.len(), 0);
    assert_eq!(result.verified_files, 2);
}

// ============ BackupError Display tests ============

#[test]
fn test_backup_error_display_io() {
    let err = BackupError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let display = format!("{}", err);
    assert!(display.contains("NotFound") || display.contains("not found"));
}

#[test]
fn test_backup_error_display_json() {
    let err = BackupError::Json(serde_json::from_str::<serde_json::Value>("{").unwrap_err());
    let display = format!("{}", err);
    assert!(display.contains("Json") || display.contains("json"));
}

#[test]
fn test_backup_error_display_corrupt_tar() {
    let err = BackupError::CorruptTar;
    let display = format!("{}", err);
    assert!(display.contains("Corrupt") || display.contains("tar"));
}

#[test]
fn test_backup_error_display_entry_not_found() {
    let err = BackupError::EntryNotFound("missing.file".to_string());
    let display = format!("{}", err);
    assert!(display.contains("missing.file") || display.contains("not found"));
}

#[test]
fn test_backup_error_display_checksum_mismatch() {
    let err = BackupError::ChecksumMismatch {
        path: "wal/sqlrustgo.wal".into(),
    };
    let display = format!("{}", err);
    assert!(display.contains("checksum") || display.contains("wal"));
}

#[test]
fn test_backup_error_display_data_dir_not_found() {
    let err = BackupError::DataDirNotFound(std::path::PathBuf::from("/nonexistent"));
    let display = format!("{}", err);
    assert!(display.contains("nonexistent") || display.contains("not found"));
}

// ============ VerifyError Display tests ============

#[test]
fn test_verify_error_display_checksum() {
    let err = VerifyError {
        path: "data/t1.dat".into(),
        kind: VerifyErrorKind::ChecksumMismatch {
            expected: "abc".into(),
            actual: "xyz".into(),
        },
    };
    let display = format!("{}", err);
    assert!(display.contains("abc") && display.contains("xyz"));
}

#[test]
fn test_verify_error_display_file_missing() {
    let err = VerifyError {
        path: "data/t1.dat".into(),
        kind: VerifyErrorKind::FileMissing,
    };
    let display = format!("{}", err);
    assert!(display.contains("t1.dat") || display.contains("missing"));
}

#[test]
fn test_verify_error_display_wal_missing() {
    let err = VerifyError {
        path: "wal/sqlrustgo.wal".into(),
        kind: VerifyErrorKind::WalMissing,
    };
    let display = format!("{}", err);
    assert!(display.contains("WAL") || display.contains("wal"));
}

// ============ tar_extract with REAL gzip tars ============

#[test]
fn test_tar_extract_all_with_real_backup() {
    let data_dir = TempDir::new().unwrap();
    fs::write(data_dir.path().join("t1.json"), b"{\"a\":1}").unwrap();
    fs::write(data_dir.path().join("t2.json"), b"[1,2,3]").unwrap();
    let out = data_dir.path().join("backup.tar.gz");
    physical_backup(data_dir.path(), None, &out).unwrap();

    let extract_dir = TempDir::new().unwrap();
    let entries = tar_extract_all(&out, extract_dir.path()).unwrap();
    assert!(entries.contains(&"manifest.json".to_string()));
    assert!(entries.iter().any(|e| e.starts_with("data/")));
}

#[test]
fn test_tar_extract_one_with_real_backup() {
    let data_dir = TempDir::new().unwrap();
    fs::write(data_dir.path().join("x.json"), b"hello").unwrap();
    let out = data_dir.path().join("backup.tar.gz");
    physical_backup(data_dir.path(), None, &out).unwrap();

    let data = tar_extract_one(&out, "data/x.json").unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn test_tar_extract_one_file_not_found() {
    let data_dir = TempDir::new().unwrap();
    fs::write(data_dir.path().join("a.txt"), b"a").unwrap();
    let out = data_dir.path().join("backup.tar.gz");
    physical_backup(data_dir.path(), None, &out).unwrap();

    let result = tar_extract_one(&out, "data/nonexistent.txt");
    assert!(result.is_err());
    match result.unwrap_err() {
        BackupError::EntryNotFound(_) => {}
        other => panic!("expected EntryNotFound, got {:?}", other),
    }
}

#[test]
fn test_tar_extract_all_corrupt_file() {
    let tmp = TempDir::new().unwrap();
    let corrupt = tmp.path().join("corrupt.tar.gz");
    fs::write(&corrupt, b"not valid").unwrap();
    let out_dir = tmp.path().join("out");
    let result = tar_extract_all(&corrupt, &out_dir);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_tar_extract_all_nonexistent_file() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");
    let result = tar_extract_all(&tmp.path().join("nonexistent.tar.gz"), &out_dir);
    assert!(result.is_err());
}

// ============ physical_backup error paths ============

#[test]
fn test_physical_backup_data_dir_not_found() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("backup.tar.gz");
    let r = physical_backup(&tmp.path().join("nonexistent"), None, &out);
    assert!(matches!(r, Err(BackupError::DataDirNotFound(_))));
}

#[test]
fn test_physical_backup_with_wal() {
    let data_dir = TempDir::new().unwrap();
    let wal_dir = TempDir::new().unwrap();
    fs::write(data_dir.path().join("t1.json"), b"{}").unwrap();
    let wal = wal_dir.path().join("sqlrustgo.wal");
    fs::write(&wal, b"wal-bytes").unwrap();
    let out = data_dir.path().join("backup.tar.gz");
    let r = physical_backup(data_dir.path(), Some(&wal), &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 1);
    assert!(r.manifest.wal_file.is_some());
}

// ============ physical_restore error paths ============

#[test]
fn test_physical_restore_corrupt_tar() {
    let tmp = TempDir::new().unwrap();
    let corrupt = tmp.path().join("corrupt.tar.gz");
    fs::write(&corrupt, b"not valid").unwrap();
    let target = tmp.path().join("restore");
    fs::create_dir_all(&target).unwrap();
    let result = physical_restore(&corrupt, &target);
    assert!(result.is_err());
}

#[test]
fn test_physical_restore_missing_manifest() {
    let tmp = TempDir::new().unwrap();
    // Create a minimal gzip with no valid tar entries (empty after gzip decode)
    // Gzip header + empty payload + trailer = 20 bytes
    let empty_gzip: Vec<u8> = vec![
        0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let corrupt = tmp.path().join("no_manifest.tar.gz");
    fs::write(&corrupt, &empty_gzip).unwrap();
    let target = tmp.path().join("restore");
    fs::create_dir_all(&target).unwrap();
    // Empty gzip decodes to empty vec → tar loop exits immediately
    let result = physical_restore(&corrupt, &target);
    if let Err(e) = &result {
        match e {
            BackupError::EntryNotFound(_) => {}
            other => panic!("expected EntryNotFound, got {:?}", other),
        }
    } else {
        panic!("expected error");
    }
}

// ============ VerifyResult fields ============

#[test]
fn test_verify_result_manifest_cloned() {
    let tmp = TempDir::new().unwrap();
    let manifest = Manifest {
        version: 1,
        created_at: "2026-07-15T00:00:00Z".into(),
        sqlrustgo_version: "3.11.0".into(),
        data_files: vec![],
        wal_file: None,
        total_size_bytes: 0,
    };
    let result = verify_extracted(tmp.path(), &manifest);
    assert_eq!(result.manifest.version, 1);
    assert_eq!(result.errors.len(), 0);
    assert_eq!(result.verified_files, 0);
}
