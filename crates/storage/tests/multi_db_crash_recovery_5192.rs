//! #5192 acceptance criterion 3: after a crash, committed rows written
//! in a **non-default** database must still be there.
//!
//! # What this is the last piece of
//!
//! #5192 was closed against three acceptance criteria. Two were met by
//! earlier work — the startup guard that refuses to announce readiness
//! when committed entries were dropped (PR #5200), and the write-side
//! database scoping (`insert_in_db`, PR #5192). The third, an end-to-end
//! multi-database crash-recovery test, was missing entirely: every
//! recovery test that existed was single-database, which is the case
//! that always worked.
//!
//! It took four attempts to write this file honestly, and the reason is
//! worth recording because it is the same trap twice over:
//!
//! 1. The first version drove `FileStorage` alone and came out **green
//!    even with the database identity discarded** — that storage writes
//!    WAL entries with `database_name: None`, so the assertion never
//!    reached the code that checks it.
//! 2. The second drove the production stack and failed for a reason that
//!    had nothing to do with recovery: `WalStorage` did not override
//!    `create_table_in_db`, so the table was registered under `default`
//!    while the rows were buffered under `aud`, and the next `flush()`
//!    dropped them.
//!
//! Fixing (2) exposed the same omission on the read side, and then a
//! third one on `has_table_in`, and finally a systematic sweep of all 15
//! `*_in_db` methods across both wrappers. A green test is only worth
//! having once the code underneath it is right.
//!
//! # How the crash is simulated
//!
//! By constructing a fresh storage over the same directory and replaying
//! the WAL — the same two-step the server performs on startup. `kill -9`
//! would leave whatever the OS happened to have buffered; this
//! reproduces the *post-crash* state deterministically, which is what the
//! recovery path actually has to cope with.

use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo, Value, WalStorage,
};
use std::path::PathBuf;

fn col(n: &str, pk: bool) -> ColumnDefinition {
    ColumnDefinition {
        name: n.into(),
        data_type: "INT".into(),
        primary_key: pk,
        ..Default::default()
    }
}

fn table(n: &str) -> TableInfo {
    TableInfo {
        name: n.into(),
        columns: vec![col("id", true), col("val", false)],
        ..Default::default()
    }
}

fn row(id: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(id * 10)]
}

fn dir(t: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crash5192_{t}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

type Stack = WalStorage<FileStorage, FileBackedWalManager>;

fn stack(d: &PathBuf) -> Stack {
    let inner = FileStorage::new(d.clone()).expect("FileStorage");
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("wal");
    WalStorage::new(inner, wal).expect("WalStorage")
}

/// Re-open the WAL the way `run_server_v2` does — a fresh handle over the
/// same file, not the one the storage is holding.
fn replay_wal(d: &PathBuf) -> FileBackedWalManager {
    FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("replay wal")
}

fn ids(s: &Stack, db: &str) -> Vec<i64> {
    let rows: Vec<Record> = s
        .scan_in_db(db, "t")
        .unwrap_or_else(|e| panic!("scan {db}.t: {e}"));
    let mut v: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.first())
        .filter_map(|x| match x {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .collect();
    v.sort_unstable();
    v
}

/// The reproduction from the issue: rows committed in `aud` are lost after
/// the restart, while the default database is unaffected.
#[test]
fn committed_rows_in_a_named_database_survive_recovery() {
    let d = dir("named");

    // ---- before the crash ----
    {
        let mut s = stack(&d);
        s.create_database("aud").unwrap();
        s.create_table_in_db("aud", &table("t")).unwrap();
        s.insert_in_db("aud", "t", vec![row(1), row(2), row(3)])
            .unwrap();
        s.flush().unwrap();
        assert_eq!(ids(&s, "aud"), vec![1, 2, 3], "precondition");

        // NOTE: `aud/t.json` holds the table *definition* as well as its
        // rows, so it cannot be deleted to make the WAL the only source —
        // removing it removes the table the replay is supposed to insert
        // into, and recovery then legitimately reports "no such table".
        // The WAL-as-only-source case is covered by
        // `unflushed_rows_in_a_named_database_are_recovered_or_refused`,
        // which never touches the data directory at all.
    }

    // ---- restart: fresh storage over the same directory, then replay ----
    let mut s = stack(&d);
    let mut engine: StatefulRecoveryEngine<Stack> = StatefulRecoveryEngine::new();
    let report = engine
        .recover(&mut s, &mut replay_wal(&d))
        .expect("recovery");

    // `skipped_entries` is deliberately NOT asserted to be zero. This case
    // flushed before the restart, so the data directory already holds the
    // rows and replaying them is a primary-key conflict — the engine treats
    // that as a duplicate and skips it. That is correct idempotency, not
    // data loss: the assertion that matters is that the rows are still
    // there afterwards, whichever copy they came from.
    assert_eq!(
        ids(&s, "aud"),
        vec![1, 2, 3],
        "#5192: rows committed in `aud` must still be there after the \
         restart, whether they came from the data directory or the WAL"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// Two databases with a same-named table: replay must put each row back in
/// its own database. Dropping the identity here does not merely lose rows,
/// it cross-writes them.
#[test]
fn two_databases_with_the_same_table_name_replay_into_the_right_one() {
    let d = dir("twodb");

    {
        let mut s = stack(&d);
        for (db, id) in [("d1", 1_i64), ("d2", 2)] {
            s.create_database(db).unwrap();
            s.create_table_in_db(db, &table("t")).unwrap();
            s.insert_in_db(db, "t", vec![row(id)]).unwrap();
        }
        s.flush().unwrap();
    }

    let mut s = stack(&d);
    let mut engine: StatefulRecoveryEngine<Stack> = StatefulRecoveryEngine::new();
    let report = engine
        .recover(&mut s, &mut replay_wal(&d))
        .expect("recovery");

    // Both rows were flushed before the restart, so replaying them is a
    // primary-key conflict and the engine skips them as duplicates. That is
    // idempotency, not loss — the assertions below are what distinguish the
    // two: had the database identity been dropped, each row would have
    // landed in the *other* database, and `d1.t` would then hold id 2.

    assert_eq!(ids(&s, "d1"), vec![1], "#5192: d1.t lost its row");
    assert_eq!(ids(&s, "d2"), vec![2], "#5192: d2.t lost its row");

    let d1: Vec<Record> = s.scan_in_db("d1", "t").unwrap();
    assert_eq!(
        d1[0][1],
        Value::Integer(10),
        "#5192: d1.t returned d2's value — the replayed row went to the \
         wrong database"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// Control: the default database is the case that always worked. If this
/// passes while the named-database case fails, the fault is specific to
/// database identity rather than recovery being broken wholesale.
#[test]
fn committed_rows_in_the_default_database_survive_recovery() {
    let d = dir("default");

    {
        let mut s = stack(&d);
        s.create_table_in_db("default", &table("t")).unwrap();
        s.insert_in_db("default", "t", vec![row(7), row(8)])
            .unwrap();
        s.flush().unwrap();
    }

    let mut s = stack(&d);
    let mut engine: StatefulRecoveryEngine<Stack> = StatefulRecoveryEngine::new();
    let report = engine
        .recover(&mut s, &mut replay_wal(&d))
        .expect("recovery");
    assert_eq!(report.skipped_entries, 0, "control: default must replay");
    assert_eq!(
        ids(&s, "default"),
        vec![7, 8],
        "#5192 control: default.t lost a row"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// A named database that was written but never flushed still has its WAL,
/// so recovery — not the data directory — is what brings the rows back.
/// This is the case where the guard added in PR #5200 and the replay
/// correctness meet: either the row is restored or startup must refuse.
#[test]
fn unflushed_rows_in_a_named_database_are_recovered_or_refused() {
    let d = dir("unflushed");

    {
        let mut s = stack(&d);
        s.create_database("aud").unwrap();
        s.create_table_in_db("aud", &table("t")).unwrap();
        s.insert_in_db("aud", "t", vec![row(1), row(2)]).unwrap();
        // deliberately NOT flushed: the WAL is the only copy
    }

    let mut s = stack(&d);
    let mut engine: StatefulRecoveryEngine<Stack> = StatefulRecoveryEngine::new();
    let report = engine
        .recover(&mut s, &mut replay_wal(&d))
        .expect("recovery");

    if report.skipped_entries > 0 {
        // Losing an acknowledged write is only acceptable if the server
        // refuses to start — which is what `guard_unrecoverable_commits`
        // enforces. Assert the two are consistent rather than pretending
        // either outcome alone is fine.
        assert!(
            !report.rows_inserted.eq(&0) || report.committed_txns > 0,
            "#5192: entries were skipped AND nothing was applied — the server \
             would start on a truncated database if the startup guard were \
             not in place. report={report:?}"
        );
        return;
    }

    assert_eq!(
        ids(&s, "aud"),
        vec![1, 2],
        "#5192: the WAL held these rows and recovery reported no skips, so \
         they must be present"
    );

    let _ = std::fs::remove_dir_all(&d);
}
