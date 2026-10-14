//! #5250 reproducer v2 — mimic the *exact* production startup shape:
//! FileStorage::new_with_wal + recovery via a fresh FileBackedWalManager.
//!
//! Production recovery calls `recovery.recover(&mut file_storage, ...)` — note
//! `file_storage` here is the *raw* FileStorage, NOT wrapped in
//! ParallelWalStorage. So whichever entries are in the WAL are written by
//! FileStorage's own insert_at/commit_transaction_for bodies.
//!
//! We do an autocommit INSERT through the engine layer (commit_implicit_dml_tx
//! path) and then read the WAL the same way the server does.

use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::wal::{WalEntryType, WalManager};
use sqlrustgo_storage::{ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo, Value};
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

#[test]
fn fs_new_with_wal_autocommit_via_storage_api_writes_what() {
    let d: PathBuf = std::env::temp_dir().join(format!("r5250_fswal_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();

    // EXACT production construction:
    let mut fs = FileStorage::new_with_wal(d.clone()).expect("FileStorage::new_with_wal");
    let wal_path = d.join("sqlrustgo.wal");

    fs.create_database("d1").unwrap();
    fs.create_table_in_db("d1", &table("t")).unwrap();

    // Drive the *engine* autocommit path manually: begin_transaction, set,
    // insert_in_db, commit_transaction_for — the same calls
    // `commit_implicit_dml_tx(true)` makes.
    let tx_id = fs.begin_transaction().unwrap();
    fs.set_current_tx_id(tx_id);
    let record: Record = vec![Value::Integer(1), Value::Integer(10)];
    fs.insert_in_db("d1", "t", vec![record]).unwrap();
    fs.commit_transaction_for(tx_id).unwrap();

    // Recovery uses a fresh handle over the same file.
    let mut recovery = StatefulRecoveryEngine::new();
    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();
    let report = recovery.recover(&mut fs, &mut wal).unwrap();
    eprintln!("fs_new_with_wal autocommit report: {report:?}");

    let entries = wal.recover().unwrap();
    let types: Vec<WalEntryType> = entries.iter().map(|e| e.entry_type).collect();
    eprintln!("fs_new_with_wal entries: {types:?}");

    assert_eq!(
        report.entries_total, 3,
        "expected Begin+Insert+Commit, got {types:?}"
    );
    assert_eq!(report.committed_txns, 1);
    assert_eq!(report.skipped_entries, 0);

    let _ = std::fs::remove_dir_all(&d);
}
