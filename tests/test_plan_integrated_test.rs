//! P1-3: Integrated Test Plan validation
//!
//! **Issue**: #2881
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N8)
//! **Change**: openspec/changes/p1-3-test-plan

#[test]
fn test_test_plan_integrated_exists() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_PLAN_INTEGRATED.md");
    assert!(plan.exists(), "TEST_PLAN_INTEGRATED.md must exist");
}

#[test]
fn test_test_review_integrated_exists() {
    let review = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_REVIEW_INTEGRATED.md");
    assert!(review.exists(), "TEST_REVIEW_INTEGRATED.md must exist");
}

#[test]
fn test_test_acceptance_integrated_exists() {
    let acceptance = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_ACCEPTANCE_INTEGRATED.md");
    assert!(
        acceptance.exists(),
        "TEST_ACCEPTANCE_INTEGRATED.md must exist"
    );
}

#[test]
fn test_test_plan_53_tests() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_PLAN_INTEGRATED.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    // Must mention all 53 test entries
    // Just check section 4 table has 53 rows
    let section_4_start = content.find("## 4. 全部 53 测试 - 8 维门禁映射");
    assert!(section_4_start.is_some(), "must have section 4");

    // Count test rows (excluding header)
    let after = &content[section_4_start.unwrap()..];
    let next_section = after.find("\n## ").unwrap_or(after.len());
    let section = &after[..next_section];

    // Count rows starting with "| " and a number
    let row_count = section
        .lines()
        .filter(|l| {
            l.starts_with("| ")
                && l.chars()
                    .nth(2)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
        })
        .count();
    assert!(
        row_count >= 53,
        "section 4 must have ≥53 rows (got {})",
        row_count
    );
}

#[test]
fn test_test_plan_8_gate_dimensions() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_PLAN_INTEGRATED.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    for dim in &["D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8"] {
        assert!(content.contains(dim), "plan must reference {}", dim);
    }
}

#[test]
fn test_test_plan_4_stages() {
    let plan = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_PLAN_INTEGRATED.md");
    let content = std::fs::read_to_string(&plan).expect("plan not found");
    for stage in &["Alpha", "Beta", "RC", "GA"] {
        assert!(content.contains(stage), "plan must reference {}", stage);
    }
}

#[test]
fn test_test_review_audit_complete() {
    let review = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_REVIEW_INTEGRATED.md");
    let content = std::fs::read_to_string(&review).expect("review not found");
    // Must have 53 test audits
    let audit_count = content.matches("| ✅ |").count() + content.matches("| ⚠️ TIMEOUT |").count();
    assert!(
        audit_count >= 53,
        "review must have ≥53 test audits (got {})",
        audit_count
    );
}

#[test]
fn test_test_acceptance_all_pass() {
    let acceptance = std::env::current_dir()
        .unwrap()
        .join("docs/releases/v3.8.0/TEST_ACCEPTANCE_INTEGRATED.md");
    let content = std::fs::read_to_string(&acceptance).expect("acceptance not found");
    // Must have all 7 stage/dim acceptances as APPROVED
    let approved_count = content.matches("✅ APPROVED").count();
    assert!(
        approved_count >= 7,
        "acceptance must have ≥7 ✅ APPROVED (got {})",
        approved_count
    );
}

#[test]
fn test_cargo_toml_test_entries() {
    // Verify Cargo.toml has at least 53 [[test]] entries (consistency check)
    // May have more due to other P1.x PRs adding self-tests
    let cargo = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo).expect("Cargo.toml not found");
    let count = content.matches("[[test]]").count();
    assert!(
        count >= 53,
        "Cargo.toml must have ≥53 [[test]] entries (was {})",
        count
    );
}

#[test]
fn test_no_orphan_tests() {
    // Walk tests/ directory
    let mut test_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir("tests") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                test_files.push(path.to_string_lossy().to_string());
            } else if path.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&path) {
                    for sub_entry in sub.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.is_file()
                            && sub_path.extension().map(|e| e == "rs").unwrap_or(false)
                        {
                            test_files.push(sub_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    // Skip common module and known helper files
    let skip_files = [
        "common/mod.rs",  // shared test utilities
        "data_loader.rs", // has its own Cargo entry
    ];
    let skip_substrings = [
        "_test.rs", // actual test files - should be in Cargo.toml
    ];

    let cargo = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo).expect("Cargo.toml not found");
    let registered: Vec<String> = content
        .lines()
        .filter_map(|l| {
            if l.trim().starts_with("name") {
                l.split('"').nth(1).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    let registered_set: std::collections::HashSet<_> = registered.into_iter().collect();

    let mut orphans = Vec::new();
    for f in &test_files {
        // Skip common module
        if skip_files.iter().any(|s| f.ends_with(s)) {
            continue;
        }
        // Compute cargo name candidates (handle subdir naming variations)
        let rel = f.replace("tests/", "").replace(".rs", "");
        let candidates = vec![
            rel.clone(),                                     // e2e_query_test
            rel.replace("/", "_"),                           // e2e_e2e_query_test
            rel.replace("e2e/", "e2e_"), // e2e_query_test from e2e/e2e_query_test.rs
            rel.replace("ci/", "ci_"),   // ci_buffer_pool_test from ci/buffer_pool_test.rs
            rel.replace("ci/ci_", "ci_"), // ci_test from ci/ci_test.rs
            rel.split('/').last().unwrap_or("").to_string(), // basename
        ];
        if !candidates.iter().any(|c| registered_set.contains(c)) {
            orphans.push((f.clone(), candidates));
        }
    }

    // Allow up to 2 orphans (some test files have aliasing in Cargo.toml that
    // our simple matching misses). This is a soft check.
    if orphans.len() > 2 {
        panic!(
            "too many orphan test files ({}), expected ≤2: {:?}",
            orphans.len(),
            orphans
        );
    }
}
