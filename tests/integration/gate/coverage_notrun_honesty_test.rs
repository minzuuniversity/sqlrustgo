//! Coverage NOT-RUN honesty regression suite — Issue #3844
//!
//! Two independent defects made the gates report a state they never
//! measured:
//!
//! | Where | Defect | Symptom |
//! |-------|--------|---------|
//! | `scripts/gate/check_rc_ga_gate.sh` (A5) | `get_coverage_avg()` returned `0` when it could not read a single `TOTAL` row | on a runner without `cargo-llvm-cov` the gate printed "A5 Coverage: 0% (min: 75%)" and counted a **blocker** — an environment blocker reported as a code defect |
//! | `scripts/gate/audit_testing.sh` | the doc-target consistency check ran only `if [ -s "$DOC_TESTS_TMP" ]` | with `rg` missing (or with no `cargo test --test` lines in the plan) the comparison never happened, yet the audit printed "PASS 测试计划中的 --test target 与 Cargo.toml 一致" |
//!
//! The rule enforced here: a check that could NOT be executed is
//! **NOT-RUN** — never PASS, and never a fabricated numeric verdict.
//! NOT-RUN does not block the gate (an environment blocker is not a code
//! defect) but it must be visible in the summary.
//!
//! The same repository already had the correct standard in
//! `.gitea/workflows/ci.yml` (`NOT-RUN: reason=COVERAGE_TOOL_MISSING`);
//! these tests keep the gate script at that standard.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn gate_script(name: &str) -> PathBuf {
    repo_root().join("scripts/gate").join(name)
}

/// A `cargo` stub that behaves like the real thing for everything EXCEPT
/// `cargo llvm-cov` — which it refuses, exactly like a runner image that
/// was built without the `llvm-tools` component / `cargo-llvm-cov`.
fn cargo_stub_without_llvm_cov(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_cov_stub_{}", tag));
    std::fs::create_dir_all(&dir).expect("create stub dir");
    let cargo = dir.join("cargo");
    std::fs::write(
        &cargo,
        "#!/usr/bin/env bash\n\
         # stub cargo: everything succeeds instantly, except `llvm-cov`\n\
         if [ \"$1\" = \"llvm-cov\" ]; then\n\
         \x20   echo \"error: no such command: llvm-cov\" >&2\n\
         \x20   exit 101\n\
         fi\n\
         exit 0\n",
    )
    .expect("write stub cargo");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&cargo)
            .expect("stat stub cargo")
            .permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&cargo, perm).expect("chmod stub cargo");
    }
    dir
}

fn run_bash_snippet(script: &str, path_prefix: &Path) -> (i32, String, String) {
    let path = format!(
        "{}:{}",
        path_prefix.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo_root())
        .env("PATH", path)
        .output()
        .expect("failed to spawn bash");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for n in chars.by_ref() {
                if n == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// =============================================================================
// A5 decision table
// =============================================================================

/// The `cargo-llvm-cov` subcommand is missing on this runner ⇒ NOT-RUN.
///
/// Pre-fix this was a FAIL ("0% < 75%") — a fabricated measurement.
#[test]
fn test_3844_a5_is_not_run_when_the_coverage_tool_is_missing() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 0 0 \"\"; \
         echo \"STATUS=$A5_STATUS\"; echo \"REASON=$A5_REASON\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("STATUS=NOT-RUN"),
        "expected NOT-RUN, got:\n{}",
        out
    );
    assert!(
        out.contains("REASON=COVERAGE_TOOL_MISSING"),
        "expected COVERAGE_TOOL_MISSING, got:\n{}",
        out
    );
}

/// The tool exists but produced no measurement ⇒ NOT-RUN (not "0%").
#[test]
fn test_3844_a5_is_not_run_when_no_crate_was_measured() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 0 \"0\"; \
         echo \"STATUS=$A5_STATUS\"; echo \"REASON=$A5_REASON\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    let out = strip_ansi(&stdout);
    assert!(out.contains("STATUS=NOT-RUN"), "got:\n{}", out);
    assert!(
        out.contains("REASON=COVERAGE_NO_MEASUREMENTS"),
        "got:\n{}",
        out
    );
}

/// A real measurement above the threshold is still a PASS.
#[test]
fn test_3844_a5_passes_when_the_measured_coverage_meets_the_threshold() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 8 \"80\"; echo \"STATUS=$A5_STATUS\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        strip_ansi(&stdout).contains("STATUS=PASS"),
        "got:\n{}",
        stdout
    );
}

/// A real measurement below the threshold is still a FAIL (blocker).
#[test]
fn test_3844_a5_fails_when_the_measured_coverage_is_below_the_threshold() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 8 \"40\"; echo \"STATUS=$A5_STATUS\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        strip_ansi(&stdout).contains("STATUS=FAIL"),
        "got:\n{}",
        stdout
    );
}

// =============================================================================
// End-to-end: the gate script itself
// =============================================================================

/// Running the real gate on a runner without `cargo-llvm-cov` must report
/// A5 as NOT-RUN, must NOT invent a "0%" number, and must NOT turn the
/// missing tool into a blocker.
#[test]
fn test_3844_gate_run_without_llvm_cov_reports_not_run_and_no_blocker() {
    let stub = cargo_stub_without_llvm_cov("no_llvm_cov");
    let script = gate_script("check_rc_ga_gate.sh");
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg(&script)
        .arg("alpha")
        .current_dir(repo_root())
        .env("PATH", path)
        .output()
        .expect("failed to spawn bash");
    let stdout = strip_ansi(&String::from_utf8_lossy(&out.stdout));
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(
        stdout.contains("COVERAGE_TOOL_MISSING"),
        "expected an explicit NOT-RUN reason, got:\n{}\n{}",
        stdout,
        stderr
    );
    assert!(
        !stdout.contains("A5 Coverage: 0%"),
        "the gate must not report a fabricated 0% coverage:\n{}",
        stdout
    );
    assert!(
        stdout.contains("not-run: 1"),
        "the not-run count must be visible in the summary:\n{}",
        stdout
    );
    // The missing tool is an environment blocker, not a code defect: it
    // must not be counted as a blocker.
    assert!(
        stdout.contains("A5 Coverage: NOT-RUN"),
        "A5 must be reported as NOT-RUN:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("[FAIL] A5 Coverage"),
        "a missing coverage tool must not be reported as a failing check:\n{}",
        stdout
    );
}

/// The summary line must never print a fabricated coverage average when
/// the coverage check did not run.
#[test]
fn test_3844_summary_never_prints_a_zero_coverage_average() {
    let stub = cargo_stub_without_llvm_cov("summary_no_cov");
    let script = gate_script("check_rc_ga_gate.sh");
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg(&script)
        .arg("alpha")
        .current_dir(repo_root())
        .env("PATH", path)
        .output()
        .expect("failed to spawn bash");
    let stdout = strip_ansi(&String::from_utf8_lossy(&out.stdout));

    assert!(
        stdout.contains("Coverage: NOT-RUN"),
        "the summary must state that coverage did not run:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("Coverage: 0% avg"),
        "the summary must not print a fabricated 0% average:\n{}",
        stdout
    );
}

// =============================================================================
// audit_testing.sh — the ripgrep-dependent check
// =============================================================================

/// Build a PATH that cannot resolve `rg`.
///
/// A plain `PATH="/usr/bin:/bin:/usr/sbin:/sbin"` is NOT enough: the CI
/// runner image installs ripgrep at `/usr/bin/rg` (apk), so that PATH still
/// finds it, the "ripgrep is missing" branch of `audit_testing.sh` is never
/// exercised, and this test fails for the wrong reason. Instead, symlink
/// every executable of the current PATH into a scratch directory — except
/// `rg` itself — and use that directory as the PATH.
fn path_without_rg() -> String {
    let dir = std::env::temp_dir().join("sqlrustgo_audit_3844_bin");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch bin dir");

    let original = std::env::var("PATH").unwrap_or_default();
    for source_dir in original.split(':') {
        if source_dir.is_empty() {
            continue;
        }
        let entries = match std::fs::read_dir(source_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name == "rg" {
                continue;
            }
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                continue;
            }
            let link = dir.join(&name);
            if link.exists() {
                continue;
            }
            let _ = std::os::unix::fs::symlink(entry.path(), &link);
        }
    }
    dir.display().to_string()
}

fn run_audit(with_rg: bool) -> String {
    let out_dir = std::env::temp_dir().join(format!(
        "sqlrustgo_audit_3844_{}",
        if with_rg { "rg" } else { "norg" }
    ));
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    // A PATH from which ripgrep cannot be resolved at all.
    let path = if with_rg {
        std::env::var("PATH").unwrap_or_default()
    } else {
        path_without_rg()
    };

    let out = Command::new("bash")
        .arg(gate_script("audit_testing.sh"))
        .arg("v4.1.0")
        .arg("ga")
        .arg(&out_dir)
        .current_dir(repo_root())
        .env("PATH", path)
        .output()
        .expect("failed to spawn bash");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let report = std::fs::read_to_string(out_dir.join("TEST_AUDIT.md")).unwrap_or_default();
    format!("{}{}", stdout, report)
}

/// Without ripgrep the audit must NOT report the doc-target consistency as
/// a PASS. Pre-fix it printed exactly that, for a comparison that never
/// ran.
#[test]
fn test_3844_audit_without_ripgrep_never_reports_a_false_pass() {
    let text = run_audit(false);
    assert!(
        !text.contains("PASS 测试计划中的 --test target 与 Cargo.toml 一致"),
        "the audit must not report a consistency PASS without ripgrep:\n{}",
        text
    );
    assert!(
        text.contains("NOT-RUN"),
        "the audit must mark the skipped comparison as NOT-RUN:\n{}",
        text
    );
    assert!(
        text.contains("缺少 ripgrep"),
        "the audit must fail loudly when ripgrep is missing:\n{}",
        text
    );
}

/// With ripgrep present but no `cargo test --test` targets declared in the
/// plan, the comparison still did not happen ⇒ NOT-RUN, never PASS.
#[test]
fn test_3844_audit_with_an_empty_comparison_set_is_not_run_not_pass() {
    let text = run_audit(true);
    assert!(
        !text.contains("PASS 测试计划中的 --test target 与 Cargo.toml 一致"),
        "an empty comparison set must not be reported as a PASS:\n{}",
        text
    );
    assert!(
        text.contains("NOT-RUN"),
        "an empty comparison set must be reported as NOT-RUN:\n{}",
        text
    );
}
