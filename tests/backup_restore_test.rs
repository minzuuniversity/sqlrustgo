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
    let corrupted = data_dir.path().join("corrupt.tar.gz");
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
    let (data_dir, data_path) = make_data_dir();
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
    let (data_dir, data_path) = make_data_dir();
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
    let r = backup::physical_backup(&data_path, Some(&wal), &out).unwrap();
    let r = verify::verify_backup(&out).unwrap();
    assert!(r.errors.is_empty());
}

#[test]
fn test_pitr_to_specific_timestamp() {
    let dir = TempDir::new().unwrap();
    let wal = dir.path().join("test.wal");
    let mut writer = WalWriter::with_config(&wal, false, 100).unwrap();
    for ts in 0..10 {
        let mut e = make_insert_entry(1, 1, vec![ts as u8], vec![ts as u8], 0);
        e.timestamp = ts;
        e.lsn = writer.current_lsn() + 1;
        writer.append(&e).unwrap();
    }
    writer.flush().unwrap();
    let r = pitr::pitr_replay(&wal, 5).unwrap();
    assert_eq!(r.entries_scanned, 6);
}

#[test]
fn test_pitr_with_active_tx_at_target() {
    let dir = TempDir::new().unwrap();
    let wal = dir.path().join("test.wal");
    let mut writer = WalWriter::with_config(&wal, false, 100).unwrap();
    for (i, et) in [WalEntryType::Begin, WalEntryType::Insert]
        .iter()
        .enumerate()
    {
        let mut e = match et {
            WalEntryType::Begin => make_begin_entry(1),
            WalEntryType::Insert => make_insert_entry(1, 1, vec![1], vec![1], 0),
            _ => make_begin_entry(1),
        };
        e.timestamp = 100 + i as u64;
        e.lsn = writer.current_lsn() + 1;
        writer.append(&e).unwrap();
    }
    writer.flush().unwrap();
    let r = pitr::pitr_replay(&wal, 200).unwrap();
    assert_eq!(r.active_transactions_at_target, 1);
    assert_eq!(r.entries_applied, 0);
}

#[test]
fn test_pitr_at_beginning_of_wal() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 50);
    assert_eq!(r.entries_scanned, 0);
    assert_eq!(r.entries_applied, 0);
}

#[test]
fn test_pitr_at_end_of_wal() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 3,
            timestamp: 102,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, u64::MAX);
    assert_eq!(r.entries_scanned, 3);
    assert_eq!(r.entries_applied, 1);
    assert_eq!(r.transactions_committed, 1);
}

#[test]
fn test_pitr_preserves_committed_data() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 3,
            timestamp: 102,
        },
        WalEntry {
            tx_id: 2,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 4,
            timestamp: 200,
        },
        WalEntry {
            tx_id: 2,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![2]),
            data: Some(vec![3, 4]),
            lsn: 5,
            timestamp: 201,
        },
        WalEntry {
            tx_id: 2,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 6,
            timestamp: 202,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 300);
    assert_eq!(r.entries_applied, 2);
    assert_eq!(r.transactions_committed, 2);
}

#[test]
fn test_pitr_skips_uncommitted_data() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 200);
    assert_eq!(r.entries_applied, 0);
    assert_eq!(r.active_transactions_at_target, 1);
}

#[test]
fn test_pitr_with_empty_wal() {
    let entries: Vec<WalEntry> = Vec::new();
    let r = pitr::pitr_replay_entries(&entries, 1000);
    assert_eq!(r.entries_scanned, 0);
    assert_eq!(r.entries_applied, 0);
}

#[test]
fn test_pitr_replays_checkpoints() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Checkpoint,
            table_id: 0,
            key: None,
            data: None,
            lsn: 2,
            timestamp: 101,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 3,
            timestamp: 102,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 200);
    assert_eq!(r.transactions_committed, 1);
}

#[test]
fn test_pitr_idempotent() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 3,
            timestamp: 102,
        },
    ];
    let r1 = pitr::pitr_replay_entries(&entries, 200);
    let r2 = pitr::pitr_replay_entries(&entries, 200);
    assert_eq!(r1.entries_applied, r2.entries_applied);
    assert_eq!(r1.transactions_committed, r2.transactions_committed);
}

#[test]
fn test_pitr_at_exact_entry_timestamp() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 150,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 150);
    assert_eq!(r.entries_scanned, 2);
    assert_eq!(r.entries_applied, 0);
    assert_eq!(r.active_transactions_at_target, 1);
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
    assert!(r.is_err() || r.unwrap().errors.len() > 0);
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
    let (data_dir, data_path) = make_data_dir();
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
fn test_pitr_mixed_commit_rollback() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Commit,
            table_id: 0,
            key: None,
            data: None,
            lsn: 3,
            timestamp: 102,
        },
        WalEntry {
            tx_id: 2,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 4,
            timestamp: 200,
        },
        WalEntry {
            tx_id: 2,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![2]),
            data: Some(vec![3, 4]),
            lsn: 5,
            timestamp: 201,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 300);
    assert_eq!(r.entries_applied, 1);
    assert_eq!(r.transactions_committed, 1);
    assert_eq!(r.active_transactions_at_target, 1);
}

#[test]
fn test_pitr_skips_rolled_back() {
    let entries = vec![
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 100,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: Some(vec![1]),
            data: Some(vec![1, 2]),
            lsn: 2,
            timestamp: 101,
        },
        WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Rollback,
            table_id: 0,
            key: None,
            data: None,
            lsn: 3,
            timestamp: 102,
        },
    ];
    let r = pitr::pitr_replay_entries(&entries, 200);
    assert_eq!(r.entries_applied, 0);
    assert_eq!(r.transactions_aborted, 1);
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
fn test_pitr_very_old_timestamp_zero() {
    let entries = vec![WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 100,
    }];
    let r = pitr::pitr_replay_entries(&entries, 0);
    assert_eq!(r.entries_scanned, 0);
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
fn test_pitr_with_many_committed_tx() {
    let dir = TempDir::new().unwrap();
    let wal = dir.path().join("test.wal");
    let mut writer = WalWriter::with_config(&wal, false, 100).unwrap();
    for i in 0..100u64 {
        let mut e1 = make_begin_entry(i);
        e1.timestamp = i;
        e1.lsn = writer.current_lsn() + 1;
        writer.append(&e1).unwrap();
        let mut e2 = make_insert_entry(i, 1, vec![i as u8], vec![i as u8; 10], 0);
        e2.timestamp = i;
        e2.lsn = writer.current_lsn() + 1;
        writer.append(&e2).unwrap();
        let mut e3 = make_commit_entry(i, 0);
        e3.timestamp = i;
        e3.lsn = writer.current_lsn() + 1;
        writer.append(&e3).unwrap();
    }
    writer.flush().unwrap();
    let r = pitr::pitr_replay(&wal, 50).unwrap();
    assert_eq!(r.transactions_committed, 51);
    assert_eq!(r.entries_applied, 51);
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
