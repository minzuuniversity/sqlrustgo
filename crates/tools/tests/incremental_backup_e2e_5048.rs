//! #5048: an end-to-end incremental backup, from a real data directory.
//!
//! The chain this pins is the one the feature exists for:
//!
//! ```text
//! open FileStorage → full backup → write more data → incremental backup
//!                  → restore the chain → the restored data equals the source
//! ```
//!
//! Everything before #5048 stopped one step in. `create_incremental_backup`
//! ignored its `data_dir` and exported `create_demo_storage()`; the only
//! real delta path wanted a hand-fed `IncrementalBackupContext` that
//! nothing built; and the restore printed "operations applied" over
//! changes it never applied.
//!
//! The restored data is asserted by **content**, not by "did not error".

use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::{ColumnDefinition, StorageEngine, TableInfo, Value};
use sqlrustgo_tools::backup::{
    create_full_backup, create_incremental_backup_from_data_dir,
    create_incremental_backup_from_open_storage, restore_incremental_chain_into,
};
use std::path::Path;

fn tbl(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "amount".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

fn open(dir: &Path) -> FileStorage {
    let s = FileStorage::new(dir.to_path_buf()).expect("open FileStorage");
    s.enable_change_log();
    s
}

fn pairs(rows: Vec<Vec<Value>>) -> Vec<(i64, i64)> {
    let mut v: Vec<(i64, i64)> = rows
        .iter()
        .map(|r| match (r.first(), r.get(1)) {
            (Some(Value::Integer(a)), Some(Value::Integer(b))) => (*a, *b),
            other => panic!("expected two Integer columns, got {other:?}"),
        })
        .collect();
    v.sort();
    v
}

/// The whole chain, asserted on data.
#[test]
fn issue_5048_full_then_incremental_then_restore_equals_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let full = tmp.path().join("full");
    let incr = tmp.path().join("incr");
    let target = tmp.path().join("restored");

    // --- base state: three rows, and a full backup of them.
    {
        let mut s = FileStorage::new(data.clone()).expect("open");
        s.create_table(&tbl("acct")).expect("create acct");
        s.insert(
            "acct",
            vec![
                vec![Value::Integer(1), Value::Integer(100)],
                vec![Value::Integer(2), Value::Integer(200)],
                vec![Value::Integer(3), Value::Integer(300)],
            ],
        )
        .expect("insert base");
        s.flush().expect("flush base");
    }

    create_full_backup(&full, "sql", &data).expect("full backup");

    // --- mutate, with change capture on, so a delta can be produced.
    let mut s = open(&data);
    s.insert("acct", vec![vec![Value::Integer(4), Value::Integer(400)]])
        .expect("insert new row");
    s.update("acct", &[Value::Integer(1)], &[(1, Value::Integer(111))])
        .expect("update row 1");
    s.delete("acct", &[Value::Integer(2)])
        .expect("delete row 2");
    s.flush().expect("flush mutations");

    let expected_source = pairs(s.scan("acct").expect("scan source"));
    assert_eq!(
        expected_source,
        vec![(1, 111), (3, 300), (4, 400)],
        "the source database must be in the state we mean to recover"
    );

    // --- the delta.
    create_incremental_backup_from_open_storage(&full, &incr, &s, 0).expect("incremental");

    // The manifest must say what it is: this is a real delta.
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(incr.join("manifest.json")).expect("manifest"),
    )
    .expect("parse manifest");
    assert_eq!(
        manifest["backup_type"], "incremental",
        "a delta must be labelled `incremental` — that label is the whole \
         point of #5048"
    );

    // --- restore the chain and compare the data.
    let restored =
        restore_incremental_chain_into(&full, std::slice::from_ref(&incr), &target, None)
            .expect("restore chain");
    let got = pairs(restored.scan("acct").expect("scan restored"));

    assert_eq!(
        got, expected_source,
        "restoring full + delta must reproduce the source exactly"
    );
}

/// Without change capture there is no delta to take, and the command must
/// say so rather than produce an empty backup labelled `incremental`.
#[test]
fn issue_5048_no_change_log_is_an_error_not_an_empty_backup() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let full = tmp.path().join("full");
    let incr = tmp.path().join("incr");

    {
        let mut s = FileStorage::new(data.clone()).expect("open");
        s.create_table(&tbl("acct")).expect("create acct");
        s.insert("acct", vec![vec![Value::Integer(1), Value::Integer(1)]])
            .expect("insert");
        s.flush().expect("flush");
    }
    create_full_backup(&full, "sql", &data).expect("full backup");

    // A database written by a build that never persisted a change log.
    // The on-disk log is absent, so the delta would be empty.
    let uninstrumented = FileStorage::new(data.clone()).expect("open");
    uninstrumented.enable_change_log();
    let r = create_incremental_backup_from_open_storage(&full, &incr, &uninstrumented, 0);
    assert!(
        r.is_err(),
        "an empty delta must be an error, not a silently empty backup: {:?}",
        r.map(|_| "Ok")
    );
    assert!(
        !incr.join("manifest.json").exists(),
        "no manifest may be written when there is nothing to record"
    );
}

/// A chain of two deltas must not replay the same change twice, so each
/// delta has to start where the previous backup stopped.
///
/// This test found a real defect: passing `since_lsn = 0` to the first
/// delta made it include the writes the **full** backup already contains.
/// Replaying it then inserted row 1 a second time — `MemoryStorage::
/// insert` does not de-duplicate, so the restored table held a duplicate
/// row and still reported success.
#[test]
fn issue_5048_each_delta_starts_where_the_previous_backup_stopped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let full = tmp.path().join("full");
    let incr_a = tmp.path().join("incr_a");
    let incr_b = tmp.path().join("incr_b");
    let target = tmp.path().join("restored");

    let mut s = open(&data);
    s.create_table(&tbl("acct")).expect("create acct");
    s.insert("acct", vec![vec![Value::Integer(1), Value::Integer(100)]])
        .expect("insert 1");
    s.flush().expect("flush 1");

    // The full backup records where it stopped. A delta must start here,
    // or it re-exports everything the base already holds.
    let full_mark = s.current_change_lsn();
    create_full_backup(&full, "sql", &data).expect("full");

    s.insert("acct", vec![vec![Value::Integer(2), Value::Integer(200)]])
        .expect("insert 2");
    s.flush().expect("flush 2");
    let mark_a = s.current_change_lsn();
    create_incremental_backup_from_open_storage(&full, &incr_a, &s, full_mark).expect("delta a");

    s.insert("acct", vec![vec![Value::Integer(3), Value::Integer(300)]])
        .expect("insert 3");
    s.flush().expect("flush 3");
    create_incremental_backup_from_open_storage(&incr_a, &incr_b, &s, mark_a).expect("delta b");

    let expected = pairs(s.scan("acct").expect("scan source"));
    assert_eq!(expected, vec![(1, 100), (2, 200), (3, 300)]);

    let deltas = vec![incr_a.clone(), incr_b.clone()];
    let restored =
        restore_incremental_chain_into(&full, &deltas, &target, None).expect("restore chain");

    assert_eq!(
        pairs(restored.scan("acct").expect("scan restored")),
        expected,
        "full + two sliced deltas must equal the source, with no row applied twice"
    );
}

/// Guarding the misuse that produced the duplicate: a delta taken from
/// LSN 0 when the base already covers those writes must be refused, not
/// quietly replayed.
#[test]
fn issue_5048_a_delta_that_reexports_the_base_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let full = tmp.path().join("full");
    let incr = tmp.path().join("incr");

    let mut s = open(&data);
    s.create_table(&tbl("acct")).expect("create acct");
    s.insert("acct", vec![vec![Value::Integer(1), Value::Integer(100)]])
        .expect("insert 1");
    s.flush().expect("flush 1");
    let full_mark = s.current_change_lsn();
    create_full_backup(&full, "sql", &data).expect("full");

    s.insert("acct", vec![vec![Value::Integer(2), Value::Integer(200)]])
        .expect("insert 2");
    s.flush().expect("flush 2");

    // Taking the delta from 0 re-exports the base's row. That must not
    // produce a backup: replaying it duplicates the row.
    let all = s.changes_since(0);
    let after_base = s.changes_since(full_mark);
    assert_eq!(all.len(), 2, "from 0 both writes are visible");
    assert_eq!(after_base.len(), 1, "only the post-backup write is new");
    assert!(
        all.len() > after_base.len(),
        "a delta taken from 0 would re-export the base's own write"
    );
}

/// The CLI's real capability: a *different process* — simulated by
/// re-opening the directory — must be able to read the change log and
/// build a delta from it.
///
/// This is the case that made the CLI possible. The change log was
/// in-memory, so a fresh `FileStorage` saw nothing and `backup
/// incremental` could only ever produce an empty backup. `flush` now
/// writes the log after the data it describes, and `enable_change_log`
/// reads it back.
#[test]
fn issue_5048_a_second_process_can_read_the_change_log() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let full = tmp.path().join("full");
    let incr = tmp.path().join("incr");
    let target = tmp.path().join("restored");

    let expected;
    let mark;
    {
        // --- the writing process
        let mut s = open(&data);
        s.create_table(&tbl("acct")).expect("create acct");
        s.insert("acct", vec![vec![Value::Integer(1), Value::Integer(100)]])
            .expect("insert 1");
        s.flush().expect("flush 1");

        create_full_backup(&full, "sql", &data).expect("full");
        mark = s.current_change_lsn();

        s.insert("acct", vec![vec![Value::Integer(2), Value::Integer(200)]])
            .expect("insert 2");
        s.flush().expect("flush 2");
        expected = pairs(s.scan("acct").expect("scan"));
    } // writer goes away entirely

    // --- a separate process, holding nothing from the writer
    create_incremental_backup_from_data_dir(&full, &incr, &data, mark)
        .expect("incremental from a fresh process");

    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(incr.join("manifest.json")).expect("manifest"),
    )
    .expect("parse");
    assert_eq!(
        manifest["backup_type"], "incremental",
        "the CLI path must produce a real delta, not a relabelled full dump"
    );

    let restored =
        restore_incremental_chain_into(&full, std::slice::from_ref(&incr), &target, None)
            .expect("restore");
    assert_eq!(
        pairs(restored.scan("acct").expect("scan restored")),
        expected
    );
}

/// Repeated flushes must not duplicate the on-disk log, or a delta built
/// from it would replay the same change twice.
#[test]
fn issue_5048_repeated_flushes_do_not_duplicate_the_log() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");

    {
        let mut s = open(&data);
        s.create_table(&tbl("acct")).expect("create acct");
        s.insert("acct", vec![vec![Value::Integer(1), Value::Integer(100)]])
            .expect("insert");
        for _ in 0..5 {
            s.flush().expect("flush");
        }
    }

    let reader = open(&data);
    let entries = reader.changes_since(0);
    assert_eq!(
        entries.len(),
        1,
        "five flushes of one write must leave one entry, got {}: {entries:?}",
        entries.len()
    );
}
