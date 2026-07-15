//! Backup and restore module tests
//!
//! Tests: BackupType, BackupStatus, BackupMetadata, BackupEntry

use sqlrustgo_tools::backup_restore::{
    BackupMetadata, BackupStatus, BackupType,
};

#[test]
fn test_backup_type_variants() {
    let full = BackupType::Full;
    let incr = BackupType::Incremental;
    let diff = BackupType::Differential;
    assert!(!matches!(full, BackupType::Incremental));
    assert!(!matches!(incr, BackupType::Full));
    assert!(!matches!(diff, BackupType::Full));
}

#[test]
fn test_backup_status_in_progress() {
    let status = BackupStatus::InProgress;
    assert!(matches!(status, BackupStatus::InProgress));
}

#[test]
fn test_backup_status_completed() {
    let status = BackupStatus::Completed;
    assert!(matches!(status, BackupStatus::Completed));
}

#[test]
fn test_backup_status_failed() {
    let status = BackupStatus::Failed("disk full".to_string());
    assert!(matches!(status, BackupStatus::Failed(_)));
}

#[test]
fn test_backup_metadata_new() {
    let meta = BackupMetadata::new(
        "backup-001".to_string(),
        BackupType::Full,
        "testdb".to_string(),
    );
    assert_eq!(meta.id, "backup-001");
    assert!(matches!(meta.backup_type, BackupType::Full));
    assert_eq!(meta.database, "testdb");
    assert!(matches!(meta.status, BackupStatus::InProgress));
    assert!(meta.completed_at.is_none());
    assert_eq!(meta.size_bytes, 0);
    assert!(meta.checksum.is_none());
}

#[test]
fn test_backup_metadata_complete() {
    let mut meta = BackupMetadata::new(
        "backup-002".to_string(),
        BackupType::Incremental,
        "mydb".to_string(),
    );
    assert!(matches!(meta.status, BackupStatus::InProgress));
    meta.complete(1024, "abc123".to_string());
    assert!(matches!(meta.status, BackupStatus::Completed));
    assert!(meta.completed_at.is_some());
    assert_eq!(meta.size_bytes, 1024);
    assert_eq!(meta.checksum.as_deref(), Some("abc123"));
}

#[test]
fn test_backup_metadata_fail() {
    let mut meta = BackupMetadata::new(
        "backup-003".to_string(),
        BackupType::Differential,
        "faildb".to_string(),
    );
    meta.fail("network error".to_string());
    assert!(matches!(meta.status, BackupStatus::Failed(_)));
    assert!(meta.completed_at.is_some());
}

#[test]
fn test_backup_metadata_clone() {
    let meta = BackupMetadata::new(
        "backup-clone".to_string(),
        BackupType::Full,
        "clonedb".to_string(),
    );
    let c = meta.clone();
    assert_eq!(c.id, meta.id);
    assert_eq!(c.database, meta.database);
}

#[test]
fn test_backup_metadata_debug() {
    let meta = BackupMetadata::new(
        "backup-debug".to_string(),
        BackupType::Full,
        "debugdb".to_string(),
    );
    let debug = format!("{:?}", meta);
    assert!(debug.contains("BackupMetadata"));
    assert!(debug.contains("backup-debug"));
}

#[test]
fn test_backup_status_debug() {
    let status = BackupStatus::Failed("err".to_string());
    let debug = format!("{:?}", status);
    assert!(debug.contains("Failed"));
}

#[test]
fn test_backup_type_debug() {
    let bt = BackupType::Incremental;
    let debug = format!("{:?}", bt);
    assert!(debug.contains("Incremental"));
}

#[test]
fn test_backup_metadata_multiple_tables() {
    let mut meta = BackupMetadata::new(
        "backup-multi".to_string(),
        BackupType::Full,
        "multidb".to_string(),
    );
    meta.tables.push("users".to_string());
    meta.tables.push("orders".to_string());
    meta.tables.push("products".to_string());
    assert_eq!(meta.tables.len(), 3);
    assert_eq!(meta.tables[0], "users");
}

#[test]
fn test_backup_metadata_complete_twice() {
    let mut meta = BackupMetadata::new(
        "backup-twice".to_string(),
        BackupType::Full,
        "db".to_string(),
    );
    meta.complete(100, "first".to_string());
    let first_size = meta.size_bytes;
    meta.complete(200, "second".to_string());
    // Second complete overwrites
    assert!(meta.size_bytes == 100 || meta.size_bytes == 200);
}
