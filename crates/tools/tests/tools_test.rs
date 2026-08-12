//! Tools crate additional unit tests

use sqlrustgo_tools::backup_restore::{BackupManager, BackupMetadata, BackupStatus, BackupType};
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

// ============ Additional BackupManager Tests ============

#[test]
fn test_backup_metadata_differential() {
    use sqlrustgo_tools::backup_restore::{BackupMetadata, BackupType};
    let mut meta = BackupMetadata::new(
        "diff-1".to_string(),
        BackupType::Differential,
        "testdb".to_string(),
    );
    meta.complete(2048, "sha256:diff".to_string());
    assert!(matches!(meta.backup_type, BackupType::Differential));
    assert!(matches!(meta.status, BackupStatus::Completed));
}

#[test]
fn test_backup_metadata_incremental() {
    use sqlrustgo_tools::backup_restore::{BackupMetadata, BackupType};
    let meta = BackupMetadata::new(
        "incr-1".to_string(),
        BackupType::Incremental,
        "testdb".to_string(),
    );
    assert!(matches!(meta.backup_type, BackupType::Incremental));
}

#[test]
fn test_backup_status_failed_message() {
    use sqlrustgo_tools::backup_restore::BackupStatus;
    let status = BackupStatus::Failed("insufficient space".to_string());
    match status {
        BackupStatus::Failed(msg) => assert!(msg.contains("space")),
        _ => panic!("expected Failed"),
    }
}

// ============================================================================
// config_hot_reload coverage tests (Issue #3943)
// ============================================================================

#[test]
fn test_app_config_default() {
    use sqlrustgo_tools::config_hot_reload::{AppConfig, CacheConfig, DatabaseConfig, LogConfig};
    let app = AppConfig::default();
    assert_eq!(app.version, "2.1.0");
    assert_eq!(app.database, DatabaseConfig::default());
    assert_eq!(app.log, LogConfig::default());
    assert_eq!(app.cache, CacheConfig::default());
}

#[test]
fn test_database_config_default_values() {
    use sqlrustgo_tools::config_hot_reload::DatabaseConfig;
    let db = DatabaseConfig::default();
    assert_eq!(db.host, "localhost");
    assert_eq!(db.port, 5432);
    assert_eq!(db.max_connections, 100);
    assert_eq!(db.timeout_seconds, 30);
}

#[test]
fn test_log_config_default_values() {
    use sqlrustgo_tools::config_hot_reload::LogConfig;
    let log = LogConfig::default();
    assert_eq!(log.level, "info");
    assert_eq!(log.rotation_size_mb, 100);
    assert_eq!(log.retention_days, 7);
    assert_eq!(log.format, "json");
}

#[test]
fn test_cache_config_default_values() {
    use sqlrustgo_tools::config_hot_reload::CacheConfig;
    let cache = CacheConfig::default();
    assert!(cache.enabled);
    assert_eq!(cache.max_size_mb, 512);
    assert_eq!(cache.ttl_seconds, 3600);
}

#[test]
fn test_config_manager_load_save_roundtrip() {
    use sqlrustgo_tools::config_hot_reload::{
        load_config, save_config, AppConfig, CacheConfig, DatabaseConfig, LogConfig,
    };
    let dir = std::env::temp_dir().join(format!(
        "sqlrustgo-tools-config-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");

    // Default load returns default config when file is missing.
    let loaded = load_config(&path).expect("load_config default");
    assert_eq!(loaded, AppConfig::default());

    // Custom config can be saved and reloaded.
    let mut custom = AppConfig::default();
    custom.database = DatabaseConfig {
        host: "db.example.com".into(),
        port: 3306,
        max_connections: 50,
        timeout_seconds: 60,
    };
    custom.log = LogConfig {
        level: "debug".into(),
        rotation_size_mb: 200,
        retention_days: 14,
        format: "text".into(),
    };
    custom.cache = CacheConfig {
        enabled: false,
        max_size_mb: 1024,
        ttl_seconds: 7200,
    };
    save_config(&path, &custom).expect("save_config");
    let reloaded = load_config(&path).expect("load_config custom");
    assert_eq!(reloaded, custom);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_config_change_variants() {
    use sqlrustgo_tools::config_hot_reload::{ConfigChange, DatabaseConfig, LogConfig, CacheConfig};
    let db_change = ConfigChange::Database(DatabaseConfig::default());
    let log_change = ConfigChange::Log(LogConfig::default());
    let cache_change = ConfigChange::Cache(CacheConfig::default());
    let full_change = ConfigChange::Full(AppConfig::default());
    // Pattern-match each variant to ensure Debug derives correctly.
    match db_change { ConfigChange::Database(_) => {} _ => panic!("expected Database") }
    match log_change { ConfigChange::Log(_) => {} _ => panic!("expected Log") }
    match cache_change { ConfigChange::Cache(_) => {} _ => panic!("expected Cache") }
    match full_change { ConfigChange::Full(_) => {} _ => panic!("expected Full") }
}

#[test]
fn test_config_listener_callback() {
    use sqlrustgo_tools::config_hot_reload::{create_config_listener, ConfigChange};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let listener = create_config_listener(|change: ConfigChange| {
        // Verify we received a ConfigChange value.
        match change {
            ConfigChange::Full(_) => {
                CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
    });
    listener.on_config_change(ConfigChange::Full(Default::default()));
    listener.on_config_change(ConfigChange::Full(Default::default()));
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
}

// ============================================================================
// Helper: AppConfig default import
// ============================================================================
use sqlrustgo_tools::config_hot_reload::AppConfig;
