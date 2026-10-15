//! #5250 reproducer — full engine layer autocommit INSERT, then read the
//! WAL. The engine path is `begin_implicit_dml_tx → storage.insert_in_db →
//! commit_implicit_dml_tx`. We need to know whether `commit_implicit_dml_tx`
//! reaches `WalStorage::commit_transaction` (which writes the Commit entry)
//! or whether something in between drops it.
//!
//! Run: cargo test -p sqlrustgo-storage --test repro_5250_via_engine -- --nocapture

use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::wal::{WalEntryType, WalManager};
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value, WalStorage,
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
fn full_engine_layer_autocommit_writes_what_to_wal() {
    use sqlrustgo_storage::Record;
    let d: PathBuf = std::env::temp_dir().join(format!("r5250_engine_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();

    let inner = FileStorage::new(d.clone()).expect("FileStorage");
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("wal");
    let mut s: WalStorage<FileStorage, FileBackedWalManager> =
        WalStorage::new(inner, wal).expect("WalStorage");

    // Mimic the *exact* code path in `engine_dml::execute_insert` for an
    // autocommit row, except go through `commit_transaction_for` to mirror
    // what `commit_implicit_dml_tx(true)` does.
    s.create_database("d1").unwrap();
    s.create_table_in_db("d1", &table("t")).unwrap();

    let tx_id = s.begin_transaction().unwrap();
    s.set_current_tx_id(tx_id);
    let record: Record = vec![Value::Integer(1), Value::Integer(10)];
    s.insert_in_db("d1", "t", vec![record]).unwrap();
    // Drop the inner guard before calling commit_transaction_for — the
    // engine layer drops its RwLock write guard too before commit.
    drop(s);
    let mut s2: WalStorage<FileStorage, FileBackedWalManager> = {
        let inner = FileStorage::new(d.clone()).expect("FileStorage");
        let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("wal");
        WalStorage::new(inner, wal).expect("WalStorage")
    };
    // Mirror `commit_implicit_dml_tx`'s second pass: re-assert the slot,
    // then commit_transaction_for. NOTE: begin_transaction_for — used by
    // the engine layer to BEGIN — exists too; let me try with it.
    s2.set_current_tx_id(tx_id);
    s2.commit_transaction_for(tx_id).unwrap();
    drop(s2);

    let mut replay_wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let entries = replay_wal.recover().unwrap();
    let types: Vec<WalEntryType> = entries.iter().map(|e| e.entry_type).collect();
    eprintln!("entries = {types:?}");
    eprintln!("count   = {}", entries.len());

    let _ = std::fs::remove_dir_all(&d);
}
