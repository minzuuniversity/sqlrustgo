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
///
/// 门槛在这里是**显式传入**的：合并到 develop/v4.1.0 之后
/// `COVERAGE_MIN` 不再是顶层常量（#5581 把它改成从 STAGE_CONFIG.yaml 读，
/// source 之后是空串），判定表若依赖环境残留就只是在测「谁先赋值」。
#[test]
fn test_3844_a5_passes_when_the_measured_coverage_meets_the_threshold() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 8 \"80\" 75; echo \"STATUS=$A5_STATUS\"",
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
         a5_coverage_decision 1 8 \"40\" 75; echo \"STATUS=$A5_STATUS\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        strip_ansi(&stdout).contains("STATUS=FAIL"),
        "got:\n{}",
        stdout
    );
}

/// #5581 × #3844: 调用方不给门槛时，判定必须现读 SSOT，而不是沿用某个
/// 顶层常量。
///
/// 来方分支上 `COVERAGE_MIN=75` 是顶层常量，source 之后永远有值，
/// 于是「门槛从哪来」这件事在该线上是不可判定的 —— 80 分被判 PASS。
/// 这一线（develop/v4.1.0）门槛写在 `docs/governance/STAGE_CONFIG.yaml`，
/// GA 是 85，source 之后 `COVERAGE_MIN` 是空串：80 必须判 FAIL。
/// 这条用例把两种血缘的差异钉成可判定的行为，也是本次合并的判据。
#[test]
fn test_5581_a5_threshold_comes_from_ssot_when_caller_gives_none() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         echo \"MIN=[$(stage_coverage_min GA)]\"; \
         COVERAGE_MIN=\"\"; a5_coverage_decision 1 8 \"85\"; echo \"AT_MIN=$A5_STATUS\"; \
         COVERAGE_MIN=\"\"; a5_coverage_decision 1 8 \"84\"; echo \"BELOW_MIN=$A5_STATUS\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("MIN=[85]"),
        "SSOT 的 GA 覆盖率门槛应是 85，否则这条用例失去意义；got:\n{}",
        out
    );
    assert!(
        out.contains("AT_MIN=PASS"),
        "正好达到 SSOT 门槛必须 PASS（门槛现读 SSOT，不是空值判红）；got:\n{}",
        out
    );
    assert!(
        out.contains("BELOW_MIN=FAIL"),
        "低于 SSOT 门槛 1 个百分点必须 FAIL；若判定沿用了 75 这种顶层常量就会是 PASS；got:\n{}",
        out
    );
}

// =============================================================================
// A5 decimal comparison — GA 实跑 005 实测复现的误判
// =============================================================================

/// 覆盖率是**小数**，判定必须用浮点口径。
///
/// 回归（GA 实跑 005）：判据用 `[ "$pct" -ge "$threshold" ]`（bash 的
/// `[ ]` 只做整数比较），对 `bc` scale=2 算出的 "87.38" 会报
/// `[: 87.38: integer expected` 并返回 2 ⇒ 落进 else 分支判红。
/// 实测同一轮里 `coverage_verdict` 已判 PASS（"average coverage:
/// 87.38% (threshold: 85%)"），A5 紧跟着打印
/// "✗ [FAIL] A5 Coverage: 87.38% (min: 85%)" —— **同一个数、两套判据**。
///
/// 这条用例钉住 GA 实测值 87.38 vs GA 门槛 85 必须 PASS。
#[test]
fn test_a5_coverage_decision_compares_decimal_values_as_floats() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 8 \"87.38\" 85; echo \"STATUS=$A5_STATUS\"; echo \"REASON=$A5_REASON\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("STATUS=PASS"),
        "87.38% ≥ 85% 必须 PASS —— 整数比较会把它误判成 FAIL；got:\n{}",
        out
    );
    assert!(
        !out.contains("REASON=COVERAGE_VALUE_UNPARSEABLE"),
        "87.38 是合法数值，不该被判为读不出；got:\n{}",
        out
    );
}

/// 浮点比较不是「一律放行」：差 0.01 个百分点也必须判红。
///
/// 这条用例是上一条的**反向锚** —— 只测「87.38 PASS」的话，把比较改成
/// 恒真也能骗过它。
#[test]
fn test_a5_coverage_decision_decimal_just_below_threshold_fails() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 8 \"84.99\" 85; echo \"STATUS=$A5_STATUS\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        strip_ansi(&stdout).contains("STATUS=FAIL"),
        "84.99% < 85% 必须 FAIL —— 浮点比较不能变成「一律放行」；got:\n{}",
        stdout
    );
}

/// 读不出数值仍判红：绝不把「读不出来」当成「通过」。
///
/// 口径与 #3844 的 NOT-RUN 一致：工具在、但**读不出数字**时，判红并写明
/// 原因（COVERAGE_VALUE_UNPARSEABLE），而不是当成 0% 或直接放行。
#[test]
fn test_a5_coverage_decision_unparseable_value_fails_closed() {
    let (code, stdout, stderr) = run_bash_snippet(
        "source scripts/gate/check_rc_ga_gate.sh; \
         a5_coverage_decision 1 8 \"N/A\" 85; echo \"STATUS=$A5_STATUS\"; echo \"REASON=$A5_REASON\"",
        &repo_root(),
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("STATUS=FAIL"),
        "读不出覆盖率数值必须 FAIL（fail-closed）；got:\n{}",
        out
    );
    assert!(
        out.contains("REASON=COVERAGE_VALUE_UNPARSEABLE"),
        "读不出数值必须写明原因，不能是无理由的红；got:\n{}",
        out
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
