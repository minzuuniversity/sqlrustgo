//! Admin commands for BINT storage format management.

use std::path::Path;
use sqlrustgo_storage::SqlResult;
use sqlrustgo_types::SqlError;

pub fn rollback_to_json(data_dir: &Path, table: &str) -> SqlResult<()> {
    let bak = data_dir.join(format!("{}.json.bak", table));
    if !bak.exists() {
        return Err(SqlError::ExecutionError(format!(
            "cannot rollback: {}.json.bak does not exist",
            table
        )));
    }
    let json = data_dir.join(format!("{}.json", table));
    std::fs::rename(&bak, &json).map_err(|e| SqlError::ExecutionError(e.to_string()))?;
    // Remove BIN files
    for ext in &["root.bin"] {
        let p = data_dir.join(format!("{}.{}", table, ext));
        if p.exists() {
            std::fs::remove_file(&p).map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        }
    }
    // Remove segment files
    if let Ok(entries) = std::fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&format!("{}_seg_", table)) && name.ends_with(".bin") {
                std::fs::remove_file(entry.path())
                    .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
            }
        }
    }
    Ok(())
}

pub fn cleanup_bak(data_dir: &Path, older_than_days: u32) -> SqlResult<usize> {
    let threshold = std::time::SystemTime::now()
        - std::time::Duration::from_secs(older_than_days as u64 * 86400);
    let mut removed = 0;
    let entries = std::fs::read_dir(data_dir).map_err(|e| SqlError::ExecutionError(e.to_string()))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".json.bak") {
            let meta = entry.metadata().map_err(|e| SqlError::ExecutionError(e.to_string()))?;
            let modified = meta.modified().map_err(|e| SqlError::ExecutionError(e.to_string()))?;
            if modified < threshold {
                std::fs::remove_file(entry.path())
                    .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn test_rollback_creates_json_from_bak() {
        let dir = tempdir().unwrap();
        let table = "t1";
        // Set up state: .json.bak exists, .bin + .root.bin exist
        std::fs::write(dir.path().join(format!("{}.json.bak", table)), b"{}").unwrap();
        std::fs::write(
            dir.path().join(format!("{}_seg_0000.bin", table)),
            b"\0",
        )
        .unwrap();
        std::fs::write(dir.path().join(format!("{}.root.bin", table)), b"\0").unwrap();
        rollback_to_json(dir.path(), table).unwrap();
        assert!(dir.path().join(format!("{}.json", table)).exists());
        assert!(!dir.path().join(format!("{}.json.bak", table)).exists());
        assert!(!dir.path().join(format!("{}.root.bin", table)).exists());
        assert!(!dir
            .path()
            .join(format!("{}_seg_0000.bin", table))
            .exists());
    }

    #[test]
    fn test_rollback_fails_without_bak() {
        let dir = tempdir().unwrap();
        let table = "t1";
        std::fs::write(dir.path().join(format!("{}.root.bin", table)), b"\0").unwrap();
        let result = rollback_to_json(dir.path(), table);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_bak_removes_old_files() {
        let dir = tempdir().unwrap();
        // Create a .json.bak with old mtime
        let bak = dir.path().join("t1.json.bak");
        std::fs::write(&bak, b"{}").unwrap();
        std::fs::set_permissions(&bak, std::fs::Permissions::from_mode(0o644)).unwrap();
        let _ = std::fs::File::options()
            .write(true)
            .open(&bak)
            .unwrap()
            .set_modified(
                std::time::SystemTime::now() - std::time::Duration::from_secs(86400 * 60),
            );
        let removed = cleanup_bak(dir.path(), 30).unwrap();
        assert_eq!(removed, 1);
        assert!(!bak.exists());
    }
}
