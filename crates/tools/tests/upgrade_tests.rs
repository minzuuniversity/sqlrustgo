//! Additional upgrade tests for create_upgrade_plan

use sqlrustgo_tools::upgrade::{create_upgrade_plan, VersionInfo};

#[test]
fn test_create_upgrade_plan_minor() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 1, patch: 0 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    assert!(!plan.migration_steps.is_empty());
    assert!(plan.pre_check_passed);
    assert_eq!(plan.from_version.major, 1);
    assert_eq!(plan.to_version.minor, 1);
}

#[test]
fn test_create_upgrade_plan_patch() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 0, patch: 5 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    // When from.minor == to.minor, only 1 step
    assert!(!plan.migration_steps.is_empty());
}

#[test]
fn test_create_upgrade_plan_has_schema_migration_step() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 2, patch: 0 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let has_schema = plan.migration_steps.iter().any(|s| s.description.contains("Schema"));
    assert!(has_schema);
}

#[test]
fn test_create_upgrade_plan_has_metadata_step() {
    let from = VersionInfo { major: 2, minor: 0, patch: 0 };
    let to = VersionInfo { major: 2, minor: 0, patch: 1 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let has_metadata = plan.migration_steps.iter().any(|s| s.description.contains("metadata"));
    assert!(has_metadata);
}

#[test]
fn test_create_upgrade_plan_has_verify_step() {
    let from = VersionInfo { major: 3, minor: 5, patch: 0 };
    let to = VersionInfo { major: 3, minor: 6, patch: 0 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let has_verify = plan.migration_steps.iter().any(|s| s.description.contains("Verify"));
    assert!(has_verify);
}

#[test]
fn test_create_upgrade_plan_step_ids_unique() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 3, patch: 0 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let ids: Vec<usize> = plan.migration_steps.iter().map(|s| s.id).collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique_ids.len());
}

#[test]
fn test_create_upgrade_plan_all_steps_not_executed() {
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 1, patch: 0 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    for step in &plan.migration_steps {
        assert!(!step.executed, "Step {} should not be pre-executed", step.id);
    }
}

#[test]
fn test_create_upgrade_plan_multiple_patch_steps() {
    // When minor changes AND patch changes, there should be multiple steps
    let from = VersionInfo { major: 1, minor: 0, patch: 0 };
    let to = VersionInfo { major: 1, minor: 1, patch: 5 };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    assert!(plan.migration_steps.len() >= 2);
}

// ============ VersionInfo::parse Tests ============

#[test]
fn test_version_info_parse_valid() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let v = VersionInfo::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
}

#[test]
fn test_version_info_parse_with_v_prefix() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let v = VersionInfo::parse("v2.0.1").unwrap();
    assert_eq!(v.major, 2);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 1);
}

#[test]
fn test_version_info_parse_leading_whitespace() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    // Leading whitespace is NOT trimmed by current implementation
    assert!(VersionInfo::parse("  v3.1.4").is_err());
}
