//! Tools crate additional unit tests

use sqlrustgo_tools::backup_restore::{
    BackupType, BackupStatus, BackupMetadata, BackupManager,
};
use std::path::PathBuf;

#[test]
fn test_backup_metadata_new() {
    let meta = BackupMetadata::new(
        "test-id".to_string(),
        BackupType::Full,
        "testdb".to_string(),
    );
    assert!(matches!(meta.id.as_str(), "test-id"));
    assert!(matches!(meta.backup_type, BackupType::Full));
    assert!(matches!(meta.database.as_str(), "testdb"));
    assert!(matches!(meta.status, BackupStatus::InProgress));
    assert!(matches!(meta.size_bytes, 0));
}

#[test]
fn test_backup_metadata_complete() {
    let mut meta = BackupMetadata::new(
        "test-id".to_string(),
        BackupType::Full,
        "testdb".to_string(),
    );
    meta.complete(1024, "sha256:abc".to_string());
    assert!(matches!(meta.status, BackupStatus::Completed));
    assert!(matches!(meta.size_bytes, 1024));
    assert!(meta.checksum.as_deref().is_some_and(|s| s.contains("abc")));
}

#[test]
fn test_backup_metadata_fail() {
    let mut meta = BackupMetadata::new(
        "test-id".to_string(),
        BackupType::Full,
        "testdb".to_string(),
    );
    meta.fail("disk full".to_string());
    assert!(matches!(meta.status, BackupStatus::Failed(e) if e == "disk full"));
}

#[test]
fn test_backup_type_variants() {
    assert!(matches!(BackupType::Full, BackupType::Full));
    assert!(matches!(BackupType::Incremental, BackupType::Incremental));
    assert!(matches!(BackupType::Differential, BackupType::Differential));
}

#[test]
fn test_backup_status_variants() {
    assert!(matches!(BackupStatus::InProgress, BackupStatus::InProgress));
    assert!(matches!(BackupStatus::Completed, BackupStatus::Completed));
    // Test Failed variant has String payload
    let failed_status = BackupStatus::Failed("test error".to_string());
    assert!(matches!(failed_status, BackupStatus::Failed(ref e) if e == "test error"));
}

#[test]
fn test_backup_manager_new() {
    let temp_dir = PathBuf::from("/tmp/test-backup-mgr");
    std::fs::create_dir_all(&temp_dir).ok();
    let manager = BackupManager::new(temp_dir.clone());
    assert_eq!(manager.backup_dir(), temp_dir.as_path());
    std::fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_backup_manager_list_empty() {
    let temp_dir = PathBuf::from("/tmp/test-backup-mgr-list");
    std::fs::create_dir_all(&temp_dir).ok();
    let manager = BackupManager::new(temp_dir);
    let backups = manager.list_backups();
    assert!(backups.is_empty());
    std::fs::remove_dir_all("/tmp/test-backup-mgr-list").ok();
}

#[test]
fn test_backup_manager_get_nonexistent() {
    let temp_dir = PathBuf::from("/tmp/test-backup-mgr-get");
    std::fs::create_dir_all(&temp_dir).ok();
    let manager = BackupManager::new(temp_dir);
    let result = manager.get_backup("nonexistent");
    assert!(result.is_none());
    std::fs::remove_dir_all("/tmp/test-backup-mgr-get").ok();
}
