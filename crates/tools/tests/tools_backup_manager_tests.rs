// Tools crate BackupManager coverage tests

use sqlrustgo_tools::backup_restore::{BackupColumn, BackupManager, BackupTable};
use tempfile::TempDir;

// ============ BackupManager tests ============

/// A one-column table with one row.
fn table_with_one_row(name: &str) -> BackupTable {
    let mut table = BackupTable::new(name, vec![BackupColumn::new("id", "INTEGER")]);
    table.rows = vec![vec![sqlrustgo_storage::Value::Integer(1)]];
    table
}

#[test]
fn test_backup_manager_new_creates_dir() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("backups");
    let mgr = BackupManager::new(dir.clone());
    assert!(dir.exists(), "backup dir should be created");
    assert!(mgr.backup_dir().exists());
}

#[test]
fn test_backup_manager_new_existing_dir() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("existing");
    std::fs::create_dir_all(&dir).unwrap();
    let mgr = BackupManager::new(dir.clone());
    assert!(dir.exists());
}

#[test]
fn test_backup_manager_list_backups_empty() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));
    let backups = mgr.list_backups();
    assert!(backups.is_empty());
}

#[test]
fn test_backup_manager_get_backup_none() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));
    let result = mgr.get_backup("nonexistent");
    assert!(result.is_none());
}

#[test]
fn test_backup_manager_backup_dir_path() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("my_backups");
    let mgr = BackupManager::new(dir.clone());
    assert_eq!(mgr.backup_dir(), dir.as_path());
}

#[test]
fn test_backup_manager_create_and_get_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let mut table = BackupTable::new(
        "users",
        vec![
            BackupColumn::new("id", "INTEGER").primary_key(),
            BackupColumn::new("name", "TEXT"),
        ],
    );
    table.rows = vec![vec![
        sqlrustgo_storage::Value::Integer(1),
        sqlrustgo_storage::Value::Text("alice".to_string()),
    ]];

    let meta = mgr.create_backup("testdb", &[table]).unwrap();
    assert_eq!(meta.database, "testdb");

    let retrieved = mgr.get_backup(&meta.id);
    assert!(retrieved.is_some());
    let r = retrieved.unwrap();
    assert_eq!(r.database, "testdb");
    assert!(r.tables.contains(&"users".to_string()));
}

#[test]
fn test_backup_manager_create_backup_checksum() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let mut table = BackupTable::new("t1", vec![BackupColumn::new("col1", "TEXT")]);
    table.rows = vec![vec![sqlrustgo_storage::Value::Text("val1".to_string())]];

    let meta = mgr.create_backup("testdb", &[table]).unwrap();
    assert!(meta.checksum.is_some());
    assert!(!meta.checksum.as_ref().unwrap().is_empty());
    // #5525: the checksum must be a real hash, not the old 32-bit helper.
    assert!(meta.payload_checksum.is_some());
}

#[test]
fn test_backup_manager_restore_nonexistent() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let result = mgr.restore("nonexistent_backup");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not found"));
}

#[test]
fn test_backup_manager_restore_existing() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let meta = mgr
        .create_backup("testdb", &[table_with_one_row("t1")])
        .unwrap();
    let data = mgr.restore(&meta.id);
    assert!(data.is_ok());
    // #5525: the payload must carry the rows, not an empty list.
    assert_eq!(data.unwrap().tables[0].rows.len(), 1);
}

#[test]
fn test_backup_manager_delete_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER")]);
    let meta = mgr.create_backup("db", &[table]).unwrap();
    let id = meta.id.clone();

    assert!(mgr.get_backup(&id).is_some());
    mgr.delete_backup(&id).unwrap();
    assert!(mgr.get_backup(&id).is_none());
}

#[test]
fn test_backup_manager_delete_nonexistent() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let result = mgr.delete_backup("nonexistent");
    assert!(result.is_err());
}

// Use sleep to ensure seconds-precision timestamp uniqueness
#[test]
fn test_backup_manager_list_backups_after_create() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER")]);
    // `std::slice::from_ref` instead of `&[table.clone()]`: the same table is
    // backed up twice, so it has to stay owned and reusable -- cloning it into
    // an array literal was both unnecessary and a clippy violation.
    mgr.create_backup("db1", std::slice::from_ref(&table))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    mgr.create_backup("db2", std::slice::from_ref(&table))
        .unwrap();

    let backups = mgr.list_backups();
    assert_eq!(backups.len(), 2);
}

#[test]
fn test_backup_manager_multiple_backups_with_delay() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER")]);

    let m1 = mgr
        .create_backup("mydb", std::slice::from_ref(&table))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let m2 = mgr
        .create_backup("mydb", std::slice::from_ref(&table))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let m3 = mgr
        .create_backup("mydb", std::slice::from_ref(&table))
        .unwrap();

    assert_ne!(m1.id, m2.id);
    assert_ne!(m2.id, m3.id);

    let backups = mgr.list_backups();
    assert_eq!(backups.len(), 3);
}
// ============ BackupManager content verification (temp dir) ============

/// Verify that restore() returns the tables of the backup, with their rows.
#[test]
fn test_backup_manager_restore_parses_table_names() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let mut table = BackupTable::new(
        "users",
        vec![
            BackupColumn::new("id", "INTEGER").primary_key(),
            BackupColumn::new("name", "TEXT"),
        ],
    );
    table.rows = vec![vec![
        sqlrustgo_storage::Value::Integer(1),
        sqlrustgo_storage::Value::Text("alice".to_string()),
    ]];

    let meta = mgr.create_backup("testdb", &[table]).unwrap();
    let restored = mgr.restore(&meta.id).unwrap();

    assert!(
        restored.tables.iter().any(|t| t.name == "users"),
        "restored payload should contain the 'users' table"
    );
}

/// BackupManager::delete removes the backup and subsequent restore fails.
#[test]
fn test_backup_manager_delete_then_restore_fails() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER")]);

    let meta = mgr.create_backup("testdb", &[table]).unwrap();
    let backup_id = meta.id.clone();

    mgr.delete_backup(&backup_id).unwrap();

    // Restore should now fail
    let result = mgr.restore(&backup_id);
    assert!(result.is_err(), "restore after delete should fail");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("not found"),
        "error should mention not found: {}",
        err_msg
    );
}

/// BackupManager checksum is non-empty after create_backup.
#[test]
fn test_backup_manager_checksum_non_empty() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER")]);

    let meta = mgr.create_backup("testdb", &[table]).unwrap();
    assert!(meta.checksum.is_some(), "checksum should be set");
    let checksum = meta.checksum.unwrap();
    assert!(!checksum.is_empty(), "checksum should be non-empty");
    assert!(
        checksum.chars().all(|c| c.is_ascii_hexdigit()),
        "checksum should be hex"
    );
}
