//! Gate behaviour contract — GPT review R1 (SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123), G1.
//!
//! The R1 review required that every harness gate prove six axes at the
//! **behavioural** level, and explicitly rejected static `swallow_hits`
//! counts as a substitute:
//!
//! | axis | meaning |
//! |------|---------|
//! | A | legal input ⇒ PASS |
//! | B | illegal input ⇒ FAIL |
//! | C | missing evidence ⇒ must not PASS |
//! | D | wrong SHA ⇒ must not PASS |
//! | E | dependency anomaly ⇒ must not PASS |
//! | F | zero execution ⇒ must not PASS |
//!
//! Every test below runs the **real gate script** against a purpose-built
//! fixture tree, and asserts the exit code. Each test fails on the
//! pre-fix scripts (verified by running the same suite against the parent
//! commit's `scripts/gate/`), because each one encodes a defect that was
//! demonstrated by an actual run, not inferred from the source.
//!
//! Defects this suite pins (all measured on the authoritative tree
//! `8cbde5b6d33c876fa1197474c46d50142a9823bd` on 2026-11-23):
//!
//! | gate | defect | symptom before the fix |
//! |------|--------|-------------------------|
//! | `check_drift_not_pass.sh` | missing baseline auto-created, findings absorbed, `FAIL=0` | 14 findings reported, then `✅ PASS`, rc=0 |
//! | `check_drift_not_pass.sh` | a scan that looked at 0 scripts | `✅ PASS` on an empty `scripts/gate/` |
//! | `check_gate_test_integrity.sh` | gate-referenced test file not found ⇒ WARN only | rc=0 with a `--test <nonexistent>` reference |
//! | `check_evidence_binding.sh` | a bare `#` counted as a CI reference | `## 全部测试通过` passed while `全部测试通过。` failed |
//! | `check_evidence_binding.sh` | negation word anywhere on the line exempted the line | `本批次不涉及其他阶段，全部测试通过` passed |
//! | `check_evidence_binding.sh` | `\| log \|` naming a missing file ⇒ WARN only | a doc citing a non-existent log passed |
//! | `check_architecture_freeze.sh` | A7-1 detected the *legal* `eng.execute()` entry, never the AD-002 bypass the spec names | `INFO (5 calls)` on a fully compliant tree; silent on a real bypass |
//! | `check_anti_fabrication.sh` | a stale `sqlancer-report.json` was certified without running sqlancer | `[PASS] sqlancer-report.json valid` with 0 sqlancer invocations |
//!
//! The fixtures are hermetic: no network, no production server, no
//! database, no cargo build. The only external binary is `python3`, which
//! the gates already require.

use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // This test lives in the root package, so CARGO_MANIFEST_DIR *is* the
    // repository root (same convention as tests/integration/gate/gate_honesty_test.rs).
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn gate_src(name: &str) -> PathBuf {
    repo_root().join("scripts/gate").join(name)
}

/// A unique scratch directory per test invocation.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_gate_contract_{}", tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn copy_file(src: &Path, dst: &Path) {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::copy(src, dst).expect("copy file");
}

/// Copy the whole `scripts/gate/` tree — the drift gate scans its siblings,
/// so a partial copy would change its verdict.
fn copy_gate_tree(dest_root: &Path) -> PathBuf {
    let src = repo_root().join("scripts/gate");
    let dst = dest_root.join("scripts/gate");
    copy_dir(&src, &dst);
    dst
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create dir");
    for entry in std::fs::read_dir(src).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("copy file");
        }
    }
}

/// Run a bash script and return (exit_code, combined_output).
fn run_bash(script: &Path, args: &[&str], cwd: &Path, extra_path: Option<&Path>) -> (i32, String) {
    let mut cmd = Command::new("bash");
    cmd.arg(script);
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    if let Some(p) = extra_path {
        let existing = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}:{}", p.display(), existing));
    }
    let out = cmd.output().expect("run bash");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

/// Write an executable shell script.
fn write_exec(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, body).expect("write script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(path).expect("stat").permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(path, perm).expect("chmod");
    }
}

// ---------------------------------------------------------------------------
// check_drift_not_pass.sh
// ---------------------------------------------------------------------------

/// Axis C for the drift gate: **a missing baseline must not PASS.**
///
/// Before the fix the gate created the baseline itself and reset its own
/// failure counter, so it reported 14 anti-patterns — including the
/// DRIFT-accepting `run_gate()` it exists to detect — and then printed
/// `✅ PASS — P14 satisfied` with rc=0.
#[test]
fn drift_gate_missing_baseline_must_not_pass() {
    let root = scratch("drift_missing_baseline");
    let gate_dir = copy_gate_tree(&root);
    // No tests/baseline/drift_baseline.json is created on purpose.

    let (code, out) = run_bash(&gate_dir.join("check_drift_not_pass.sh"), &[], &root, None);
    assert_ne!(
        code, 0,
        "a missing baseline must FAIL, not PASS (gate self-heals its reference data).\noutput:\n{}",
        out
    );
    assert!(
        out.contains("baseline not found") || out.contains("baseline"),
        "the failure must name the missing baseline, got:\n{}",
        out
    );
}

/// Axis F for the drift gate: **zero execution must not PASS.**
#[test]
fn drift_gate_zero_scanned_scripts_must_not_pass() {
    let root = scratch("drift_zero_scan");
    let gate_dir = root.join("scripts/gate");
    std::fs::create_dir_all(&gate_dir).expect("create gate dir");
    // Only the detector itself — it polices *other* gate scripts, so a tree
    // with nothing to police is a vacuous green.
    copy_file(
        &gate_src("check_drift_not_pass.sh"),
        &gate_dir.join("check_drift_not_pass.sh"),
    );
    std::fs::create_dir_all(root.join("tests/baseline")).expect("create baseline dir");
    std::fs::write(
        root.join("tests/baseline/drift_baseline.json"),
        "{\n  \"created_at\": \"2026-11-23T00:00:00Z\",\n  \"total_findings\": 0,\n  \"findings\": []\n}\n",
    )
    .expect("write baseline");

    let (code, out) = run_bash(&gate_dir.join("check_drift_not_pass.sh"), &[], &root, None);
    assert_ne!(
        code, 0,
        "a scan that examined 0 gate scripts must not PASS.\noutput:\n{}",
        out
    );
}

/// Axis A for the drift gate: a clean tree with a baseline ⇒ PASS.
#[test]
fn drift_gate_clean_tree_passes() {
    let root = scratch("drift_clean");
    let gate_dir = root.join("scripts/gate");
    std::fs::create_dir_all(&gate_dir).expect("create gate dir");
    // A minimal, clean gate directory: the detector plus one sibling gate
    // with no anti-patterns. Copying the whole real tree would drag in the
    // repository's genuine (baselined) findings, which are not what this
    // axis is about.
    copy_file(
        &gate_src("check_drift_not_pass.sh"),
        &gate_dir.join("check_drift_not_pass.sh"),
    );
    write_exec(
        &gate_dir.join("check_dshb_clean_sibling.sh"),
        "#!/usr/bin/env bash\necho 'clean gate'\nexit 0\n",
    );
    std::fs::create_dir_all(root.join("tests/baseline")).expect("create baseline dir");
    std::fs::write(
        root.join("tests/baseline/drift_baseline.json"),
        "{\n  \"created_at\": \"2026-11-23T00:00:00Z\",\n  \"total_findings\": 0,\n  \"findings\": []\n}\n",
    )
    .expect("write baseline");

    let (code, out) = run_bash(&gate_dir.join("check_drift_not_pass.sh"), &[], &root, None);
    assert_eq!(
        code, 0,
        "a clean tree with a baseline must PASS.\noutput:\n{}",
        out
    );
}

/// Axis B for the drift gate: a **new** anti-pattern ⇒ FAIL.
#[test]
fn drift_gate_new_antipattern_fails() {
    let root = scratch("drift_new_antipattern");
    let gate_dir = root.join("scripts/gate");
    std::fs::create_dir_all(&gate_dir).expect("create gate dir");
    copy_file(
        &gate_src("check_drift_not_pass.sh"),
        &gate_dir.join("check_drift_not_pass.sh"),
    );
    write_exec(
        &gate_dir.join("check_dshb_clean_sibling.sh"),
        "#!/usr/bin/env bash\necho 'clean gate'\nexit 0\n",
    );
    std::fs::create_dir_all(root.join("tests/baseline")).expect("create baseline dir");
    std::fs::write(
        root.join("tests/baseline/drift_baseline.json"),
        "{\n  \"created_at\": \"2026-11-23T00:00:00Z\",\n  \"total_findings\": 0,\n  \"findings\": []\n}\n",
    )
    .expect("write baseline");
    write_exec(
        &gate_dir.join("check_dshb_contract_probe.sh"),
        "#!/usr/bin/env bash\ncargo test --lib --contract-probe 2>&1 | grep 'test result' || true\n",
    );

    let (code, out) = run_bash(&gate_dir.join("check_drift_not_pass.sh"), &[], &root, None);
    assert_ne!(
        code, 0,
        "a newly introduced `|| true` swallow must FAIL.\noutput:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// check_gate_test_integrity.sh
// ---------------------------------------------------------------------------

/// Axis E for the P16 gate: **a gate-referenced test that cannot be
/// located is a broken dependency and must not PASS.**
///
/// Before the fix this produced `WARN: gate test ... not found in any
/// tests/ dir (referenced but missing)` and rc=0 — the gate certified a
/// reference it could not verify, and the referenced test would never run.
#[test]
fn gate_test_integrity_missing_test_reference_must_not_pass() {
    let root = scratch("p16_missing_test");
    let gate_dir = copy_gate_tree(&root);
    std::fs::create_dir_all(root.join("tests/baseline")).expect("create baseline dir");
    std::fs::write(
        root.join("tests/baseline/gate_test_baseline.json"),
        "{\n  \"created_at\": \"2026-11-23T00:00:00Z\",\n  \"gate_test_count\": 1,\n  \"total_ignore_hits\": 0,\n  \"gate_tests\": [\"dsb_contract_missing_test\"],\n  \"violations\": []\n}\n",
    )
    .expect("write baseline");
    write_exec(
        &gate_dir.join("check_dshb_contract_probe.sh"),
        "#!/usr/bin/env bash\ncargo test --test dsb_contract_missing_test\n",
    );

    let (code, out) = run_bash(
        &gate_dir.join("check_gate_test_integrity.sh"),
        &[],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a gate script referencing a non-existent test must FAIL, not WARN.\noutput:\n{}",
        out
    );
    assert!(
        out.contains("dsb_contract_missing_test"),
        "the failure must name the missing test, got:\n{}",
        out
    );
}

/// Axis C for the P16 gate: a missing baseline must not PASS (this was
/// already fail-closed under #5113 — pinned so it stays that way).
#[test]
fn gate_test_integrity_missing_baseline_must_not_pass() {
    let root = scratch("p16_missing_baseline");
    let gate_dir = copy_gate_tree(&root);

    let (code, out) = run_bash(
        &gate_dir.join("check_gate_test_integrity.sh"),
        &[],
        &root,
        None,
    );
    assert_ne!(code, 0, "a missing baseline must FAIL.\noutput:\n{}", out);
}

/// Axis F for the P16 gate: zero gate-referenced tests ⇒ must not PASS.
#[test]
fn gate_test_integrity_zero_tests_must_not_pass() {
    let root = scratch("p16_zero_tests");
    let gate_dir = root.join("scripts/gate");
    std::fs::create_dir_all(&gate_dir).expect("create gate dir");
    copy_file(
        &gate_src("check_gate_test_integrity.sh"),
        &gate_dir.join("check_gate_test_integrity.sh"),
    );

    let (code, out) = run_bash(
        &gate_dir.join("check_gate_test_integrity.sh"),
        &[],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a scan that found 0 gate-referenced tests must not PASS.\noutput:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// check_evidence_binding.sh
// ---------------------------------------------------------------------------

/// Build a minimal git-backed fixture tree for the evidence-binding gate.
fn evidence_fixture(tag: &str) -> (PathBuf, PathBuf) {
    let root = scratch(tag);
    let gate_dir = copy_gate_tree(&root);
    std::fs::create_dir_all(root.join("docs/releases/v9.9.9-contract")).expect("create docs dir");
    let init = Command::new("git")
        .args(["init", "-q", "."])
        .current_dir(&root)
        .status()
        .expect("git init");
    assert!(init.success());
    for (k, v) in [
        ("user.email", "gate@example.invalid"),
        ("user.name", "gate"),
    ] {
        let _ = Command::new("git")
            .args(["config", k, v])
            .current_dir(&root)
            .status();
    }
    (root, gate_dir)
}

fn git_add_commit(root: &Path) {
    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(root)
        .status()
        .expect("git add");
    assert!(add.success());
    let commit = Command::new("git")
        .args(["commit", "-qm", "fixture"])
        .current_dir(root)
        .status()
        .expect("git commit");
    assert!(commit.success());
}

/// Axis B/C for the evidence gate: a fabricated PASS claim with no evidence
/// ⇒ FAIL.
#[test]
fn evidence_binding_fabricated_claim_fails() {
    let (root, gate_dir) = evidence_fixture("evidence_no_evidence");
    std::fs::write(
        root.join("docs/releases/v9.9.9-contract/NOEVID.md"),
        "# Report\n\n全部测试通过。\n",
    )
    .expect("write doc");
    git_add_commit(&root);

    let out_dir = root.join("out");
    let (code, out) = run_bash(
        &gate_dir.join("check_evidence_binding.sh"),
        &["v9.9.9-contract", out_dir.to_str().unwrap()],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a PASS claim with no CI/gate/commit evidence must FAIL.\noutput:\n{}",
        out
    );
}

/// The same defect, hidden behind a Markdown heading marker.
///
/// Before the fix the CI-reference regex contained a **bare `#`**, which is
/// the Markdown heading character. `全部测试通过。` was correctly reported
/// as a Type A/B violation, while the identical claim carrying a `##`
/// heading marker was silently accepted. This is the single most direct
/// false-negative in the gate: the verdict depended on the *punctuation* of
/// the line rather than on its meaning.
#[test]
fn evidence_binding_heading_marker_does_not_excuse_claim() {
    let (root, gate_dir) = evidence_fixture("evidence_heading");
    std::fs::write(
        root.join("docs/releases/v9.9.9-contract/HEADING.md"),
        "# Report\n\n## 全部测试通过\n",
    )
    .expect("write doc");
    git_add_commit(&root);

    let out_dir = root.join("out");
    let (code, out) = run_bash(
        &gate_dir.join("check_evidence_binding.sh"),
        &["v9.9.9-contract", out_dir.to_str().unwrap()],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a Markdown heading marker must not turn an unevidenced claim into a pass.\noutput:\n{}",
        out
    );
}

/// A negation word in an **unrelated clause** must not exempt the line.
///
/// The old rule scanned the whole line for `不 / 未 / 无 / 非`, so a single
/// unrelated negation anywhere on the line exempted every claim on it.
/// The rule now applies the negation to the clause that carries the claim.
#[test]
fn evidence_binding_unrelated_negation_does_not_excuse_claim() {
    let (root, gate_dir) = evidence_fixture("evidence_negation");
    std::fs::write(
        root.join("docs/releases/v9.9.9-contract/NEG.md"),
        "# Report\n\n本批次不涉及其他阶段，全部测试通过\n",
    )
    .expect("write doc");
    git_add_commit(&root);

    let out_dir = root.join("out");
    let (code, out) = run_bash(
        &gate_dir.join("check_evidence_binding.sh"),
        &["v9.9.9-contract", out_dir.to_str().unwrap()],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "an unrelated negation clause must not exempt a positive claim.\noutput:\n{}",
        out
    );
}

/// A genuinely negated statement is NOT a claim and must not be reported.
#[test]
fn evidence_binding_negated_statement_is_not_a_violation() {
    let (root, gate_dir) = evidence_fixture("evidence_negated_ok");
    std::fs::write(
        root.join("docs/releases/v9.9.9-contract/NEGOK.md"),
        "# Report\n\n本批次没有改变任何一道门禁的『通过/不通过』结论。\n",
    )
    .expect("write doc");
    git_add_commit(&root);

    let out_dir = root.join("out");
    let (code, out) = run_bash(
        &gate_dir.join("check_evidence_binding.sh"),
        &["v9.9.9-contract", out_dir.to_str().unwrap()],
        &root,
        None,
    );
    assert_eq!(
        code, 0,
        "a negated statement is not a claim and must not be reported.\noutput:\n{}",
        out
    );
}

/// Axis D for the evidence gate: a wrong SHA ⇒ FAIL.
#[test]
fn evidence_binding_wrong_sha_fails() {
    let (root, gate_dir) = evidence_fixture("evidence_wrong_sha");
    std::fs::write(
        root.join("docs/releases/v9.9.9-contract/BADSHA.md"),
        "# Report\n\n**Commit**: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n\n测试通过。\n",
    )
    .expect("write doc");
    git_add_commit(&root);

    let out_dir = root.join("out");
    let (code, out) = run_bash(
        &gate_dir.join("check_evidence_binding.sh"),
        &["v9.9.9-contract", out_dir.to_str().unwrap()],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a commit hash that does not exist must FAIL.\noutput:\n{}",
        out
    );
}

/// Axis E for the evidence gate: a `| log |` row naming a **missing**
/// file is a broken dependency and must not be reported as a mere warning.
#[test]
fn evidence_binding_missing_log_file_must_not_pass() {
    let (root, gate_dir) = evidence_fixture("evidence_missing_log");
    std::fs::write(
        root.join("docs/releases/v9.9.9-contract/MISSINGLOG.md"),
        "# Report\n\n| evidence_hash | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa |\n| log | `no/such/log.txt` |\n\n测试通过。\n",
    )
    .expect("write doc");
    git_add_commit(&root);

    let out_dir = root.join("out");
    let (code, out) = run_bash(
        &gate_dir.join("check_evidence_binding.sh"),
        &["v9.9.9-contract", out_dir.to_str().unwrap()],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a doc citing a non-existent log file must FAIL, not WARN.\noutput:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// check_architecture_freeze.sh
// ---------------------------------------------------------------------------

/// A minimal, compliant architecture tree for the A7 checks.
fn architecture_fixture(tag: &str) -> (PathBuf, PathBuf) {
    let root = scratch(tag);
    let gate_dir = copy_gate_tree(&root);
    std::fs::create_dir_all(root.join("crates/executor/src")).expect("create crates");
    std::fs::create_dir_all(root.join("crates/mysql-server/src")).expect("create crates");
    std::fs::create_dir_all(root.join("src")).expect("create src");
    std::fs::write(
        root.join("crates/executor/src/drift_gate.rs"),
        "pub struct DriftGate;\nimpl DriftGate {\n    pub fn validate(&self) -> Result<(), String> { Ok(()) }\n}\n#[cfg(test)]\nmod tests {\n    use super::DriftGate;\n    #[test]\n    fn rejects() { assert!(DriftGate.validate().is_err()); }\n}\n",
    )
    .expect("write drift gate");
    std::fs::write(
        root.join("crates/executor/src/tx.rs"),
        "pub struct TransactionContext;\n",
    )
    .expect("write tx ctx");
    std::fs::write(
        root.join("crates/mysql-server/src/lib.rs"),
        "pub fn handler() {}\n",
    )
    .expect("write handler");
    std::fs::write(root.join("src/execution_engine.rs"), "fn main() {}\n").expect("write ee");
    (root, gate_dir)
}

/// Axis A for the architecture gate: a compliant tree ⇒ PASS.
#[test]
fn architecture_freeze_compliant_tree_passes() {
    let (root, gate_dir) = architecture_fixture("arch_ok");
    let (code, out) = run_bash(
        &gate_dir.join("check_architecture_freeze.sh"),
        &[],
        &root,
        None,
    );
    assert_eq!(code, 0, "a compliant tree must PASS.\noutput:\n{}", out);
}

/// Axis B for the architecture gate: a **real** AD-002 bypass in a
/// compiled server module must be reported.
///
/// Before the fix A7-1 grepped `eng.execute(` — the *legal* single-path
/// entry — so it reported "INFO (5 calls)" on a fully compliant tree and
/// stayed silent on a tree with a genuine bypass. The check had zero
/// discriminating power, which `GATE_CATALOG.jsonl` itself admits
/// ("不可证伪（kind=unfalsifiable）").
#[test]
fn architecture_freeze_detects_real_ad002_bypass() {
    let (root, gate_dir) = architecture_fixture("arch_bypass");
    std::fs::write(
        root.join("crates/mysql-server/src/lib.rs"),
        "pub fn handler() {\n    let _ = storage.insert(\"t\", b\"k\", b\"v\");\n}\n",
    )
    .expect("write bypass");

    let (code, out) = run_bash(
        &gate_dir.join("check_architecture_freeze.sh"),
        &[],
        &root,
        None,
    );
    assert!(
        out.contains("storage DML bypass"),
        "A7-1 must report the AD-002 bypass it is specified to detect (SPEC-011 §3.3).\noutput:\n{}",
        out
    );
    assert_ne!(
        code, 0,
        "a real AD-002 bypass must not report a clean PASS.\noutput:\n{}",
        out
    );
}

/// Axis C/E for the architecture gate: when the A7-1 detector is missing,
/// the check **cannot be completed** — that is INCOMPLETE (2), not PASS (0).
#[test]
fn architecture_freeze_missing_detector_is_incomplete() {
    let (root, gate_dir) = architecture_fixture("arch_no_detector");
    std::fs::remove_file(gate_dir.join("architecture_freeze_bypass_detector.py"))
        .expect("remove detector");

    let (code, out) = run_bash(
        &gate_dir.join("check_architecture_freeze.sh"),
        &[],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "a missing detector means the check could not be completed ⇒ not PASS.\noutput:\n{}",
        out
    );
}

/// Axis B for the architecture gate: removing the DriftGate path must FAIL
/// (A7-2 is the fail-closed reference implementation of "illegal input ⇒
/// FAIL" inside this gate).
#[test]
fn architecture_freeze_missing_drift_gate_fails() {
    let (root, gate_dir) = architecture_fixture("arch_no_drift");
    std::fs::write(
        root.join("crates/executor/src/drift_gate.rs"),
        "pub struct NotADriftGate;\n",
    )
    .expect("write non-drift");

    let (code, out) = run_bash(
        &gate_dir.join("check_architecture_freeze.sh"),
        &[],
        &root,
        None,
    );
    assert_ne!(
        code, 0,
        "removing the DriftGate path must FAIL.\noutput:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// check_anti_fabrication.sh
// ---------------------------------------------------------------------------

/// Build a stub environment for the anti-fabrication gate: a `cargo` that
/// always succeeds, a `sqlancer` that records its invocations, and a
/// `test-runner` that emits a schema-valid report.
fn afp_fixture(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(tag);
    let gate_dir = copy_gate_tree(&root);
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).expect("create bin");
    write_exec(
        &bin.join("cargo"),
        "#!/usr/bin/env bash\ncase \"$*\" in\n  *metadata*) echo '{\"packages\":[{\"name\":\"stub\"}]}' ;;\n  *--message-format=json*) echo '{\"reason\":\"compiler-artifact\"}' ;;\nesac\nexit 0\n",
    );
    std::fs::create_dir_all(root.join("target/release")).expect("create target");
    write_exec(
        &root.join("target/release/sqlancer"),
        &format!(
            "#!/usr/bin/env bash\necho \"INVOKED $@\" >> {}/sqlancer_invocations.log\n",
            root.display()
        ),
    );
    write_exec(
        &root.join("target/release/test-runner"),
        &format!(
            "#!/usr/bin/env bash\ncat > {}/target/test-runner-report.json <<'JSON'\n{{\"started_at\":\"2026-11-23T00:00:00Z\",\"finished_at\":\"2026-11-23T00:00:01Z\",\"config\":{{\"manifest\":\"x\"}},\"summary\":{{\"total\":1,\"passed\":1,\"failed\":0,\"crashed\":0,\"total_duration_ms\":12}},\"results\":[{{\"name\":\"t\",\"status\":\"passed\"}}]}}\nJSON\n",
            root.display()
        ),
    );
    std::fs::create_dir_all(root.join("config")).expect("create config");
    std::fs::write(
        root.join("config/test-runner-manifest.toml"),
        "[[test]]\nbinary = \"target/release/stub\"\n",
    )
    .expect("write manifest");
    (root, gate_dir, bin)
}

/// Axis F for the anti-fabrication gate: a **stale** sqlancer report must
/// not be certified.
///
/// Before the fix the sqlancer report was only regenerated when missing, so
/// a report left by any earlier invocation was accepted as this run's
/// result — the gate printed `[PASS] target/sqlancer-report.json valid`
/// while the sqlancer binary was never invoked. The test-runner branch of
/// the same gate had already been fixed for exactly this reason.
#[test]
fn anti_fabrication_stale_sqlancer_report_must_not_be_certified() {
    let (root, gate_dir, bin) = afp_fixture("afp_stale_sqlancer");
    // A schema-valid report left over from an earlier run.
    std::fs::write(
        root.join("target/sqlancer-report.json"),
        "{\"successful_queries\": 10, \"failed_queries\": 0, \"iterations_requested\": 100}\n",
    )
    .expect("write stale report");
    let log = root.join("sqlancer_invocations.log");

    let (_code, out) = run_bash(
        &gate_dir.join("check_anti_fabrication.sh"),
        &[],
        &root,
        Some(&bin),
    );
    assert!(
        log.exists(),
        "the gate must re-run sqlancer instead of certifying a stale report.\noutput:\n{}",
        out
    );
}

/// Axis C for the anti-fabrication gate: a test-runner report that
/// executed **nothing** must not be certified (the `results is empty`
/// guard).
#[test]
fn anti_fabrication_zero_tests_report_must_not_pass() {
    let (root, gate_dir, bin) = afp_fixture("afp_zero_tests");
    write_exec(
        &root.join("target/release/test-runner"),
        &format!(
            "#!/usr/bin/env bash\ncat > {}/target/test-runner-report.json <<'JSON'\n{{\"started_at\":\"2026-11-23T00:00:00Z\",\"finished_at\":\"2026-11-23T00:00:01Z\",\"config\":{{\"manifest\":\"x\"}},\"summary\":{{\"total\":0,\"passed\":0,\"failed\":0,\"crashed\":0,\"total_duration_ms\":0}},\"results\":[]}}\nJSON\n",
            root.display()
        ),
    );

    let (code, out) = run_bash(
        &gate_dir.join("check_anti_fabrication.sh"),
        &[],
        &root,
        Some(&bin),
    );
    assert_ne!(
        code, 0,
        "a report that executed no tests must not PASS.\noutput:\n{}",
        out
    );
}

/// Axis E for the anti-fabrication gate: a missing test-runner manifest
/// must not PASS (the runner would fall back to probe mode and synthesise
/// a `Crashed` entry).
#[test]
fn anti_fabrication_missing_manifest_must_not_pass() {
    let (root, gate_dir, bin) = afp_fixture("afp_missing_manifest");
    std::fs::remove_file(root.join("config/test-runner-manifest.toml")).expect("remove manifest");

    let (code, out) = run_bash(
        &gate_dir.join("check_anti_fabrication.sh"),
        &[],
        &root,
        Some(&bin),
    );
    assert_ne!(
        code, 0,
        "a missing test-runner manifest must not PASS.\noutput:\n{}",
        out
    );
}
