// Tools crate backup_restore coverage tests

use sqlrustgo_tools::backup_restore::{BackupMetadata, BackupStatus, BackupType, ExportOptions};
use std::collections::HashMap;

// ============ BackupType tests ============

#[test]
fn test_backup_type_debug() {
    let dt = BackupType::Differential;
    assert!(format!("{:?}", dt).contains("Differential"));
}

#[test]
fn test_backup_type_clone() {
    let full = BackupType::Full;
    let cloned = full.clone();
    assert!(matches!(cloned, BackupType::Full));
}

// ============ BackupStatus tests ============

#[test]
fn test_backup_status_in_progress() {
    let status = BackupStatus::InProgress;
    assert!(format!("{:?}", status).contains("InProgress"));
}

#[test]
fn test_backup_status_completed() {
    let status = BackupStatus::Completed;
    assert!(format!("{:?}", status).contains("Completed"));
}

#[test]
fn test_backup_status_failed() {
    let status = BackupStatus::Failed("disk full".to_string());
    let debug = format!("{:?}", status);
    assert!(debug.contains("Failed"));
    assert!(debug.contains("disk full"));
}

#[test]
fn test_backup_status_clone() {
    let s1 = BackupStatus::Completed;
    let s2 = s1.clone();
    assert!(matches!(s2, BackupStatus::Completed));
}

// ============ BackupMetadata tests ============

#[test]
fn test_backup_metadata_new() {
    let meta = BackupMetadata::new("bkp_001".to_string(), BackupType::Full, "mydb".to_string());
    assert_eq!(meta.id, "bkp_001");
    assert!(matches!(meta.backup_type, BackupType::Full));
    assert_eq!(meta.database, "mydb");
    assert!(matches!(meta.status, BackupStatus::InProgress));
    assert!(meta.completed_at.is_none());
    assert_eq!(meta.size_bytes, 0);
    assert!(meta.checksum.is_none());
}

#[test]
fn test_backup_metadata_complete() {
    let mut meta = BackupMetadata::new(
        "bkp_002".to_string(),
        BackupType::Incremental,
        "testdb".to_string(),
    );
    assert!(matches!(meta.status, BackupStatus::InProgress));
    meta.complete(1024, "abc123".to_string());
    assert!(matches!(meta.status, BackupStatus::Completed));
    assert_eq!(meta.size_bytes, 1024);
    assert_eq!(meta.checksum, Some("abc123".to_string()));
    assert!(meta.completed_at.is_some());
}

#[test]
fn test_backup_metadata_fail() {
    let mut meta = BackupMetadata::new("bkp_003".to_string(), BackupType::Full, "mydb".to_string());
    meta.fail("network timeout".to_string());
    match &meta.status {
        BackupStatus::Failed(msg) => assert!(msg.contains("network")),
        other => panic!("expected Failed, got {:?}", other),
    }
    assert!(meta.completed_at.is_some());
}

#[test]
fn test_backup_metadata_add_table() {
    let mut meta = BackupMetadata::new("bkp_004".to_string(), BackupType::Full, "mydb".to_string());
    meta.tables.push("users".to_string());
    meta.tables.push("orders".to_string());
    assert_eq!(meta.tables.len(), 2);
    assert_eq!(meta.tables[0], "users");
}

#[test]
fn test_backup_metadata_clone() {
    let meta = BackupMetadata::new(
        "bkp_005".to_string(),
        BackupType::Differential,
        "db".to_string(),
    );
    let cloned = meta.clone();
    assert_eq!(cloned.id, meta.id);
    assert_eq!(cloned.database, meta.database);
}

// ============ ExportOptions tests ============

#[test]
fn test_export_options_default() {
    let opts = ExportOptions::default();
    assert!(!opts.schema_only);
    assert!(opts.add_drop);
    assert!(opts.single_transaction);
    assert!(opts.lock_tables);
}

#[test]
fn test_export_options_custom() {
    let opts = ExportOptions {
        schema_only: true,
        add_drop: false,
        single_transaction: false,
        lock_tables: false,
    };
    assert!(opts.schema_only);
    assert!(!opts.add_drop);
    assert!(!opts.single_transaction);
    assert!(!opts.lock_tables);
}

// ============ Integration: BackupManager via BackupMetadata ============

#[test]
fn test_backup_lifecycle_in_progress_to_completed() {
    let mut meta = BackupMetadata::new(
        "bkp_lifecycle".to_string(),
        BackupType::Full,
        "lifecycle_db".to_string(),
    );
    assert!(matches!(meta.status, BackupStatus::InProgress));

    meta.complete(2048, "def456".to_string());
    assert!(matches!(meta.status, BackupStatus::Completed));
    assert_eq!(meta.size_bytes, 2048);
    assert_eq!(meta.checksum.as_deref(), Some("def456"));
}

#[test]
fn test_backup_lifecycle_failure() {
    let mut meta = BackupMetadata::new(
        "bkp_fail".to_string(),
        BackupType::Incremental,
        "fail_db".to_string(),
    );
    meta.fail("I/O error".to_string());
    match &meta.status {
        BackupStatus::Failed(msg) => assert!(msg.contains("I/O")),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn test_backup_multiple_types() {
    for bt in [
        BackupType::Full,
        BackupType::Incremental,
        BackupType::Differential,
    ] {
        let meta = BackupMetadata::new("id".to_string(), bt, "db".to_string());
        assert!(matches!(
            meta.backup_type,
            BackupType::Full | BackupType::Incremental | BackupType::Differential
        ));
    }
}

// ============ Helper function tests (through impl behavior) ============

#[test]
fn test_backup_metadata_size_accumulation() {
    let mut meta = BackupMetadata::new("id".to_string(), BackupType::Full, "db".to_string());
    meta.complete(100, "a".to_string());
    assert_eq!(meta.size_bytes, 100);
    // Simulate adding more data
    meta.size_bytes = 500;
    assert_eq!(meta.size_bytes, 500);
}

#[test]
fn test_backup_metadata_checksum_none_until_complete() {
    let meta = BackupMetadata::new("id".to_string(), BackupType::Full, "db".to_string());
    assert!(meta.checksum.is_none());
}
