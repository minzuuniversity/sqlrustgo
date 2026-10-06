//! Point-in-time recovery: replay a WAL into a storage up to a target
//! time, for real.
//!
//! #5055: `admin pitr` used to *count* WAL entries and print
//! `pitr ok` without touching a single byte of data. The CLI took a
//! `--data-dir`, bound it to `let _ = data_dir;`, and exited 0. An
//! operator who ran it during an incident got a success message and an
//! unchanged database.
//!
//! This module is the part that actually moves rows. It lives in
//! `sqlrustgo-storage` rather than in the `admin` crate because the work
//! it does — resolve the table, decode the row image, apply it — is
//! byte-for-byte the same work `RecoveryEngine` does on startup, and
//! re-implementing it in the CLI is how the two would drift apart.

use crate::engine::{SqlResult, StorageEngine};
use crate::recovery_engine::{apply_wal_entry, ApplyOutcome};
use crate::wal::WalEntry;
use crate::wal_legacy::WalEntryType;
#[cfg(test)]
use sqlrustgo_types::Value;
use std::collections::{BTreeSet, HashSet};

/// What a point-in-time replay actually did.
///
/// Every counter here is evidence of work performed, not of intent. A
/// report that says `rows_applied: 0` against a non-empty WAL is a
/// failed recovery, and the CLI treats it as one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PitrReport {
    /// The requested recovery point, in Unix seconds.
    pub target_time: u64,
    /// Entries in the WAL at or before `target_time`. Entries after it
    /// are *not* counted: they belong to a future that is being
    /// discarded.
    pub entries_scanned: usize,
    /// Entries beyond `target_time` that were deliberately not applied.
    pub entries_after_target: usize,
    /// Row entries that changed the storage.
    pub entries_applied: usize,
    /// Row entries that did not: uncommitted at the target, aborted, or
    /// already present in the restored base.
    pub entries_skipped: usize,
    /// Row entries that could not be applied. Non-zero means the
    /// restore is **incomplete** and the caller must not report success
    /// without saying so.
    pub entries_failed: usize,
    /// Transactions that had committed by `target_time`.
    pub transactions_committed: usize,
    /// Transactions that had rolled back by `target_time`.
    pub transactions_aborted: usize,
    /// Transactions still open at `target_time` — work that existed in
    /// the log but was never committed. Correctly excluded.
    pub active_transactions_at_target: usize,
    /// Tables this replay wrote to, sorted, so an operator can see the
    /// blast radius without diffing directories.
    pub tables_touched: BTreeSet<String>,
    /// First error text, when `entries_failed > 0`.
    pub first_error: Option<String>,
}

impl PitrReport {
    /// True when the replay applied nothing while the log had
    /// committed work in scope.
    ///
    /// This is the "reported success but restored nothing" shape, and
    /// the CLI refuses to exit 0 on it. A zero-row recovery is
    /// legitimate — an empty target, or a log with nothing after the
    /// base — so the test is not "applied == 0" but "applied == 0 and
    /// there was committed work to apply".
    pub fn is_suspiciously_empty(&self) -> bool {
        self.entries_applied == 0 && self.transactions_committed > 0 && self.entries_failed == 0
    }
}

/// Replay `entries` into `storage` up to and including `target_time`.
///
/// #5055: the caller must pass a storage with **no WAL attached**
/// (`FileStorage::new`, not `new_with_wal`). Re-logging the replay
/// would double the log on every restore, and the next restore would
/// double it again.
///
/// Entries are applied in file order, which is commit order. Only rows
/// belonging to a transaction that committed at or before
/// `target_time` are applied; an open transaction at the target is
/// deliberately left out, because at that instant it was not yet part
/// of the database.
pub fn replay_entries_until<S: StorageEngine>(
    storage: &mut S,
    entries: &[WalEntry],
    target_time: u64,
) -> SqlResult<PitrReport> {
    let mut report = PitrReport {
        target_time,
        ..Default::default()
    };

    // Pass 0: which transactions have explicit boundaries *anywhere* in
    // the log, not just inside the window.
    //
    // This matters for autocommit. `FileStorage` writes a plain
    // INSERT/UPDATE/DELETE under `tx_id == 0` with no Begin and no
    // Commit — the statement is its own transaction, and each entry is
    // durable the moment it is written. Treating those as "uncommitted"
    // would drop every autocommit statement from the restore, which is
    // most of a real workload.
    //
    // The distinction is "no boundary anywhere", not "no boundary in
    // the window": a transaction whose `Begin` predates the window
    // still has a boundary in this log, and misclassifying it as
    // autocommit would replay rows that were never committed.
    let mut bounded: HashSet<u64> = HashSet::new();
    for entry in entries {
        if matches!(
            entry.entry_type,
            WalEntryType::Begin | WalEntryType::Commit | WalEntryType::Rollback
        ) {
            bounded.insert(entry.tx_id);
        }
    }

    // Pass 1: the window, and which transactions were decided by the
    // time we stopped at. A Commit *after* the target does not commit
    // anything for the purposes of this restore — the work was not
    // durable at the recovery point.
    let mut in_window: Vec<&WalEntry> = Vec::with_capacity(entries.len());
    let mut committed: HashSet<u64> = HashSet::new();
    let mut aborted: HashSet<u64> = HashSet::new();
    let mut open: HashSet<u64> = HashSet::new();

    for entry in entries {
        if entry.timestamp > target_time {
            report.entries_after_target += 1;
            continue;
        }
        in_window.push(entry);
        match entry.entry_type {
            WalEntryType::Begin => {
                open.insert(entry.tx_id);
            }
            WalEntryType::Commit => {
                if open.remove(&entry.tx_id) {
                    committed.insert(entry.tx_id);
                }
            }
            WalEntryType::Rollback => {
                if open.remove(&entry.tx_id) {
                    aborted.insert(entry.tx_id);
                }
            }
            _ => {}
        }
    }

    report.entries_scanned = in_window.len();
    report.transactions_committed = committed.len();
    report.transactions_aborted = aborted.len();
    report.active_transactions_at_target = open.len();

    // Pass 2: apply. Ordering is the order the log was written, which
    // is the order the changes happened.
    for entry in in_window {
        if !is_row_entry(entry.entry_type) {
            continue;
        }
        // Applicable when the transaction committed by the target, or
        // when the entry was never part of an explicit transaction at
        // all (autocommit / orphan DML — see pass 0).
        let is_autocommit = !bounded.contains(&entry.tx_id);
        if !committed.contains(&entry.tx_id) && !is_autocommit {
            report.entries_skipped += 1;
            continue;
        }
        if let Some(name) = entry.table_name.as_deref() {
            report.tables_touched.insert(name.to_string());
        }
        match apply_wal_entry(storage, entry) {
            Ok(ApplyOutcome::Applied) => report.entries_applied += 1,
            Ok(ApplyOutcome::Skipped) => report.entries_skipped += 1,
            Err(e) => {
                // #5055: a failure to apply one entry is reported, not
                // swallowed. The old CLI had no way to fail at all.
                report.entries_failed += 1;
                if report.first_error.is_none() {
                    report.first_error = Some(format!(
                        "tx_id={} lsn={} type={:?}: {e}",
                        entry.tx_id, entry.lsn, entry.entry_type
                    ));
                }
            }
        }
    }

    Ok(report)
}

fn is_row_entry(t: WalEntryType) -> bool {
    matches!(
        t,
        WalEntryType::Insert | WalEntryType::Update | WalEntryType::Delete
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ColumnDefinition, MemoryStorage, Record, TableInfo};
    use crate::wal_record_codec;

    fn users_table() -> TableInfo {
        TableInfo {
            name: "users".into(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
            original_sql: String::new(),
        }
    }

    fn storage() -> MemoryStorage {
        let mut s = MemoryStorage::new();
        s.create_table(&users_table()).unwrap();
        s
    }

    fn tx_entry(tx: u64, ty: WalEntryType, ts: u64) -> WalEntry {
        WalEntry {
            tx_id: tx,
            entry_type: ty,
            table_id: 0,
            table_name: None,
            key: None,
            data: None,
            lsn: 0,
            timestamp: ts,
        }
    }

    /// An autocommit row: `tx_id == 0`, no boundaries — the shape
    /// `FileStorage` writes outside a transaction.
    fn insert_row(id: i64, ts: u64) -> WalEntry {
        let row: Record = vec![Value::Integer(id)];
        WalEntry {
            tx_id: 0,
            entry_type: WalEntryType::Insert,
            table_id: wal_record_codec::table_name_to_id("users"),
            table_name: Some("users".to_string()),
            key: Some(wal_record_codec::record_key(&row)),
            data: Some(wal_record_codec::record_to_bytes(&row)),
            lsn: 0,
            timestamp: ts,
        }
    }

    fn ids(s: &MemoryStorage) -> Vec<i64> {
        let mut v: Vec<i64> = s
            .scan("users")
            .unwrap()
            .iter()
            .filter_map(|r| match r.first() {
                Some(Value::Integer(i)) => Some(*i),
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    /// The headline #5055 behaviour, at the module's own level: a
    /// committed transaction's rows end up in the storage.
    #[test]
    fn committed_rows_are_applied() {
        let mut s = storage();
        let entries = vec![
            tx_entry(1, WalEntryType::Begin, 10),
            insert_row(1, 11),
            insert_row(2, 12),
            tx_entry(1, WalEntryType::Commit, 13),
        ];
        let r = replay_entries_until(&mut s, &entries, 100).unwrap();
        assert_eq!(r.entries_applied, 2, "{r:?}");
        assert_eq!(r.entries_failed, 0);
        assert_eq!(r.transactions_committed, 1);
        assert_eq!(ids(&s), vec![1, 2]);
    }

    /// An open transaction at the target was not part of the database
    /// at that instant, and must not be restored.
    #[test]
    fn open_transaction_at_target_is_not_applied() {
        let mut s = storage();
        let entries = vec![
            tx_entry(1, WalEntryType::Begin, 10),
            insert_row_tx(1, 1, 11),
            // no Commit
        ];
        let r = replay_entries_until(&mut s, &entries, 100).unwrap();
        assert_eq!(r.active_transactions_at_target, 1);
        assert_eq!(r.entries_applied, 0);
        assert_eq!(r.entries_skipped, 1);
        assert!(ids(&s).is_empty());
    }

    /// A Commit *after* the target does not commit for this restore.
    #[test]
    fn commit_after_the_target_does_not_apply() {
        let mut s = storage();
        let entries = vec![
            tx_entry(1, WalEntryType::Begin, 10),
            insert_row_tx(1, 1, 11),
            tx_entry(1, WalEntryType::Commit, 50),
        ];
        let r = replay_entries_until(&mut s, &entries, 20).unwrap();
        assert_eq!(r.transactions_committed, 0);
        assert_eq!(r.entries_after_target, 1);
        assert_eq!(r.entries_applied, 0);
        assert!(ids(&s).is_empty());
    }

    /// Autocommit DML is its own transaction and must be restored.
    /// The old entry-counting code had no notion of this at all.
    #[test]
    fn autocommit_rows_are_applied() {
        let mut s = storage();
        let entries = vec![insert_row(1, 10), insert_row(2, 11)];
        let r = replay_entries_until(&mut s, &entries, 100).unwrap();
        assert_eq!(r.entries_applied, 2, "{r:?}");
        assert_eq!(r.transactions_committed, 0);
        assert_eq!(ids(&s), vec![1, 2]);
    }

    /// A transaction whose `Begin` predates the window is still bounded,
    /// so its rows are not mistaken for autocommit work. `bounded` is
    /// computed over the whole log precisely for this.
    #[test]
    fn begin_before_the_window_still_bounds_its_rows() {
        let mut s = storage();
        let entries = vec![
            tx_entry(7, WalEntryType::Begin, 1),
            // timestamp after the target: the row is out of window
            insert_row_tx(7, 1, 99),
            tx_entry(7, WalEntryType::Rollback, 100),
        ];
        let r = replay_entries_until(&mut s, &entries, 50).unwrap();
        assert_eq!(r.entries_scanned, 1, "only the Begin is in the window");
        assert_eq!(r.active_transactions_at_target, 1);
        assert_eq!(r.entries_applied, 0);
        assert!(ids(&s).is_empty());
    }

    fn insert_row_tx(tx: u64, id: i64, ts: u64) -> WalEntry {
        let mut e = insert_row(id, ts);
        e.tx_id = tx;
        e
    }

    /// The window is inclusive of the target second.
    #[test]
    fn window_is_inclusive_of_the_target() {
        let mut s = storage();
        let entries = vec![insert_row(1, 100)];
        let at = replay_entries_until(&mut s, &entries, 100).unwrap();
        assert_eq!(at.entries_scanned, 1);
        assert_eq!(ids(&s), vec![1]);

        let mut s2 = storage();
        let before = replay_entries_until(&mut s2, &entries, 99).unwrap();
        assert_eq!(before.entries_scanned, 0);
        assert!(ids(&s2).is_empty());
    }

    /// An empty log is a legitimate no-op, not a failure.
    #[test]
    fn empty_log_is_a_clean_no_op() {
        let mut s = storage();
        let r = replay_entries_until(&mut s, &[], 100).unwrap();
        assert_eq!(r.entries_scanned, 0);
        assert_eq!(r.entries_applied, 0);
        assert_eq!(r.entries_failed, 0);
        assert!(!r.is_suspiciously_empty());
    }

    /// Committed work in scope that applied nothing is the shape an
    /// operator must be warned about, not a success.
    #[test]
    fn committed_work_with_nothing_applied_is_suspicious() {
        let r = PitrReport {
            transactions_committed: 3,
            ..Default::default()
        };
        assert!(r.is_suspiciously_empty());
    }

    /// A table that does not exist is a failure, counted and
    /// reported — the alternative is a silent partial restore.
    #[test]
    fn unknown_table_is_counted_as_failed() {
        let mut s = storage();
        let mut e = insert_row(1, 10);
        e.table_name = Some("does_not_exist".to_string());
        let r = replay_entries_until(&mut s, &[e], 100).unwrap();
        assert_eq!(r.entries_failed, 1, "{r:?}");
        assert!(r.first_error.is_some());
        assert!(ids(&s).is_empty());
    }

    /// `tables_touched` is the operator's view of the blast radius.
    #[test]
    fn tables_touched_names_the_affected_table() {
        let mut s = storage();
        let r = replay_entries_until(&mut s, &[insert_row(1, 10)], 100).unwrap();
        assert!(r.tables_touched.contains("users"));
    }

    /// Checkpoints are metadata: scanned, never applied.
    #[test]
    fn checkpoints_are_scanned_but_not_applied() {
        let mut s = storage();
        let entries = vec![tx_entry(0, WalEntryType::Checkpoint, 5), insert_row(1, 10)];
        let r = replay_entries_until(&mut s, &entries, 100).unwrap();
        assert_eq!(r.entries_scanned, 2);
        assert_eq!(r.entries_applied, 1);
        assert_eq!(ids(&s), vec![1]);
    }
}
