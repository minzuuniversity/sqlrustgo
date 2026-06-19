use crate::backup::{tar_extract_all, BackupError};
use crate::manifest::{sha256_file, Manifest};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyError {
    pub path: String,
    pub kind: VerifyErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyErrorKind {
    ChecksumMismatch { expected: String, actual: String },
    FileMissing,
    WalMissing,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            VerifyErrorKind::ChecksumMismatch { expected, actual } => write!(
                f,
                "checksum mismatch for {}: expected={}, actual={}",
                self.path, expected, actual
            ),
            VerifyErrorKind::FileMissing => write!(f, "file missing: {}", self.path),
            VerifyErrorKind::WalMissing => write!(f, "WAL file missing: {}", self.path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_error_checksum_mismatch() {
        let err = VerifyError {
            path: "data/table1".to_string(),
            kind: VerifyErrorKind::ChecksumMismatch {
                expected: "abc123".to_string(),
                actual: "def456".to_string(),
            },
        };
        assert_eq!(err.path, "data/table1");
        assert!(err.to_string().contains("checksum mismatch"));
        assert!(err.to_string().contains("abc123"));
    }

    #[test]
    fn test_verify_error_file_missing() {
        let err = VerifyError {
            path: "data/table2".to_string(),
            kind: VerifyErrorKind::FileMissing,
        };
        assert_eq!(err.path, "data/table2");
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn test_verify_error_wal_missing() {
        let err = VerifyError {
            path: "wal/sqlrustgo.wal".to_string(),
            kind: VerifyErrorKind::WalMissing,
        };
        assert!(err.to_string().contains("WAL file missing"));
    }

    #[test]
    fn test_verify_error_clone_and_debug() {
        let err1 = VerifyError {
            path: "data/t".to_string(),
            kind: VerifyErrorKind::FileMissing,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
        let debug_str = format!("{:?}", err1);
        assert!(debug_str.contains("VerifyError"));
    }

    #[test]
    fn test_verify_error_kind_equality() {
        let k1 = VerifyErrorKind::ChecksumMismatch {
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        let k2 = VerifyErrorKind::ChecksumMismatch {
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        assert_eq!(k1, k2);
        assert_ne!(k1, VerifyErrorKind::FileMissing);
        assert_ne!(k1, VerifyErrorKind::WalMissing);
    }

    #[test]
    fn test_verify_result_new() {
        let manifest = Manifest::new();
        let result = VerifyResult {
            manifest: manifest.clone(),
            errors: vec![],
            verified_files: 5,
        };
        assert_eq!(result.verified_files, 5);
        assert!(result.errors.is_empty());
        // Verify Clone
        let cloned = result.clone();
        assert_eq!(result.verified_files, cloned.verified_files);
    }

    #[test]
    fn test_verify_result_with_errors() {
        let errors = vec![
            VerifyError {
                path: "data/t1".to_string(),
                kind: VerifyErrorKind::FileMissing,
            },
            VerifyError {
                path: "data/t2".to_string(),
                kind: VerifyErrorKind::ChecksumMismatch {
                    expected: "aaa".to_string(),
                    actual: "bbb".to_string(),
                },
            },
        ];
        let result = VerifyResult {
            manifest: Manifest::new(),
            errors,
            verified_files: 0,
        };
        assert_eq!(result.errors.len(), 2);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub manifest: Manifest,
    pub errors: Vec<VerifyError>,
    pub verified_files: usize,
}

pub fn verify_backup(input: &Path) -> Result<VerifyResult, BackupError> {
    let staging = input
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(format!(
            ".sqlrustgo-verify-{}",
            input
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "x".to_string())
        ));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).ok();
    }
    std::fs::create_dir_all(&staging).map_err(BackupError::Io)?;
    tar_extract_all(input, &staging)?;
    let manifest_path = staging.join("manifest.json");
    if !manifest_path.exists() {
        std::fs::remove_dir_all(&staging).ok();
        return Err(BackupError::EntryNotFound("manifest.json".to_string()));
    }
    let manifest = Manifest::read_from(&manifest_path).map_err(BackupError::Io)?;
    let r = verify_extracted(&staging, &manifest);
    std::fs::remove_dir_all(&staging).ok();
    Ok(r)
}

pub fn verify_extracted(staging: &Path, manifest: &Manifest) -> VerifyResult {
    let mut errors = Vec::new();
    let mut verified = 0;
    for entry in &manifest.data_files {
        let path = staging.join("data").join(&entry.path);
        if !path.exists() {
            errors.push(VerifyError {
                path: format!("data/{}", entry.path),
                kind: VerifyErrorKind::FileMissing,
            });
            continue;
        }
        let actual = match sha256_file(&path) {
            Ok(s) => s,
            Err(_) => {
                errors.push(VerifyError {
                    path: format!("data/{}", entry.path),
                    kind: VerifyErrorKind::FileMissing,
                });
                continue;
            }
        };
        if actual != entry.sha256 {
            errors.push(VerifyError {
                path: format!("data/{}", entry.path),
                kind: VerifyErrorKind::ChecksumMismatch {
                    expected: entry.sha256.clone(),
                    actual,
                },
            });
        } else {
            verified += 1;
        }
    }
    if let Some(wf) = &manifest.wal_file {
        let wal_path = staging.join("wal").join("sqlrustgo.wal");
        if !wal_path.exists() {
            errors.push(VerifyError {
                path: "wal/sqlrustgo.wal".to_string(),
                kind: VerifyErrorKind::WalMissing,
            });
        } else {
            let actual = sha256_file(&wal_path).unwrap_or_default();
            if actual != wf.sha256 {
                errors.push(VerifyError {
                    path: "wal/sqlrustgo.wal".to_string(),
                    kind: VerifyErrorKind::ChecksumMismatch {
                        expected: wf.sha256.clone(),
                        actual,
                    },
                });
            } else {
                verified += 1;
            }
        }
    }
    VerifyResult {
        manifest: manifest.clone(),
        errors,
        verified_files: verified,
    }
}
