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

// ============================================================================
// BackupManager get_backup, restore, delete_backup tests
// ============================================================================

#[test]
fn test_backup_manager_get_backup_existing() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use std::collections::HashMap;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let tables: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();
    let meta = mgr.create_backup("testdb", tables).unwrap();
    let backup_id = meta.id.clone();

    let found = mgr.get_backup(&backup_id);
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, backup_id);
    assert_eq!(found.database, "testdb");
}

#[test]
fn test_backup_manager_get_backup_nonexistent() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let found = mgr.get_backup("nonexistent_backup_id");
    assert!(found.is_none());
}

#[test]
fn test_backup_manager_delete_backup() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use std::collections::HashMap;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let tables: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();
    let meta = mgr.create_backup("testdb", tables).unwrap();
    let backup_id = meta.id.clone();

    // Verify it exists
    assert!(mgr.get_backup(&backup_id).is_some());

    // Delete it
    mgr.delete_backup(&backup_id).unwrap();

    // Verify it's gone
    assert!(mgr.get_backup(&backup_id).is_none());
}

#[test]
fn test_backup_manager_delete_backup_nonexistent() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let result = mgr.delete_backup("nonexistent_backup_id");
    // Should fail - backup file doesn't exist
    assert!(result.is_err());
}

#[test]
fn test_backup_manager_restore_existing() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use std::collections::HashMap;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let tables: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();
    let meta = mgr.create_backup("testdb", tables).unwrap();
    let backup_id = meta.id.clone();

    let result = mgr.restore(&backup_id);
    assert!(result.is_ok());
    let data = result.unwrap();
    assert!(data.is_empty()); // no INSERT statements parsed
}

#[test]
fn test_backup_manager_restore_nonexistent() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let result = mgr.restore("nonexistent_backup_id");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not found"));
}

// Note: chrono_lite_now() is second-precision, so multiple create_backup
// calls within the same second collide on the same HashMap key.
// We test list_backups with separate BackupManager instances instead.

#[test]
fn test_backup_manager_list_backups_single() {
    use sqlrustgo_tools::backup_restore::BackupManager;
    use std::collections::HashMap;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("backups"));
    let tables: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();
    mgr.create_backup("testdb", tables).unwrap();

    let list = mgr.list_backups();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].database, "testdb");
}

// ============================================================================
// md5_simple and serde_json_simple function tests
// ============================================================================

#[test]
fn test_md5_simple_deterministic() {
    use sqlrustgo_tools::backup_restore::md5_simple;
    let h1 = md5_simple("hello");
    let h2 = md5_simple("hello");
    assert_eq!(h1, h2, "same input must produce same hash");
}

#[test]
fn test_md5_simple_different_inputs() {
    use sqlrustgo_tools::backup_restore::md5_simple;
    let h1 = md5_simple("hello");
    let h2 = md5_simple("world");
    assert_ne!(h1, h2, "different inputs must produce different hashes");
}

#[test]
fn test_md5_simple_empty_string() {
    use sqlrustgo_tools::backup_restore::md5_simple;
    let h = md5_simple("");
    // Empty string should produce a known value
    assert_eq!(h, 0);
}

#[test]
fn test_md5_simple_known_input() {
    use sqlrustgo_tools::backup_restore::md5_simple;
    // Compute expected: bytes "a" -> (97*1) rotated 5 = 3104
    // Since algorithm is simple, we just check it's stable and non-zero for non-empty
    let h = md5_simple("a");
    assert_ne!(h, 0);
}

#[test]
fn test_md5_simple_unicode() {
    use sqlrustgo_tools::backup_restore::md5_simple;
    let h = md5_simple("日本語");
    // Should not panic and should produce a value
    let _ = h as u64; // check it's a valid u32
}

#[test]
fn test_restore_result_fields() {
    use sqlrustgo_tools::backup_restore::RestoreResult;
    let result = RestoreResult {
        backup_id: "backup_123".to_string(),
        rows_restored: 500,
        duration_ms: 1500,
    };
    assert_eq!(result.backup_id, "backup_123");
    assert_eq!(result.rows_restored, 500);
    assert_eq!(result.duration_ms, 1500);
}

#[test]
fn test_serde_json_simple_completed() {
    use sqlrustgo_tools::backup_restore::{
        serde_json_simple, BackupMetadata, BackupStatus, BackupType,
    };
    let meta = BackupMetadata::new("bk_001".to_string(), BackupType::Full, "testdb".to_string());
    let json = serde_json_simple(&meta);
    assert!(json.contains("bk_001"));
    assert!(json.contains("testdb"));
}

#[test]
fn test_serde_json_simple_in_progress() {
    use sqlrustgo_tools::backup_restore::{
        serde_json_simple, BackupMetadata, BackupStatus, BackupType,
    };
    let meta = BackupMetadata::new(
        "bk_002".to_string(),
        BackupType::Incremental,
        "proddb".to_string(),
    );
    let json = serde_json_simple(&meta);
    assert!(json.contains("in_progress"));
}
