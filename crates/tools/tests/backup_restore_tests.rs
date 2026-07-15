//! Backup and restore module tests
//!
//! Tests: BackupType, BackupStatus, BackupMetadata, BackupEntry,
//!        md5_simple, chrono_lite_now, serde_json_simple

use sqlrustgo_tools::backup_restore::{
    chrono_lite_now, md5_simple, serde_json_simple,
    BackupMetadata, BackupStatus, BackupType,
};

// ============ BackupType tests ============

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
fn test_backup_type_debug() {
    let bt = BackupType::Incremental;
    let debug = format!("{:?}", bt);
    assert!(debug.contains("Incremental"));
}

// ============ BackupStatus tests ============

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
fn test_backup_status_debug() {
    let status = BackupStatus::Failed("err".to_string());
    let debug = format!("{:?}", status);
    assert!(debug.contains("Failed"));
}

// ============ BackupMetadata tests ============

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

// ============ md5_simple tests ============

#[test]
fn test_md5_simple_empty() {
    let hash = md5_simple("");
    // empty input produces zero hash
    assert_eq!(hash, 0);
}

#[test]
fn test_md5_simple_hello() {
    let hash = md5_simple("hello");
    assert_ne!(hash, 0);
}

#[test]
fn test_md5_simple_deterministic() {
    let h1 = md5_simple("test data");
    let h2 = md5_simple("test data");
    assert_eq!(h1, h2, "md5_simple should be deterministic");
}

#[test]
fn test_md5_simple_different_inputs() {
    let h1 = md5_simple("abc");
    let h2 = md5_simple("xyz");
    assert_ne!(h1, h2, "Different inputs should produce different hashes");
}

#[test]
fn test_md5_simple_longer_string() {
    let data = "This is a much longer string that should still work correctly with the md5_simple hash function";
    let hash = md5_simple(data);
    assert_ne!(hash, 0);
}

#[test]
fn test_md5_simple_unicode() {
    let hash = md5_simple("héllo wörld");
    assert_ne!(hash, 0);
}

// ============ chrono_lite_now tests ============

#[test]
fn test_chrono_lite_now_format() {
    let ts = chrono_lite_now();
    // Returns Unix timestamp string
    assert!(!ts.is_empty());
    assert!(ts.len() >= 6);
}

#[test]
fn test_chrono_lite_now_deterministic() {
    let ts1 = chrono_lite_now();
    let ts2 = chrono_lite_now();
    assert_eq!(ts1, ts2, "Within same second, should be equal");
}

#[test]
fn test_chrono_lite_now_different_times() {
    let ts1 = chrono_lite_now();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let ts2 = chrono_lite_now();
    assert_ne!(ts1, ts2, "Different seconds should produce different timestamps");
}

// ============ serde_json_simple tests ============

#[test]
fn test_serde_json_simple_empty_metadata() {
    let meta = BackupMetadata::new(
        "backup_empty".to_string(),
        BackupType::Full,
        "testdb".to_string(),
    );
    let json = serde_json_simple(&meta);
    assert!(!json.is_empty());
    assert!(json.contains("backup_empty"));
    assert!(json.contains("Full"));
}

#[test]
fn test_serde_json_simple_incremental() {
    let meta = BackupMetadata::new(
        "backup_incr".to_string(),
        BackupType::Incremental,
        "mydb".to_string(),
    );
    let json = serde_json_simple(&meta);
    assert!(!json.is_empty());
    assert!(json.contains("backup_incr"));
    assert!(json.contains("Incremental"));
}

#[test]
fn test_serde_json_simple_differential() {
    let mut meta = BackupMetadata::new(
        "backup_diff".to_string(),
        BackupType::Differential,
        "otherdb".to_string(),
    );
    meta.complete(2048, "checksum123".to_string());
    let json = serde_json_simple(&meta);
    assert!(!json.is_empty());
    assert!(json.contains("checksum123"));
}

#[test]
fn test_serde_json_simple_with_failed_status() {
    let mut meta = BackupMetadata::new(
        "backup_fail".to_string(),
        BackupType::Full,
        "faildb".to_string(),
    );
    meta.fail("disk error".to_string());
    let json = serde_json_simple(&meta);
    assert!(!json.is_empty());
    assert!(json.contains("failed"));
    assert!(json.contains("disk error"));
}

#[test]
fn test_serde_json_simple_with_tables() {
    let mut meta = BackupMetadata::new(
        "backup_tables".to_string(),
        BackupType::Full,
        "db".to_string(),
    );
    meta.tables.push("t1".to_string());
    meta.tables.push("t2".to_string());
    let json = serde_json_simple(&meta);
    assert!(json.contains("t1"));
    assert!(json.contains("t2"));
}
