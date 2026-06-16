use crate::backup::{tar_extract_all, BackupError};
use crate::manifest::Manifest;
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct RestoreResult {
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
