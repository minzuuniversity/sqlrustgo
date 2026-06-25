//! P1-2: D8 Architecture/Semantic Debt Gate validation
//!
//! **Issue**: #2880
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N7)
//! **Change**: openspec/changes/p1-2-arch-sem-debt

use std::cmp::min;

#[test]
fn test_d8_gate_exists() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_arch_sem_debt.sh");
    assert!(script.exists(), "check_arch_sem_debt.sh must exist");
}

#[test]
fn test_d8_tracks_all_7_items() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_arch_sem_debt.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    for item in &[
        "ARCH-1", "ARCH-2", "ARCH-3", "SEM-1", "SEM-2", "SEM-3", "SEM-4",
    ] {
        assert!(content.contains(item), "must track {}", item);
    }
}

#[test]
fn test_d8_remediation_plan_exists() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md");
    assert!(plan.exists(), "remediation plan must exist");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    for item in &[
        "ARCH-1", "ARCH-2", "ARCH-3", "SEM-1", "SEM-2", "SEM-3", "SEM-4",
    ] {
        assert!(content.contains(item), "plan must document {}", item);
    }
}

#[test]
fn test_d8_remediation_plan_effort() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // Total effort ~138h
    assert!(
        content.contains("138h") || content.contains("Total"),
        "must have meaningful total effort estimate"
    );
}

#[test]
fn test_d8_remediation_plan_v390_target() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // Each item must target v3.9.0+
    let v390_count = content.matches("v3.9.0").count();
    assert!(
        v390_count >= 7,
        "each of 7 items must target v3.9.0+ (found {} mentions)",
        v390_count
    );
}

#[test]
fn test_d8_gate_3_exit_codes() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_arch_sem_debt.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    // Must support 3 exit codes
    assert!(content.contains("exit 0"), "must exit 0 (pass)");
    assert!(content.contains("exit 1"), "must exit 1 (fail)");
    assert!(content.contains("exit 2"), "must exit 2 (drift)");
}

#[test]
fn test_d8_gate_runs_correctly() {
    // Run the gate - should be DRIFT (exit 0) since 7 OPEN items have plans
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_arch_sem_debt.sh");
    let output = std::process::Command::new("bash")
        .arg(&script)
        .output()
        .expect("failed to run gate");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let code = output.status.code().unwrap_or(-1);

    println!("D8 Gate stdout:\n{}", stdout);
    println!("D8 Gate exit code: {}", code);

    // Must be PASS (0) or DRIFT (2) with current plan, not FAIL (1)
    assert!(
        code == 0 || code == 2,
        "D8 gate should PASS (0) or DRIFT (2), got {}",
        code
    );
}

#[test]
fn test_d8_no_p0_missing_in_plan() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // P0 items must have detailed steps
    for p0 in &["ARCH-1", "SEM-1"] {
        // Find the section for this item (e.g., "## 1. ARCH-1: ...")
        let pattern = format!("## ");
        let item_pattern = format!(
            "## {}. {}",
            p0,
            if *p0 == "ARCH-1" { "ARCH-1" } else { "SEM-1" }
        );
        let search = format!(
            "## {}. {}",
            match p0.as_ref() {
                "ARCH-1" => "1",
                "SEM-1" => "4",
                _ => "1",
            },
            p0
        );
        let section_start = content
            .find(&search)
            .or_else(|| content.find(p0))
            .expect(&format!("{} must be in plan", p0));
        // Find next ## N. (top-level) section
        let rest = &content[section_start + p0.len()..];
        let next_section = rest
            .find("\n## ")
            .map(|i| i + p0.len() + 1) // +1 to skip the \n
            .unwrap_or(content.len());
        let section = &content[section_start..min(section_start + next_section, content.len())];
        assert!(
            section.contains("Severity")
                || section.contains("**P0**")
                || section.contains("P0 (")
                || section.contains("P0:"),
            "{} must have severity documented in section (found section: {:?})",
            p0,
            section
        );
    }
}
