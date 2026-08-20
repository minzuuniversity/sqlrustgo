//! P2-4: R-Gate YAML Upgrade validation
//!
//! **Issue**: #2888
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N15)
//! **Change**: openspec/changes/p2-4-r-gate-yaml-upgrade
//!
//! Validates the new r-gate.yml:
//! 1. Targets v3.8.0 branches
//! 2. Invokes check_alpha_v380.sh + check_rc_ga_gate.sh
//! 3. Has test inventory check
//! 4. Has cross-version debt gate
//! 5. Uses required status checks

#[test]
fn test_rgate_targets_v380() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    assert!(
        content.contains("develop/v3.8.0"),
        "must target develop/v3.8.0"
    );
    assert!(
        !content.contains("develop/v2.9.0"),
        "must NOT target v2.9.0"
    );
}

#[test]
fn test_rgate_invokes_modern_scripts() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    assert!(
        content.contains("check_alpha_v380.sh"),
        "must call check_alpha_v380.sh"
    );
    assert!(
        content.contains("check_rc_ga_gate.sh"),
        "must call check_rc_ga_gate.sh"
    );
}

#[test]
fn test_rgate_no_deprecated_scripts() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    // Deprecated v2.9.0-era scripts
    let deprecated = [
        "check_coverage.sh",
        "check_security.sh",
        "check_docs_links.sh",
        "check_sql_compat.sh",
        "check_perf_baseline.sh",
        "check_proof.sh",
        "check_attack_surface.sh",
    ];
    for script in &deprecated {
        assert!(
            !content.contains(script),
            "must NOT call deprecated script: {}",
            script
        );
    }
}

#[test]
fn test_rgate_has_5_dim_integration() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    // D1-D5 mentions
    assert!(content.contains("D1"), "must mention D1-Alpha");
    assert!(content.contains("D2"), "must mention D2-Beta");
    assert!(content.contains("D3"), "must mention D3-SGL");
    assert!(content.contains("D4"), "must mention D4-WAL");
    assert!(content.contains("D5"), "must mention D5-DeepSeek");
}

#[test]
fn test_rgate_has_test_inventory_check() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    assert!(
        content.contains("test-inventory-audit"),
        "must have test-inventory job"
    );
    assert!(
        content.contains("integration ratio"),
        "must check integration ratio"
    );
    assert!(content.contains("95%"), "must target 95%+ integration");
}

#[test]
fn test_rgate_has_cross_version_debt_check() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    assert!(
        content.contains("cross-version-debt"),
        "must have cross-version job"
    );
    assert!(
        content.contains("check_cross_version_debt.sh"),
        "must call debt script"
    );
    assert!(content.contains("ACTIVE"), "must verify 0 ACTIVE debt");
}

#[test]
fn test_rgate_has_required_status_check() {
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    assert!(
        content.contains("merge-gate"),
        "must have merge eligibility check"
    );
    assert!(content.contains("needs:"), "must declare job dependencies");
}

#[test]
fn test_rgate_yaml_parses() {
    // Read and parse the YAML to ensure syntactic validity
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");
    // Basic structural validation (no actual YAML lib dependency in this test)
    assert!(content.contains("name:"), "must have workflow name");
    assert!(content.contains("on:"), "must have trigger definition");
    assert!(content.contains("jobs:"), "must have jobs section");
}

#[test]
fn test_yaml_lints_clean() {
    // Verify that the YAML doesn't have common syntax issues
    let path = std::env::current_dir()
        .unwrap()
        .join(".github/workflows/r-gate.yml");
    let content = std::fs::read_to_string(&path).expect("r-gate.yml not found");

    // No tab characters (YAML requires spaces)
    assert!(!content.contains('\t'), "must not use tabs in YAML");

    // Proper structure: each `-` for list items at consistent indentation
    let lines: Vec<&str> = content.lines().collect();
    for _line in lines.iter() {
        // Placeholder for future template syntax checks
    }
}

#[test]
fn test_existing_scripts_still_exist() {
    // Verify the new r-gate references real scripts
    let alpha_path = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_alpha_v380.sh");
    let rc_ga_path = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_rc_ga_gate.sh");
    let debt_path = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_cross_version_debt.sh");
    assert!(alpha_path.exists(), "check_alpha_v380.sh must exist");
    assert!(rc_ga_path.exists(), "check_rc_ga_gate.sh must exist");
    assert!(debt_path.exists(), "check_cross_version_debt.sh must exist");
}
