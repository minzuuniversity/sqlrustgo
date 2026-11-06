//! Tests for [`BackupManager`](sqlrustgo_tools::backup_restore::BackupManager).

use sqlrustgo_storage::Value;
use sqlrustgo_tools::backup_restore::{BackupColumn, BackupManager, BackupTable};
use tempfile::TempDir;

/// A one-column table with one row — the smallest typed backup.
fn users_table() -> BackupTable {
    let mut table = BackupTable::new(
        "users",
        vec![
            BackupColumn::new("id", "INTEGER").primary_key(),
            BackupColumn::new("name", "TEXT"),
        ],
    );
    table.rows = vec![
        vec![Value::Integer(1), Value::Text("alice".to_string())],
        vec![Value::Integer(2), Value::Text("bob".to_string())],
    ];
    table
}

/// Verify [`BackupManager::new`] creates the backup directory.
#[test]
fn test_backup_manager_new_creates_dir() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("backup_root");
    let _mgr = BackupManager::new(dir.clone());
    assert!(dir.is_dir(), "backup directory should be created");
}

/// Verify [`BackupManager::create_backup`] produces a .sql file and metadata.
#[test]
fn test_create_backup_produces_files() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let result = mgr.create_backup("testdb", &[users_table()]);
    assert!(result.is_ok(), "create_backup failed: {:?}", result);

    let metadata = result.unwrap();
    assert!(!metadata.id.is_empty());
    let type_str = format!("{:?}", metadata.backup_type);
    assert!(type_str.contains("Full"), "expected Full, got {}", type_str);

    // .sql file should exist
    let sql_file = tmp.path().join(format!("{}.sql", metadata.id));
    assert!(sql_file.exists(), "backup .sql file should exist");

    // .meta.json file should exist
    let meta_file = tmp.path().join(format!("{}.meta.json", metadata.id));
    assert!(meta_file.exists(), "metadata .json file should exist");

    // #5525: the machine-readable payload must exist too — it is what
    // `restore` consumes.
    let payload_file = tmp.path().join(format!("{}.data.json", metadata.id));
    assert!(
        payload_file.exists(),
        "backup payload .json file should exist"
    );
}

/// Verify [`BackupManager::list_backups`] returns an empty vec initially.
#[test]
fn test_list_backups_empty() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());
    assert!(mgr.list_backups().is_empty());
}

/// Verify [`BackupManager::list_backups`] after creating a backup.
#[test]
fn test_list_backups_after_create() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let table = BackupTable::new("t", vec![BackupColumn::new("id", "INTEGER")]);
    mgr.create_backup("mydb", &[table]).ok();

    let backups = mgr.list_backups();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].database, "mydb");
}

/// Verify [`BackupManager::get_backup`] returns correct metadata.
#[test]
fn test_get_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let table = BackupTable::new("t", vec![BackupColumn::new("id", "INTEGER")]);
    let created = mgr.create_backup("mydb", &[table]).unwrap();

    let retrieved = mgr.get_backup(&created.id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().database, "mydb");
}

/// Verify [`BackupManager::delete_backup`] removes files.
#[test]
fn test_delete_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let table = BackupTable::new("t", vec![BackupColumn::new("id", "INTEGER")]);
    let created = mgr.create_backup("mydb", &[table]).unwrap();

    let sql_file = tmp.path().join(format!("{}.sql", created.id));
    let meta_file = tmp.path().join(format!("{}.meta.json", created.id));
    let payload_file = tmp.path().join(format!("{}.data.json", created.id));
    assert!(sql_file.exists());
    assert!(meta_file.exists());
    assert!(payload_file.exists());

    let result = mgr.delete_backup(&created.id);
    assert!(result.is_ok());

    assert!(!sql_file.exists(), ".sql file should be deleted");
    assert!(!payload_file.exists(), "payload file should be deleted");
}

/// Verify [`BackupManager::restore`] returns error for non-existent backup.
#[test]
fn test_restore_nonexistent() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let result = mgr.restore("nonexistent_backup_id");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("not found")
            || err.contains("NotFound")
            || err.contains("not exist")
            || err.contains("does not exist"),
        "error should mention not found: {}",
        err
    );
}
/// Verify [`BackupManager::restore`] reads back the payload of a backup.
///
/// #5525: the old `restore` counted `INSERT INTO` lines and returned an
/// empty row list per table. It now returns the payload, with the rows.
#[test]
fn test_restore_parses_insert() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let meta = mgr.create_backup("testdb", &[users_table()]).unwrap();

    let content = mgr.restore(&meta.id).expect("restore failed");
    assert_eq!(content.tables.len(), 1);
    assert_eq!(content.tables[0].name, "users");
    assert_eq!(content.tables[0].rows.len(), 2);
    assert_eq!(
        content.tables[0].rows[0][1],
        Value::Text("alice".to_string())
    );
}

/// Verify [`BackupManager::delete_backup`] returns error for non-existent backup.
#[test]
fn test_delete_backup_nonexistent() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let result = mgr.delete_backup("nonexistent_backup");
    assert!(result.is_err(), "delete of non-existent should error");
    let err = result.unwrap_err();
    assert!(
        err.contains("not found") || err.contains("remove") || err.contains("No such file"),
        "unexpected error: {}",
        err
    );
}

/// Verify [`BackupManager::restore`] returns error if the payload is missing.
#[test]
fn test_restore_missing_sql_file() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    // Create only the .meta.json without the payload file
    let backup_id = "backup_missing_sql";
    let meta_content = r#"{"id":"backup_missing_sql","type":"Full","started":"1234567890","completed":"1234567891","size":100,"database":"testdb","tables":[],"status":"completed","checksum":"abc"}"#;
    let meta_file = tmp.path().join(format!("{}.meta.json", backup_id));
    std::fs::write(&meta_file, meta_content).ok();

    let result = mgr.restore(backup_id);
    assert!(
        result.is_err(),
        "restore should fail without the payload file"
    );
}

/// Verify [`BackupManager::create_backup`] with no tables at all.
///
/// #5525: this used to succeed and write a backup with zero tables. An
/// empty backup is now an error — the caller has to ask for a backup of
/// something.
#[test]
fn test_create_backup_empty_tables() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let result = mgr.create_backup("emptydb", &[]);
    assert!(
        result.is_err(),
        "create_backup with no tables must fail: {:?}",
        result
    );
    let err = result.unwrap_err();
    assert!(err.contains("no tables"), "unexpected error: {}", err);
}

/// Verify [`BackupManager::backup_dir`] returns the configured directory.
#[test]
fn test_backup_dir_getter() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    assert_eq!(mgr.backup_dir(), tmp.path());
}

/// Verify md5_simple produces consistent hashes.
#[test]
fn test_md5_simple() {
    use sqlrustgo_tools::backup_restore::md5_simple;

    let h1 = md5_simple("hello");
    let h2 = md5_simple("hello");
    let h3 = md5_simple("world");
    assert_eq!(h1, h2, "same input should produce same hash");
    assert_ne!(h1, h3, "different input should produce different hash");
    assert_ne!(h3, 0, "hash should not be zero");
}

/// Verify serde_json_simple produces valid JSON for all status variants.
#[test]
fn test_serde_json_simple_all_statuses() {
    use sqlrustgo_tools::backup_restore::{BackupMetadata, BackupType};

    // InProgress
    let m1 = BackupMetadata::new("b1".into(), BackupType::Full, "db".into());
    let j1 = sqlrustgo_tools::backup_restore::serde_json_simple(&m1);
    assert!(j1.contains("in_progress"), "InProgress JSON: {}", j1);

    // Completed
    let mut m2 = BackupMetadata::new("b2".into(), BackupType::Full, "db".into());
    m2.complete(100, "abc123".into());
    let j2 = sqlrustgo_tools::backup_restore::serde_json_simple(&m2);
    assert!(j2.contains("completed"), "Completed JSON: {}", j2);

    // Failed
    let mut m3 = BackupMetadata::new("b3".into(), BackupType::Full, "db".into());
    m3.fail("disk full".into());
    let j3 = sqlrustgo_tools::backup_restore::serde_json_simple(&m3);
    assert!(j3.contains("failed"), "Failed JSON: {}", j3);
    assert!(
        j3.contains("disk full"),
        "Failed JSON should contain error: {}",
        j3
    );
}
