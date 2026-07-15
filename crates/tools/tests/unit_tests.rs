//! Unit tests for sqlrustgo-tools public API.

use sqlrustgo_tools::{
    backup_restore::{
        chrono_lite_now, md5_simple, serde_json_simple, BackupManager, BackupMetadata, BackupStatus,
        BackupType, ExportOptions, RestoreResult,
    },
    mysqldump::{
        ColumnDef, DumpImporter, ForeignKeyRef, ImportMode, ImportStats, SqlStatement,
    },
    upgrade::{
        create_upgrade_plan, list_history, show_status, UpgradeCommand, UpgradeManifest,
        UpgradePlan, UpgradeStatus, VersionInfo,
    },
};

// ============================================================================
// backup_restore module
// ============================================================================

#[test]
fn test_backup_type_variants() {
    let variants = [
        BackupType::Full,
        BackupType::Incremental,
        BackupType::Differential,
    ];
    assert_eq!(variants.len(), 3);
}

#[test]
fn test_backup_status_variants() {
    let variants = [
        BackupStatus::InProgress,
        BackupStatus::Completed,
        BackupStatus::Failed("disk full".to_string()),
    ];
    assert_eq!(variants.len(), 3);
}

#[test]
fn test_backup_metadata_new() {
    let metadata = BackupMetadata::new(
        "bak-001".to_string(),
        BackupType::Full,
        "mydb".to_string(),
    );
    assert_eq!(metadata.id, "bak-001");
    assert!(matches!(metadata.backup_type, BackupType::Full));
    assert_eq!(metadata.database, "mydb");
    assert!(metadata.tables.is_empty());
}

#[test]
fn test_backup_metadata_fields() {
    let mut metadata = BackupMetadata::new("bak-002".to_string(), BackupType::Incremental, "testdb".to_string());
    metadata.size_bytes = 2048;
    metadata.checksum = Some("sha256hash".to_string());
    metadata.tables = vec!["t1".to_string(), "t2".to_string()];
    assert_eq!(metadata.size_bytes, 2048);
    assert_eq!(metadata.checksum, Some("sha256hash".to_string()));
    assert_eq!(metadata.tables.len(), 2);
}

#[test]
fn test_export_options_default() {
    let opts = ExportOptions::default();
    assert!(!opts.schema_only);
    assert!(opts.add_drop);
    assert!(opts.single_transaction);
    assert!(opts.lock_tables);
}

#[test]
fn test_restore_result_fields() {
    let result = RestoreResult {
        backup_id: "bak-001".to_string(),
        rows_restored: 5000,
        duration_ms: 1234,
    };
    assert_eq!(result.backup_id, "bak-001");
    assert_eq!(result.rows_restored, 5000);
}

#[test]
fn test_chrono_lite_now_format() {
    let now = chrono_lite_now();
    // Returns Unix timestamp string in seconds
    // Should be numeric string, at least 10 digits for timestamps after 2009
    assert!(now.len() >= 10);
    // Should parse as a valid u64
    let ts: u64 = now.parse().unwrap();
    assert!(ts > 1_000_000_000); // After year 2001
}

#[test]
fn test_md5_simple_empty() {
    let hash = md5_simple("");
    assert_eq!(hash, 0u32); // empty string hash is 0
}

#[test]
fn test_md5_simple_hello() {
    let hash = md5_simple("hello world");
    let hash2 = md5_simple("hello world");
    assert_eq!(hash, hash2);
}

#[test]
fn test_serde_json_simple() {
    let metadata = BackupMetadata::new("bak-test".to_string(), BackupType::Full, "test".to_string());
    let json = serde_json_simple(&metadata);
    assert!(!json.is_empty());
    assert!(json.contains("bak-test"));
}

// ============================================================================
// mysqldump module
// ============================================================================

#[test]
fn test_sql_statement_create_table() {
    let stmt = SqlStatement::CreateTable {
        name: "users".to_string(),
        columns: vec![ColumnDef {
            name: "id".to_string(),
            data_type: "INT".to_string(),
            nullable: false,
            primary_key: true,
            auto_increment: true,
            unique: true,
            default: None,
            references: None,
        }],
    };
    match stmt {
        SqlStatement::CreateTable { name, columns } => {
            assert_eq!(name, "users");
            assert_eq!(columns.len(), 1);
            assert_eq!(columns[0].name, "id");
        }
        _ => panic!("expected CreateTable"),
    }
}

#[test]
fn test_sql_statement_insert() {
    let stmt = SqlStatement::Insert {
        table: "t1".to_string(),
        columns: vec!["id".to_string(), "name".to_string()],
        values: vec![
            vec!["1".to_string(), "'alice'".to_string()],
            vec!["2".to_string(), "'bob'".to_string()],
        ],
    };
    match stmt {
        SqlStatement::Insert { table, columns, values } => {
            assert_eq!(table, "t1");
            assert_eq!(columns.len(), 2);
            assert_eq!(values.len(), 2);
        }
        _ => panic!("expected Insert"),
    }
}

#[test]
fn test_sql_statement_drop_table() {
    let stmt = SqlStatement::DropTable { name: "t1".to_string(), if_exists: true };
    match stmt {
        SqlStatement::DropTable { name, if_exists } => {
            assert_eq!(name, "t1");
            assert!(if_exists);
        }
        _ => panic!("expected DropTable"),
    }
}

#[test]
fn test_sql_statement_control() {
    assert!(matches!(SqlStatement::Begin, SqlStatement::Begin));
    assert!(matches!(SqlStatement::Commit, SqlStatement::Commit));
    assert!(matches!(SqlStatement::Rollback, SqlStatement::Rollback));
}

#[test]
fn test_sql_statement_unknown() {
    let stmt = SqlStatement::Unknown("ANALYZE TABLE t1".to_string());
    match stmt {
        SqlStatement::Unknown(sql) => assert!(sql.contains("ANALYZE")),
        _ => panic!("expected Unknown"),
    }
}

#[test]
fn test_column_def_fields() {
    let col = ColumnDef {
        name: "age".to_string(),
        data_type: "INT".to_string(),
        nullable: true,
        primary_key: false,
        auto_increment: false,
        unique: false,
        default: Some("0".to_string()),
        references: None,
    };
    assert_eq!(col.name, "age");
    assert!(col.nullable);
    assert!(!col.primary_key);
    assert_eq!(col.default, Some("0".to_string()));
}

#[test]
fn test_column_def_with_foreign_key() {
    let col = ColumnDef {
        name: "dept_id".to_string(),
        data_type: "INT".to_string(),
        nullable: false,
        primary_key: false,
        auto_increment: false,
        unique: false,
        default: None,
        references: Some(ForeignKeyRef {
            table: "departments".to_string(),
            column: "id".to_string(),
        }),
    };
    assert!(col.references.is_some());
    let fk = col.references.unwrap();
    assert_eq!(fk.table, "departments");
}

#[test]
fn test_foreign_key_ref_fields() {
    let fk = ForeignKeyRef {
        table: "departments".to_string(),
        column: "id".to_string(),
    };
    assert_eq!(fk.table, "departments");
    assert_eq!(fk.column, "id");
}

#[test]
fn test_import_mode_variants() {
    let modes = [
        ImportMode::Full,
        ImportMode::SchemaOnly,
        ImportMode::DataOnly,
        ImportMode::ContinueOnError,
    ];
    assert_eq!(modes.len(), 4);
}

#[test]
fn test_import_stats_fields() {
    let stats = ImportStats {
        tables_created: 5,
        tables_dropped: 0,
        rows_inserted: 5000,
        queries_executed: 100,
        errors: 2,
        warnings: vec![],
    };
    assert_eq!(stats.queries_executed, 100);
    assert_eq!(stats.errors, 2);
}

// ============================================================================
// upgrade module
// ============================================================================

#[test]
fn test_version_info_parse_valid() {
    let v = VersionInfo::parse("3.10.0").unwrap();
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 10);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_version_info_parse_another() {
    let v = VersionInfo::parse("4.2.1").unwrap();
    assert_eq!(v.major, 4);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 1);
}

#[test]
fn test_version_info_parse_invalid() {
    // Must be exactly X.Y.Z
    assert!(VersionInfo::parse("").is_err());
    assert!(VersionInfo::parse("abc").is_err());
    assert!(VersionInfo::parse("3.11").is_err()); // only 2 parts
    assert!(VersionInfo::parse("1.2.3.4").is_err()); // 4 parts
    assert!(VersionInfo::parse("abc.def.ghi").is_err());
}

#[test]
fn test_version_info_display() {
    let v = VersionInfo::parse("3.11.5").unwrap();
    assert_eq!(format!("{}", v), "3.11.5");
}

#[test]
fn test_version_info_comparison() {
    let v310 = VersionInfo::parse("3.10.0").unwrap();
    let v311 = VersionInfo::parse("3.11.0").unwrap();
    let v400 = VersionInfo::parse("4.0.0").unwrap();
    // Verify major/minor/patch fields are set correctly
    assert_eq!(v310.major, 3); assert_eq!(v310.minor, 10); assert_eq!(v310.patch, 0);
    assert_eq!(v311.major, 3); assert_eq!(v311.minor, 11); assert_eq!(v311.patch, 0);
    assert_eq!(v400.major, 4); assert_eq!(v400.minor, 0); assert_eq!(v400.patch, 0);
    // can_upgrade_to for ordering check
    assert!(v310.can_upgrade_to(&v311));
    assert!(!v311.can_upgrade_to(&v400));
}

#[test]
fn test_version_info_can_upgrade() {
    let v310 = VersionInfo::parse("3.10.0").unwrap();
    let v311 = VersionInfo::parse("3.11.0").unwrap();
    let v320 = VersionInfo::parse("3.20.0").unwrap();
    let v400 = VersionInfo::parse("4.0.0").unwrap();

    // Same major, higher minor: allowed
    assert!(v310.can_upgrade_to(&v311));
    assert!(v310.can_upgrade_to(&v320));
    // Same major, same minor, higher patch: allowed
    assert!(v311.can_upgrade_to(&VersionInfo::parse("3.11.5").unwrap()));
    // Different major: not allowed
    assert!(!v310.can_upgrade_to(&v400));
}

#[test]
fn test_upgrade_status_variants() {
    let statuses = [
        UpgradeStatus::Pending,
        UpgradeStatus::InProgress,
        UpgradeStatus::Completed,
        UpgradeStatus::Failed,
        UpgradeStatus::RolledBack,
    ];
    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_upgrade_plan_fields() {
    let plan = UpgradePlan {
        from_version: VersionInfo::parse("3.10.0").unwrap(),
        to_version: VersionInfo::parse("3.11.0").unwrap(),
        migration_steps: vec![],
        pre_check_passed: true,
        estimated_duration_secs: 60,
    };
    assert_eq!(plan.from_version.major, 3);
    assert_eq!(plan.to_version.minor, 11);
    assert!(plan.pre_check_passed);
}

#[test]
fn test_upgrade_manifest_fields() {
    let manifest = UpgradeManifest {
        from_version: "3.10.0".to_string(),
        to_version: "3.11.0".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        status: UpgradeStatus::Completed,
        backup_path: Some(std::path::PathBuf::from("/backup")),
        rollback_enabled: true,
        steps_completed: 3,
        total_steps: 3,
        checksum: "abc123".to_string(),
    };
    assert_eq!(manifest.from_version, "3.10.0");
    assert_eq!(manifest.to_version, "3.11.0");
    assert!(matches!(manifest.status, UpgradeStatus::Completed));
    assert!(manifest.rollback_enabled);
}

#[test]
fn test_create_upgrade_plan() {
    let from = VersionInfo::parse("3.10.0").unwrap();
    let to = VersionInfo::parse("3.11.0").unwrap();
    let plan = create_upgrade_plan(&from, &to).unwrap();
    assert_eq!(plan.from_version.major, 3);
    assert_eq!(plan.to_version.minor, 11);
}

#[test]
fn test_migration_step_fields() {
    use sqlrustgo_tools::upgrade::MigrationStep;
    let step = MigrationStep {
        id: 1,
        description: "backup".to_string(),
        executed: true,
        rollback_sql: Some("RESTORE".to_string()),
    };
    assert_eq!(step.id, 1);
    assert!(step.executed);
    assert_eq!(step.rollback_sql, Some("RESTORE".to_string()));
}
