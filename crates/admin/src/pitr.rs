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

/// The process exit code for a completed replay.
///
/// #5055: `admin pitr` used to exit 0 unconditionally, after printing
/// `pitr ok`, having restored nothing. The rule is deliberately
/// conservative — anything short of a complete restore is a non-zero
/// exit, because an operator scripting this needs the shell to notice:
///
/// - `0` — everything in scope applied.
/// - `3` — some entries could not be applied. The directory is in a
///   mixed state and does **not** match the target time.
/// - `4` — nothing failed, but committed transactions existed in scope
///   and nothing was applied. That is the "reported success, restored
///   nothing" shape; it usually means the log and the data directory
///   are not from the same base backup.
///
/// A legitimate zero-row restore (an empty target, or a log with
/// nothing after the base) is not flagged: the test is not
/// "applied == 0" but "applied == 0 *and* there was committed work".
///
/// Kept out of `main.rs` on purpose. Inline in the command arm it was
/// untestable, and a mutation that restored the old "always 0"
/// behaviour was caught only by an unrelated already-failing test.
pub fn exit_code_for(report: &PitrReport) -> i32 {
    if report.entries_failed > 0 {
        return 3;
    }
    if report.is_suspiciously_empty() {
        return 4;
    }
    0
}

/// Human-readable reasons for a non-zero [`exit_code_for`].
pub fn incompleteness_warnings(report: &PitrReport) -> Vec<String> {
    let mut out = Vec::new();
    if report.entries_failed > 0 {
        out.push(format!(
            "pitr: INCOMPLETE — {} entries could not be applied; the data \
             directory does not match the target time",
            report.entries_failed
        ));
    }
    if report.is_suspiciously_empty() {
        out.push(format!(
            "pitr: WARNING — {} transactions were committed by the target time \
             but 0 entries were applied; the data directory may not match the \
             log's base backup",
            report.transactions_committed
        ));
    }
    out
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

    fn report_with(applied: usize, failed: usize, committed: usize) -> PitrReport {
        PitrReport {
            transactions_committed: committed,
            entries_applied: applied,
            entries_failed: failed,
            ..Default::default()
        }
    }

    /// #5055: the exact shape the old CLI called success — committed
    /// work in scope, nothing applied, exit 0.
    #[test]
    fn exit_code_is_non_zero_when_nothing_was_restored() {
        let r = report_with(0, 0, 7);
        assert_eq!(exit_code_for(&r), 4);
        assert!(!incompleteness_warnings(&r).is_empty());
    }

    #[test]
    fn exit_code_is_non_zero_when_entries_failed() {
        let r = report_with(3, 2, 1);
        assert_eq!(exit_code_for(&r), 3);
    }

    /// Failures dominate: a restore that also applied rows is still
    /// incomplete, and the "nothing applied" warning would be
    /// misleading on top of it.
    #[test]
    fn failures_take_precedence_over_the_empty_warning() {
        let r = report_with(0, 5, 2);
        assert_eq!(exit_code_for(&r), 3);
        let warnings = incompleteness_warnings(&r);
        assert_eq!(
            warnings.len(),
            1,
            "expected only the failure notice: {warnings:?}"
        );
        assert!(warnings[0].contains("INCOMPLETE"));
    }

    #[test]
    fn exit_code_is_zero_for_a_clean_restore() {
        assert_eq!(exit_code_for(&report_with(10, 0, 3)), 0);
        assert!(incompleteness_warnings(&report_with(10, 0, 3)).is_empty());
    }

    /// A restore with no committed work and nothing to do is
    /// legitimate, not suspicious.
    #[test]
    fn empty_log_is_not_suspicious() {
        let r = report_with(0, 0, 0);
        assert_eq!(exit_code_for(&r), 0);
        assert!(incompleteness_warnings(&r).is_empty());
    }
}
