// Additional admin verify.rs coverage tests

use sqlrustgo_admin::verify::{VerifyError, VerifyErrorKind, VerifyResult};

// ============ VerifyErrorKind tests ============

#[test]
fn test_verify_error_kind_checksum_mismatch_debug() {
    let err = VerifyErrorKind::ChecksumMismatch {
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("abc123"));
    assert!(debug.contains("def456"));
    assert!(debug.contains("ChecksumMismatch"));
}

#[test]
fn test_verify_error_kind_file_missing_debug() {
    let err = VerifyErrorKind::FileMissing;
    let debug = format!("{:?}", err);
    assert!(debug.contains("FileMissing"));
}

#[test]
fn test_verify_error_kind_wal_missing_debug() {
    let err = VerifyErrorKind::WalMissing;
    let debug = format!("{:?}", err);
    assert!(debug.contains("WalMissing"));
}

#[test]
fn test_verify_error_kind_eq() {
    let k1 = VerifyErrorKind::FileMissing;
    let k2 = VerifyErrorKind::FileMissing;
    let k3 = VerifyErrorKind::WalMissing;
    assert_eq!(k1, k2);
    assert_ne!(k1, k3);
}

#[test]
fn test_verify_error_kind_clone() {
    let k1 = VerifyErrorKind::ChecksumMismatch {
        expected: "a".to_string(),
        actual: "b".to_string(),
    };
    let k2 = k1.clone();
    assert_eq!(k1, k2);
}

// ============ VerifyError tests ============

#[test]
fn test_verify_error_display() {
    let err = VerifyError {
        path: "data/t1.dat".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    let display = format!("{}", err);
    assert!(display.contains("data/t1.dat"));
    assert!(display.contains("file missing"));
}

#[test]
fn test_verify_error_checksum_mismatch() {
    let err = VerifyError {
        path: "data/t1.dat".to_string(),
        kind: VerifyErrorKind::ChecksumMismatch {
            expected: "hash1".to_string(),
            actual: "hash2".to_string(),
        },
    };
    let display = format!("{}", err);
    assert!(display.contains("hash1"));
    assert!(display.contains("hash2"));
}

#[test]
fn test_verify_error_eq() {
    let err1 = VerifyError {
        path: "data/t1.dat".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    let err2 = VerifyError {
        path: "data/t1.dat".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    let err3 = VerifyError {
        path: "data/t2.dat".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    assert_eq!(err1, err2);
    assert_ne!(err1, err3);
}

#[test]
fn test_verify_error_clone() {
    let err1 = VerifyError {
        path: "data/t1.dat".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    let err2 = err1.clone();
    assert_eq!(err1, err2);
}

#[test]
fn test_verify_error_debug() {
    let err = VerifyError {
        path: "data/t1.dat".to_string(),
        kind: VerifyErrorKind::WalMissing,
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("WalMissing"));
}

// ============ VerifyResult tests ============

#[test]
fn test_verify_result_debug() {
    let result = VerifyResult {
        manifest: sqlrustgo_admin::manifest::Manifest::default(),
        errors: vec![],
        verified_files: 0,
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("VerifyResult"));
}

#[test]
fn test_verify_result_with_errors() {
    let errors = vec![
        VerifyError {
            path: "data/t1.dat".to_string(),
            kind: VerifyErrorKind::FileMissing,
        },
        VerifyError {
            path: "data/t2.dat".to_string(),
            kind: VerifyErrorKind::WalMissing,
        },
    ];
    let result = VerifyResult {
        manifest: sqlrustgo_admin::manifest::Manifest::default(),
        errors,
        verified_files: 5,
    };
    assert_eq!(result.errors.len(), 2);
    assert_eq!(result.verified_files, 5);
}

#[test]
fn test_verify_result_empty() {
    let result = VerifyResult {
        manifest: sqlrustgo_admin::manifest::Manifest::default(),
        errors: vec![],
        verified_files: 0,
    };
    assert!(result.errors.is_empty());
    assert_eq!(result.verified_files, 0);
}
