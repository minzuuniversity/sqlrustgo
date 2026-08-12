//! Additional upgrade tests for create_upgrade_plan

use sqlrustgo_tools::upgrade::{create_upgrade_plan, VersionInfo};

#[test]
fn test_create_upgrade_plan_minor() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    assert!(!plan.migration_steps.is_empty());
    assert!(plan.pre_check_passed);
    assert_eq!(plan.from_version.major, 1);
    assert_eq!(plan.to_version.minor, 1);
}

#[test]
fn test_create_upgrade_plan_patch() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 0,
        patch: 5,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    // When from.minor == to.minor, only 1 step
    assert!(!plan.migration_steps.is_empty());
}

#[test]
fn test_create_upgrade_plan_has_schema_migration_step() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 2,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let has_schema = plan
        .migration_steps
        .iter()
        .any(|s| s.description.contains("Schema"));
    assert!(has_schema);
}

#[test]
fn test_create_upgrade_plan_has_metadata_step() {
    let from = VersionInfo {
        major: 2,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 2,
        minor: 0,
        patch: 1,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let has_metadata = plan
        .migration_steps
        .iter()
        .any(|s| s.description.contains("metadata"));
    assert!(has_metadata);
}

#[test]
fn test_create_upgrade_plan_has_verify_step() {
    let from = VersionInfo {
        major: 3,
        minor: 5,
        patch: 0,
    };
    let to = VersionInfo {
        major: 3,
        minor: 6,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let has_verify = plan
        .migration_steps
        .iter()
        .any(|s| s.description.contains("Verify"));
    assert!(has_verify);
}

#[test]
fn test_create_upgrade_plan_step_ids_unique() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 3,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    let ids: Vec<usize> = plan.migration_steps.iter().map(|s| s.id).collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique_ids.len());
}

#[test]
fn test_create_upgrade_plan_all_steps_not_executed() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    for step in &plan.migration_steps {
        assert!(
            !step.executed,
            "Step {} should not be pre-executed",
            step.id
        );
    }
}

#[test]
fn test_create_upgrade_plan_multiple_patch_steps() {
    // When minor changes AND patch changes, there should be multiple steps
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 5,
    };
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

#[test]
fn test_version_info_can_upgrade_same_major_minor() {
    let from = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 5,
    };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_version_info_can_upgrade_same_major_new_minor() {
    let from = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 2,
        patch: 0,
    };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_version_info_can_upgrade_false_different_major() {
    let from = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let to = VersionInfo {
        major: 2,
        minor: 0,
        patch: 0,
    };
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_version_info_can_upgrade_false_older() {
    let from = VersionInfo {
        major: 1,
        minor: 2,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    assert!(!from.can_upgrade_to(&to));
}

#[test]
fn test_upgrade_plan_struct() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    assert!(plan.pre_check_passed);
    assert_eq!(plan.from_version.major, 1);
    assert_eq!(plan.to_version.minor, 1);
}

#[test]
fn test_upgrade_plan_no_steps_executed() {
    let from = VersionInfo {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let to = VersionInfo {
        major: 1,
        minor: 1,
        patch: 0,
    };
    let plan = create_upgrade_plan(&from, &to).unwrap();
    assert!(plan.migration_steps.iter().all(|s| !s.executed));
}

// ============================================================================
// Additional VersionInfo + can_upgrade_to tests (Issue #3943)
// ============================================================================

#[test]
fn test_version_info_parse_basic() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let v = VersionInfo::parse("3.12.0").expect("parse 3.12.0");
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 12);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_version_info_parse_v_prefix_v3() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let v = VersionInfo::parse("v2.5.7").expect("parse v2.5.7");
    assert_eq!(v.major, 2);
    assert_eq!(v.minor, 5);
    assert_eq!(v.patch, 7);
}

#[test]
fn test_version_info_parse_invalid_format() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    assert!(VersionInfo::parse("1.2").is_err(), "two-part must fail");
    assert!(VersionInfo::parse("1.2.3.4").is_err(), "four-part must fail");
}

#[test]
fn test_version_info_parse_invalid_number() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    assert!(VersionInfo::parse("a.b.c").is_err(), "non-numeric parts must fail");
    assert!(VersionInfo::parse("1.b.3").is_err(), "non-numeric minor must fail");
}

#[test]
fn test_can_upgrade_to_minor_bump() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let from = VersionInfo { major: 3, minor: 11, patch: 0 };
    let to = VersionInfo { major: 3, minor: 12, patch: 0 };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_to_patch_bump_same_minor() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let from = VersionInfo { major: 3, minor: 12, patch: 0 };
    let to = VersionInfo { major: 3, minor: 12, patch: 5 };
    assert!(from.can_upgrade_to(&to));
}

#[test]
fn test_can_upgrade_to_rejects_downgrade() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let from = VersionInfo { major: 3, minor: 12, patch: 0 };
    let to = VersionInfo { major: 3, minor: 11, patch: 5 };
    assert!(!from.can_upgrade_to(&to), "downgrade must be rejected");
    let to_same = VersionInfo { major: 3, minor: 12, patch: 0 };
    assert!(!from.can_upgrade_to(&to_same), "same version must be rejected");
}

#[test]
fn test_can_upgrade_to_rejects_major_bump() {
    use sqlrustgo_tools::upgrade::VersionInfo;
    let from = VersionInfo { major: 3, minor: 12, patch: 0 };
    let to = VersionInfo { major: 4, minor: 0, patch: 0 };
    assert!(!from.can_upgrade_to(&to), "major version bump must be rejected");
    let to_major_downgrade = VersionInfo { major: 2, minor: 5, patch: 0 };
    assert!(!from.can_upgrade_to(&to_major_downgrade), "major version downgrade must be rejected");
}
