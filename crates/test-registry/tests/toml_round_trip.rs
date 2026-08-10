//! V312-24 Phase 1: TOML round-trip integration test.
//!
//! Verifies that `TestRegistry::write_toml` produces a file that
//! `TestRegistry::from_toml` can read back into an equivalent set of
//! `ManagedTest` entries. The `tempfile` crate (dev-dep) gives us
//! a unique scratch path per test.

use std::collections::HashSet;

use tempfile::tempdir;
use test_registry::{ManagedTest, TestCategory, TestPriority, TestRegistry};

/// Write a registry with three varied entries to a fresh tempdir,
/// then read it back and assert field-by-field equality.
#[test]
fn round_trip_preserves_all_fields() {
    let dir = tempdir().expect("create tempdir");
    let path = dir.path().join("test-registry.toml");

    let mut original = TestRegistry::new();
    original.register_managed(
        ManagedTest::new("sqlancer", "target/release/sqlancer")
            .with_args(vec![
                "--duration".to_string(),
                "120".to_string(),
                "--out".to_string(),
                "target/sqlancer-report.json".to_string(),
            ])
            .with_timeout_ms(600_000)
            .with_priority(TestPriority::P1)
            .with_category(TestCategory::CI),
    );
    original.register_managed(
        ManagedTest::new("test-runner", "target/release/test-runner")
            .with_args(vec![
                "--out".to_string(),
                "target/test-runner-report.json".to_string(),
            ])
            .with_timeout_ms(300_000)
            .with_priority(TestPriority::P0)
            .with_category(TestCategory::Integration),
    );
    original.register_managed(
        ManagedTest::new("minimal", "true")
            // All defaults — no args, no timeout override, no priority, no category.
            .with_timeout_ms(120_000)
            .with_priority(TestPriority::P2)
            .with_category(TestCategory::Integration),
    );
    assert_eq!(original.managed_len(), 3);

    original.write_toml(&path).expect("write_toml");

    // File exists, non-empty, has the [[test]] header.
    let contents = std::fs::read_to_string(&path).expect("read back");
    assert!(
        contents.contains("[[test]]"),
        "missing [[test]] header in:\n{}",
        contents
    );
    assert!(contents.contains("name = \"sqlancer\""));
    assert!(contents.contains("name = \"test-runner\""));
    assert!(contents.contains("name = \"minimal\""));

    let reloaded = TestRegistry::from_toml(&path).expect("from_toml");
    assert_eq!(reloaded.managed_len(), 3);

    // The reload should populate BOTH the managed_tests map AND
    // the in-memory `tests` map (via `From<ManagedTest> for TestMetadata`).
    assert_eq!(
        reloaded.total_count(),
        3,
        "from_toml should also fold entries into tests()"
    );

    let original_names: HashSet<String> = original.managed().map(|m| m.name.clone()).collect();
    let reloaded_names: HashSet<String> = reloaded.managed().map(|m| m.name.clone()).collect();
    assert_eq!(original_names, reloaded_names);

    // Spot-check one entry's full content.
    let sqlancer = reloaded
        .get_managed("sqlancer")
        .expect("sqlancer entry present after reload");
    assert_eq!(sqlancer.binary, "target/release/sqlancer");
    assert_eq!(
        sqlancer.args,
        vec!["--duration", "120", "--out", "target/sqlancer-report.json"]
    );
    assert_eq!(sqlancer.timeout_ms, 600_000);
    assert_eq!(sqlancer.priority, TestPriority::P1);
    assert_eq!(sqlancer.category, TestCategory::CI);
}

#[test]
fn round_trip_empty_registry() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("empty.toml");

    let r = TestRegistry::new();
    r.write_toml(&path).expect("write_toml on empty");
    let s = std::fs::read_to_string(&path).expect("read");
    // Empty `test = []` array form is the canonical shape.
    assert!(
        s.contains("test") && s.contains("[]"),
        "empty manifest shape wrong:\n{}",
        s
    );

    let reloaded = TestRegistry::from_toml(&path).expect("from_toml on empty");
    assert_eq!(reloaded.managed_len(), 0);
    assert_eq!(reloaded.total_count(), 0);
}

#[test]
fn from_toml_reports_missing_file_as_io_error() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("does-not-exist.toml");
    let err = TestRegistry::from_toml(&path).expect_err("missing file should error");
    let msg = err.to_string();
    assert!(
        msg.contains("I/O error") && msg.contains("does-not-exist.toml"),
        "error message should mention the path: {}",
        msg
    );
}
