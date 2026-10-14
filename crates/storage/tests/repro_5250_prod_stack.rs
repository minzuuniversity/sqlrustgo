//! #5250 reproducer v3 — full production stack via storage API.
//!
//! Production stack:
//!   FileStorage::new_with_wal -> MvccStorage::new -> ParallelWalStorage::new
//!
//! Both FileStorage (via new_with_wal) and ParallelWalStorage hold their
//! own FileBackedWalManager handle over the same `sqlrustgo.wal` file.
//! Engine's `commit_implicit_dml_tx(true)` calls storage.commit_transaction_for
//! which dispatches via trait default to ParallelWalStorage::commit_transaction
//! (writing a Commit entry to the outer WAL).
//!
//! Insert path goes engine -> storage.insert_in_db -> ParallelWalStorage
//! (delegates) -> MvccStorage (delegates) -> FileStorage::insert_at
//! (writes an Insert entry via its internal WAL).
//!
//! Run: cargo test -p sqlrustgo-storage --test repro_5250_prod_stack -- --nocapture

use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::wal::{WalEntryType, WalManager};
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, MvccStorage, ParallelWalStorage, Record, StorageEngine,
    TableInfo, Value,
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

#[test]
fn production_stack_autocommit_writes_what_to_wal() {
    let d: PathBuf = std::env::temp_dir().join(format!("r5250_prodstack_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();

    let file_storage = FileStorage::new_with_wal(d.clone()).expect("FileStorage::new_with_wal");
    let mvcc_inner = MvccStorage::new(file_storage);
    let wal_manager = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let mut stack = ParallelWalStorage::new(mvcc_inner, wal_manager);

    stack.create_database("d1").unwrap();
    stack.create_table_in_db("d1", &table("t")).unwrap();

    // engine path: begin_implicit_dml_tx(true)
    let tx_id = stack.begin_transaction().unwrap();
    stack.set_current_tx_id(tx_id);
    let record: Record = vec![Value::Integer(1), Value::Integer(10)];
    stack.insert_in_db("d1", "t", vec![record]).unwrap();
    // engine path: commit_implicit_dml_tx(true)
    stack.commit_transaction_for(tx_id).unwrap();

    drop(stack);

    // Recovery uses a fresh FileBackedWalManager on the same path —
    // mirrors production (lib.rs:6904).
    let mut fs_for_recovery = FileStorage::new_with_wal(d.clone()).unwrap();
    let mut recovery_wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let mut recovery: StatefulRecoveryEngine<FileStorage> = StatefulRecoveryEngine::new();
    let report = recovery
        .recover(&mut fs_for_recovery, &mut recovery_wal)
        .unwrap();
    eprintln!("production-stack report: {report:?}");

    let entries = recovery_wal.recover().unwrap();
    let types: Vec<WalEntryType> = entries.iter().map(|e| e.entry_type).collect();
    eprintln!("production-stack entries: {types:?}");

    let _ = std::fs::remove_dir_all(&d);
}
