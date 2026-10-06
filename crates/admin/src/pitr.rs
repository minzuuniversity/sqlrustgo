use crate::backup::BackupError;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::pitr::replay_entries_until;
use sqlrustgo_storage::wal::WalEntry;
use sqlrustgo_storage::wal_legacy::WalReader;
use std::path::Path;

pub use sqlrustgo_storage::pitr::PitrReport;

/// #5055: replay `wal_path` into `data_dir`, up to `target_time`.
///
/// This is the function the CLI used to pretend to have. The old
/// `pitr_replay` read the WAL, counted the entries it would have
/// applied, and returned; `main.rs` bound `--data-dir` to `let _ =
/// data_dir;` and printed `pitr ok` with exit code 0. Nothing was
/// written and nothing could fail.
///
/// Now the data directory is opened, the rows are decoded and applied,
/// and the buffers are flushed so the result is on disk before this
/// returns. `Ok(())` means data moved.
pub fn pitr_replay_into(
    data_dir: &Path,
    wal_path: &Path,
    target_time: u64,
) -> Result<PitrReport, BackupError> {
    if !wal_path.exists() {
        return Err(BackupError::EntryNotFound(format!(
            "WAL: {}",
            wal_path.display()
        )));
    }
    if !data_dir.exists() {
        return Err(BackupError::DataDirNotFound(data_dir.to_path_buf()));
    }

    let mut reader = WalReader::new(&wal_path.to_path_buf()).map_err(BackupError::Io)?;
    let entries: Vec<WalEntry> = reader.read_all().map_err(BackupError::Io)?;

    // `FileStorage::new`, deliberately NOT `new_with_wal`: the replay
    // must not append to the very log it is reading, or each restore
    // would double the log for the next one.
    let mut storage = FileStorage::new(data_dir.to_path_buf())?;
    let report = replay_entries_until(&mut storage, &entries, target_time)?;

    // Without this the rows sit in the insert buffer and the data
    // directory is unchanged the moment we return — a restore that
    // reports success and leaves the data behind.
    storage.flush_all_buffers()?;

    Ok(report)
}

/// Backwards-compatible name for callers that only want the file-based
/// replay. #5055 kept this as a thin wrapper; the old behaviour (count
/// only, no writes) is gone, so a caller that relied on it getting a
/// count is now getting a restore.
pub fn pitr_replay(wal_path: &Path, target_time: u64) -> Result<PitrReport, BackupError> {
    // A data directory that is a sibling of the WAL is the historical
    // layout. Used only by the wrapper; the CLI passes `--data-dir`
    // explicitly.
    let data_dir = wal_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    pitr_replay_into(&data_dir, wal_path, target_time)
}

pub fn parse_target_time(s: &str) -> Result<u64, BackupError> {
    use chrono::DateTime;
    let formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S%z",
    ];
    for fmt in formats {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(naive.and_utc().timestamp() as u64);
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.timestamp() as u64);
        }
    }
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }
    Err(BackupError::EntryNotFound(format!(
        "unparseable target time: {}",
        s
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::BackupError;
    use sqlrustgo_storage::wal_legacy::WalWriter;
    use tempfile::TempDir;

    fn write_wal(path: &Path, entries: &[WalEntry]) {
        let mut writer = WalWriter::with_config(&path.to_path_buf(), false, 100).unwrap();
        for e in entries {
            writer.append(e).unwrap();
        }
        writer.flush().unwrap();
    }

    #[test]
    fn pitr_replay_into_missing_data_dir_errors() {
        let dir = TempDir::new().unwrap();
        let wal = dir.path().join("x.wal");
        write_wal(&wal, &[]);
        let missing = dir.path().join("nope");
        let err = pitr_replay_into(&missing, &wal, 100).unwrap_err();
        assert!(matches!(err, BackupError::DataDirNotFound(_)));
    }

    #[test]
    fn pitr_replay_into_missing_wal_errors() {
        let dir = TempDir::new().unwrap();
        let err = pitr_replay_into(dir.path(), &dir.path().join("absent.wal"), 100).unwrap_err();
        assert!(matches!(err, BackupError::EntryNotFound(_)));
    }

    #[test]
    fn parse_target_time_rfc3339() {
        let t = parse_target_time("2026-06-05T10:00:00Z").unwrap();
        assert!(t > 0);
    }

    #[test]
    fn parse_target_time_unix() {
        let t = parse_target_time("1700000000").unwrap();
        assert_eq!(t, 1700000000);
    }

    #[test]
    fn parse_target_time_invalid() {
        assert!(parse_target_time("not a time").is_err());
    }
}
