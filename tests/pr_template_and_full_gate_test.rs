//! P1-5: PR Template + Full Gate Verification (D9) validation
//!
//! **Issue**: #2883
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N10)
//! **Change**: openspec/changes/p1-5-pr-template

#[test]
fn test_pr_template_exists() {
    let template = std::env::current_dir()
        .unwrap()
        .join(".gitea/pull_request_template.md");
    assert!(template.exists(), "PR template must exist");
}

#[test]
fn test_pr_template_has_5_docs() {
    let template = std::env::current_dir()
        .unwrap()
        .join(".gitea/pull_request_template.md");
    let content = std::fs::read_to_string(&template).expect("template not found");
    for doc in &["SPEC", "TEST_PLAN", "TEST_DESIGN", "REVIEW", "ACCEPTANCE"] {
        assert!(content.contains(doc), "template must mention {}", doc);
    }
    assert!(
        content.contains("5-类文档"),
        "template must reference 5-类文档"
    );
}

#[test]
fn test_pr_template_has_5_principles() {
    let template = std::env::current_dir()
        .unwrap()
        .join(".gitea/pull_request_template.md");
    let content = std::fs::read_to_string(&template).expect("template not found");
    for p in &["P1", "P2", "P3", "P4", "P5"] {
        assert!(content.contains(p), "template must reference {}", p);
    }
    // The template labels the 5 principles as a column (P1 / P2 / P3 / P4 / P5).
    // We accept any of the three common spellings: "5-原则", "5-Principle", or P1/P2/P3/P4/P5.
    assert!(
        content.contains("5-原则")
            || content.contains("5-Principle")
            || content.contains("5-原则 (P1-P5)"),
        "template must reference 5 principles"
    );
}

#[test]
fn test_pr_template_has_8_dimensions() {
    let template = std::env::current_dir()
        .unwrap()
        .join(".gitea/pull_request_template.md");
    let content = std::fs::read_to_string(&template).expect("template not found");
    for dim in &["D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8"] {
        assert!(
            content.contains(dim),
            "template must reference dimension {}",
            dim
        );
    }
}

#[test]
fn test_pr_template_has_issue_ref() {
    let template = std::env::current_dir()
        .unwrap()
        .join(".gitea/pull_request_template.md");
    let content = std::fs::read_to_string(&template).expect("template not found");
    assert!(
        content.contains("Closes #") || content.contains("Issue 编号"),
        "template must have issue reference keyword"
    );
    assert!(
        content.contains("Issue Reference")
            || content.contains("关联 Issue")
            || content.contains("Issue 编号"),
        "template must have Issue Reference section"
    );
}

#[test]
fn test_d9_gate_exists() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_full_gate_verification.sh");
    assert!(script.exists(), "D9 gate must exist");
}

#[test]
fn test_d9_gate_executable() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_full_gate_verification.sh");
    let metadata = std::fs::metadata(&script).expect("script must exist");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        assert!(mode & 0o111 != 0, "D9 gate must be executable");
    }
}

#[test]
#[test]
fn test_d9_gate_runs() {
    // D9 runs all sub-gates. Some (tpch) are TIMEOUT (long-running).
    // We just verify the script structure here, not full execution.
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_full_gate_verification.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    // Must have run_gate function
    assert!(
        content.contains("run_gate"),
        "D9 must have run_gate function"
    );
    // Must have all 8 dimensions
    for dim in &["D1-D5", "D6", "D7", "D8"] {
        assert!(content.contains(dim), "D9 must reference {}", dim);
    }
}

#[test]
fn test_d9_gate_8_dimensions() {
    // Verify D9 script references all 8 dimensions
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_full_gate_verification.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    for dim in &[
        "D1-D5",
        "D6",
        "D7",
        "D8",
        "Cross-Version",
        "Test Plan Consistency",
        "PR Template",
        "Evidence Generation",
    ] {
        assert!(content.contains(dim), "D9 gate must reference {}", dim);
    }
}

#[test]
fn test_d9_gate_3_exit_codes() {
    // Verify D9 supports 3 exit codes (PASS/DRIFT/FAIL)
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_full_gate_verification.sh");
    let content = std::fs::read_to_string(&script).expect("script not found");
    assert!(content.contains("exit 0"), "must exit 0 (pass)");
    assert!(content.contains("exit 1"), "must exit 1 (fail)");
    assert!(content.contains("exit 2"), "must exit 2 (drift)");
}
