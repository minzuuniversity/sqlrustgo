use crate::backup::BackupError;
use sqlrustgo_storage::wal::WalEntry;
use sqlrustgo_storage::wal_legacy::{WalEntryType, WalReader};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PitrResult {
    pub target_time: u64,
    pub entries_scanned: usize,
    pub entries_applied: usize,
    pub entries_skipped: usize,
    pub transactions_committed: usize,
    pub transactions_aborted: usize,
    pub active_transactions_at_target: usize,
}

pub fn pitr_replay(wal_path: &Path, target_time: u64) -> Result<PitrResult, BackupError> {
    if !wal_path.exists() {
        return Err(BackupError::EntryNotFound(format!(
            "WAL: {}",
            wal_path.display()
        )));
    }
    let mut reader = WalReader::new(&wal_path.to_path_buf()).map_err(BackupError::Io)?;
    let entries = reader.read_all().map_err(BackupError::Io)?;
    Ok(pitr_replay_entries(&entries, target_time))
}

pub fn pitr_replay_entries(entries: &[WalEntry], target_time: u64) -> PitrResult {
    let mut committed: HashSet<u64> = HashSet::new();
    let mut aborted: HashSet<u64> = HashSet::new();
    let mut active_at_target: HashSet<u64> = HashSet::new();
    let mut applied = 0;
    let mut skipped = 0;
    #[allow(unused_assignments)] // scanned is reported in PitrReport
    let mut scanned = 0;

    let in_window: Vec<&WalEntry> = entries
        .iter()
        .filter(|e| e.timestamp <= target_time)
        .collect();
    scanned = in_window.len();

    for entry in &in_window {
        match entry.entry_type {
            WalEntryType::Commit => {
                committed.insert(entry.tx_id);
            }
            WalEntryType::Rollback => {
                aborted.insert(entry.tx_id);
            }
            WalEntryType::Begin => {}
            _ => {}
        }
    }

    for entry in &in_window {
        match entry.entry_type {
            WalEntryType::Begin => {
                active_at_target.insert(entry.tx_id);
            }
            WalEntryType::Commit => {
                active_at_target.remove(&entry.tx_id);
            }
            WalEntryType::Rollback => {
                active_at_target.remove(&entry.tx_id);
            }
            WalEntryType::Checkpoint
            | WalEntryType::Insert
            | WalEntryType::Update
            | WalEntryType::Delete
            | WalEntryType::Prepare => {
                if committed.contains(&entry.tx_id) {
                    applied += 1;
                } else {
                    skipped += 1;
                }
            }
        }
    }
    PitrResult {
        target_time,
        entries_scanned: scanned,
        entries_applied: applied,
        entries_skipped: skipped,
        transactions_committed: committed.len(),
        transactions_aborted: aborted.len(),
        active_transactions_at_target: active_at_target.len(),
    }
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
    use sqlrustgo_storage::wal::{
        make_begin_entry, make_commit_entry, make_delete_entry, make_insert_entry,
        make_rollback_entry, make_update_entry,
    };
    use std::io::Write;
    use tempfile::TempDir;

    fn entry(tx: u64, ty: WalEntryType, ts: u64, lsn: u64) -> WalEntry {
        let mut e = match ty {
            WalEntryType::Begin => make_begin_entry(tx),
            WalEntryType::Commit => make_commit_entry(tx, lsn),
            WalEntryType::Rollback => make_rollback_entry(tx, lsn),
            WalEntryType::Insert => make_insert_entry(tx, 1, vec![1, 2, 3], vec![1, 2, 3], lsn),
            WalEntryType::Update => make_update_entry(tx, 1, vec![1, 2, 3], vec![4, 5, 6], lsn),
            WalEntryType::Delete => make_delete_entry(tx, 1, vec![1, 2, 3], lsn),
            WalEntryType::Checkpoint => WalEntry {
                tx_id: tx,
                entry_type: WalEntryType::Checkpoint,
                table_id: 0,
                key: None,
                data: None,
                lsn,
                timestamp: ts,
            },
            WalEntryType::Prepare => WalEntry {
                tx_id: tx,
                entry_type: WalEntryType::Prepare,
                table_id: 0,
                key: None,
                data: None,
                lsn,
                timestamp: ts,
            },
        };
        e.timestamp = ts;
        e
    }

    #[test]
    fn test_pitr_replay_committed_tx_applied() {
        let entries = vec![
            entry(1, WalEntryType::Begin, 100, 1),
            entry(1, WalEntryType::Insert, 101, 2),
            entry(1, WalEntryType::Commit, 102, 3),
        ];
        let r = pitr_replay_entries(&entries, 200);
        assert_eq!(r.entries_applied, 1);
        assert_eq!(r.entries_skipped, 0);
        assert_eq!(r.transactions_committed, 1);
        assert_eq!(r.transactions_aborted, 0);
    }

    #[test]
    fn test_pitr_replay_rolled_back_tx_skipped() {
        let entries = vec![
            entry(1, WalEntryType::Begin, 100, 1),
            entry(1, WalEntryType::Insert, 101, 2),
            entry(1, WalEntryType::Rollback, 102, 3),
        ];
        let r = pitr_replay_entries(&entries, 200);
        assert_eq!(r.entries_applied, 0);
        assert_eq!(r.transactions_aborted, 1);
    }

    #[test]
    fn test_pitr_replay_uncommitted_at_target_aborted() {
        let entries = vec![
            entry(1, WalEntryType::Begin, 100, 1),
            entry(1, WalEntryType::Insert, 101, 2),
        ];
        let r = pitr_replay_entries(&entries, 200);
        assert_eq!(r.entries_applied, 0);
        assert_eq!(r.active_transactions_at_target, 1);
    }

    #[test]
    fn test_pitr_replay_target_time_filters() {
        let entries = vec![
            entry(1, WalEntryType::Begin, 100, 1),
            entry(1, WalEntryType::Insert, 101, 2),
            entry(1, WalEntryType::Commit, 102, 3),
            entry(2, WalEntryType::Begin, 200, 4),
            entry(2, WalEntryType::Insert, 201, 5),
            entry(2, WalEntryType::Commit, 202, 6),
        ];
        let r = pitr_replay_entries(&entries, 150);
        assert_eq!(r.entries_applied, 1);
        assert_eq!(r.transactions_committed, 1);
    }

    #[test]
    fn test_pitr_replay_empty() {
        let r = pitr_replay_entries(&[], 1000);
        assert_eq!(r.entries_scanned, 0);
        assert_eq!(r.entries_applied, 0);
    }

    #[test]
    fn test_pitr_replay_wal_file() {
        use sqlrustgo_storage::wal::WalWriter;
        let dir = TempDir::new().unwrap();
        let wal_path = dir.path().join("test.wal");
        let mut writer = WalWriter::with_config(&wal_path, false, 100).unwrap();
        let mut entries = vec![
            entry(1, WalEntryType::Begin, 100, 1),
            entry(1, WalEntryType::Insert, 101, 2),
            entry(1, WalEntryType::Commit, 102, 3),
        ];
        for e in &mut entries {
            e.lsn = writer.current_lsn() + 1;
            writer.append(e).unwrap();
            writer.flush().unwrap();
        }
        drop(writer);
        let r = pitr_replay(&wal_path, 200).unwrap();
        assert_eq!(r.entries_applied, 1);
    }

    #[test]
    fn test_parse_target_time_rfc3339() {
        let t = parse_target_time("2026-06-05T10:00:00Z").unwrap();
        assert!(t > 0);
    }

    #[test]
    fn test_parse_target_time_unix() {
        let t = parse_target_time("1700000000").unwrap();
        assert_eq!(t, 1700000000);
    }

    #[test]
    fn test_parse_target_time_invalid() {
        assert!(parse_target_time("not a time").is_err());
    }
}
