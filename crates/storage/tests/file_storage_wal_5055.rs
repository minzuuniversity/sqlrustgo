//! #5055: `FileStorage::new_with_wal` must actually write a WAL.
//!
//! Before this, the constructor computed the WAL path and dropped it:
//!
//! ```ignore
//! let _wal_path = data_dir.join("sqlrustgo.wal");
//! ```
//!
//! Nothing was ever appended, and `is_wal_enabled()` was never
//! overridden so the trait default answered `false`. `admin pitr` then
//! found an empty or absent WAL and still printed `pitr ok` and exited
//! 0. Every test here is written to fail if the write path regresses to
//! a no-op, and none of them can be satisfied by "the log file exists"
//! alone — they assert on entry *content*.

use sqlrustgo_storage::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, StorageEngine, TableInfo,
};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::wal::{WalEntry, WalEntryType};
use sqlrustgo_storage::wal_legacy::WalReader;
use sqlrustgo_types::Value;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fs_wal_5055_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn users_table() -> TableInfo {
    TableInfo {
        name: "users".into(),
        columns: vec![
            ColumnDefinition::new("id", "INTEGER"),
            ColumnDefinition::new("name", "VARCHAR(64)"),
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
        original_sql: String::new(),
    }
}

fn read_wal(dir: &std::path::Path) -> Vec<WalEntry> {
    let path = dir.join("sqlrustgo.wal");
    assert!(
        path.exists(),
        "no WAL at {} — new_with_wal is not opening one",
        path.display()
    );
    let mut reader = WalReader::new(&path).unwrap();
    reader.read_all().unwrap()
}

#[test]
fn new_with_wal_reports_wal_enabled() {
    let dir = temp_dir("enabled");
    let s = FileStorage::new_with_wal(dir.clone()).unwrap();
    assert!(
        s.is_wal_enabled(),
        "new_with_wal must report is_wal_enabled() == true; the trait \
         default is false, so this is exactly what the old code got wrong"
    );
}

#[test]
fn plain_new_reports_wal_disabled() {
    let dir = temp_dir("disabled");
    let s = FileStorage::new(dir.clone()).unwrap();
    assert!(
        !s.is_wal_enabled(),
        "a FileStorage built by `new` has no WAL and must say so"
    );
    assert!(
        !dir.join("sqlrustgo.wal").exists(),
        "`new` must not create a WAL file"
    );
}

#[test]
fn insert_writes_one_entry_per_row_with_table_name() {
    let dir = temp_dir("insert");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert(
        "users",
        vec![
            vec![Value::Integer(1), Value::Text("Alice".into())],
            vec![Value::Integer(2), Value::Text("Bob".into())],
        ],
    )
    .unwrap();

    let entries = read_wal(&dir);
    let inserts: Vec<&WalEntry> = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Insert)
        .collect();
    assert_eq!(inserts.len(), 2, "expected one Insert entry per row");
    for e in &inserts {
        assert_eq!(
            e.table_name.as_deref(),
            Some("users"),
            "every row entry must name its table; a replay that has to \
             guess would write rows into the wrong table"
        );
    }
    // The payload must be decodable back to the original row.
    let decoded =
        sqlrustgo_storage::wal_record_codec::bytes_to_record(inserts[0].data.as_ref().unwrap())
            .unwrap();
    assert_eq!(
        decoded,
        vec![Value::Integer(1), Value::Text("Alice".into())]
    );
}

#[test]
fn update_writes_entry_with_post_image() {
    let dir = temp_dir("update");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert(
        "users",
        vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
    )
    .unwrap();
    // #5055: `insert` parks rows in the insert buffer and `update`
    // reads `tables`, so the buffer has to be flushed first. (That
    // visibility gap is its own defect, tracked separately — it is
    // not what this file is testing.)
    s.flush_all_buffers().unwrap();
    s.update(
        "users",
        &[Value::Integer(1)],
        &[(1, Value::Text("Alicia".into()))],
    )
    .unwrap();

    let entries = read_wal(&dir);
    let updates: Vec<&WalEntry> = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Update)
        .collect();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].table_name.as_deref(), Some("users"));
    let decoded =
        sqlrustgo_storage::wal_record_codec::bytes_to_record(updates[0].data.as_ref().unwrap())
            .unwrap();
    assert_eq!(
        decoded,
        vec![Value::Integer(1), Value::Text("Alicia".into())],
        "an Update entry must carry the full post-image, not the \
         assignment list — replay must not have to re-run the predicate"
    );
}

#[test]
fn delete_writes_entry_per_removed_row() {
    let dir = temp_dir("delete");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert(
        "users",
        vec![
            vec![Value::Integer(1), Value::Text("Alice".into())],
            vec![Value::Integer(2), Value::Text("Bob".into())],
        ],
    )
    .unwrap();
    s.flush_all_buffers().unwrap();
    let n = s.delete("users", &[Value::Integer(1)]).unwrap();
    assert_eq!(n, 1);

    let entries = read_wal(&dir);
    let deletes: Vec<&WalEntry> = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Delete)
        .collect();
    assert_eq!(deletes.len(), 1, "only the matching row should be logged");
    assert_eq!(deletes[0].table_name.as_deref(), Some("users"));
    assert_eq!(
        deletes[0].key.as_deref(),
        Some(1i64.to_le_bytes().as_slice()),
        "a Delete is keyed by primary key so replay can remove by key"
    );
}

#[test]
fn delete_if_writes_entries() {
    let dir = temp_dir("delete_if");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert(
        "users",
        vec![
            vec![Value::Integer(1), Value::Text("Alice".into())],
            vec![Value::Integer(2), Value::Text("Bob".into())],
        ],
    )
    .unwrap();
    s.flush_all_buffers().unwrap();
    // Filter on the second column — the path `delete` does not take.
    let bob_filter: RowFilter =
        Box::new(|row: &Record| row.get(1) == Some(&Value::Text("Bob".into())));
    let n = s.delete_if("users", &bob_filter).unwrap();
    assert_eq!(n, 1);

    let entries = read_wal(&dir);
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Delete)
            .count(),
        1,
        "delete_if must write to the WAL too, not just `delete`"
    );
}

#[test]
fn update_if_writes_entries() {
    let dir = temp_dir("update_if");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert(
        "users",
        vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
    )
    .unwrap();
    s.flush_all_buffers().unwrap();
    let alice_filter: RowFilter =
        Box::new(|row: &Record| row.get(1) == Some(&Value::Text("Alice".into())));
    let mutation = RowMutation::new(vec![(1, Value::Text("Alicia".into()))], 0);
    let n = s.update_if("users", &alice_filter, &mutation).unwrap();
    assert_eq!(n, 1);

    let entries = read_wal(&dir);
    let updates: Vec<&WalEntry> = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Update)
        .collect();
    assert_eq!(updates.len(), 1, "update_if must write to the WAL too");
    assert_eq!(
        sqlrustgo_storage::wal_record_codec::bytes_to_record(updates[0].data.as_ref().unwrap())
            .unwrap(),
        vec![Value::Integer(1), Value::Text("Alicia".into())]
    );
}

#[test]
fn committed_transaction_logs_begin_and_commit_under_same_tx_id() {
    let dir = temp_dir("commit");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    let tx = s.begin_transaction().unwrap();
    s.insert(
        "users",
        vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
    )
    .unwrap();
    s.commit_transaction().unwrap();

    let entries = read_wal(&dir);
    let begin = entries
        .iter()
        .find(|e| e.entry_type == WalEntryType::Begin)
        .expect("no Begin entry");
    let commit = entries
        .iter()
        .find(|e| e.entry_type == WalEntryType::Commit)
        .expect("no Commit entry");
    let insert = entries
        .iter()
        .find(|e| e.entry_type == WalEntryType::Insert)
        .expect("no Insert entry");

    assert_eq!(begin.tx_id, tx);
    assert_eq!(
        commit.tx_id, tx,
        "Commit must name the same tx as its rows. Logging it after \
         current_tx_id is reset would record tx 0 and orphan the rows."
    );
    assert_eq!(insert.tx_id, tx);
}

#[test]
fn rolled_back_transaction_logs_rollback_under_same_tx_id() {
    let dir = temp_dir("rollback");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    let tx = s.begin_transaction().unwrap();
    s.insert(
        "users",
        vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
    )
    .unwrap();
    s.rollback_transaction().unwrap();

    let entries = read_wal(&dir);
    let rb = entries
        .iter()
        .find(|e| e.entry_type == WalEntryType::Rollback)
        .expect("no Rollback entry");
    assert_eq!(rb.tx_id, tx);
    assert!(
        !entries.iter().any(|e| e.entry_type == WalEntryType::Commit),
        "a rolled-back tx must not also log a Commit"
    );
}

#[test]
fn no_wal_backend_writes_nothing() {
    let dir = temp_dir("nowal");
    let mut s = FileStorage::new(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert(
        "users",
        vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
    )
    .unwrap();
    assert!(
        !dir.join("sqlrustgo.wal").exists(),
        "a FileStorage without a WAL must not create one"
    );
}

#[test]
fn reopening_continues_the_lsn_sequence() {
    // #5055: `WalWriter::with_config` opened the file with
    // OpenOptions::append but started `lsn` at 0, so a second process
    // restarting LSN numbering produced duplicate LSNs in one file.
    let dir = temp_dir("lsn");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
    }
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(2), Value::Text("Bob".into())]],
        )
        .unwrap();
    }
    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let lsns: Vec<u64> = entries.iter().map(|e| e.lsn).collect();
    assert!(
        lsns.windows(2).all(|w| w[1] > w[0]),
        "LSNs must be strictly increasing across a reopen, got {lsns:?}"
    );
}

#[test]
fn wal_manager_sees_the_entries_written() {
    // The `WalManager::recover` path is what a real recovery engine
    // uses; assert it agrees with the on-disk reader.
    let dir = temp_dir("recover");
    let s = FileStorage::new_with_wal(dir.clone()).unwrap();
    assert_eq!(s.wal_current_lsn(), 0);
    drop(s);

    let mut s2 = FileStorage::new_with_wal(dir.clone()).unwrap();
    s2.create_table(&users_table()).unwrap();
    s2.insert(
        "users",
        vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
    )
    .unwrap();
    let entries = s2.recover_wal_entries().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.entry_type == WalEntryType::Insert)
            .count(),
        1
    );
}

// ---------------------------------------------------------------------------
// #5055: the whole point — WAL written by the engine, replayed into a
// data directory, rows actually present afterwards.
// ---------------------------------------------------------------------------

use sqlrustgo_storage::pitr::{replay_entries_until, PitrReport};

fn restored_rows(dir: &std::path::Path) -> Vec<Vec<Value>> {
    // Reopen from disk: anything still sitting in an insert buffer would
    // not be here, which is exactly the failure this guards against.
    let s = FileStorage::new(dir.to_path_buf()).unwrap();
    s.scan("users").unwrap()
}

#[test]
fn committed_inserts_survive_a_pitr_replay() {
    let dir = temp_dir("pitr_insert");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert(
            "users",
            vec![
                vec![Value::Integer(1), Value::Text("Alice".into())],
                vec![Value::Integer(2), Value::Text("Bob".into())],
            ],
        )
        .unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }

    // Restore into a *different*, empty directory — the real PITR
    // shape: base backup plus log, not log applied over itself.
    let target = temp_dir("pitr_insert_target");
    {
        let mut s = FileStorage::new(target.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
    }

    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let target_ts = entries.iter().map(|e| e.timestamp).max().unwrap();

    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = replay_entries_until(&mut restore, &entries, target_ts).unwrap();
    restore.flush_all_buffers().unwrap();

    assert_eq!(report.entries_applied, 2, "report: {report:?}");
    assert_eq!(report.entries_failed, 0);
    assert_eq!(report.transactions_committed, 1);
    assert!(report.tables_touched.contains("users"));

    let rows = restored_rows(&target);
    assert_eq!(
        rows.len(),
        2,
        "the restored directory must actually contain the rows, got {rows:?}"
    );
    assert!(rows.contains(&vec![Value::Integer(1), Value::Text("Alice".into())]));
    assert!(rows.contains(&vec![Value::Integer(2), Value::Text("Bob".into())]));
}

#[test]
fn uncommitted_rows_are_not_restored() {
    let dir = temp_dir("pitr_uncommitted");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
        s.flush_all_buffers().unwrap();
        // No COMMIT: a crash here must not resurrect the row.
        s.begin_transaction().unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(2), Value::Text("Ghost".into())]],
        )
        .unwrap();
        s.flush_all_buffers().unwrap();
    }

    let target = temp_dir("pitr_uncommitted_target");
    {
        let mut s = FileStorage::new(target.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
    }
    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let target_ts = entries.iter().map(|e| e.timestamp).max().unwrap();

    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = replay_entries_until(&mut restore, &entries, target_ts).unwrap();
    restore.flush_all_buffers().unwrap();

    assert_eq!(report.entries_applied, 1, "only the autocommit row");
    assert_eq!(report.entries_skipped, 1, "the open tx must be skipped");
    assert_eq!(report.active_transactions_at_target, 1);

    let rows = restored_rows(&target);
    assert!(
        !rows.contains(&vec![Value::Integer(2), Value::Text("Ghost".into())]),
        "an uncommitted row must not be restored, got {rows:?}"
    );
}

#[test]
fn rolled_back_rows_are_not_restored() {
    let dir = temp_dir("pitr_rollback");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
        s.rollback_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }

    let target = temp_dir("pitr_rollback_target");
    {
        let mut s = FileStorage::new(target.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
    }
    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let target_ts = entries.iter().map(|e| e.timestamp).max().unwrap();

    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = replay_entries_until(&mut restore, &entries, target_ts).unwrap();
    restore.flush_all_buffers().unwrap();

    assert_eq!(report.transactions_aborted, 1);
    assert_eq!(report.entries_applied, 0);
    assert!(restored_rows(&target).is_empty());
}

#[test]
fn entries_after_the_target_are_not_applied() {
    let dir = temp_dir("pitr_window");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }

    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let max_ts = entries.iter().map(|e| e.timestamp).max().unwrap();

    let target = temp_dir("pitr_window_target");
    {
        let mut s = FileStorage::new(target.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
    }
    // A target one second *before* the last entry excludes it.
    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = replay_entries_until(&mut restore, &entries, max_ts.saturating_sub(1)).unwrap();
    restore.flush_all_buffers().unwrap();

    assert!(report.entries_after_target > 0);
    assert_eq!(report.entries_applied, 0);
    assert!(
        restored_rows(&target).is_empty(),
        "nothing at or before the target should have been replayed"
    );
}

#[test]
fn update_and_delete_in_the_log_are_replayed_in_order() {
    let dir = temp_dir("pitr_order");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
        s.flush_all_buffers().unwrap();
        s.update(
            "users",
            &[Value::Integer(1)],
            &[(1, Value::Text("Alicia".into()))],
        )
        .unwrap();
        s.delete("users", &[Value::Integer(1)]).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(2), Value::Text("Bob".into())]],
        )
        .unwrap();
        s.flush_all_buffers().unwrap();
    }

    let target = temp_dir("pitr_order_target");
    {
        let mut s = FileStorage::new(target.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
    }
    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let target_ts = entries.iter().map(|e| e.timestamp).max().unwrap();

    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = replay_entries_until(&mut restore, &entries, target_ts).unwrap();
    restore.flush_all_buffers().unwrap();

    assert_eq!(report.entries_failed, 0, "{:?}", report.first_error);
    let rows = restored_rows(&target);
    assert_eq!(
        rows,
        vec![vec![Value::Integer(2), Value::Text("Bob".into())]],
        "insert -> update -> delete must replay in that order; \
         got {rows:?}"
    );
}

#[test]
fn replay_does_not_duplicate_rows_already_in_the_base() {
    // PITR replays on top of a base backup that already holds the rows
    // the log mentions. Without dedup, every restore doubles the table.
    let dir = temp_dir("pitr_dedup");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
        s.flush_all_buffers().unwrap();
    }

    // The "base backup" is a copy of the finished directory.
    let target = temp_dir("pitr_dedup_target");
    std::fs::copy(dir.join("users.json"), target.join("users.json")).expect("base copy");

    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let entries = reader.read_all().unwrap();
    let target_ts = entries.iter().map(|e| e.timestamp).max().unwrap();

    let mut restore = FileStorage::new(target.clone()).unwrap();
    replay_entries_until(&mut restore, &entries, target_ts).unwrap();
    restore.flush_all_buffers().unwrap();

    let rows = restored_rows(&target);
    assert_eq!(
        rows.len(),
        1,
        "replaying onto a base that already has the row must not \
         duplicate it; got {rows:?}"
    );
}

#[test]
fn row_entry_without_a_table_name_is_refused_not_guessed() {
    // #5055: a WAL written before table names were recorded. The
    // id hash is not reversible, so a name-less entry whose table is
    // absent must fail loudly rather than land in a guessed table.
    let dir = temp_dir("pitr_noname");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(1), Value::Text("Alice".into())]],
        )
        .unwrap();
        s.flush_all_buffers().unwrap();
    }

    // Strip the recorded names, as a pre-#5055 writer would have left
    // them.
    let mut reader = WalReader::new(&dir.join("sqlrustgo.wal")).unwrap();
    let mut entries = reader.read_all().unwrap();
    for e in &mut entries {
        e.table_name = None;
    }

    let target = temp_dir("pitr_noname_target");
    {
        // Deliberately no `users` table: the id cannot be resolved.
        let _s = FileStorage::new(target.clone()).unwrap();
    }
    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = replay_entries_until(&mut restore, &entries, u64::MAX).unwrap();
    restore.flush_all_buffers().unwrap();

    assert_eq!(
        report.entries_failed, 1,
        "an unresolvable row entry must be counted as failed"
    );
    assert!(
        report
            .first_error
            .as_deref()
            .is_some_and(|m| m.contains("cannot resolve table")),
        "expected an explicit 'cannot resolve table' error, got {:?}",
        report.first_error
    );
    assert!(
        restored_rows(&target).is_empty(),
        "nothing may be written when the table cannot be resolved"
    );
}

/// #5055 regression: `next_tx_id` used to derive ids from
/// `now_nanos % 1_000_000` plus the undo-log length, so back-to-back
/// BEGIN/COMMIT pairs could reuse an id. In the log that is not
/// cosmetic — a replay decides "committed" per `tx_id`, so a recycled
/// id lets a rolled-back transaction replay as committed.
#[test]
fn back_to_back_transactions_get_distinct_tx_ids() {
    let dir = temp_dir("txids");
    let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    let mut ids = std::collections::HashSet::new();
    for i in 0..200i64 {
        let tx = s.begin_transaction().unwrap();
        assert!(
            ids.insert(tx),
            "tx id {tx} was handed out twice (iteration {i}); a recycled \\
             id makes a rolled-back transaction replay as committed"
        );
        s.insert(
            "users",
            vec![vec![Value::Integer(i), Value::Text(format!("u{i}"))]],
        )
        .unwrap();
        s.commit_transaction().unwrap();
    }
    s.flush_all_buffers().unwrap();

    // And the log agrees: every Commit names a distinct transaction.
    let entries = read_wal(&dir);
    let commits: std::collections::HashSet<u64> = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Commit)
        .map(|e| e.tx_id)
        .collect();
    assert_eq!(commits.len(), 200, "200 transactions, 200 distinct ids");
}

/// The consequence, stated as a data assertion: a rolled-back
/// transaction must not come back even when a later transaction
/// reuses the same id.
#[test]
fn recycled_tx_id_does_not_resurrect_a_rolled_back_row() {
    let dir = temp_dir("recycle");
    {
        let mut s = FileStorage::new_with_wal(dir.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice_row(1, "Doomed")]).unwrap();
        s.rollback_transaction().unwrap();
        // A second transaction with an id of its own.
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice_row(2, "Keeper")]).unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }

    let target = temp_dir("recycle_target");
    {
        let mut s = FileStorage::new(target.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
    }
    let entries = read_wal(&dir);
    let mut restore = FileStorage::new(target.clone()).unwrap();
    let report = crate_pitr(&mut restore, &entries);
    restore.flush_all_buffers().unwrap();

    assert_eq!(report.entries_failed, 0, "{:?}", report.first_error);
    let rows = FileStorage::new(target.clone())
        .unwrap()
        .scan("users")
        .unwrap();
    assert!(
        !rows.contains(&alice_row(1, "Doomed")),
        "a rolled-back row must not be restored, got {rows:?}"
    );
    assert!(rows.contains(&alice_row(2, "Keeper")));
}

fn alice_row(id: i64, name: &str) -> Vec<Value> {
    vec![Value::Integer(id), Value::Text(name.to_string())]
}

fn crate_pitr(storage: &mut FileStorage, entries: &[WalEntry]) -> PitrReport {
    replay_entries_until(storage, entries, u64::MAX).unwrap()
}
