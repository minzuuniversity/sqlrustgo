use crate::backup::{tar_extract_all, BackupError};
use crate::manifest::Manifest;
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct RestoreResult {
    #[allow(dead_code)]
    pub manifest: Manifest,
    pub restored_data_files: usize,
    pub restored_wal: bool,
}

pub fn physical_restore(input: &Path, target_dir: &Path) -> Result<RestoreResult, BackupError> {
    let staging = target_dir
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(format!(
            ".sqlrustgo-restore-{}",
            target_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "staging".to_string())
        ));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(BackupError::Io)?;
    }
    fs::create_dir_all(&staging).map_err(BackupError::Io)?;
    tar_extract_all(input, &staging)?;

    let manifest_path = staging.join("manifest.json");
    if !manifest_path.exists() {
        return Err(BackupError::EntryNotFound("manifest.json".to_string()));
    }
    let manifest = Manifest::read_from(&manifest_path).map_err(BackupError::Io)?;
    let verify = crate::verify::verify_extracted(&staging, &manifest);
    if !verify.errors.is_empty() {
        fs::remove_dir_all(&staging).ok();
        return Err(BackupError::CorruptTar);
    }

    if target_dir.exists() {
        fs::remove_dir_all(target_dir).map_err(BackupError::Io)?;
    }
    fs::create_dir_all(target_dir).map_err(BackupError::Io)?;
    let data_dir = target_dir.join("data");
    fs::create_dir_all(&data_dir).map_err(BackupError::Io)?;
    let mut restored_data = 0;
    for entry in &manifest.data_files {
        let src = staging.join("data").join(&entry.path);
        if !src.exists() {
            fs::remove_dir_all(target_dir).ok();
            return Err(BackupError::EntryNotFound(format!("data/{}", entry.path)));
        }
        let dst = data_dir.join(&entry.path);
        if let Some(p) = dst.parent() {
            fs::create_dir_all(p).map_err(BackupError::Io)?;
        }
        let bytes = fs::read(&src).map_err(BackupError::Io)?;
        let mut f = fs::File::create(&dst).map_err(BackupError::Io)?;
        f.write_all(&bytes).map_err(BackupError::Io)?;
        restored_data += 1;
    }
    let mut restored_wal = false;
    if let Some(wf) = &manifest.wal_file {
        let src = staging.join("wal").join("sqlrustgo.wal");
        if src.exists() {
            let wal_dir = target_dir.join("wal");
            fs::create_dir_all(&wal_dir).map_err(BackupError::Io)?;
            let dst = wal_dir.join("sqlrustgo.wal");
            let bytes = fs::read(&src).map_err(BackupError::Io)?;
            let mut f = fs::File::create(&dst).map_err(BackupError::Io)?;
            f.write_all(&bytes).map_err(BackupError::Io)?;
            restored_wal = true;
            let _ = wf;
        }
    }
    fs::remove_dir_all(&staging).ok();
    Ok(RestoreResult {
        manifest,
        restored_data_files: restored_data,
        restored_wal,
    })
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::physical_backup;
    use crate::verify::{VerifyError, VerifyErrorKind};
    use std::io::Write;
    use tempfile::TempDir;

    fn make_backup_tar() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let data_dir = TempDir::new().unwrap();
        std::fs::write(data_dir.path().join("t1.json"), b"data1").unwrap();
        std::fs::write(data_dir.path().join("t2.json"), b"data2").unwrap();
        let out = dir.path().join("backup.tar.gz");
        physical_backup(data_dir.path(), None, &out).unwrap();
        (dir, out)
    }

    #[test]
    fn test_physical_restore_success() {
        let (_src, tar_path) = make_backup_tar();
        let target = TempDir::new().unwrap();
        let result = physical_restore(&tar_path, target.path()).unwrap();
        assert!(
            result.restored_data_files == 2,
            "expected restored_data_files=2"
        );
        assert!(!result.restored_wal);
        assert!(target.path().join("data/t1.json").exists());
        assert!(target.path().join("data/t2.json").exists());
    }

    #[test]
    fn test_physical_restore_missing_manifest() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("no_manifest.tar.gz");
        let mut f = std::fs::File::create(&out).unwrap();
        f.write_all(&[0; 128]).unwrap();
        drop(f);
        let target = TempDir::new().unwrap();
        let err = match physical_restore(&out, target.path()) {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        };
        assert!(
            format!("{}", err).len() > 0,
            "error message should not be empty"
        );
    }

    #[test]
    fn test_physical_restore_with_wal() {
        let dir = TempDir::new().unwrap();
        let data_dir = TempDir::new().unwrap();
        let wal_dir = TempDir::new().unwrap();
        std::fs::write(data_dir.path().join("t1.json"), b"x").unwrap();
        let wal = wal_dir.path().join("sqlrustgo.wal");
        std::fs::write(&wal, b"w0w00").unwrap();
        let out = dir.path().join("backup.tar.gz");
        physical_backup(data_dir.path(), Some(&wal), &out).unwrap();
        let target = TempDir::new().unwrap();
        let result = physical_restore(&out, target.path()).unwrap();
        assert!(
            result.restored_data_files == 1,
            "expected restored_data_files=1"
        );
        assert!(result.restored_wal);
        assert!(target.path().join("wal/sqlrustgo.wal").exists());
    }

    // --- VerifyError Display tests ---

    #[test]
    fn test_verify_error_display_checksum() {
        let err = VerifyError {
            path: "data/t.json".to_string(),
            kind: VerifyErrorKind::ChecksumMismatch {
                expected: "abc".to_string(),
                actual: "xyz".to_string(),
            },
        };
        let s = format!("{}", err);
        assert!(s.contains("checksum mismatch"));
        assert!(s.contains("abc"));
        assert!(s.contains("xyz"));
    }

    #[test]
    fn test_verify_error_display_file_missing() {
        let err = VerifyError {
            path: "data/t.json".to_string(),
            kind: VerifyErrorKind::FileMissing,
        };
        assert!(format!("{}", err).contains("file missing"));
    }

    #[test]
    fn test_verify_error_display_wal_missing() {
        let err = VerifyError {
            path: "wal/sqlrustgo.wal".to_string(),
            kind: VerifyErrorKind::WalMissing,
        };
        assert!(format!("{}", err).contains("WAL file missing"));
    }
}
