//! Verify module tests
//!
//! Tests: VerifyError, VerifyErrorKind, VerifyResult display/debug

use sqlrustgo_admin::manifest::Manifest;
use sqlrustgo_admin::verify::{VerifyError, VerifyErrorKind, VerifyResult};

#[test]
fn test_verify_error_kind_checksum_mismatch() {
    let kind = VerifyErrorKind::ChecksumMismatch {
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    assert!(matches!(kind, VerifyErrorKind::ChecksumMismatch { .. }));
}

#[test]
fn test_verify_error_kind_file_missing() {
    let kind = VerifyErrorKind::FileMissing;
    assert!(matches!(kind, VerifyErrorKind::FileMissing));
}

#[test]
fn test_verify_error_kind_wal_missing() {
    let kind = VerifyErrorKind::WalMissing;
    assert!(matches!(kind, VerifyErrorKind::WalMissing));
}

#[test]
fn test_verify_error_file_missing() {
    let err = VerifyError {
        path: "/data/users.frm".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    assert_eq!(err.path, "/data/users.frm");
}

#[test]
fn test_verify_error_wal_missing() {
    let err = VerifyError {
        path: "/wal/0000.log".to_string(),
        kind: VerifyErrorKind::WalMissing,
    };
    assert_eq!(err.path, "/wal/0000.log");
}

#[test]
fn test_verify_error_checksum_mismatch() {
    let err = VerifyError {
        path: "/data/orders.ibd".to_string(),
        kind: VerifyErrorKind::ChecksumMismatch {
            expected: "aaaa".to_string(),
            actual: "bbbb".to_string(),
        },
    };
    assert_eq!(err.path, "/data/orders.ibd");
    if let VerifyErrorKind::ChecksumMismatch { expected, actual } = &err.kind {
        assert_eq!(expected, "aaaa");
        assert_eq!(actual, "bbbb");
    }
}

#[test]
fn test_verify_error_clone() {
    let err = VerifyError {
        path: "/data/test.frm".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    let c = err.clone();
    assert_eq!(c.path, err.path);
}

#[test]
fn test_verify_error_debug() {
    let err = VerifyError {
        path: "/data/test.frm".to_string(),
        kind: VerifyErrorKind::FileMissing,
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("VerifyError"));
    assert!(debug.contains("test.frm"));
}

#[test]
fn test_verify_error_kind_clone() {
    let kind = VerifyErrorKind::ChecksumMismatch {
        expected: "x".to_string(),
        actual: "y".to_string(),
    };
    let c = kind.clone();
    assert!(matches!(c, VerifyErrorKind::ChecksumMismatch { .. }));
}

#[test]
fn test_verify_error_kind_debug() {
    let kind = VerifyErrorKind::WalMissing;
    let debug = format!("{:?}", kind);
    assert!(debug.contains("WalMissing"));
}

#[test]
fn test_verify_result_empty() {
    let manifest = Manifest::new();
    let result = VerifyResult {
        manifest,
        errors: vec![],
        verified_files: 0,
    };
    assert!(result.errors.is_empty());
    assert_eq!(result.verified_files, 0);
}

#[test]
fn test_verify_result_with_errors() {
    let manifest = Manifest::new();
    let result = VerifyResult {
        manifest,
        errors: vec![
            VerifyError {
                path: "/data/a.frm".to_string(),
                kind: VerifyErrorKind::FileMissing,
            },
            VerifyError {
                path: "/data/b.ibd".to_string(),
                kind: VerifyErrorKind::ChecksumMismatch {
                    expected: "x".to_string(),
                    actual: "y".to_string(),
                },
            },
        ],
        verified_files: 10,
    };
    assert_eq!(result.errors.len(), 2);
    assert_eq!(result.verified_files, 10);
}

#[test]
fn test_verify_result_debug() {
    let manifest = Manifest::new();
    let result = VerifyResult {
        manifest,
        errors: vec![],
        verified_files: 5,
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("VerifyResult"));
}

#[test]
fn test_verify_error_kind_equality() {
    let kind1 = VerifyErrorKind::FileMissing;
    let kind2 = VerifyErrorKind::FileMissing;
    let kind3 = VerifyErrorKind::WalMissing;
    assert_eq!(kind1, kind2);
    assert_ne!(kind1, kind3);
}
