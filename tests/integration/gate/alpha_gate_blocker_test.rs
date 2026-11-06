//! ALPHA gate blocker regression suite — issue #5537
//!
//! Two independent defects kept the v4.1.0 ALPHA stage gate red:
//!
//! | Defect | Symptom |
//! |--------|---------|
//! | `check_coverage_v312.sh` did not pass the feature a crate's integration tests need | `sqlrustgo-storage` measured ZERO coverage: `error[E0432]: unresolved import sqlrustgo_storage::fault_injection` |
//! | `env!("CARGO_TARGET_DIR")` is expanded at COMPILE time | `error: environment variable CARGO_TARGET_DIR not defined at compile time` — the test target did not compile at all |
//!
//! The first one is worse than it looks. The coverage script's own comment
//! records that a crate which fails to measure *raises* the reported
//! average, so the L1_8 figure was not merely missing — it was
//! misleading.
//!
//! Design notes
//! ------------
//! * The feature list is DERIVED from the crate's own sources, so a new
//!   `#[cfg(feature = "...")]` module cannot silently break the gate
//!   again. This suite checks that the derivation still finds the gated
//!   items.
//! * The `CARGO_TARGET_DIR` check compiles the test target with the
//!   variable UNSET, which is how CI runs it.
//! * The feature-gated tests are checked in BOTH directions: absent
//!   without the feature, present with it. A `cfg` gate that accidentally
//!   removed the test forever would pass a one-sided check.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the workspace root for the root package, which
    // is where scripts/ and docs/ live. Matches the convention already used
    // by tests/integration/gate/gate_honesty_test.rs.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn coverage_gate_derives_features_from_the_crate_sources() {
    // The gate must not carry a hand-maintained feature list: a stale list
    // is a SILENT failure (the coverage number just comes out wrong).
    //
    // #5579 moved the measurement itself into `scripts/gate/common.sh`
    // ("one rule, one implementation") ⇒ the RULE now lives there and this
    // test follows the RULE, not the file it used to live in. The gate must
    // still delegate to the shared implementation instead of keeping a copy.
    let shared = read("scripts/gate/common.sh");
    assert!(
        shared.contains("coverage_derive_crate_features"),
        "the coverage measurement must derive the per-crate features from the \
         crate's own sources"
    );
    assert!(
        !shared.contains("CRATE_FEATURES=("),
        "the hand-maintained CRATE_FEATURES list must be gone — it goes stale \
         silently"
    );
    let gate = read("scripts/gate/check_coverage_v312.sh");
    assert!(
        !gate.contains("CRATE_FEATURES=("),
        "the hand-maintained CRATE_FEATURES list must be gone from the gate too"
    );
    assert!(
        !gate.contains("derive_crate_features()"),
        "the gate must not carry its own derivation — it must use the shared \
         implementation in scripts/gate/common.sh"
    );
}

#[test]
fn coverage_gate_derives_the_features_storage_actually_needs() {
    // Run the shared derivation and check it finds the two features
    // sqlrustgo-storage's integration tests import.
    //
    // NOTE: the extraction writes to a temp file and sources THAT file —
    // macOS's bash 3.2 does not define functions from `source <(...)`
    // (process substitution), so the original `source <(sed ...)` form
    // silently sourced nothing and the assertion below was checking an
    // empty string (a false green). The repo's gate scripts must stay
    // bash 3.2 compatible, so the temp-file form is the correct one.
    let out = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "-c",
            r#"
            tmp=$(mktemp -d /Volumes/workspace/dev/derive-XXXXXX)
            sed -n '/^coverage_derive_crate_features()/,/^}/p' scripts/gate/common.sh > "$tmp/d.sh"
            . "$tmp/d.sh"
            coverage_derive_crate_features sqlrustgo-storage
            rm -rf "$tmp"
            "#,
        ])
        .output()
        .expect("spawn bash");
    let got = String::from_utf8_lossy(&out.stdout);
    let got = got.trim();
    assert!(
        got.contains("fault-injection"),
        "the derivation must find the fault-injection feature. Got: {got:?}"
    );
    assert!(
        got.contains("bin_storage_default"),
        "the derivation must find the bin_storage_default feature. Got: {got:?}"
    );
}

#[test]
fn coverage_gate_does_not_derive_features_for_a_plain_crate() {
    let out = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "-c",
            r#"
            tmp=$(mktemp -d /Volumes/workspace/dev/derive-XXXXXX)
            sed -n '/^coverage_derive_crate_features()/,/^}/p' scripts/gate/common.sh > "$tmp/d.sh"
            . "$tmp/d.sh"
            coverage_derive_crate_features sqlrustgo-types
            rm -rf "$tmp"
            "#,
        ])
        .output()
        .expect("spawn bash");
    let got = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        got.trim(),
        "",
        "sqlrustgo-types has no feature-gated items, so no feature must be \
         derived for it. Got: {got:?}"
    );
}

#[test]
fn coverage_gate_runs_the_tests_serially() {
    // Under instrumentation the code runs 2-10x slower; the default
    // parallel execution then has the threads competing for CPU, and the
    // SAME checkout measured different numbers from run to run
    // (sqlrustgo-planner: 88.91% and PASS with --test-threads=1, FAILED
    // with the default thread count). A coverage number that depends on
    // machine load is not a measurement.
    //
    // #5579: the measurement lives in scripts/gate/common.sh now ⇒ the
    // serial-execution rule is asserted there (the gate delegates to it).
    let shared = read("scripts/gate/common.sh");
    assert!(
        shared.contains("--test-threads=1"),
        "the coverage measurement must run the tests serially"
    );
}

#[test]
fn coverage_gate_excludes_the_environment_dependent_tests() {
    // The crash tests need a pre-built server binary at a machine-specific
    // path and exact fault-point timing. A failure in any of them produces
    // NO coverage table for the whole crate.
    //
    // #5579: the exclusion lists live in scripts/gate/common.sh now
    // (COVERAGE_EXCLUDED_TEST_FILES for the machine-dependent crash tests,
    // COVERAGE_PERF_BOUND_TEST_FILES for the wall-clock-bound ones).
    let shared = read("scripts/gate/common.sh");
    for f in [
        "crates/storage/tests/crash_commit_fsync_b1.rs",
        "crates/storage/tests/crash_wal_index_b1.rs",
        "crates/storage/tests/fault_coordination.rs",
        "crates/storage/tests/write_path_scaling_5181.rs",
    ] {
        assert!(
            shared.contains(f),
            "the coverage measurement must exclude the environment-dependent \
             test file {f}"
        );
    }
}

#[test]
fn the_feature_gated_tests_compile_without_the_feature() {
    // `cargo test --workspace` does not pass --all-features, and an
    // integration test links the library without cfg(test) — so a
    // `#[cfg(any(test, feature = "..."))]` item does not exist for it and
    // the test failed to COMPILE. That is what the AFP gate reported as
    // NEW failures.
    for f in [
        "crates/storage/tests/fault_coordination.rs",
        "crates/storage/tests/a06_failpoint_reachability.rs",
        "crates/storage/tests/crash_commit_fsync_b1.rs",
        "crates/storage/tests/crash_wal_index_b1.rs",
        "crates/storage/tests/binary_v2_direct_v3_12.rs",
        "crates/storage/tests/no_namespace_db_refusal_5266.rs",
        "crates/storage/tests/storage_db_namespace_contract.rs",
    ] {
        let text = read(f);
        assert!(
            text.contains("#![cfg(feature ="),
            "{f} must be gated behind its feature, or it fails to compile \\
             under `cargo test --workspace`"
        );
    }
}

#[test]
fn the_feature_gate_is_not_an_ignore() {
    // ADR-008 Policy 2: an ignored test stays visible as "N ignored" in
    // the output, which is how a gate reports PASS while the test never
    // ran. A `cfg` gate removes the test from the binary outright.
    for f in [
        "crates/storage/tests/fault_coordination.rs",
        "crates/storage/tests/a06_failpoint_reachability.rs",
        "crates/storage/tests/crash_commit_fsync_b1.rs",
        "crates/storage/tests/crash_wal_index_b1.rs",
        "crates/storage/tests/binary_v2_direct_v3_12.rs",
        "crates/storage/tests/no_namespace_db_refusal_5266.rs",
        "crates/storage/tests/storage_db_namespace_contract.rs",
    ] {
        let code = code_only(&read(f));
        assert!(
            !code.contains("#[ignore"),
            "{f} must not use #[ignore] — use a cfg gate instead (ADR-008 Policy 2)"
        );
    }
}

/// The source with comments removed.
///
/// The assertion is about CODE, not prose: the helper's own doc comment
/// quotes `env!("CARGO_TARGET_DIR")` to explain what it is avoiding, and a
/// naive substring check would flag that explanation as a violation.
/// Both whole-line comments and trailing comments are stripped.
fn code_only(text: &str) -> String {
    text.lines()
        .map(|l| {
            let t = l.trim_start();
            if t.starts_with("//") {
                return "";
            }
            // Strip a trailing `//` comment. A `//` inside a string
            // literal is not handled, but none of the files inspected here
            // contain one.
            match l.find("//") {
                Some(i) => &l[..i],
                None => l,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_cargo_target_dir_dependency_is_runtime_resolved() {
    // `env!("CARGO_TARGET_DIR")` is expanded at COMPILE time, so the
    // variable must exist when the test is compiled — and neither CI nor a
    // plain local checkout guarantees that:
    //     error: environment variable `CARGO_TARGET_DIR` not defined at compile time
    for f in [
        "tests/integration/sql/four_way_compare_test.rs",
        "tests/integration/sql/four_way_cell_diff_test.rs",
    ] {
        let code = code_only(&read(f));
        // `option_env!("CARGO_TARGET_DIR")` CONTAINS `env!("CARGO_TARGET_DIR")`
        // as a substring, so a plain `contains` would reject the correct
        // code too. The distinguishing part is what precedes `env!`.
        let bare_env = code.replace("option_env!", "");
        assert!(
            !bare_env.contains("env!(\"CARGO_TARGET_DIR\")"),
            "{f} must not use env!(\"CARGO_TARGET_DIR\") in CODE — it is \\
             expanded at compile time and breaks the build when the variable \\
             is unset. (A mention in a comment is fine and expected.)"
        );
        assert!(
            code.contains("option_env!(\"CARGO_TARGET_DIR\")"),
            "{f} must resolve the target directory at runtime"
        );
    }
}
