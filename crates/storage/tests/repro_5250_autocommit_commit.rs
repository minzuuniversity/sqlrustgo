//! #5250 minimal reproducer — does an autocommit INSERT (via storage-level
//! `insert_in_db`, no engine/facade) into WalStorage<FileStorage,
//! FileBackedWalManager> write a Commit entry to the WAL?
//!
//! WalStorage::insert_in_db calls `log_insert` for each row, but Begin/Commit
//! are written only by FileStorage::begin_transaction /
//! FileStorage::commit_transaction_for — which the engine layer is supposed
//! to invoke. Without those calls, the only entry the WAL has is Insert,
//! and on recovery `entries_total = 1, committed_txns = 0` — the exact
//! pattern reported in the #5250 2026-10-12 recheck.
//!
//! Run: cargo test -p sqlrustgo-storage --test repro_5250_autocommit_commit -- --nocapture

use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::wal::{WalEntryType, WalManager};
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
        columns: vec![col("id", true), col("v", false)],
        ..Default::default()
    }
}

fn row(id: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(id * 10)]
}

type Stack = WalStorage<FileStorage, FileBackedWalManager>;

fn fresh_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("r5250_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn stack(d: &PathBuf) -> Stack {
    let inner = FileStorage::new(d.clone()).expect("FileStorage");
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("wal");
    WalStorage::new(inner, wal).expect("WalStorage")
}

/// Path A — call the storage API the same way `engine_dml::execute_insert`
/// does for the "started_implicit" autocommit case: begin_transaction, then
/// insert_in_db, then commit_transaction_for. This is what *should* land
/// in the production binary's WAL when an autocommit INSERT runs.
#[test]
fn autocommit_via_begin_insert_commit_writes_commit_entry() {
    let d = fresh_dir("begin_insert_commit");

    let mut s = stack(&d);
    s.create_database("d1").unwrap();
    s.create_table_in_db("d1", &table("t")).unwrap();

    // Mimic `begin_implicit_dml_tx` for autocommit.
    let tx_id = s.begin_transaction().unwrap();
    s.set_current_tx_id(tx_id);
    s.insert_in_db("d1", "t", vec![row(1)]).unwrap();
    s.commit_transaction_for(tx_id).unwrap();

    let mut replay_wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let entries = replay_wal.recover().unwrap();
    let types: Vec<WalEntryType> = entries.iter().map(|e| e.entry_type).collect();
    eprintln!("begin_insert_commit path: entries = {types:?}");
    assert!(
        types.contains(&WalEntryType::Begin),
        "WAL must contain a Begin entry for tx_id={tx_id}, got {types:?}"
    );
    assert!(
        types.contains(&WalEntryType::Insert),
        "WAL must contain an Insert entry, got {types:?}"
    );
    assert!(
        types.contains(&WalEntryType::Commit),
        "WAL must contain a Commit entry for tx_id={tx_id}, got {types:?}"
    );

    let mut engine: StatefulRecoveryEngine<Stack> = StatefulRecoveryEngine::new();
    let report = engine.recover(&mut s, &mut replay_wal).unwrap();
    eprintln!("begin_insert_commit report: {report:?}");
    assert_eq!(report.entries_total, 3);
    assert_eq!(report.committed_txns, 1);
    assert_eq!(report.skipped_entries, 0);

    let _ = std::fs::remove_dir_all(&d);
}

/// Path B — call only `insert_in_db` (the path that `WalStorage` exposes
/// for engine-side DML). This is what happens if the engine never invokes
/// via `commit_transaction_for` — the bug shape.
#[test]
fn autocommit_via_insert_only_does_not_write_commit_entry() {
    let d = fresh_dir("insert_only");

    let mut s = stack(&d);
    s.create_database("d1").unwrap();
    s.create_table_in_db("d1", &table("t")).unwrap();
    s.insert_in_db("d1", "t", vec![row(1)]).unwrap();

    let mut replay_wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let entries = replay_wal.recover().unwrap();
    let types: Vec<WalEntryType> = entries.iter().map(|e| e.entry_type).collect();
    eprintln!("insert_only path: entries = {types:?}");

    // Show, don't assert — this test is diagnostic. The point is to
    // observe whether `commit_implicit_dml_tx` was reached or not in the
    // production case. If the production binary mirrors Path B, that's
    // the bug.
    let _ = std::fs::remove_dir_all(&d);
}
