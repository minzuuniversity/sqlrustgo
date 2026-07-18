// Tools crate BackupManager coverage tests

use sqlrustgo_tools::backup_restore::BackupManager;
use std::collections::HashMap;
use tempfile::TempDir;

// ============ BackupManager tests ============

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

    let mut tables = HashMap::new();
    let mut users = Vec::new();
    let mut user_row = HashMap::new();
    user_row.insert("id".to_string(), "1".to_string());
    user_row.insert("name".to_string(), "alice".to_string());
    users.push(user_row);
    tables.insert("users".to_string(), users);

    let meta = mgr.create_backup("testdb", tables).unwrap();
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

    let mut tables = HashMap::new();
    let mut rows = Vec::new();
    let mut row = HashMap::new();
    row.insert("col1".to_string(), "val1".to_string());
    rows.push(row);
    tables.insert("t1".to_string(), rows);

    let meta = mgr.create_backup("testdb", tables).unwrap();
    assert!(meta.checksum.is_some());
    assert!(!meta.checksum.as_ref().unwrap().is_empty());
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

    let mut tables = HashMap::new();
    let mut rows = Vec::new();
    let mut row = HashMap::new();
    row.insert("a".to_string(), "1".to_string());
    rows.push(row);
    tables.insert("t1".to_string(), rows);

    let meta = mgr.create_backup("testdb", tables).unwrap();
    let data = mgr.restore(&meta.id);
    assert!(data.is_ok());
}

#[test]
fn test_backup_manager_delete_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let mut tables = HashMap::new();
    let rows: Vec<HashMap<String, String>> = Vec::new();
    tables.insert("t1".to_string(), rows);

    let meta = mgr.create_backup("db", tables).unwrap();
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

    let empty_tables: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();
    mgr.create_backup("db1", empty_tables.clone()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    mgr.create_backup("db2", empty_tables.clone()).unwrap();

    let backups = mgr.list_backups();
    assert_eq!(backups.len(), 2);
}

#[test]
fn test_backup_manager_multiple_backups_with_delay() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let empty: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();

    let m1 = mgr.create_backup("mydb", empty.clone()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let m2 = mgr.create_backup("mydb", empty.clone()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let m3 = mgr.create_backup("mydb", empty).unwrap();

    assert_ne!(m1.id, m2.id);
    assert_ne!(m2.id, m3.id);

    let backups = mgr.list_backups();
    assert_eq!(backups.len(), 3);
}
// ============ BackupManager content verification (temp dir) ============

/// Verify that restore() parses table names from the SQL backup content.
#[test]
fn test_backup_manager_restore_parses_table_names() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let mut tables = HashMap::new();
    let mut rows = Vec::new();
    let mut row = HashMap::new();
    row.insert("id".to_string(), "1".to_string());
    row.insert("name".to_string(), "alice".to_string());
    rows.push(row);
    tables.insert("users".to_string(), rows);

    let meta = mgr.create_backup("testdb", tables).unwrap();
    let restored = mgr.restore(&meta.id).unwrap();

    // restore() parses INSERT INTO table names (simplified parser)
    // The table 'users' should be in the restored data
    assert!(
        restored.contains_key("users"),
        "restored data should contain 'users' table"
    );
}

/// BackupManager::delete removes the backup and subsequent restore fails.
#[test]
fn test_backup_manager_delete_then_restore_fails() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("b"));

    let mut tables = HashMap::new();
    tables.insert("t1".to_string(), Vec::new());

    let meta = mgr.create_backup("testdb", tables).unwrap();
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

    let mut tables = HashMap::new();
    tables.insert("t1".to_string(), Vec::new());

    let meta = mgr.create_backup("testdb", tables).unwrap();
    assert!(meta.checksum.is_some(), "checksum should be set");
    let checksum = meta.checksum.unwrap();
    assert!(!checksum.is_empty(), "checksum should be non-empty");
    assert!(
        checksum.chars().all(|c| c.is_ascii_hexdigit()),
        "checksum should be hex"
    );
}
