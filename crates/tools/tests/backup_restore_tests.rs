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
/// Verify [`BackupManager::restore`] can parse INSERT INTO statements.
#[test]
fn test_restore_parses_insert() {
    use sqlrustgo_tools::backup_restore::md5_simple;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    // Pre-create a backup file manually so restore can parse it
    let backup_id = "backup_restore_parse_test";
    let sql_content = "-- SQLRustGo Backup\n-- Database: testdb\n\n-- Table: users\n-- Rows: 2\n\nCREATE TABLE IF NOT EXISTS users (id, name);\nINSERT INTO users (id, name) VALUES (1, 'alice');\nINSERT INTO users (id, name) VALUES (2, 'bob');\n";
    let backup_file = tmp.path().join(format!("{}.sql", backup_id));
    std::fs::write(&backup_file, sql_content).unwrap();

    // Create metadata
    let mut metadata = sqlrustgo_tools::backup_restore::BackupMetadata::new(
        backup_id.to_string(),
        sqlrustgo_tools::backup_restore::BackupType::Full,
        "testdb".to_string(),
    );
    metadata.complete(
        sql_content.len() as u64,
        format!("{:x}", md5_simple(sql_content)),
    );
    let meta_content = sqlrustgo_tools::backup_restore::serde_json_simple(&metadata);
    let meta_file = tmp.path().join(format!("{}.meta.json", backup_id));
    std::fs::write(&meta_file, &meta_content).ok();

    let result = mgr.restore(backup_id);
    assert!(result.is_ok(), "restore failed: {:?}", result);
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
        err.contains("not found")
            || err.contains("remove")
            || err.contains("No such file"),
        "unexpected error: {}",
        err
    );
}

/// Verify [`BackupManager::restore`] returns error if .sql file is missing.
#[test]
fn test_restore_missing_sql_file() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    // Create only the .meta.json without the .sql file
    let backup_id = "backup_missing_sql";
    let meta_content = r#"{"id":"backup_missing_sql","type":"Full","started":"1234567890","completed":"1234567891","size":100,"database":"testdb","tables":[],"status":"completed","checksum":"abc"}"#;
    let meta_file = tmp.path().join(format!("{}.meta.json", backup_id));
    std::fs::write(&meta_file, meta_content).ok();

    let result = mgr.restore(backup_id);
    assert!(result.is_err(), "restore should fail without .sql file");
}

/// Verify [`BackupManager::create_backup`] with empty tables map.
#[test]
fn test_create_backup_empty_tables() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().to_path_buf());

    let tables: std::collections::HashMap<String, Vec<std::collections::HashMap<String, String>>> =
        std::collections::HashMap::new();

    let result = mgr.create_backup("emptydb", tables);
    assert!(result.is_ok(), "create_backup with empty tables failed: {:?}", result);
    let metadata = result.unwrap();
    assert!(metadata.tables.is_empty(), "should have no tables");
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
    use sqlrustgo_tools::backup_restore::{BackupMetadata, BackupStatus, BackupType};

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
    assert!(j3.contains("disk full"), "Failed JSON should contain error: {}", j3);
}
