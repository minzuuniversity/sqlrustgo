//! Tests for [`BackupManager`](sqlrustgo_tools::backup_restore::BackupManager).

use std::collections::HashMap;
use sqlrustgo_tools::backup_restore::BackupManager;
use tempfile::TempDir;

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

    let mut row1 = HashMap::new();
    row1.insert("id".to_string(), "1".to_string());
    let mut row2 = HashMap::new();
    row2.insert("id".to_string(), "2".to_string());
    let mut tables = HashMap::new();
    tables.insert("users".to_string(), vec![row1, row2]);

    let result = mgr.create_backup("testdb", tables);
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

    let mut row = HashMap::new();
    row.insert("id".to_string(), "1".to_string());
    let mut tables = HashMap::new();
    tables.insert("t".to_string(), vec![row]);
    mgr.create_backup("mydb", tables).ok();

    let backups = mgr.list_backups();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].database, "mydb");
}

/// Verify [`BackupManager::get_backup`] returns correct metadata.
#[test]
fn test_get_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let mut row = HashMap::new();
    row.insert("id".to_string(), "1".to_string());
    let mut tables = HashMap::new();
    tables.insert("t".to_string(), vec![row]);
    let created = mgr.create_backup("mydb", tables).unwrap();

    let retrieved = mgr.get_backup(&created.id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().database, "mydb");
}

/// Verify [`BackupManager::delete_backup`] removes files.
#[test]
fn test_delete_backup() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let mut row = HashMap::new();
    row.insert("id".to_string(), "1".to_string());
    let mut tables = HashMap::new();
    tables.insert("t".to_string(), vec![row]);
    let created = mgr.create_backup("mydb", tables).unwrap();

    let sql_file = tmp.path().join(format!("{}.sql", created.id));
    let meta_file = tmp.path().join(format!("{}.meta.json", created.id));
    assert!(sql_file.exists());
    assert!(meta_file.exists());

    let result = mgr.delete_backup(&created.id);
    assert!(result.is_ok());

    assert!(!sql_file.exists(), ".sql file should be deleted");
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
