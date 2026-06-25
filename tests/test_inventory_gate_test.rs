//! P0-1: D6 Test Inventory Gate validation
//!
//! **Issue**: #2874
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N1)
//! **Change**: openspec/changes/p0-1-integrate-35-tests

use std::path::Path;

#[test]
fn test_d6_runs_all_tests() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // Must loop over all test files
    assert!(content.contains("TEST_FILES"), "must collect TEST_FILES");
    assert!(
        content.contains("cargo test --test"),
        "must invoke cargo test --test"
    );
    assert!(content.contains("find tests"), "must find all test files");
}

#[test]
fn test_d6_no_fake_passing() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // Must parse actual test results, not just claim PASS
    assert!(
        content.contains("test result:"),
        "must parse test result line"
    );
    assert!(content.contains("grep -oE"), "must extract P/F counts");
    assert!(content.contains("FAILED"), "must track failures");
}

#[test]
fn test_d6_generates_evidence() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    assert!(
        content.contains("d6_test_inventory.json"),
        "must write evidence JSON"
    );
    assert!(content.contains("artifacts/gate"), "must use artifacts dir");
}

#[test]
fn test_d6_handles_subdir_tests() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // D6b resolves subdir test names via a PATH_TO_NAME map built from
    // [[test]] blocks in Cargo.toml (so tests/ci/buffer_pool_test.rs uses the
    // canonical name "buffer_pool_test", not the path-derived "ci_buffer_pool_test").
    assert!(
        content.contains("PATH_TO_NAME"),
        "must build PATH_TO_NAME map from [[test]] blocks"
    );
    assert!(
        content.contains("awk") && content.contains("name = "),
        "must parse [[test]] name = entries"
    );
}

#[test]
fn test_d6_long_running_not_failed() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // Must mark tpch/long_run tests as TIMEOUT, not FAILED
    assert!(content.contains("tpch"), "must handle tpch tests");
    assert!(content.contains("long_run"), "must handle long_run tests");
    assert!(content.contains("TIMEOUT"), "must report TIMEOUT status");
    assert!(
        content.contains("LONG_RUNNING_TESTS"),
        "must track long-running"
    );
}

#[test]
fn test_d6_timeout_per_test() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // Must have timeout per test to prevent hang
    assert!(content.contains("timeout"), "must have timeout per test");
}

#[test]
fn test_d6_records_pass_fail_counts() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // Must record both passed and failed
    assert!(content.contains("TOTAL_PASSED"), "must track total passed");
    assert!(content.contains("TOTAL_FAILED"), "must track total failed");
    assert!(
        content.contains("TOTAL_IGNORED"),
        "must track total ignored"
    );
}

#[test]
fn test_d6_integration_ratio() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let content = std::fs::read_to_string(&script).expect("check_test_inventory.sh not found");
    // Must report integration ratio
    assert!(
        content.contains("INTEGRATION_RATIO") || content.contains("integration_ratio"),
        "must report integration ratio"
    );
}

#[test]
fn test_d6_executable() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh");
    let metadata = std::fs::metadata(&script).expect("script must exist");
    let permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = permissions.mode();
        assert!(mode & 0o111 != 0, "must be executable");
    }
}
