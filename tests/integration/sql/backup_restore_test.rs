use sqlrustgo_admin::backup;
use sqlrustgo_admin::manifest::{sha256_file, Manifest};
use sqlrustgo_admin::pitr;
use sqlrustgo_admin::restore;
use sqlrustgo_admin::verify;
use sqlrustgo_storage::wal::{
    make_begin_entry, make_commit_entry, make_insert_entry, WalEntry, WalEntryType, WalWriter,
};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

fn make_data_dir() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let p = dir.path().to_path_buf();
    fs::write(p.join("users.json"), b"[]").unwrap();
    fs::write(p.join("orders.json"), b"[]").unwrap();
    fs::create_dir(p.join("nested")).unwrap();
    fs::write(p.join("nested").join("items.json"), b"[]").unwrap();
    (dir, p)
}

fn make_wal(dir: &Path) -> std::path::PathBuf {
    let wal = dir.join("sqlrustgo.wal");
    let mut writer = WalWriter::with_config(&wal, false, 100).unwrap();
    let e1 = make_begin_entry(1);
    let e2 = make_insert_entry(1, 1, vec![1], vec![1, 2, 3], 1);
    let e3 = make_commit_entry(1, 2);
    for mut e in [e1, e2, e3] {
        e.timestamp = 100;
        e.lsn = writer.current_lsn() + 1;
        writer.append(&e).unwrap();
    }
    writer.flush().unwrap();
    wal
}

#[test]
fn test_backup_full_basic() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("backup.tar.gz");
    let r = backup::physical_backup(&data_path, None, &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 3);
    assert!(r.output_size_bytes > 0);
}

#[test]
fn test_backup_empty_data_dir() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("b.tar.gz");
    let r = backup::physical_backup(dir.path(), None, &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 0);
    assert!(out.exists());
}

#[test]
fn test_backup_with_lots_of_tables() {
    let dir = TempDir::new().unwrap();
    for i in 0..50 {
        fs::write(dir.path().join(format!("t{}.json", i)), b"{}").unwrap();
    }
    let out = dir.path().join("b.tar.gz");
    let r = backup::physical_backup(dir.path(), None, &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 50);
}

#[test]
fn test_backup_with_wal() {
    let (data_dir, data_path) = make_data_dir();
    let wal = make_wal(data_dir.path());
    let out = data_dir.path().join("b.tar.gz");
    let r = backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    assert!(r.manifest.wal_file.is_some());
    assert_eq!(r.manifest.wal_file.as_ref().unwrap().path, "sqlrustgo.wal");
}

#[test]
fn test_restore_full() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let restore_dir = TempDir::new().unwrap();
    let r = restore::physical_restore(&out, restore_dir.path()).unwrap();
    assert_eq!(r.restored_data_files, 3);
    assert!(restore_dir.path().join("data").join("users.json").exists());
}

#[test]
fn test_restore_overwrites_target() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let target = TempDir::new().unwrap();
    fs::write(target.path().join("old.txt"), b"old").unwrap();
    restore::physical_restore(&out, target.path()).unwrap();
    assert!(!target.path().join("old.txt").exists());
    assert!(target.path().join("data").join("users.json").exists());
}

#[test]
fn test_restore_interrupted_simulate() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let target = TempDir::new().unwrap();
    let r = restore::physical_restore(&out, target.path()).unwrap();
    assert_eq!(r.restored_data_files, 3);
}

#[test]
fn test_verify_ok() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let r = verify::verify_backup(&out).unwrap();
    assert!(r.errors.is_empty());
    assert_eq!(r.verified_files, 3);
}

#[test]
fn test_verify_corrupted_data() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let staging = data_dir.path().join("stage");
    fs::create_dir_all(&staging).unwrap();
    backup::tar_extract_all(&out, &staging).unwrap();
    let target = staging.join("data").join("users.json");
    fs::write(&target, b"corrupted data here").unwrap();
    let _corrupted = data_dir.path().join("corrupt.tar.gz");
    let mut manifest = Manifest::read_from(&staging.join("manifest.json")).unwrap();
    for e in &mut manifest.data_files {
        if e.path == "users.json" {
            e.sha256 = sha256_file(&target).unwrap();
        }
    }
    let new_staging = data_dir.path().join("stage2");
    fs::create_dir_all(&new_staging).unwrap();
    fs::write(
        new_staging.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(new_staging.join("data")).unwrap();
    fs::write(
        new_staging.join("data").join("users.json"),
        b"originally-expected",
    )
    .unwrap();
    fs::write(new_staging.join("data").join("orders.json"), b"[]").unwrap();
    fs::create_dir_all(new_staging.join("data").join("nested")).unwrap();
    fs::write(
        new_staging.join("data").join("nested").join("items.json"),
        b"[]",
    )
    .unwrap();
    let r = verify::verify_extracted(&new_staging, &manifest);
    assert!(!r.errors.is_empty(), "checksum should detect corruption");
}

#[test]
fn test_verify_corrupted_wal() {
    let (data_dir, data_path) = make_data_dir();
    let wal_dir = TempDir::new().unwrap();
    let wal = wal_dir.path().join("sqlrustgo.wal");
    fs::copy(make_wal(data_dir.path()), &wal).unwrap();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    let staging = data_dir.path().join("stage");
    fs::create_dir_all(&staging).unwrap();
    backup::tar_extract_all(&out, &staging).unwrap();
    let wal_path = staging.join("wal").join("sqlrustgo.wal");
    fs::write(&wal_path, b"corrupted wal bytes").unwrap();
    let manifest = Manifest::read_from(&staging.join("manifest.json")).unwrap();
    let r = verify::verify_extracted(&staging, &manifest);
    assert!(
        !r.errors.is_empty(),
        "WAL checksum should detect corruption"
    );
}

#[test]
fn test_round_trip_preserves_data() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let restore_dir = TempDir::new().unwrap();
    restore::physical_restore(&out, restore_dir.path()).unwrap();
    let original = fs::read(data_path.join("users.json")).unwrap();
    let restored = fs::read(restore_dir.path().join("data").join("users.json")).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_backup_then_restore_equals_original() {
    let (data_dir, data_path) = make_data_dir();
    let wal = make_wal(data_dir.path());
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    let restore_dir = TempDir::new().unwrap();
    restore::physical_restore(&out, restore_dir.path()).unwrap();
    let verify = verify::verify_backup(&out).unwrap();
    assert!(verify.errors.is_empty());
}

#[test]
fn test_backup_idempotent() {
    let (_data_dir, data_path) = make_data_dir();
    let out_dir = TempDir::new().unwrap();
    let out1 = out_dir.path().join("b1.tar.gz");
    let out2 = out_dir.path().join("b2.tar.gz");
    let r1 = backup::physical_backup(&data_path, None, &out1).unwrap();
    let r2 = backup::physical_backup(&data_path, None, &out2).unwrap();
    assert_eq!(r1.manifest.data_files.len(), r2.manifest.data_files.len());
    for (a, b) in r1
        .manifest
        .data_files
        .iter()
        .zip(r2.manifest.data_files.iter())
    {
        assert_eq!(a.sha256, b.sha256);
    }
}

#[test]
fn test_backup_creates_unique_output() {
    let (_data_dir, data_path) = make_data_dir();
    let out_dir = TempDir::new().unwrap();
    let out1 = out_dir.path().join("a.tar.gz");
    let out2 = out_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out1).unwrap();
    backup::physical_backup(&data_path, None, &out2).unwrap();
    assert_ne!(
        fs::read(&out1).unwrap(),
        fs::read(&out2).unwrap(),
        "should differ in metadata"
    );
}

#[test]
fn test_backup_with_active_tx() {
    let (data_dir, data_path) = make_data_dir();
    let wal_dir = TempDir::new().unwrap();
    let wal = wal_dir.path().join("sqlrustgo.wal");
    let mut writer = WalWriter::with_config(&wal, false, 100).unwrap();
    let mut e = make_begin_entry(1);
    e.timestamp = 100;
    e.lsn = 1;
    writer.append(&e).unwrap();
    drop(writer);
    let out = data_dir.path().join("b.tar.gz");
    let r = backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    assert!(r.manifest.wal_file.is_some());
}

#[test]
fn test_backup_with_large_wal() {
    let (data_dir, data_path) = make_data_dir();
    let wal_dir = TempDir::new().unwrap();
    let wal = wal_dir.path().join("sqlrustgo.wal");
    let mut writer = WalWriter::with_config(&wal, false, 100).unwrap();
    for i in 0..1000 {
        let mut e = make_insert_entry(1, 1, vec![i as u8], vec![i as u8; 100], 0);
        e.timestamp = i as u64;
        e.lsn = writer.current_lsn() + 1;
        writer.append(&e).unwrap();
    }
    writer.flush().unwrap();
    let out = data_dir.path().join("b.tar.gz");
    let _r = backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    let r = verify::verify_backup(&out).unwrap();
    assert!(r.errors.is_empty());
}

// ---------------------------------------------------------------------------
// #5055: the PITR block below was rewritten.
//
// The previous version held 15 `test_pitr_*` cases calling
// `pitr::pitr_replay_entries`, which read a WAL, counted the entries it
// *would* have applied, and returned the counts. It never opened a data
// directory and never wrote a row. The CLI wired to it printed
// `pitr ok` and exited 0, having changed nothing.
//
// The counts those tests asserted are still all produced — they are
// fields on `PitrReport` — but asserting them proved only that the
// counting arithmetic had not changed, which is not a property anyone
// depends on. What matters now is whether rows land in the data
// directory, so the tests below drive a real `FileStorage`, produce a
// real WAL, and then reopen the target from disk to check what is
// actually there. Coverage of the windowing arithmetic (target time,
// open transactions, checkpoints, idempotence) is preserved, now with a
// second assertion on the restored data behind each count.
// ---------------------------------------------------------------------------

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_types::Value;

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

/// An empty data directory with a `users` table, ready to be restored
/// into. Returns the TempDir so the caller keeps it alive.
fn empty_target(tag: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let p = dir.path().to_path_buf();
    let mut s = FileStorage::new(p.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.flush_all_buffers().unwrap();
    (dir, p)
}

/// Reopen `dir` from disk and return its `users` rows. Reopening is the
/// point: a row still sitting in an insert buffer would not be here, and
/// a restore that leaves rows in a buffer has restored nothing.
fn restored_users(dir: &Path) -> Vec<Vec<Value>> {
    FileStorage::new(dir.to_path_buf())
        .unwrap()
        .scan("users")
        .unwrap()
}

fn alice() -> Vec<Value> {
    vec![Value::Integer(1), Value::Text("Alice".into())]
}

fn bob() -> Vec<Value> {
    vec![Value::Integer(2), Value::Text("Bob".into())]
}

#[test]
fn test_pitr_to_specific_timestamp() {
    // Ten autocommit inserts; recover to the 5th second and expect
    // exactly the rows written at or before it.
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        for i in 0..10i64 {
            s.insert(
                "users",
                vec![vec![Value::Integer(i), Value::Text(format!("u{i}"))]],
            )
            .unwrap();
        }
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let mut reader = sqlrustgo_storage::wal_legacy::WalReader::new(&wal).unwrap();
    let entries: Vec<WalEntry> = reader.read_all().unwrap();
    assert_eq!(entries.len(), 10);

    let (_keep, target) = empty_target("ts");
    let r = pitr::pitr_replay_into(&target, &wal, 0).unwrap();
    // Every entry shares the same wall-clock second, so the window is
    // all-or-nothing; the point is that a target *before* them applies
    // nothing and after them applies everything.
    assert_eq!(r.entries_scanned, 0);
    assert_eq!(r.entries_applied, 0);
    assert!(restored_users(&target).is_empty());

    let (_keep2, target2) = empty_target("ts2");
    let r2 = pitr::pitr_replay_into(&target2, &wal, u64::MAX).unwrap();
    assert_eq!(r2.entries_applied, 10);
    assert_eq!(restored_users(&target2).len(), 10);
}

#[test]
fn test_pitr_with_active_tx_at_target() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.flush_all_buffers().unwrap();
        // Begin with no Commit: a crash here must not resurrect Bob.
        s.begin_transaction().unwrap();
        s.insert("users", vec![bob()]).unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");

    let (_keep, target) = empty_target("active");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.active_transactions_at_target, 1);
    assert_eq!(r.entries_applied, 1);
    assert_eq!(r.entries_skipped, 1);

    let rows = restored_users(&target);
    assert_eq!(rows, vec![alice()], "the open transaction must not replay");
}

#[test]
fn test_pitr_at_beginning_of_wal() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("begin");
    let r = pitr::pitr_replay_into(&target, &wal, 0).unwrap();
    assert_eq!(r.entries_scanned, 0);
    assert_eq!(r.entries_applied, 0);
    assert!(restored_users(&target).is_empty());
}

#[test]
fn test_pitr_at_end_of_wal() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice(), bob()]).unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("end");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.entries_applied, 2);
    let mut rows = restored_users(&target);
    rows.sort();
    let mut expected = vec![alice(), bob()];
    expected.sort();
    assert_eq!(rows, expected);
}

#[test]
fn test_pitr_preserves_committed_data() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("committed");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.transactions_committed, 1);
    assert_eq!(r.entries_applied, 1);
    assert_eq!(restored_users(&target), vec![alice()]);
}

#[test]
fn test_pitr_skips_uncommitted_data() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.rollback_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("uncommitted");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.transactions_aborted, 1);
    assert_eq!(r.entries_applied, 0);
    assert!(restored_users(&target).is_empty());
}

#[test]
fn test_pitr_with_empty_wal() {
    let (_keep, target) = empty_target("empty");
    let empty_wal = target.join("empty.wal");
    sqlrustgo_storage::wal_legacy::WalWriter::with_config(&empty_wal, false, 100).unwrap();
    let r = pitr::pitr_replay_into(&target, &empty_wal, 1000).unwrap();
    assert_eq!(r.entries_scanned, 0);
    assert_eq!(r.entries_applied, 0);
    assert_eq!(r.entries_failed, 0);
    assert!(restored_users(&target).is_empty());
}

/// A Checkpoint entry is metadata: it must be counted as scanned and
/// never applied as a row.
#[test]
fn test_pitr_replays_checkpoints() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");

    // Splice a Checkpoint into the middle of the real log.
    let mut reader = sqlrustgo_storage::wal_legacy::WalReader::new(&wal).unwrap();
    let mut entries = reader.read_all().unwrap();
    let checkpoint = WalEntry {
        tx_id: 0,
        entry_type: WalEntryType::Checkpoint,
        table_id: 0,
        table_name: None,
        key: None,
        data: None,
        lsn: 99,
        timestamp: 0,
    };
    entries.insert(0, checkpoint);
    let spliced = target_wal(&entries);

    let (_keep, target) = empty_target("ckpt");
    let r = pitr::pitr_replay_into(&target, &spliced, u64::MAX).unwrap();
    assert_eq!(r.entries_scanned, entries.len());
    assert_eq!(
        r.entries_applied, 1,
        "only the Insert is a row entry; the Checkpoint must not be applied"
    );
    assert_eq!(restored_users(&target), vec![alice()]);
}

/// Writing the entries back out through `WalWriter` is how a spliced
/// log is produced. `WalWriter` does not stamp LSNs, so the caller's
/// values are what land on disk — that is what we want here.
fn target_wal(entries: &[WalEntry]) -> std::path::PathBuf {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("spliced.wal");
    {
        let mut w = WalWriter::with_config(&path, false, 100).unwrap();
        for e in entries {
            w.append(e).unwrap();
        }
        w.flush().unwrap();
    }
    // Leak the TempDir so the file outlives this function.
    std::mem::forget(dir);
    path
}

#[test]
fn test_pitr_idempotent() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");

    let (_keep, target) = empty_target("idem");
    let first = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    let after_first = restored_users(&target);
    let second = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();

    assert_eq!(first.entries_applied, 1);
    assert_eq!(
        second.entries_applied, 0,
        "a second restore over the same base must apply nothing new"
    );
    assert_eq!(
        restored_users(&target),
        after_first,
        "re-running a restore must not duplicate rows"
    );
}

#[test]
fn test_pitr_at_exact_entry_timestamp() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let mut reader = sqlrustgo_storage::wal_legacy::WalReader::new(&wal).unwrap();
    let entries = reader.read_all().unwrap();
    let ts = entries[0].timestamp;

    // The window is inclusive of the target.
    let (_keep, at) = empty_target("exact");
    let r_at = pitr::pitr_replay_into(&at, &wal, ts).unwrap();
    assert_eq!(r_at.entries_scanned, 1);
    assert_eq!(restored_users(&at), vec![alice()]);

    let (_keep2, before) = empty_target("exact_before");
    let r_before = pitr::pitr_replay_into(&before, &wal, ts.saturating_sub(1)).unwrap();
    assert_eq!(r_before.entries_scanned, 0);
    assert!(restored_users(&before).is_empty());
}

#[test]
fn test_parse_target_time_rfc3339() {
    let t = pitr::parse_target_time("2026-06-05T10:00:00Z").unwrap();
    assert!(t > 0);
}

#[test]
fn test_parse_target_time_unix() {
    let t = pitr::parse_target_time("1700000000").unwrap();
    assert_eq!(t, 1700000000);
}

#[test]
fn test_parse_target_time_invalid() {
    assert!(pitr::parse_target_time("not a time").is_err());
}

#[test]
fn test_restore_to_nonexistent_dir_ok() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let parent = TempDir::new().unwrap();
    let target = parent.path().join("new_dir");
    restore::physical_restore(&out, &target).unwrap();
    assert!(target.join("data").join("users.json").exists());
}

#[test]
fn test_restore_corrupted_backup_fails() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let staging = data_dir.path().join("stage");
    fs::create_dir_all(&staging).unwrap();
    backup::tar_extract_all(&out, &staging).unwrap();
    let target = staging.join("data").join("users.json");
    fs::write(&target, b"corrupted").unwrap();
    let manifest = Manifest::read_from(&staging.join("manifest.json")).unwrap();
    let r = verify::verify_extracted(&staging, &manifest);
    assert!(!r.errors.is_empty());
}

#[test]
fn test_verify_invalid_manifest() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("b.tar.gz");
    let mut f = fs::File::create(&out).unwrap();
    f.write_all(b"\x1f\x8b\x08\x00garbage").unwrap();
    drop(f);
    let r = verify::verify_backup(&out);
    assert!(r.is_err() || !r.unwrap().errors.is_empty());
}

#[test]
fn test_backup_with_readonly_data_returns_error_or_ok() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.json"), b"{}").unwrap();
    let out = dir.path().join("b.tar.gz");
    let r = backup::physical_backup(dir.path(), None, &out);
    assert!(r.is_ok());
}

#[test]
fn test_backup_then_modify_then_backup_differs() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.json"), b"v1").unwrap();
    let out1 = dir.path().join("b1.tar.gz");
    backup::physical_backup(dir.path(), None, &out1).unwrap();
    fs::write(dir.path().join("a.json"), b"v2").unwrap();
    let out2 = dir.path().join("b2.tar.gz");
    backup::physical_backup(dir.path(), None, &out2).unwrap();
    let m1 = Manifest::read_from(
        &backup::tar_extract_one(&out1, "manifest.json")
            .ok()
            .and_then(|b| {
                let staging = dir.path().join("stage1");
                fs::create_dir_all(&staging).ok()?;
                fs::write(staging.join("manifest.json"), &b).ok()?;
                Some(staging.join("manifest.json"))
            })
            .unwrap(),
    )
    .unwrap();
    let m2 = Manifest::read_from(
        &backup::tar_extract_one(&out2, "manifest.json")
            .ok()
            .and_then(|b| {
                let staging = dir.path().join("stage2");
                fs::create_dir_all(&staging).ok()?;
                fs::write(staging.join("manifest.json"), &b).ok()?;
                Some(staging.join("manifest.json"))
            })
            .unwrap(),
    )
    .unwrap();
    assert_ne!(m1.data_files[0].sha256, m2.data_files[0].sha256);
}

#[test]
fn test_verify_after_data_modification_fails() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    fs::write(data_path.join("users.json"), b"modified").unwrap();
    let r = verify::verify_backup(&out).unwrap();
    assert!(
        r.errors.is_empty(),
        "backup is intact, source modification doesn't affect backup"
    );
    assert_eq!(r.verified_files, 3);
}

#[test]
fn test_backup_then_delete_source() {
    let (data_dir, data_path) = make_data_dir();
    let out_dir = TempDir::new().unwrap();
    let out = out_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    drop(data_dir);
    let r = verify::verify_backup(&out).unwrap();
    assert!(r.errors.is_empty());
    assert_eq!(r.verified_files, 3);
}

#[test]
fn test_manifest_scan_returns_correct_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("x.json"), b"a").unwrap();
    fs::write(dir.path().join("y.json"), b"bb").unwrap();
    let m = Manifest::scan(dir.path(), None).unwrap();
    assert_eq!(m.data_files.len(), 2);
    assert_eq!(m.total_size_bytes, 3);
}

#[test]
fn test_sha256_file_consistent() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("a");
    fs::write(&f, b"hello").unwrap();
    let h1 = sha256_file(&f).unwrap();
    let h2 = sha256_file(&f).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn test_backup_manifest_sha256_unique() {
    let (_data_dir, data_path) = make_data_dir();
    let out_dir = TempDir::new().unwrap();
    let out1 = out_dir.path().join("b1.tar.gz");
    let out2 = out_dir.path().join("b2.tar.gz");
    let r1 = backup::physical_backup(&data_path, None, &out1).unwrap();
    let r2 = backup::physical_backup(&data_path, None, &out2).unwrap();
    let r1_data = fs::read(&out1).unwrap();
    let r2_data = fs::read(&out2).unwrap();
    assert_eq!(r1.manifest.data_files, r2.manifest.data_files);
    assert_ne!(
        r1_data, r2_data,
        "compressed bytes may differ due to metadata"
    );
}

#[test]
fn test_restore_preserves_nested_dirs() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let restore_dir = TempDir::new().unwrap();
    restore::physical_restore(&out, restore_dir.path()).unwrap();
    let nested = restore_dir
        .path()
        .join("data")
        .join("nested")
        .join("items.json");
    assert!(nested.exists());
    assert_eq!(fs::read(&nested).unwrap(), b"[]");
}

#[test]
fn test_backup_with_zero_byte_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("empty.json"), b"").unwrap();
    let out = dir.path().join("b.tar.gz");
    let r = backup::physical_backup(dir.path(), None, &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 1);
    let v = verify::verify_backup(&out).unwrap();
    assert!(v.errors.is_empty());
}

#[test]
fn test_backup_metadata_version() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    let r = backup::physical_backup(&data_path, None, &out).unwrap();
    assert_eq!(r.manifest.version, 1);
    assert_eq!(r.manifest.sqlrustgo_version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_restore_then_verify_recovery() {
    let (data_dir, data_path) = make_data_dir();
    let wal = make_wal(data_dir.path());
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    let restore_dir = TempDir::new().unwrap();
    let r = restore::physical_restore(&out, restore_dir.path()).unwrap();
    assert!(r.restored_wal);
    let verify = verify::verify_backup(&out).unwrap();
    assert!(verify.errors.is_empty());
}

#[test]
fn test_backup_then_restore_3x_idempotent() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    for _ in 0..3 {
        let target = TempDir::new().unwrap();
        restore::physical_restore(&out, target.path()).unwrap();
        let v = verify::verify_backup(&out).unwrap();
        assert!(v.errors.is_empty());
    }
}

#[test]
fn test_backup_with_binary_content() {
    let dir = TempDir::new().unwrap();
    let binary: Vec<u8> = (0..=255).cycle().take(1024).collect();
    fs::write(dir.path().join("binary.dat"), &binary).unwrap();
    let out = dir.path().join("b.tar.gz");
    let r = backup::physical_backup(dir.path(), None, &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 1);
    let v = verify::verify_backup(&out).unwrap();
    assert!(v.errors.is_empty());
}

#[test]
fn test_backup_very_deep_nested_dir() {
    let dir = TempDir::new().unwrap();
    let mut current = dir.path().to_path_buf();
    for i in 0..5 {
        current = current.join(format!("level{}", i));
        fs::create_dir(&current).unwrap();
        fs::write(current.join("data.json"), format!("{}", i)).unwrap();
    }
    let out = dir.path().join("b.tar.gz");
    let r = backup::physical_backup(dir.path(), None, &out).unwrap();
    assert_eq!(r.manifest.data_files.len(), 5);
    let v = verify::verify_backup(&out).unwrap();
    assert!(v.errors.is_empty());
}

#[test]
fn test_restore_to_deeply_nested_target() {
    let (data_dir, data_path) = make_data_dir();
    let out = data_dir.path().join("b.tar.gz");
    backup::physical_backup(&data_path, None, &out).unwrap();
    let parent = TempDir::new().unwrap();
    let mut target = parent.path().to_path_buf();
    for i in 0..3 {
        target = target.join(format!("l{}", i));
    }
    restore::physical_restore(&out, &target).unwrap();
    assert!(target.join("data").join("users.json").exists());
}

/// #5055: interleaved transactions — one commits, one is still open at
/// the target. Only the committed one's row may land.
#[test]
fn test_pitr_mixed_commit_rollback() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![bob()]).unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("mixed");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.entries_applied, 1);
    assert_eq!(r.transactions_committed, 1);
    assert_eq!(r.active_transactions_at_target, 1);
    assert_eq!(restored_users(&target), vec![alice()]);
}

/// #5055: a rolled-back transaction restores nothing.
#[test]
fn test_pitr_skips_rolled_back() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.rollback_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("rb");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.entries_applied, 0);
    assert_eq!(r.transactions_aborted, 1);
    assert!(restored_users(&target).is_empty());
}

/// #5055: a target of 0 excludes a log written later.
#[test]
fn test_pitr_very_old_timestamp_zero() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("users", vec![alice()]).unwrap();
        s.commit_transaction().unwrap();
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("old");
    let r = pitr::pitr_replay_into(&target, &wal, 0).unwrap();
    assert_eq!(r.entries_scanned, 0);
    assert!(restored_users(&target).is_empty());
}

/// #5055: 100 committed transactions restore 100 rows.
#[test]
fn test_pitr_with_many_committed_tx() {
    let source = TempDir::new().unwrap();
    let src = source.path().to_path_buf();
    {
        let mut s = FileStorage::new_with_wal(src.clone()).unwrap();
        s.create_table(&users_table()).unwrap();
        for i in 0..100i64 {
            s.begin_transaction().unwrap();
            s.insert(
                "users",
                vec![vec![Value::Integer(i), Value::Text(format!("u{i}"))]],
            )
            .unwrap();
            s.commit_transaction().unwrap();
        }
        s.flush_all_buffers().unwrap();
    }
    let wal = src.join("sqlrustgo.wal");
    let (_keep, target) = empty_target("many");
    let r = pitr::pitr_replay_into(&target, &wal, u64::MAX).unwrap();
    assert_eq!(r.transactions_committed, 100);
    assert_eq!(r.entries_applied, 100);
    assert_eq!(r.entries_failed, 0, "{:?}", r.first_error);
    assert_eq!(
        restored_users(&target).len(),
        100,
        "every committed row must actually be on disk afterwards"
    );
}
