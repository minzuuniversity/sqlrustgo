//! Gate honesty regression suite — Issues #5417 / #5418 / #5419
//!
//! These tests assert that the RC/GA gate scripts **report what actually
//! happened**. Before the fixes, three independent defects made the gate
//! structurally incapable of reporting a failure:
//!
//! | Issue | Defect | Symptom |
//! |-------|--------|---------|
//! | #5417 | `output=$(bash "$script" 2>&1 \|\| true)` in `run_gate()` | `$?` was always 0 ⇒ every one of the 8 aggregate dimensions reported "✅ PASS (exit 0)" |
//! | #5418 | `GATE="${1:-all}"` used for both the flag and the dimension selector | `check_rc_ga_gate.sh --skip-a5` skipped D1/D2/D5/D6/D7/D8/D9 and still printed "✓ GATE: PASS — All dimensions validated" |
//! | #5419 | `for t in ${D6_INTEGRATION_TESTS}` (no index) in bash | Only the first array element was iterated ⇒ D6 ran 0 tests and printed "✓ PASS D6 summary: 0/0 integration tests PASS" |
//!
//! On top of that, D6/D7 failures never reached the verdict at all, so a
//! failing D6 still produced "GATE: PASS".
//!
//! Design notes
//! ------------
//! * The gate scripts are *shell*, so the decision logic was extracted into
//!   side-effect-free functions (`gate_parse_args`, `gate_expected_dims`,
//!   `gate_verdict_for`, `resolve_d6_target`) that this suite can exercise
//!   directly after `source`-ing the script.
//! * The CLI-level tests run the real gate script with a **stub `cargo`** on
//!   `PATH`. The stub makes the orchestration layer (argument parsing,
//!   dimension selection, verdict) observable in ~4 seconds instead of
//!   ~40 minutes, and it can never turn a real failure into a pass — the
//!   stub always exits 0, so any dimension that needs a *verdict* from the
//!   test harness correctly reports "no verdict ⇒ FAIL".
//! * Every test in this file fails on the pre-fix scripts (verified by
//!   re-running the same suite against `git show HEAD:scripts/gate/...`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn gate_script(name: &str) -> PathBuf {
    repo_root().join("scripts/gate").join(name)
}

/// Create a directory containing a `cargo` stub that always exits 0.
fn stub_cargo_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_gate_stub_{}", tag));
    std::fs::create_dir_all(&dir).expect("create stub dir");
    let cargo = dir.join("cargo");
    std::fs::write(
        &cargo,
        "#!/usr/bin/env bash\n# stub cargo: always succeeds instantly\nexit 0\n",
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

fn run_bash_with_stub_cargo(script: &Path, args: &[&str], tag: &str) -> (i32, String, String) {
    let stub = stub_cargo_dir(tag);
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg(script)
        .args(args)
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

fn run_bash_snippet(script: &str, tag: &str) -> (i32, String, String) {
    let path = format!(
        "{}:{}",
        stub_cargo_dir(tag).display(),
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
            // consume until the end of the CSI sequence
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

/// Drop whole-line comments so that source-pattern assertions look at
/// *code* only. The gate scripts document the pre-fix anti-patterns in
/// their comments — e.g. the literal `|| true` — and a naive grep would
/// flag the documentation of the bug as the bug.
fn strip_comments(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract a numeric `KEY=<n>` field. The key may appear anywhere in the
/// output and several fields may share one line.
fn field_u64(out: &str, key: &str) -> Option<u64> {
    for token in out.split_whitespace() {
        if let Some(rest) = token.strip_prefix(&format!("{}=", key)) {
            return rest.parse::<u64>().ok();
        }
    }
    None
}

// =============================================================================
// #5417 — check_full_gate_verification.sh: run_gate() swallowed the exit code
// =============================================================================

/// Drive `run_gate()` from the real script against a stub that exits 1.
///
/// Pre-fix, `run_gate()` executed `output=$(bash "$script" 2>&1 || true)`,
/// so `$?` was 0 and the dimension was counted as PASS. The stub exits 1,
/// so the fixed script must count it as a FAIL.
#[test]
fn test_5417_run_gate_counts_nonzero_exit_as_failure() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_full_gate_verification.sh
STUB=$(mktemp)
printf '#!/usr/bin/env bash\nexit 1\n' > "$STUB"
chmod +x "$STUB"
run_gate "StubFailingGate" "$STUB" 0
echo "COUNTS FAIL_COUNT=$FAIL_COUNT PASS_COUNT=$PASS_COUNT DRIFT_COUNT=$DRIFT_COUNT"
echo "RESULTS=${RESULTS[*]}"
rm -f "$STUB"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5417_fail");
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("FAIL_COUNT=1"),
        "a gate that exits 1 must be counted as one failure; got:\n{}\n{}",
        out,
        stderr
    );
    assert!(
        out.contains("PASS_COUNT=0"),
        "a gate that exits 1 must NOT be counted as a pass; got:\n{}",
        out
    );
    assert!(
        out.contains("FAIL (exit 1)"),
        "the result row must record the real exit code; got:\n{}",
        out
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// A gate that exits 0 is still PASS — the fix must not invert the verdict.
#[test]
fn test_5417_run_gate_counts_zero_exit_as_pass() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_full_gate_verification.sh
STUB=$(mktemp)
printf '#!/usr/bin/env bash\nexit 0\n' > "$STUB"
chmod +x "$STUB"
run_gate "StubPassingGate" "$STUB" 0
echo "COUNTS FAIL_COUNT=$FAIL_COUNT PASS_COUNT=$PASS_COUNT DRIFT_COUNT=$DRIFT_COUNT"
rm -f "$STUB"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5417_pass");
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("FAIL_COUNT=0") && out.contains("PASS_COUNT=1"),
        "a gate that exits 0 must be counted as one pass; got:\n{}\n{}",
        out,
        stderr
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// exit 2 is DRIFT, which the aggregator reports separately from PASS.
#[test]
fn test_5417_run_gate_counts_exit_two_as_drift() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_full_gate_verification.sh
STUB=$(mktemp)
printf '#!/usr/bin/env bash\nexit 2\n' > "$STUB"
chmod +x "$STUB"
run_gate "StubDriftGate" "$STUB" 0
echo "COUNTS FAIL_COUNT=$FAIL_COUNT PASS_COUNT=$PASS_COUNT DRIFT_COUNT=$DRIFT_COUNT"
rm -f "$STUB"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5417_drift");
    let out = strip_ansi(&stdout);
    assert!(
        out.contains("DRIFT_COUNT=1") && out.contains("PASS_COUNT=0"),
        "exit 2 must be DRIFT, never PASS; got:\n{}\n{}",
        out,
        stderr
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// The `|| true` that swallowed the exit code must not come back.
#[test]
fn test_5417_run_gate_does_not_swallow_exit_code() {
    let text = std::fs::read_to_string(gate_script("check_full_gate_verification.sh"))
        .expect("read check_full_gate_verification.sh");
    let start = text.find("run_gate() {").expect("run_gate() must exist");
    let body = &text[start..];
    let end = body.find("\n}\n").expect("run_gate() must be closed") + 3;
    let body = strip_comments(&body[..end]);
    assert!(
        !body.contains("|| true"),
        "run_gate() must not swallow the child exit code with `|| true`:\n{}",
        body
    );
    assert!(
        body.contains("code=$?"),
        "run_gate() must capture the child exit code:\n{}",
        body
    );
}

/// The `--skip-a5` flag must be passed as an *argument*, not glued into the
/// script path. Pre-fix the call site was
/// `run_gate "..." "$SCRIPT_DIR/check_rc_ga_gate.sh --skip-a5" 0`, and the
/// path did not exist ⇒ the dimension was reported as NOT_FOUND.
#[test]
fn test_5417_skip_a5_is_passed_as_an_argument_not_a_path() {
    let text = std::fs::read_to_string(gate_script("check_full_gate_verification.sh"))
        .expect("read check_full_gate_verification.sh");
    let code = strip_comments(&text);
    assert!(
        !code.contains("check_rc_ga_gate.sh --skip-a5\""),
        "the --skip-a5 flag must not be concatenated into the script path"
    );
    assert!(
        code.contains("check_rc_ga_gate.sh\" 0 --skip-a5"),
        "the --skip-a5 flag must be passed to the gate script as an argument"
    );
}

// =============================================================================
// #5418 — check_rc_ga_gate.sh: flags and dimension selector were the same slot
// =============================================================================

/// An unrecognised argument must be a usage error, never a silently
/// reinterpreted dimension selector.
#[test]
fn test_5418_unknown_argument_is_a_usage_error() {
    let (code, stdout, stderr) = run_bash_with_stub_cargo(
        &gate_script("check_rc_ga_gate.sh"),
        &["--not-a-real-flag"],
        "5418_unknown",
    );
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("Unknown argument"),
        "an unknown argument must be reported; got:\n{}",
        combined
    );
    assert_eq!(
        code, 2,
        "an unknown argument must exit 2 (usage error), got {}\n{}",
        code, combined
    );
}

/// `--skip-a5` is a flag, not a dimension selector. Pre-fix it made the gate
/// skip D1/D2/D5/D6/D7/D8/D9 and still print "All dimensions validated".
#[test]
fn test_5418_skip_a5_still_executes_every_dimension() {
    let (code, stdout, stderr) = run_bash_with_stub_cargo(
        &gate_script("check_rc_ga_gate.sh"),
        &["--skip-a5"],
        "5418_skip",
    );
    let out = strip_ansi(&stdout);
    assert!(
        !out.contains("All dimensions validated"),
        "the summary must report the dimensions that actually ran, not a fixed phrase:\n{}",
        out
    );
    assert!(
        out.contains("Dimensions executed:"),
        "the summary must list the executed dimensions:\n{}",
        out
    );
    assert!(
        out.contains("D1 D2 D3 D4 D5 D6 D7 D8 D9"),
        "every dimension must be executed under --skip-a5:\n{}",
        out
    );
    // The D1/D2/D5/D7/D8/D9 rows must not be the "0/0" placeholder that the
    // pre-fix script produced for every skipped dimension.
    assert!(
        !out.contains("D1-Alpha:  0/0"),
        "D1 must actually run under --skip-a5:\n{}",
        out
    );
    assert!(
        !out.contains("D9-Coverage:    0/0"),
        "D9 must actually run under --skip-a5:\n{}",
        out
    );
    let _ = code;
    let _ = stderr;
}

/// The dimension plan must match what main() actually executes.
#[test]
fn test_5418_expected_dims_cover_every_mode() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
echo "ALL=[$(gate_expected_dims all)]"
echo "GA=[$(gate_expected_dims ga)]"
echo "RC=[$(gate_expected_dims rc)]"
echo "BETA=[$(gate_expected_dims beta)]"
echo "ALPHA=[$(gate_expected_dims alpha)]"
echo "BOGUS=[$(gate_expected_dims bogus)]"
gate_parse_args --skip-a5; echo "PARSE_RC=$? TYPE=$GATE_TYPE SKIP_A5=$SKIP_A5"
gate_parse_args ga; echo "PARSE_RC=$? TYPE=$GATE_TYPE SKIP_A5=$SKIP_A5"
gate_parse_args --bogus 2>/dev/null; echo "PARSE_RC=$? TYPE=$GATE_TYPE"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5418_dims");
    let out = stdout.trim().to_string();
    assert!(
        out.contains("ALL=[D1 D2 D3 D4 D5 D6 D7 D8 D9]"),
        "got:\n{}",
        out
    );
    assert!(
        out.contains("GA=[D3 D4 D5 D6 D7 D8 D9]"),
        "the ga plan runs the release-stage dimensions, NOT D1/D2 (those are \
         the alpha/beta stage gates, covered by the 'all' plan); got:\n{}",
        out
    );
    assert!(out.contains("RC=[D3 D4 D5 D9]"), "got:\n{}", out);
    assert!(out.contains("BETA=[D2 D3 D4]"), "got:\n{}", out);
    assert!(out.contains("ALPHA=[D1 D3 D4]"), "got:\n{}", out);
    assert!(out.contains("BOGUS=[]"), "got:\n{}", out);
    assert!(
        out.contains("PARSE_RC=0 TYPE=all SKIP_A5=1"),
        "--skip-a5 must not be mistaken for a dimension selector; got:\n{}",
        out
    );
    assert!(
        out.contains("PARSE_RC=0 TYPE=ga SKIP_A5=0"),
        "a dimension selector must still be honoured; got:\n{}",
        out
    );
    assert!(
        out.contains("PARSE_RC=2"),
        "an unknown argument must be rejected; got:\n{}",
        out
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

// =============================================================================
// #5419 — D6a integration gate never ran anything, and D6/D7 never counted
// =============================================================================

/// Every curated entry must resolve to a real Cargo test target.
///
/// The pre-fix list contained two entries whose files were deleted in
/// v3.12.0 ("retire: remove dead tests + unregister from Cargo.toml");
/// they were removed from the list together with the fix.
#[test]
fn test_5419_every_curated_d6_entry_resolves_to_a_cargo_target() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
build_d6_target_map
count=0
unresolved=""
for entry in "${D6_INTEGRATION_TESTS[@]}"; do
    if target=$(resolve_d6_target "$entry"); then
        count=$((count+1))
    else
        unresolved="$unresolved $entry"
    fi
done
echo "RESOLVED=$count"
echo "UNRESOLVED=[$unresolved]"
echo "LIST_SIZE=${#D6_INTEGRATION_TESTS[@]}"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5419_resolve");
    let out = stdout.trim().to_string();
    assert!(
        out.contains("UNRESOLVED=[]"),
        "every curated D6 entry must map to a Cargo test target; got:\n{}\n{}",
        out,
        stderr
    );
    assert!(
        out.contains("LIST_SIZE=49"),
        "the curated list must keep its 49 entries; got:\n{}",
        out
    );
    assert!(
        out.contains("RESOLVED=49"),
        "all 49 curated entries must resolve; got:\n{}",
        out
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// Create a directory containing a `cargo` stub that emulates the REAL
/// cargo test output, including the ANSI colour codes.
///
/// The CI workflow sets `CARGO_TERM_COLOR: always` globally, so cargo
/// emits
///     ESC[1m ESC[92m     Running ESC[0m <path> (target/debug/deps/...)
/// The colour codes sit BETWEEN the "Running" label and the path.
fn ansi_cargo_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_gate_ansi_stub_{}", tag));
    std::fs::create_dir_all(&dir).expect("create ansi stub dir");
    let cargo = dir.join("cargo");
    std::fs::write(
        &cargo,
        r#"#!/usr/bin/env bash
# stub cargo: emulate the real `cargo test` output, honouring CARGO_TERM_COLOR
targets=()
prev=""
for a in "$@"; do
  if [[ "$prev" == "--test" ]]; then targets+=("$a"); fi
  prev="$a"
done
for t in ${targets[@]+"${targets[@]}"}; do
  path="tests/probe/${t}.rs"
  if [[ "${CARGO_TERM_COLOR:-}" == "always" ]]; then
    printf '\033[1m\033[92m     Running\033[0m %s (target/debug/deps/%s-abcdef)\n' "$path" "$t"
  else
    printf '     Running %s (target/debug/deps/%s-abcdef)\n' "$path" "$t"
  fi
  printf 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n'
done
exit 0
"#,
    )
    .expect("write ansi stub cargo");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&cargo)
            .expect("stat ansi stub cargo")
            .permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&cargo, perm).expect("chmod ansi stub cargo");
    }
    dir
}

/// D6 must be able to attribute a verdict to a target whose cargo output
/// carries ANSI colour codes.
///
/// This is the exact defect observed in CI run 6335: the D6 awk matched
/// `/Running /` against the RAW log, but with `CARGO_TERM_COLOR: always`
/// the bytes between "Running" and the path are `ESC[0m`, so the pattern
/// never matched. `cur` stayed empty, every `test result:` line was
/// dropped by the `if (cur == "") next` guard, and all 47 D6 targets were
/// reported as "no verdict" — even though cargo exited 0 and every test
/// passed.
#[test]
fn test_5456_d6_parses_verdicts_from_ansi_coloured_cargo_output() {
    let stub = ansi_cargo_dir("d6_ansi");
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
# One fake target so the dimension is small and deterministic.
build_d6_target_map() {
  D6_TARGET_NAME_TO_STEM="d6_ansi_probe=d6_ansi_probe"
  D6_TARGET_NAME_TO_PATH="d6_ansi_probe=tests/probe/d6_ansi_probe.rs"
}
D6_INTEGRATION_TESTS=("d6_ansi_probe")
D6_TOTAL=0; D6_PASS=0; D6_FAIL=0
D6_FAILED_TESTS=(); D6_UNRESOLVED=(); D6_TARGETS=()
run_d6_integration_tests >/dev/null 2>&1
echo "D6_PASS=$D6_PASS D6_FAIL=$D6_FAIL"
"#;
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo_root())
        .env("PATH", path)
        // This is what the CI workflow sets for the whole job.
        .env("CARGO_TERM_COLOR", "always")
        .output()
        .expect("failed to spawn bash");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let result = stdout.trim().to_string();

    assert_eq!(
        field_u64(&result, "D6_PASS"),
        Some(1),
        "the ANSI-coloured 'Running' line must yield a verdict; got:\n{}\n{}",
        result,
        stderr
    );
    assert_eq!(
        field_u64(&result, "D6_FAIL"),
        Some(0),
        "no dimension failure expected; got:\n{}\n{}",
        result,
        stderr
    );
}

/// The D6 log parser must strip ANSI colour codes before matching.
///
/// This is a source-level guard: the strip has to happen BEFORE the
/// `/Running/` match, otherwise the regex sees `RunningESC[0m <path>` and
/// the "Running" token no longer ends the label.
#[test]
fn test_5456_d6_strips_ansi_before_parsing() {
    let text = std::fs::read_to_string(gate_script("check_rc_ga_gate.sh"))
        .expect("read check_rc_ga_gate.sh");
    let code = strip_comments(&text);

    let strip_pos = code.find("sed \"s/${ESC}").expect(
        "the D6 parser must strip ANSI sequences (sed on ${ESC}) before matching 'Running'",
    );
    let awk_pos = code
        .find("Running[[:space:]]")
        .expect("the D6 awk must match the 'Running' label");
    assert!(
        strip_pos < awk_pos,
        "the ANSI strip must come BEFORE the 'Running' match (strip at {}, match at {})",
        strip_pos,
        awk_pos
    );
}

/// The D6 dimension must attribute a verdict to the RIGHT target when the
/// cargo output is ANSI-coloured: the path that follows the colour codes
/// is the source path, and it must resolve back to the Cargo target name.
#[test]
fn test_5456_d6_ansi_verdict_is_attributed_to_the_right_target() {
    let stub = ansi_cargo_dir("d6_ansi_attr");
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
build_d6_target_map() {
  D6_TARGET_NAME_TO_STEM="d6_ansi_probe=d6_ansi_probe"
  D6_TARGET_NAME_TO_PATH="d6_ansi_probe=tests/probe/d6_ansi_probe.rs"
}
D6_INTEGRATION_TESTS=("d6_ansi_probe")
D6_TOTAL=0; D6_PASS=0; D6_FAIL=0
D6_FAILED_TESTS=(); D6_UNRESOLVED=(); D6_TARGETS=()
run_d6_integration_tests 2>&1
"#;
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo_root())
        .env("PATH", path)
        .env("CARGO_TERM_COLOR", "always")
        .output()
        .expect("failed to spawn bash");
    // The gate prints its own ANSI colours ("\u{1b}[0;32mPASS\u{1b}[0m D6: ..."),
    // so the log line has to be stripped before the plain-text match.
    let stdout = strip_ansi(String::from_utf8_lossy(&out.stdout).as_ref());
    assert!(
        stdout.contains("PASS D6: d6_ansi_probe"),
        "the verdict must be attributed to the Cargo target name; got:\n{}",
        stdout
    );
}

/// The declared contract (`gate_expected_dims`) and the executor's plan
/// (`plan_dims`) must agree for EVERY mode.
///
/// This is the defect observed in CI run 6335 (postcheck step 24): the
/// executor ran D3..D9 for the `ga` mode while `gate_expected_dims("ga")
/// demanded D1..D9, so every `ga` run failed with
/// "missing dimension(s): D1 D2" even though the gate itself was fine.
///
/// The two functions are deliberately kept separate (spec vs. impl) so
/// that this test is NOT vacuous: merging them would make the comparison
/// trivially true.
#[test]
fn test_5456_expected_dims_and_plan_dims_agree_for_every_mode() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
for mode in alpha beta rc ga all bogus; do
  echo "MODE=$mode EXPECTED=[$(gate_expected_dims "$mode")] PLAN=[$(plan_dims "$mode")]"
done
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5456_consistency");
    let out = stdout.trim().to_string();
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);

    let mut seen = 0;
    for line in out.lines() {
        if !line.starts_with("MODE=") {
            continue;
        }
        seen += 1;
        let expected = extract_between(line, "EXPECTED=[", "]");
        let plan = extract_between(line, "PLAN=[", "]");
        assert_eq!(
            expected, plan,
            "the declared contract and the executor plan must agree; got:\n{}",
            line
        );
    }
    assert_eq!(seen, 6, "every mode must be compared; got:\n{}", out);
}

/// Helper: return the substring between `start` and the following `]`.
fn extract_between(s: &str, start: &str, end: &str) -> String {
    let after = match s.find(start) {
        Some(i) => &s[i + start.len()..],
        None => return String::new(),
    };
    match after.find(end) {
        Some(j) => after[..j].to_string(),
        None => after.to_string(),
    }
}

/// The `all` plan must cover D1 and D2 — they are the alpha/beta stage
/// gates, and CI's postcheck job runs the `all` mode, so `all` is where
/// D1/D2 are actually validated.
#[test]
fn test_5456_all_plan_covers_d1_and_d2() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
echo "ALL=[$(plan_dims all)]"
echo "GA=[$(plan_dims ga)]"
echo "RC=[$(plan_dims rc)]"
echo "BETA=[$(plan_dims beta)]"
echo "ALPHA=[$(plan_dims alpha)]"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5456_all_plan");
    let out = stdout.trim().to_string();
    assert!(out.contains("ALL=[D1 D2 D3 D4 D5 D6 D7 D8 D9]"), "got:\n{}", out);
    assert!(out.contains("GA=[D3 D4 D5 D6 D7 D8 D9]"), "got:\n{}", out);
    assert!(out.contains("RC=[D3 D4 D5 D9]"), "got:\n{}", out);
    assert!(out.contains("BETA=[D2 D3 D4]"), "got:\n{}", out);
    assert!(out.contains("ALPHA=[D1 D3 D4]"), "got:\n{}", out);
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// `--list-dims` must print the PLAN (what would run), never the expected
/// set — otherwise the CLI-level consistency check would compare the
/// expected set with itself and prove nothing.
#[test]
fn test_5456_list_dims_prints_the_plan_not_the_expected_set() {
    let script = gate_script("check_rc_ga_gate.sh");
    let out = Command::new("bash")
        .arg(&script)
        .arg("ga")
        .arg("--list-dims")
        .current_dir(repo_root())
        .output()
        .expect("failed to spawn bash");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let plan = stdout.trim().to_string();
    assert_eq!(plan, "D3 D4 D5 D6 D7 D8 D9", "got:\n{}\n{}", stdout, stderr);

    // Sanity: the expected set for `ga` differs from the plan, otherwise
    // this test would be vacuous.
    let snippet = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
echo "[$(gate_expected_dims ga)]"
"#;
    let (_, so, _) = run_bash_snippet(snippet, "5456_list_dims_sanity");
    assert_ne!(
        so.trim(),
        plan,
        "the expected set and the plan must differ for 'ga', otherwise the \
         --list-dims check proves nothing"
    );
}

/// The D6 loop must iterate the whole array.
///
/// Pre-fix: `for t in ${D6_INTEGRATION_TESTS}` expands to `${ARRAY[0]}`
/// only — one element out of 51.
#[test]
fn test_5419_d6_loop_iterates_the_whole_array() {
    let text = std::fs::read_to_string(gate_script("check_rc_ga_gate.sh"))
        .expect("read check_rc_ga_gate.sh");
    let code = strip_comments(&text);
    assert!(
        !code.contains("for t in ${D6_INTEGRATION_TESTS}"),
        "the un-indexed array expansion must not come back"
    );
    assert!(
        code.contains("\"${D6_INTEGRATION_TESTS[@]}\""),
        "the D6 loop must iterate every array element"
    );
}

/// A D6 run that executed nothing must be a FAILURE, not a PASS.
///
/// This is the exact defect: the pre-fix script printed
/// "✓ PASS D6 summary: 0/0 integration tests PASS".
#[test]
fn test_5419_d6_zero_execution_is_a_failure() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
D6_INTEGRATION_TESTS=()
D6_TOTAL=0; D6_PASS=0; D6_FAIL=0
D6_FAILED_TESTS=(); D6_UNRESOLVED=(); D6_TARGETS=()
run_d6_integration_tests >/dev/null 2>&1
echo "D6_TOTAL=$D6_TOTAL D6_PASS=$D6_PASS D6_FAIL=$D6_FAIL"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5419_zero");
    let out = stdout.trim().to_string();
    assert_eq!(
        field_u64(&out, "D6_TOTAL"),
        Some(0),
        "no target was expected to run; got:\n{}\n{}",
        out,
        stderr
    );
    let fails = field_u64(&out, "D6_FAIL").unwrap_or(0);
    assert!(
        fails >= 1,
        "a D6 run that executed nothing must be a failure; got:\n{}\n{}",
        out,
        stderr
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// A curated entry that does not map to a Cargo target means the coverage
/// contract is not met — that is a failure, not a note.
#[test]
fn test_5419_unresolved_curated_entry_is_a_failure() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
D6_INTEGRATION_TESTS=("__definitely_not_a_cargo_target__")
D6_TOTAL=0; D6_PASS=0; D6_FAIL=0
D6_FAILED_TESTS=(); D6_UNRESOLVED=(); D6_TARGETS=()
run_d6_integration_tests >/dev/null 2>&1
echo "D6_FAIL=$D6_FAIL UNRESOLVED=[${D6_UNRESOLVED[*]}]"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5419_unresolved");
    let out = stdout.trim().to_string();
    let fails = field_u64(&out, "D6_FAIL").unwrap_or(0);
    assert!(
        fails >= 1,
        "an unresolvable curated entry must fail the dimension; got:\n{}\n{}",
        out,
        stderr
    );
    assert!(
        out.contains("__definitely_not_a_cargo_target__"),
        "the unresolvable entry must be named in the report; got:\n{}",
        out
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

/// D6 and D7 failures must reach the verdict.
#[test]
fn test_5419_d6_and_d7_failures_reach_the_verdict() {
    let script = r#"
set -uo pipefail
source scripts/gate/check_rc_ga_gate.sh
ALL="D1 D2 D3 D4 D5 D6 D7 D8 D9"
gate_verdict_for ga "$ALL" 0 0 0 0 0 0 0 0;   echo "GREEN=$GATE_VERDICT:$GATE_EXIT_CODE"
gate_verdict_for ga "$ALL" 0 0 0 0 1 0 0 0;   echo "D6FAIL=$GATE_VERDICT:$GATE_EXIT_CODE"
gate_verdict_for ga "$ALL" 0 0 0 0 0 1 0 0;   echo "D7FAIL=$GATE_VERDICT:$GATE_EXIT_CODE"
gate_verdict_for ga "$ALL" 0 0 0 0 0 0 1 0;   echo "D3FAIL=$GATE_VERDICT:$GATE_EXIT_CODE"
gate_verdict_for ga "D1 D2 D3" 0 0 0 0 0 0 0 0; echo "MISSING=$GATE_VERDICT:$GATE_EXIT_CODE [$GATE_MISSING_DIMS]"
"#;
    let (code, stdout, stderr) = run_bash_snippet(script, "5419_verdict");
    let out = stdout.trim().to_string();
    assert!(out.contains("GREEN=PASS:0"), "got:\n{}\n{}", out, stderr);
    assert!(
        out.contains("D6FAIL=FAIL:1"),
        "a failing D6 must fail the gate; got:\n{}",
        out
    );
    assert!(
        out.contains("D7FAIL=FAIL:1"),
        "a failing D7 must fail the gate; got:\n{}",
        out
    );
    assert!(
        out.contains("D3FAIL=FAIL:1"),
        "a failing D3 must fail the gate; got:\n{}",
        out
    );
    assert!(
        out.contains("MISSING=FAIL:1 [D4 D5 D6 D7 D8 D9]"),
        "dimensions that were expected but did not run must fail the gate; got:\n{}",
        out
    );
    assert_eq!(code, 0, "the snippet itself must succeed:\n{}", stderr);
}

// =============================================================================
// #5426 — the P14/P16 scanners missed the "swallowed exit code" shape
// =============================================================================

/// No gate script may swallow an exit code with `|| true` and then read it.
///
/// The pre-fix scanners only matched `cargo ... || true`, so they missed
///
///     output=$(bash "$script" 2>&1 || true)
///     local code=$?
///
/// This is the same rule the new P14 Check 5 / P16 step 2.6 enforce; it
/// is asserted here as well so that the invariant is covered by an
/// ordinary cargo test target.
#[test]
fn test_5426_no_gate_script_reads_a_swallowed_exit_code() {
    let mut offenders: Vec<String> = Vec::new();
    let gate_dir = repo_root().join("scripts/gate");
    let mut stack: Vec<std::path::PathBuf> = vec![gate_dir];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {:?}: {}", dir, e));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("sh") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_string();
            // The detectors contain the pattern they look for.
            if name == "check_drift_not_pass.sh" || name == "check_gate_test_integrity.sh" {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read gate script");
            let mut was_or_true = false;
            for (i, raw) in text.lines().enumerate() {
                let line = raw.trim_start();
                if line.starts_with('#') {
                    was_or_true = false;
                    continue;
                }
                if was_or_true && line.contains("$?") {
                    offenders.push(format!(
                        "{}:{}: swallowed exit code is read on the next line",
                        path.display(),
                        i + 1
                    ));
                }
                was_or_true = line.contains("|| true") || line.contains("||true");
            }
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "gate scripts must not swallow an exit code and then read it:\n{}",
        offenders.join("\n")
    );
}

/// The two other swallowed-exit-code sites in check_rc_ga_gate.sh (B5
/// integration gate, D4 WAL invariants) must capture the real exit code.
#[test]
fn test_5426_b5_and_d4_capture_the_real_exit_code() {
    let text = std::fs::read_to_string(gate_script("check_rc_ga_gate.sh"))
        .expect("read check_rc_ga_gate.sh");
    let code = strip_comments(&text);
    assert!(
        !code.contains("check_integration_gate.sh 2>&1 || true"),
        "B5 must capture the integration gate's real exit code"
    );
    assert!(
        !code.contains("wal_invariant.sh 2>&1 || true"),
        "D4 must capture the WAL invariant script's real exit code"
    );
    assert!(
        code.contains("check_integration_gate.sh 2>&1)"),
        "B5 must invoke the integration gate without masking its exit code"
    );
}

// =============================================================================
// #5456 — 摘要里的「未执行」必须写成 NOT RUN，而不是 0/0
// =============================================================================

/// 摘要里每一条维度行都必须先判断该维度是否在本模式的计划内。
///
/// 修前：不属于本模式的维度照样打印 `0/0`，读起来像「检查了 0 项、
/// 0 个 blocker」—— 也就是「干净通过」。实际含义是「从未执行」。
#[test]
fn test_5456_every_summary_dimension_line_is_guarded_by_dim_ran() {
    let text = std::fs::read_to_string(gate_script("check_rc_ga_gate.sh"))
        .expect("read check_rc_ga_gate.sh");
    let code = strip_comments(&text);

    // 摘要里出现的维度标签（键名）。
    let labels = [
        "D1-Alpha",
        "D2-Beta",
        "D5-DeepSeek",
        "D6-Integration",
        "D7-Reliability",
        "D8-V312-19",
        "D9-Coverage",
    ];

    for label in labels {
        let dim = label.split('-').next().expect("label has a dimension");
        let guarded = code
            .lines()
            .any(|l| l.trim_start().starts_with("if dim_ran") && l.contains(dim));
        assert!(
            guarded,
            "the summary line for `{}` must be guarded by `if dim_ran {}` — \
             a dimension that was never executed must not be printed as 0/0",
            label,
            dim
        );
    }
}

/// `NOT RUN` 分支必须明确写出「不在本模式计划内」，并指出它由哪个计划覆盖。
#[test]
fn test_5456_not_run_lines_name_the_plan_that_excludes_them() {
    let text = std::fs::read_to_string(gate_script("check_rc_ga_gate.sh"))
        .expect("read check_rc_ga_gate.sh");
    let code = strip_comments(&text);
    let not_run_lines = code
        .lines()
        .filter(|l| l.contains("NOT RUN"))
        .collect::<Vec<_>>();
    assert!(
        not_run_lines.len() >= 7,
        "every guarded dimension needs a NOT RUN branch; found {}:\n{}",
        not_run_lines.len(),
        not_run_lines.join("\n")
    );
    for line in not_run_lines {
        assert!(
            line.contains("not in '$GATE' plan"),
            "a NOT RUN line must name the plan that excluded the dimension; got: {}",
            line.trim()
        );
    }
}
