//! P0-2: Cargo.toml test path configuration validation
//!
//! **Issue**: #2875
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N2)
//! **Change**: openspec/changes/p0-2-cargo-toml-test-paths

use std::path::Path;

#[test]
fn test_cargo_toml_has_test_section() {
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");
    assert!(content.contains("[[test]]"), "must have [[test]] sections");
}

#[test]
fn test_cargo_toml_test_count_52() {
    // 16 original + 36 new = 52 explicit [[test]] entries
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");
    let count = content.matches("[[test]]").count();
    assert!(
        count >= 52,
        "must have at least 52 [[test]] entries (was {}), one per test file",
        count
    );
}

#[test]
fn test_cargo_toml_all_paths_exist() {
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");

    // Extract all path = "..." entries
    let paths: Vec<&str> = content
        .lines()
        .filter_map(|l| {
            if l.trim().starts_with("path") {
                l.split('"').nth(1)
            } else {
                None
            }
        })
        .collect();

    let mut missing = Vec::new();
    for p in paths {
        // Skip crate paths (not test files)
        if p.starts_with("tests/") {
            let full_path = std::env::current_dir().unwrap().join(p);
            if !full_path.exists() {
                missing.push(p.to_string());
            }
        }
    }

    assert!(
        missing.is_empty(),
        "all test paths must exist, missing: {:?}",
        missing
    );
}

#[test]
fn test_cargo_toml_no_duplicate_test_names() {
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");

    // Extract only `name = "..."` lines that belong to a `[[test]]`
    // section. Previously the test naively matched every `name =`
    // in the file, which false-positived on `[package] name = "..."`
    // (the root crate name) and `[[bin]] name = "..."` (a binary
    // entry). Two `[[test]]` entries with the same name is the
    // actual constraint we want to enforce — duplicate test names
    // cause `cargo test --test <name>` to be ambiguous.
    let names: Vec<String> = content
        .lines()
        .scan(false, |in_test, l| {
            let t = l.trim();
            if t.starts_with("[[test]]") {
                *in_test = true;
            } else if t.starts_with("[[") {
                *in_test = false;
            }
            if *in_test && t.starts_with("name") {
                l.split('"').nth(1).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    let mut seen = std::collections::HashSet::new();
    for n in &names {
        if !seen.insert(n.clone()) {
            panic!("duplicate [[test]] name: {}", n);
        }
    }
}

#[test]
fn test_cargo_toml_p02_section_exists() {
    // Verify the P0-2 marker comment exists
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");
    assert!(content.contains("P0-2"), "must have P0-2 marker comment");
    assert!(
        content.contains("36 new [[test]] entries"),
        "must document 36 new entries"
    );
}

#[test]
fn test_cargo_toml_p02_critical_tests() {
    // Verify the 6 critical tests from audit are now registered
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");

    let critical = [
        "adaptive_hash_index_test",
        "buffer_pool_benchmark_test",
        "e2e_monitoring_test",
        "ci_test",
        "wal_integration_test",
        "wal_tx_contract_test",
    ];
    for t in &critical {
        assert!(
            content.contains(t),
            "critical test {} must be registered",
            t
        );
    }
}

#[test]
fn test_cargo_toml_e2e_subdir_aliases() {
    // Verify the e2e subdir tests have correct Cargo.toml aliases
    // (basename != cargo name for e2e/)
    let path = std::env::current_dir().unwrap().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("Cargo.toml not found");

    // e2e_observability_test should point to tests/e2e/observability_test.rs
    assert!(
        content.contains("e2e_observability_test"),
        "e2e_observability_test must be registered"
    );
    assert!(
        content.contains("e2e_monitoring_test"),
        "e2e_monitoring_test must be registered"
    );
}
