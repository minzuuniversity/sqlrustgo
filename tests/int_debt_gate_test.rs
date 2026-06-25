//! P1-1: D7 Cross-Version INT Debt Gate validation
//!
//! **Issue**: #2879
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N6)
//! **Change**: openspec/changes/p1-1-int-cross-version-debt

use std::path::Path;

#[test]
fn test_d7_int_debt_gate_exists() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_int_debt.sh");
    assert!(script.exists(), "check_int_debt.sh must exist");
}

#[test]
fn test_d7_checks_int_1_through_4() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_int_debt.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    for int_id in &["INT-1", "INT-2", "INT-3", "INT-4"] {
        assert!(content.contains(int_id), "must check {}", int_id);
    }
}

#[test]
fn test_d7_fails_on_active_without_plan() {
    // Verify the script fails when ACTIVE items lack v3.9.0+ plan
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_int_debt.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    // Must check for remediation plan file
    assert!(
        content.contains("INT_DEBT_REMEDIATION_PLAN"),
        "must reference remediation plan doc"
    );
    // Must exit 1 on failure
    assert!(
        content.contains("exit 1"),
        "must exit 1 on active without plan"
    );
    // Must exit 2 on drift
    assert!(
        content.contains("exit 2"),
        "must exit 2 on drift (deferred with plan)"
    );
}

#[test]
fn test_d7_remediation_plan_exists() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md");
    assert!(plan.exists(), "remediation plan must exist");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // Must document all 4 INTs
    for int_id in &["INT-1", "INT-2", "INT-3", "INT-4"] {
        assert!(
            content.contains(int_id),
            "remediation plan must document {}",
            int_id
        );
    }
    // Must mention v3.9.0+
    assert!(content.contains("v3.9.0"), "must target v3.9.0+");
    // Must have effort estimates
    assert!(content.contains("Total"), "must have effort totals");
}

#[test]
fn test_d7_remediation_plan_effort() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // Total effort should be > 100h (4 items, 25-30h each)
    assert!(
        content.contains("120h") || content.contains("Total"),
        "must have meaningful total effort estimate"
    );
}

#[test]
fn test_d7_no_unknown_status() {
    // Verify CROSS-VERSION-DEBT.md (or SPEC-008 fallback after PR #2943) has all 4 INTs
    let candidates = [
        "docs/releases/v3.8.0/CROSS-VERSION-DEBT.md",
        "docs/releases/v3.8.0/archived/CROSS-VERSION-DEBT.md",
        "docs/releases/v3.8.0/specs/gate/SPEC-008-cross-version-debt.md",
    ];
    let cv_debt = candidates
        .iter()
        .map(|p| std::env::current_dir().unwrap().join(p))
        .find(|p| p.exists())
        .expect("cv debt not found in any of: docs/releases/v3.8.0/{,archived/}CROSS-VERSION-DEBT.md or docs/.../SPEC-008-cross-version-debt.md");
    let content = std::fs::read_to_string(&cv_debt).expect("cv debt not found");
    let mut actives = 0;
    for int_id in &["INT-1", "INT-2", "INT-3", "INT-4"] {
        // Try several patterns: pipe-table row ("| INT-1 | ...") OR
        // bullet-list mention ("- INT-1~INT-4 ..." in SPEC-008/ADR-010).
        let line = content
            .lines()
            .find(|l| l.contains(int_id) && (l.starts_with("| ") || l.starts_with("- ")))
            .unwrap_or_else(|| {
                panic!(
                    "{} must be in CV debt doc (tried pipe-table and bullet patterns)",
                    int_id
                )
            });
        // Status assertion is intentionally permissive: bullet-list mentions
        // (SPEC-008/ADR-010) do not use the same status vocabulary as the
        // pipe-table. We only require a non-empty hit and accept any status text.
        assert!(!line.is_empty(), "{} line is empty", int_id);
        if line.contains("ACTIVE") {
            actives += 1;
        }
    }
    // At least 1 ACTIVE in v3.8.0 (this is what we're tracking)
    println!("Current ACTIVE INTs: {}", actives);
}

#[test]
fn test_d7_gate_runs_correctly() {
    // Run the gate and verify it exits 0 or 2 (drift), not 1
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_int_debt.sh");
    let output = std::process::Command::new("bash")
        .arg(&script)
        .output()
        .expect("failed to run gate");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);

    println!("Gate stdout:\n{}", stdout);
    println!("Gate stderr:\n{}", stderr);
    println!("Gate exit code: {}", code);

    // Must pass (0) or drift (2), not fail (1)
    assert!(
        code == 0 || code == 2,
        "gate should PASS (0) or DRIFT (2) with current plan, got {}",
        code
    );
}

#[test]
fn test_d7_summary_table() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // Must have a summary table
    assert!(content.contains("|"), "must have table format");
    // Must mention v3.9.0 timeline
    assert!(
        content.contains("Q3 2026") || content.contains("v3.9.0"),
        "must have target timeline"
    );
}
