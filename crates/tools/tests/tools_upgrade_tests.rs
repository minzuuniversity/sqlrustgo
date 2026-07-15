// Tools crate upgrade.rs coverage tests

use sqlrustgo_tools::upgrade::{
    MigrationStep, UpgradeManifest, UpgradePlan, UpgradeStatus, VersionInfo,
};

// ============ VersionInfo tests ============

#[test]
fn test_version_info_parse_valid() {
    let v = VersionInfo::parse("3.11.0").unwrap();
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 11);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_version_info_parse_with_v_prefix() {
    let v = VersionInfo::parse("v3.10.5").unwrap();
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 10);
    assert_eq!(v.patch, 5);
}

#[test]
fn test_version_info_parse_invalid_format() {
    let r = VersionInfo::parse("3.11");
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("Invalid version format"));
}

#[test]
fn test_version_info_parse_invalid_major() {
    let r = VersionInfo::parse("abc.11.0");
    assert!(r.is_err());
}

#[test]
fn test_version_info_parse_invalid_minor() {
    let r = VersionInfo::parse("3.xyz.0");
    assert!(r.is_err());
}

#[test]
fn test_version_info_parse_too_many_parts() {
    let r = VersionInfo::parse("3.11.0.1");
    assert!(r.is_err());
}

#[test]
fn test_version_info_display() {
    let v = VersionInfo::parse("3.11.0").unwrap();
    assert_eq!(format!("{}", v), "3.11.0");
}

#[test]
fn test_version_info_display_v_prefix() {
    let v = VersionInfo::parse("v1.2.3").unwrap();
    assert_eq!(format!("{}", v), "1.2.3");
}

// ============ VersionInfo::can_upgrade_to tests ============

#[test]
fn test_can_upgrade_patch() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("3.11.5").unwrap();
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_minor() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("3.12.0").unwrap();
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_skip_minor() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("3.13.0").unwrap();
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_same_version() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("3.11.0").unwrap();
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_cannot_upgrade_major() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("4.0.0").unwrap();
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_cannot_downgrade() {
    let from = VersionInfo::parse("3.11.5").unwrap();
    let to = VersionInfo::parse("3.11.0").unwrap();
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_multiple_patches() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("3.11.99").unwrap();
    assert!(from.can_upgrade_to(&to));
}

// ============ UpgradeStatus tests ============

#[test]
fn test_upgrade_status_serde_lowercase() {
    let status = UpgradeStatus::Completed;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"completed\"");
}

#[test]
fn test_upgrade_status_all_variants() {
    for s in [
        UpgradeStatus::Pending,
        UpgradeStatus::InProgress,
        UpgradeStatus::Completed,
        UpgradeStatus::Failed,
        UpgradeStatus::RolledBack,
    ] {
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
fn test_upgrade_status_debug() {
    let s = UpgradeStatus::Failed;
    assert!(format!("{:?}", s).contains("Failed"));
}

#[test]
fn test_upgrade_status_clone() {
    let s = UpgradeStatus::InProgress;
    let c = s.clone();
    assert_eq!(c, s);
}

// ============ VersionInfo serde tests ============

#[test]
fn test_version_info_serde() {
    let v = VersionInfo::parse("3.11.0").unwrap();
    let json = serde_json::to_string(&v).unwrap();
    let parsed: VersionInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.major, v.major);
    assert_eq!(parsed.minor, v.minor);
    assert_eq!(parsed.patch, v.patch);
}

// ============ MigrationStep tests ============

#[test]
fn test_migration_step_new() {
    let step = MigrationStep {
        id: 1,
        description: "Migrate users table".to_string(),
        executed: false,
        rollback_sql: Some("DROP TABLE users".to_string()),
    };
    assert_eq!(step.id, 1);
    assert!(!step.executed);
    assert!(step.rollback_sql.is_some());
}

#[test]
fn test_migration_step_no_rollback() {
    let step = MigrationStep {
        id: 2,
        description: "Read-only view".to_string(),
        executed: false,
        rollback_sql: None,
    };
    assert!(step.rollback_sql.is_none());
}

// ============ UpgradePlan tests ============

#[test]
fn test_upgrade_plan_new() {
    let from = VersionInfo::parse("3.11.0").unwrap();
    let to = VersionInfo::parse("3.12.0").unwrap();
    let steps = vec![MigrationStep {
        id: 1,
        description: "Step 1".to_string(),
        executed: false,
        rollback_sql: None,
    }];
    let plan = UpgradePlan {
        from_version: from.clone(),
        to_version: to.clone(),
        migration_steps: steps,
        pre_check_passed: true,
        estimated_duration_secs: 60,
    };
    assert_eq!(plan.from_version.major, 3);
    assert_eq!(plan.to_version.minor, 12);
    assert!(plan.pre_check_passed);
    assert_eq!(plan.migration_steps.len(), 1);
}

// ============ UpgradeManifest serde tests ============

#[test]
fn test_upgrade_manifest_serde() {
    let manifest = UpgradeManifest {
        from_version: "3.11.0".to_string(),
        to_version: "3.12.0".to_string(),
        timestamp: "1234567890".to_string(),
        status: UpgradeStatus::Completed,
        backup_path: None,
        rollback_enabled: true,
        steps_completed: 4,
        total_steps: 4,
        checksum: "abc123".to_string(),
    };
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("\"completed\""));
    assert!(json.contains("\"3.11.0\""));
}

#[test]
fn test_upgrade_manifest_with_backup_path() {
    let manifest = UpgradeManifest {
        from_version: "3.11.0".to_string(),
        to_version: "3.12.0".to_string(),
        timestamp: "1234567890".to_string(),
        status: UpgradeStatus::InProgress,
        backup_path: Some(std::path::PathBuf::from("/backups/bkp_001")),
        rollback_enabled: true,
        steps_completed: 2,
        total_steps: 4,
        checksum: "xyz".to_string(),
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed: UpgradeManifest = serde_json::from_str(&json).unwrap();
    assert!(parsed.backup_path.is_some());
}

// ============ UpgradeCommand type check ============

#[test]
fn test_upgrade_command_type_exists() {
    use sqlrustgo_tools::upgrade::UpgradeCommand;
    let _: Option<UpgradeCommand> = None;
}

// ============ Integration: VersionInfo upgrade path ============

#[test]
fn test_version_info_chain_valid_upgrades() {
    let current = VersionInfo::parse("3.10.0").unwrap();
    assert!(current.can_upgrade_to(&VersionInfo::parse("3.11.0").unwrap()));
    assert!(VersionInfo::parse("3.11.0").unwrap().can_upgrade_to(&VersionInfo::parse("3.11.5").unwrap()));
    assert!(VersionInfo::parse("3.11.5").unwrap().can_upgrade_to(&VersionInfo::parse("3.12.0").unwrap()));
}

#[test]
fn test_version_info_edge_zero() {
    let v0 = VersionInfo { major: 0, minor: 0, patch: 0 };
    let v1 = VersionInfo { major: 0, minor: 0, patch: 1 };
    assert!(v0.can_upgrade_to(&v1));
    assert!(!v0.can_upgrade_to(&v0));
}
