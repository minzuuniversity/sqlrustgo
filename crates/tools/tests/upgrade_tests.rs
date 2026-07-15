//! Upgrade module tests
//!
//! Tests: VersionInfo parsing, can_upgrade_to, UpgradePlan, MigrationStep

use sqlrustgo_tools::upgrade::{
    MigrationStep, UpgradePlan, VersionInfo,
};

#[test]
fn test_version_info_parse_valid() {
    let v = VersionInfo::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
}

#[test]
fn test_version_info_parse_with_v_prefix() {
    let v = VersionInfo::parse("v2.0.0").unwrap();
    assert_eq!(v.major, 2);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_version_info_parse_invalid_format() {
    let r = VersionInfo::parse("1.2");
    assert!(r.is_err());
}

#[test]
fn test_version_info_parse_invalid_major() {
    let r = VersionInfo::parse("abc.2.3");
    assert!(r.is_err());
}

#[test]
fn test_version_info_display() {
    let v = VersionInfo { major: 3, minor: 10, patch: 5 };
    let s = format!("{}", v);
    assert_eq!(s, "3.10.5");
}

#[test]
fn test_can_upgrade_same_minor() {
    let from = VersionInfo { major: 1, minor: 2, patch: 0 };
    let to = VersionInfo { major: 1, minor: 2, patch: 5 };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_next_minor() {
    let from = VersionInfo { major: 1, minor: 2, patch: 0 };
    let to = VersionInfo { major: 1, minor: 3, patch: 0 };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_next_patch() {
    let from = VersionInfo { major: 2, minor: 0, patch: 0 };
    let to = VersionInfo { major: 2, minor: 0, patch: 1 };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_cannot_upgrade_cross_major() {
    let from = VersionInfo { major: 1, minor: 2, patch: 0 };
    let to = VersionInfo { major: 2, minor: 0, patch: 0 };
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_cannot_upgrade_downgrade() {
    let from = VersionInfo { major: 1, minor: 5, patch: 0 };
    let to = VersionInfo { major: 1, minor: 4, patch: 0 };
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_cannot_upgrade_same_version() {
    let from = VersionInfo { major: 3, minor: 10, patch: 0 };
    let to = VersionInfo { major: 3, minor: 10, patch: 0 };
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_migration_step_new() {
    let step = MigrationStep {
        id: 1,
        description: "Migrate schema".to_string(),
        executed: false,
        rollback_sql: Some("DROP TABLE t".to_string()),
    };
    assert_eq!(step.id, 1);
    assert!(!step.executed);
    assert!(step.rollback_sql.is_some());
}

#[test]
fn test_migration_step_executed() {
    let step = MigrationStep {
        id: 2,
        description: "Copy data".to_string(),
        executed: true,
        rollback_sql: None,
    };
    assert!(step.executed);
}

#[test]
fn test_migration_step_clone() {
    let step = MigrationStep {
        id: 3,
        description: "Verify".to_string(),
        executed: false,
        rollback_sql: None,
    };
    let c = step.clone();
    assert_eq!(c.id, 3);
    assert_eq!(c.description, "Verify");
}

#[test]
fn test_migration_step_debug() {
    let step = MigrationStep {
        id: 5,
        description: "test step".to_string(),
        executed: true,
        rollback_sql: None,
    };
    let debug = format!("{:?}", step);
    assert!(debug.contains("5"));
    assert!(debug.contains("test step"));
}

#[test]
fn test_upgrade_plan_new() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 1, patch: 0 };
    let steps = vec![
        MigrationStep {
            id: 1,
            description: "Step 1".to_string(),
            executed: false,
            rollback_sql: None,
        },
        MigrationStep {
            id: 2,
            description: "Step 2".to_string(),
            executed: false,
            rollback_sql: Some("undo".to_string()),
        },
    ];
    let plan = UpgradePlan {
        from_version: from.clone(),
        to_version: to.clone(),
        migration_steps: steps,
        pre_check_passed: true,
        estimated_duration_secs: 300,
    };
    assert_eq!(plan.migration_steps.len(), 2);
    assert!(plan.pre_check_passed);
    assert_eq!(plan.estimated_duration_secs, 300);
}

#[test]
fn test_upgrade_plan_empty_steps() {
    let from = VersionInfo { major: 2, minor: 0, patch: 0 };
    let to = VersionInfo { major: 2, minor: 0, patch: 1 };
    let plan = UpgradePlan {
        from_version: from,
        to_version: to,
        migration_steps: vec![],
        pre_check_passed: false,
        estimated_duration_secs: 60,
    };
    assert!(plan.migration_steps.is_empty());
    assert!(!plan.pre_check_passed);
}

#[test]
fn test_upgrade_plan_debug() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 1, patch: 0 };
    let plan = UpgradePlan {
        from_version: from,
        to_version: to,
        migration_steps: vec![],
        pre_check_passed: true,
        estimated_duration_secs: 60,
    };
    let debug = format!("{:?}", plan);
    assert!(debug.contains("UpgradePlan"));
}

#[test]
fn test_version_info_ordering() {
    // Test that display produces correct output for ordering
    let versions = vec![
        VersionInfo { major: 3, minor: 9, patch: 0 },
        VersionInfo { major: 3, minor: 10, patch: 0 },
        VersionInfo { major: 3, minor: 9, patch: 1 },
        VersionInfo { major: 4, minor: 0, patch: 0 },
    ];
    let strings: Vec<String> = versions.iter().map(|v| format!("{}", v)).collect();
    assert_eq!(strings[0], "3.9.0");
    assert_eq!(strings[1], "3.10.0");
    assert_eq!(strings[2], "3.9.1");
    assert_eq!(strings[3], "4.0.0");
}

#[test]
fn test_can_upgrade_all_patches_between_minors() {
    // 1.0.0 can upgrade to 1.1.0 (all patches in between are covered)
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 1, patch: 0 };
    assert!(from.can_upgrade_to(&to));

    let from2 = VersionInfo { major: 1, minor: 0, patch: 9 };
    assert!(from2.can_upgrade_to(&to));
}
