//! Gate criterion existence regression suite — Issues #5457 / #5458
//!
//! These tests assert that the gate criterion documents only reference
//! scripts that actually exist, and that the three scripts introduced to
//! enforce that rule behave correctly.
//!
//! | Issue | Defect | Symptom |
//! |-------|--------|---------|
//! | #5457 | `scripts/gate/verify_pr_dag.sh` did not exist, yet `GATE_CONDITIONS.md` cites it as the criterion for BE5 (line 103) and B-F5 (line 129) | The criterion was NOT EXECUTABLE — no PASS, no FAIL, ever. v3.11.0 and v4.0.0 BETA gate reports had already recorded it as "PASS" with no reproducible way to re-verify. |
//! | #5457 | `scripts/gate/check_pr_dag.sh` — the SAME gate under a SECOND name, cited by `DEVELOPMENT_PLAN.md:92` and `ALPHA_STAGE_REVIEW.md:81` | A second dangling citation for the same missing gate. |
//! | #5450 | Same class: 3 of 4 scripts in `RELEASE_GATE_CHECKLIST.md` do not exist | Third instance of "criterion table accepted without an existence check" |
//! | #5458 | `AGENTS.md` cited a `PR_TEMPLATE.md` path that does not exist | Fourth instance, in the agent-facing entry point |
//!
//! Design notes
//! ------------
//! * The scripts are *shell*, so the CLI-level tests drive them with real
//!   temporary fixture files rather than mocking bash internals. A
//!   fabricated PR body must produce a FAIL; an empty one must not be
//!   reported as PASS.
//! * Every assertion here is about *the script's own behaviour*, never
//!   about a document's prose. Per ADR-008 Policy 1, "the document says
//!   the right thing" is not evidence that the check works.
//! * This file MUST NOT carry `#[ignore]`: once a gate script names a
//!   cargo test target, that test becomes a GATE TEST, and an ignored gate
//!   test is exactly the "gate PASS with the test never running" defect
//!   this suite exists to prevent (ADR-008 Policy 2 / P16).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the workspace root for the root package, which
    // is where scripts/ and docs/ live. Matches the convention already used
    // by tests/integration/gate/gate_honesty_test.rs.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn script_path(name: &str) -> PathBuf {
    repo_root().join("scripts").join("gate").join(name)
}

fn run_script(name: &str, args: &[&str]) -> (i32, String) {
    let out = Command::new("bash")
        .arg(script_path(name))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("failed to spawn gate script");
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), combined)
}

fn write_temp_fixture(name: &str, contents: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo-gate-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(name);
    fs::write(&path, contents).expect("write fixture");
    path
}

/// A minimal cross-process lock built on `O_EXCL` lock files.
///
/// Some tests in this suite touch the same criterion document: one reads
/// it, one temporarily mutates it. Rust runs the tests of a binary on
/// separate threads, so without exclusion the reading test can observe
/// the mutation and fail. `O_EXCL` creation is atomic on POSIX, so this
/// needs no dependency and works for concurrent processes too.
struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(name: &str) -> FileLock {
        let dir = std::env::temp_dir().join(format!("sqlrustgo-gate-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(format!("{name}.lock"));
        // Bounded spin: a stale lock (e.g. from a killed run) must not hang
        // the suite forever.
        for _ in 0..600 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return FileLock { path },
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
        // A stale lock would otherwise deadlock the suite. Break it and
        // take it; the mutation it guards is short-lived.
        let _ = fs::remove_file(&path);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("acquire lock after breaking a stale one");
        FileLock { path }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// verify_pr_dag.sh
// ---------------------------------------------------------------------------

#[test]
fn verify_pr_dag_script_exists() {
    assert!(
        script_path("verify_pr_dag.sh").exists(),
        "verify_pr_dag.sh must exist — GATE_CONDITIONS.md cites it for BE5 / B-F5"
    );
}

#[test]
fn verify_pr_dag_script_is_executable() {
    let meta = fs::metadata(script_path("verify_pr_dag.sh")).expect("stat");
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "verify_pr_dag.sh must be executable"
    );
}

#[test]
fn check_pr_dag_alias_exists_and_delegates() {
    // The same gate is cited under a SECOND name by
    // DEVELOPMENT_PLAN.md:92 and ALPHA_STAGE_REVIEW.md:81. Both citations
    // must resolve.
    let alias = script_path("check_pr_dag.sh");
    assert!(
        alias.exists(),
        "check_pr_dag.sh must exist — DEVELOPMENT_PLAN.md:92 cites it"
    );
    let meta = fs::metadata(&alias).expect("stat");
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "check_pr_dag.sh must be executable"
    );

    // The alias must behave identically to the implementation it delegates
    // to. An unresolvable base ref is a cheap, deterministic probe.
    let (alias_code, alias_out) = run_script(
        "check_pr_dag.sh",
        &["--base", "refs/heads/definitely_nonexistent_ref_zzz"],
    );
    let (impl_code, impl_out) = run_script(
        "verify_pr_dag.sh",
        &["--base", "refs/heads/definitely_nonexistent_ref_zzz"],
    );
    assert_eq!(
        alias_code, impl_code,
        "the alias must return the same exit code as the implementation.\nalias:\n{alias_out}\nimpl:\n{impl_out}"
    );
}

#[test]
fn verify_pr_dag_rejects_a_plan_without_a_pr_dag_section() {
    // A document with no PR DAG section cannot be judged. Per ADR-006 P14
    // and AFP §7.4, DRIFT must not be reported as PASS.
    let path = write_temp_fixture(
        "no_dag_plan.md",
        "# Some Release Plan\n\n## 1. Goals\n\nNothing to verify here.\n",
    );
    let (code, out) = run_script(
        "verify_pr_dag.sh",
        &["--plan", path.to_str().unwrap(), "--allow-shallow"],
    );
    assert_eq!(
        code, 2,
        "a plan without a PR DAG section must be DRIFT (exit 2), not PASS. Output:\n{out}"
    );
    assert!(
        out.contains("DRIFT"),
        "the output must state that this is DRIFT. Output:\n{out}"
    );
}

#[test]
fn verify_pr_dag_reports_drift_for_unresolvable_base() {
    // An unresolvable base ref means the check CANNOT judge.
    let (code, out) = run_script(
        "verify_pr_dag.sh",
        &["--base", "refs/heads/definitely_nonexistent_ref_zzz"],
    );
    assert_eq!(
        code, 2,
        "an unresolvable base ref must be DRIFT (exit 2), not PASS. Output:\n{out}"
    );
    assert!(
        out.contains("DRIFT"),
        "the output must state that this is DRIFT. Output:\n{out}"
    );
}

#[test]
fn verify_pr_dag_detects_a_false_pass() {
    // The dangerous direction: a PR declared as merged with no implementation
    // commit behind it.
    //
    // PR-810 is used because it genuinely has ZERO implementation commits
    // in the repository's history (measured 2026-10-22) — it is one of the
    // ADR-010 ghost PRs. PR-800 was used before, but it turned out to have
    // 8 implementation commits, so declaring it merged was no longer a
    // false pass and the test stopped testing what it claimed to.
    let fixture = "\
## 2. PR DAG（依赖执行图）

PR-810  ExecutionEngine → Router            ← 解耦层（先做）

### 2.1 PR DAG 实际执行状态

| PR | 规划阶段 | 实际状态 | 说明 |
|----|----------|----------|------|
| PR-810 (ExecutionEngine → Router) | Phase 0 | ✅ 已合并 | fabricated |
";
    let path = write_temp_fixture("false_pass_plan.md", fixture);
    let (code, out) = run_script(
        "verify_pr_dag.sh",
        &["--plan", path.to_str().unwrap(), "--allow-shallow"],
    );
    assert_eq!(
        code, 1,
        "a PR declared merged with no merge commit must FAIL (exit 1). Output:\n{out}"
    );
    assert!(
        out.contains("false pass"),
        "the failure must say this is a false pass. Output:\n{out}"
    );
}

#[test]
fn verify_pr_dag_detects_an_inverted_dependency() {
    // PR-810 declares a dependency on PR-900 but appears before it in the
    // chain — a topological inversion.
    let fixture = "\
## 2. PR DAG（依赖执行图）

PR-810  ExecutionEngine → Router            ← 解耦层（依赖900）
   ↓
PR-900  ExecutionEngine 拆分清理             ← 收尾重构

### 2.1 PR DAG 实际执行状态

| PR | 规划阶段 | 实际状态 | 说明 |
|----|----------|----------|------|
| PR-810 (ExecutionEngine → Router) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
| PR-900 (ExecutionEngine 拆分清理) | Phase 4 | ❌ 未合并 | 无 Issue 追踪 |
";
    // The fixture name deliberately does NOT contain the word "inverted":
    // the script echoes the plan path, so a fixture named "inverted_*.md"
    // would satisfy a substring assertion even when the check was disabled.
    // (That was the defect: the assertion was satisfied by the PATH.)
    let path = write_temp_fixture("dep_order_plan.md", fixture);
    let (code, out) = run_script(
        "verify_pr_dag.sh",
        &["--plan", path.to_str().unwrap(), "--allow-shallow"],
    );
    assert_eq!(
        code, 1,
        "an inverted dependency must FAIL (exit 1). Output:\n{out}"
    );
    assert!(
        out.contains("Dependency order inverted"),
        "the failure must say the dependency order is inverted. Output:\n{out}"
    );
    // Guard against a second path to exit 1: the fixture must trigger the
    // INVERSION specifically, not some other structural error.
    assert!(
        !out.contains("which is not a node in the §2 chain"),
        "the fixture must isolate the inversion check; it also tripped an \
         unrelated structural error. Output:\n{out}"
    );
}

#[test]
fn verify_pr_dag_detects_duplicate_status_rows() {
    // A PR appearing TWICE in the §2.1 status table is a self-contradictory
    // document. Taking the first row would silently ignore the second, which
    // is exactly how a false green hides. (Found by adversarial testing on
    // 2026-10-22: a table with "❌ 未合并" then "✅ 已合并" was judged by
    // the first row only.)
    let fixture = "\
## 2. PR DAG（依赖执行图）

PR-800  COM_QUERY AST Routing (L0)           ← 入口重构（先做）

### 2.1 PR DAG 实际执行状态

| PR | 规划阶段 | 实际状态 | 说明 |
|----|----------|----------|------|
| PR-800 (COM_QUERY AST Routing) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
| PR-800 (COM_QUERY AST Routing) | Phase 0 | ✅ 已合并 | duplicate row |
";
    let path = write_temp_fixture("duplicate_rows_plan.md", fixture);
    let (code, out) = run_script(
        "verify_pr_dag.sh",
        &["--plan", path.to_str().unwrap(), "--allow-shallow"],
    );
    assert_eq!(
        code, 1,
        "a PR with duplicate status rows must FAIL (exit 1). Output:\n{out}"
    );
    assert!(
        out.contains("times in the §2.1 status table"),
        "the failure must say the rows are duplicated. Output:\n{out}"
    );
}

#[test]
fn verify_pr_dag_detects_a_status_row_outside_the_chain() {
    // A §2.1 row whose PR is not a §2 chain node would never be checked by
    // a chain-driven loop — so a claimed-merged PR nobody verifies is
    // exactly the false green this gate exists to remove. (Found by
    // adversarial testing on 2026-10-22.)
    let fixture = "\
## 2. PR DAG（依赖执行图）

PR-800  COM_QUERY AST Routing (L0)           ← 入口重构（先做）

### 2.1 PR DAG 实际执行状态

| PR | 规划阶段 | 实际状态 | 说明 |
|----|----------|----------|------|
| PR-800 (COM_QUERY AST Routing) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
| PR-999 (not a chain node) | Phase 9 | ✅ 已合并 | fabricated |
";
    let path = write_temp_fixture("row_outside_chain_plan.md", fixture);
    let (code, out) = run_script(
        "verify_pr_dag.sh",
        &["--plan", path.to_str().unwrap(), "--allow-shallow"],
    );
    assert_eq!(
        code, 1,
        "a status row outside the chain must FAIL (exit 1). Output:\n{out}"
    );
    assert!(
        out.contains("NOT a node in the §2 chain"),
        "the failure must say the row is outside the chain. Output:\n{out}"
    );
}

/// True when this checkout has a truncated history, in which case the gate
/// reports DRIFT instead of a verdict.
///
/// CI checks out with a shallow depth (see .gitea/workflows/ci.yml), so the
/// gate cannot judge there — it says so and exits 2. A test that demanded
/// exit 0 unconditionally would therefore FAIL the CI test job for a
/// reason that has nothing to do with the code under test.
fn checkout_is_shallow() -> bool {
    let out = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo_root())
        .output()
        .expect("git rev-parse");
    if !out.status.success() {
        return false;
    }
    let git_dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut shallow = std::path::PathBuf::from(&git_dir);
    shallow.push("shallow");
    shallow.exists()
}

#[test]
fn verify_pr_dag_passes_on_the_real_repository() {
    // The real release plan must satisfy the gate. If this fails, either
    // the plan is genuinely inconsistent or the gate is wrong — either way
    // it must be surfaced, not silenced.
    //
    // On a truncated history the gate cannot judge at all: it reports
    // DRIFT (exit 2) and says why. Demanding exit 0 there would fail the
    // CI test job for a reason unrelated to the code — the exact defect
    // this suite exists to prevent, one level up.
    if checkout_is_shallow() {
        // The DEFAULT behaviour on a truncated history is to refuse and
        // report DRIFT — that is what protects a CI runner from being
        // handed a verdict it cannot justify.
        let (code, out) = run_script("verify_pr_dag.sh", &[]);
        assert_eq!(
            code, 2,
            "on a shallow checkout the gate must report DRIFT (exit 2), not a \
             verdict. Output:\n{out}"
        );
        assert!(
            out.contains("SHALLOW"),
            "the DRIFT message must say the checkout is shallow. Output:\n{out}"
        );

        // And with --allow-shallow it must proceed to a real verdict
        // (whatever that verdict is — the point is that it is reached).
        let (allowed_code, allowed_out) = run_script("verify_pr_dag.sh", &["--allow-shallow"]);
        assert_ne!(
            allowed_code, 2,
            "--allow-shallow must let the gate reach a verdict instead of \
             reporting DRIFT. Output:\n{allowed_out}"
        );
        return;
    }

    let (code, out) = run_script("verify_pr_dag.sh", &[]);
    assert_eq!(
        code, 0,
        "the real PR DAG must satisfy the gate. Output:\n{out}"
    );
}

#[test]
fn verify_pr_dag_emits_evidence_fields() {
    // ADR-014 requires source_agent / source_run / timestamp in the output
    // of every agent-produced artifact.
    let (_, out) = run_script("verify_pr_dag.sh", &["--allow-shallow"]);
    assert!(
        out.contains("source_agent:") && out.contains("source_run:") && out.contains("timestamp:"),
        "the gate output must carry the ADR-014 evidence fields. Output:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// check_gate_script_refs.sh
// ---------------------------------------------------------------------------

#[test]
fn gate_script_refs_check_exists() {
    assert!(
        script_path("check_gate_script_refs.sh").exists(),
        "check_gate_script_refs.sh must exist — it is the root fix for the
         'criterion cites a missing script' class of defect"
    );
}

#[test]
fn gate_script_refs_check_is_executable() {
    let meta = fs::metadata(script_path("check_gate_script_refs.sh")).expect("stat");
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "check_gate_script_refs.sh must be executable"
    );
}

#[test]
fn gate_script_refs_baseline_exists_and_every_entry_has_a_reason() {
    // A baseline entry without a reason is a HIDDEN problem rather than
    // tracked debt — the opposite of this gate's purpose.
    let path = repo_root().join("tests/baseline/gate_script_refs_baseline.json");
    assert!(
        path.exists(),
        "the baseline file must exist: {}",
        path.display()
    );
    let text = fs::read_to_string(&path).expect("read baseline");
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("the baseline must be valid JSON: {e}"));
    let known = parsed
        .get("known_missing")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("the baseline must have a `known_missing` object"));
    assert!(
        !known.is_empty(),
        "the baseline must record at least the known-missing scripts"
    );
    for (path, reason) in known {
        let text = match reason {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(_) => reason
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            _ => String::new(),
        };
        assert!(
            !text.trim().is_empty(),
            "baseline entry `{path}` has no reason — that is a hidden problem, not tracked debt"
        );
    }
}

#[test]
fn gate_script_refs_check_passes_on_real_repository() {
    // Every `scripts/**.sh` path cited by the SSOT criterion documents must
    // exist, unless it is registered in the baseline as known debt.
    //
    // The lock is required: a sibling test temporarily appends a fabricated
    // reference to a criterion document, and this test runs the gate over
    // the same documents.
    let _guard = FileLock::acquire("gitea_criterion_doc");
    let (code, out) = run_script("check_gate_script_refs.sh", &[]);
    assert_eq!(
        code, 0,
        "every gate script referenced by a criterion document must exist, \
         or be registered in the baseline. Output:\n{out}"
    );
}

#[test]
fn gate_script_refs_check_detects_a_missing_reference() {
    // Append a fabricated reference to a criterion document and confirm
    // the gate catches it. The document is restored immediately after.
    //
    // NOTE: this test temporarily mutates a file that other tests in this
    // same binary read. Rust runs tests in threads, so the mutation window
    // is a genuine race unless it is serialised.
    let doc = repo_root().join("docs/governance/GATE_CONDITIONS.md");
    let original = fs::read_to_string(&doc).expect("read GATE_CONDITIONS.md");

    let _guard = FileLock::acquire("gitea_criterion_doc");

    let mutated = format!(
        "{}\n<!-- test fixture -->\n| BE99 | fabricated | `scripts/gate/zzz_nonexistent_probe.sh` | exit 0 | PASS |\n",
        original
    );
    fs::write(&doc, &mutated).expect("mutate GATE_CONDITIONS.md");

    let (code, out) = run_script("check_gate_script_refs.sh", &[]);

    // Restore no matter what — the document must not be left mutated.
    fs::write(&doc, &original).expect("restore GATE_CONDITIONS.md");

    assert_eq!(
        code, 1,
        "a missing referenced script must FAIL (exit 1). Output:\n{out}"
    );
    assert!(
        out.contains("zzz_nonexistent_probe.sh"),
        "the failure must name the missing script. Output:\n{out}"
    );
}

#[test]
fn gate_script_refs_check_emits_evidence_fields() {
    let (_, out) = run_script("check_gate_script_refs.sh", &[]);
    assert!(
        out.contains("source_agent:") && out.contains("source_run:") && out.contains("timestamp:"),
        "the gate output must carry the ADR-014 evidence fields. Output:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// check_pr_template_sections.sh
// ---------------------------------------------------------------------------

#[test]
fn pr_template_sections_check_exists_and_is_executable() {
    let path = script_path("check_pr_template_sections.sh");
    assert!(path.exists(), "check_pr_template_sections.sh must exist");
    let meta = fs::metadata(&path).expect("stat");
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "check_pr_template_sections.sh must be executable"
    );
}

#[test]
fn pr_template_sections_check_accepts_the_template() {
    // The template itself must satisfy the gate it defines.
    let (code, out) = run_script(
        "check_pr_template_sections.sh",
        &["--pr-body", ".gitea/pull_request_template.md"],
    );
    assert_eq!(
        code, 0,
        "the PR template must carry all five section markers. Output:\n{out}"
    );
}

#[test]
fn pr_template_sections_check_rejects_a_body_without_markers() {
    let path = write_temp_fixture(
        "pr_body_without_sections.md",
        "# My PR\n\nThis body has no section markers at all.\n",
    );
    let (code, out) = run_script(
        "check_pr_template_sections.sh",
        &["--pr-body", path.to_str().unwrap()],
    );
    assert_eq!(
        code, 1,
        "a PR body without the five markers must FAIL (exit 1). Output:\n{out}"
    );
}

#[test]
fn pr_template_sections_check_rejects_an_empty_body() {
    let path = write_temp_fixture("pr_body_empty.md", "");
    let (code, out) = run_script(
        "check_pr_template_sections.sh",
        &["--pr-body", path.to_str().unwrap()],
    );
    assert_eq!(
        code, 2,
        "an empty PR body must be DRIFT (exit 2), not PASS. Output:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// The criterion documents themselves
// ---------------------------------------------------------------------------

#[test]
fn every_gate_script_cited_by_criterion_documents_exists() {
    // The same invariant as the shell gate, asserted from the Rust side so
    // that a regression is caught by `cargo test` even when the shell gate
    // is not wired into CI yet.
    let docs = [
        "docs/governance/GATE_CONDITIONS.md",
        "docs/governance/RELEASE_GATE_CHECKLIST.md",
        "docs/governance/STAGE_CONFIG.yaml",
        "docs/governance/GATE_CI_CD.md",
        "docs/governance/STAGE_CONTROL_FRAMEWORK.md",
        "docs/governance/INDEX.md",
        "docs/governance/RC_TO_GA_GATE_CHECKLIST.md",
        "AGENTS.md",
    ];

    let mut cited: Vec<String> = Vec::new();
    for doc in docs {
        let path = repo_root().join(doc);
        if !path.exists() {
            continue;
        }
        // Take the same lock the mutating test uses, so this test never
        // observes a half-applied mutation of a criterion document.
        let _guard = FileLock::acquire("gitea_criterion_doc");
        let text = fs::read_to_string(&path).expect("read criterion document");
        drop(_guard);
        for line in text.lines() {
            let mut rest = line;
            while let Some(idx) = rest.find("scripts/") {
                rest = &rest[idx + "scripts/".len()..];
                let end = rest
                    .find(|c: char| {
                        !c.is_ascii_alphanumeric() && c != '_' && c != '.' && c != '/' && c != '-'
                    })
                    .unwrap_or(rest.len());
                let candidate = &rest[..end];
                if candidate.ends_with(".sh") && candidate.len() > 3 {
                    cited.push(format!("scripts/{candidate}"));
                }
            }
        }
    }
    cited.sort();
    cited.dedup();

    assert!(
        !cited.is_empty(),
        "expected the criterion documents to cite gate scripts — the extraction is broken"
    );

    // Known debt is allowed, but only when the baseline registers it.
    let baseline_path = repo_root().join("tests/baseline/gate_script_refs_baseline.json");
    let baseline_text = fs::read_to_string(&baseline_path).expect("read baseline");
    let baseline: serde_json::Value =
        serde_json::from_str(&baseline_text).expect("baseline must be valid JSON");
    let known = baseline
        .get("known_missing")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let unregistered: Vec<&String> = cited
        .iter()
        .filter(|p| !repo_root().join(p).exists())
        .filter(|p| !known.contains_key(*p))
        .collect();

    assert!(
        unregistered.is_empty(),
        "these gate scripts are cited by a criterion document, do not exist, \
         and are not registered in the baseline: {unregistered:?}"
    );
}

#[test]
fn pr_template_reference_in_agent_guide_points_at_a_real_file() {
    // Issue #5458: AGENTS.md cited `PR_TEMPLATE.md`, a path that does not
    // exist. The real template is `.gitea/pull_request_template.md`.
    let agents_md = repo_root().join("AGENTS.md");
    let text = fs::read_to_string(&agents_md).expect("read AGENTS.md");

    // Locate the line by a STABLE marker rather than by the file name it
    // happens to mention. The previous version searched for the literal
    // "PR_TEMPLATE" — but this PR's own fix changed that line to
    // ".gitea/pull_request_template.md" (lowercase), so the search
    // returned None and the ENTIRE assertion block was skipped. The test
    // passed vacuously: pointing AGENTS.md at a non-existent .md still
    // passed. (Independently verified by the review on 2026-10-22.)
    let line = text
        .lines()
        .find(|l| l.contains("5 类文档") || l.contains("5-类文档"))
        .expect(
            "AGENTS.md must contain a line describing the required PR-body \
             documents; if that line was renamed, this test must be updated \
             rather than silently skipping its assertion",
        );

    let mut checked = 0;
    for token in line.split_whitespace() {
        let token = token.trim_matches(|c: char| c == '`' || c == '(' || c == ')' || c == ',');
        if token.ends_with(".md") && !token.contains('*') {
            let path = repo_root().join(token);
            assert!(
                path.exists(),
                "AGENTS.md references '{token}' but that file does not exist.\n\
                 offending line: {line}"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "the AGENTS.md line must reference at least one .md file, otherwise \
         this test asserts nothing. offending line: {line}"
    );
}
