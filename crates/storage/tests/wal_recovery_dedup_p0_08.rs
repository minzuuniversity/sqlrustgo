//! P0-08 — the recovery replay's dedup semantics.
//!
//! # What is being guarded
//!
//! The replay has to answer "is this row already present?" before it
//! applies an `Insert`, because a point-in-time restore replays the log on
//! top of a base backup that already holds the rows the base had: applying
//! the log again would duplicate every one of them.
//!
//! The answer used to come from a full table scan per replayed row, which
//! made the replay quadratic (`RecoveryRowIndex` in
//! `crates/storage/src/recovery_engine.rs` has the measurement). It now
//! comes from a per-table index.
//!
//! An index can answer a *different* question from a scan if it is not fed
//! the same information. These tests pin the four cases where the two
//! disagree:
//!
//! 1. a row the table already holds → **skipped** (the PITR case);
//! 2. a row the same replay inserted earlier → **skipped**;
//! 3. a row the replay **updated** earlier → **applied**, because the
//!    update replaced it and the table no longer holds it;
//! 4. a row the replay **deleted** earlier → **applied**, for the same
//!    reason.
//!
//! Cases 3 and 4 are the ones a stale index would get wrong, and getting
//! them wrong loses committed data.

use std::path::PathBuf;

use sqlrustgo_storage::engine::{ColumnDefinition, TableData, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::recovery_engine::{RecoveryEngine, RecoveryEngineImpl};
use sqlrustgo_storage::wal::{MemoryWalManager, WalEntry, WalEntryType, WalManager};
use sqlrustgo_storage::wal_record_codec::record_to_bytes;
use sqlrustgo_storage::{Record, StorageEngine, Value};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_p008_sem_{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_table_info(name: &str) -> TableInfo {
    TableInfo {
        name: name.into(),
        columns: vec![
            ColumnDefinition {
                name: "id".into(),
                data_type: "INTEGER".into(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "v".into(),
                data_type: "INTEGER".into(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            },
        ],
        ..Default::default()
    }
}

fn row(id: i64, v: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(v)]
}

/// The 8-byte row key the WAL carries for an `Update` / `Delete`.
fn key_bytes(id: i64) -> Vec<u8> {
    id.to_le_bytes().to_vec()
}

fn insert_entry(lsn: u64, record: &Record) -> WalEntry {
    WalEntry {
        tx_id: 0,
        entry_type: WalEntryType::Insert,
        table_id: 0,
        key: None,
        data: Some(record_to_bytes(record)),
        lsn,
        timestamp: 0,
        table_name: Some("t".into()),
        database_name: Some("default".into()),
    }
}

fn update_entry(lsn: u64, id: i64, record: &Record) -> WalEntry {
    WalEntry {
        tx_id: 0,
        entry_type: WalEntryType::Update,
        table_id: 0,
        key: Some(key_bytes(id)),
        data: Some(record_to_bytes(record)),
        lsn,
        timestamp: 0,
        table_name: Some("t".into()),
        database_name: Some("default".into()),
    }
}

fn delete_entry(lsn: u64, id: i64) -> WalEntry {
    WalEntry {
        tx_id: 0,
        entry_type: WalEntryType::Delete,
        table_id: 0,
        key: Some(key_bytes(id)),
        data: None,
        lsn,
        timestamp: 0,
        table_name: Some("t".into()),
        database_name: Some("default".into()),
    }
}

fn storage_with_table(dir: &PathBuf) -> FileStorage {
    let storage = FileStorage::new(dir.clone()).unwrap();
    storage
        .insert_table(
            "t".into(),
            TableData {
                info: make_table_info("t"),
                rows: vec![],
            },
        )
        .unwrap();
    storage
}

fn rows_of(storage: &FileStorage) -> Vec<Record> {
    storage.scan_in_db("default", "t").unwrap()
}

/// 1. A row the table already holds is not applied again.
///
/// This is the point-in-time restore case: the base backup holds the row,
/// the log replays it, and applying it again would duplicate it.
#[test]
fn p0_08_row_already_in_the_table_is_not_duplicated() {
    let dir = temp_dir("already_present");
    let mut storage = storage_with_table(&dir);

    // The base backup's row.
    storage.force_insert_in_db("default", "t", row(1, 10)).unwrap();
    assert_eq!(rows_of(&storage).len(), 1);

    let mut wal = MemoryWalManager::new();
    wal.append(insert_entry(1, &row(1, 10))).unwrap();

    let mut engine = RecoveryEngineImpl;
    engine.recover(&mut storage, &mut wal).unwrap();

    let rows = rows_of(&storage);
    assert_eq!(
        rows.len(),
        1,
        "the replay must not duplicate a row the table already held: {:?}",
        rows
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 2. A row the same replay inserted earlier is not applied twice.
#[test]
fn p0_08_row_inserted_twice_in_one_replay_is_applied_once() {
    let dir = temp_dir("twice");
    let mut storage = storage_with_table(&dir);

    let mut wal = MemoryWalManager::new();
    wal.append(insert_entry(1, &row(1, 10))).unwrap();
    wal.append(insert_entry(2, &row(1, 10))).unwrap();

    let mut engine = RecoveryEngineImpl;
    engine.recover(&mut storage, &mut wal).unwrap();

    let rows = rows_of(&storage);
    assert_eq!(
        rows.len(),
        1,
        "the second insert of the same row must be skipped: {:?}",
        rows
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 3. A row the replay **updated** earlier is applied again.
///
/// The update replaced the row, so the table no longer holds the old
/// values. An index that was not told about the update would still list
/// the old row and skip this insert — losing a committed row.
#[test]
fn p0_08_row_updated_earlier_in_the_replay_is_applied_again() {
    let dir = temp_dir("updated");
    let mut storage = storage_with_table(&dir);

    let mut wal = MemoryWalManager::new();
    wal.append(insert_entry(1, &row(1, 10))).unwrap();
    wal.append(update_entry(2, 1, &row(1, 99))).unwrap();
    wal.append(insert_entry(3, &row(1, 10))).unwrap();

    let mut engine = RecoveryEngineImpl;
    engine.recover(&mut storage, &mut wal).unwrap();

    let rows = rows_of(&storage);
    assert_eq!(
        rows.len(),
        2,
        "the insert after the update must be applied: {:?}",
        rows
    );
    assert!(
        rows.iter().any(|r| r == &row(1, 10)),
        "the re-inserted row must be present: {:?}",
        rows
    );
    assert!(
        rows.iter().any(|r| r == &row(1, 99)),
        "the updated row must be present: {:?}",
        rows
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 4. A row the replay **deleted** earlier is applied again.
///
/// Same shape as case 3: the delete removed the row, so the table does not
/// hold it and the insert must be applied.
#[test]
fn p0_08_row_deleted_earlier_in_the_replay_is_applied_again() {
    let dir = temp_dir("deleted");
    let mut storage = storage_with_table(&dir);

    let mut wal = MemoryWalManager::new();
    wal.append(insert_entry(1, &row(1, 10))).unwrap();
    wal.append(delete_entry(2, 1)).unwrap();
    wal.append(insert_entry(3, &row(1, 10))).unwrap();

    let mut engine = RecoveryEngineImpl;
    engine.recover(&mut storage, &mut wal).unwrap();

    let rows = rows_of(&storage);
    assert_eq!(
        rows.len(),
        1,
        "the insert after the delete must be applied: {:?}",
        rows
    );
    assert!(
        rows.iter().any(|r| r == &row(1, 10)),
        "the re-inserted row must be present: {:?}",
        rows
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 5. Distinct rows are all applied.
#[test]
fn p0_08_distinct_rows_are_all_applied() {
    let dir = temp_dir("distinct");
    let mut storage = storage_with_table(&dir);

    let mut wal = MemoryWalManager::new();
    for i in 0..25i64 {
        wal.append(insert_entry(i as u64 + 1, &row(i, i * 2))).unwrap();
    }

    let mut engine = RecoveryEngineImpl;
    engine.recover(&mut storage, &mut wal).unwrap();

    let rows = rows_of(&storage);
    assert_eq!(rows.len(), 25, "every distinct row must be applied");
    let _ = std::fs::remove_dir_all(&dir);
}
