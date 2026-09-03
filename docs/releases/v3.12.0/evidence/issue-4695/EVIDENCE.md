# Issue #4695 — Per-Issue Evidence (V312-RC-GA Triage)

> **provenance:** generated_by=claude-macmini, generated_at=2026-09-04T03:50:00+08:00,
> branch=develop/v3.12.0, HEAD=`d08d1e7382`, source_repo=openclaw/sqlrustgo,
> source_run=v312-rcga-partial-scope-note-2026-09-04,
> policy=Anti-Fabrication-Policy-v1.0
>
> **supersedes:** none (initial creation)
> **superseded by:** none
> **closure path:** PARTIAL — single commit landed; full closure pending INTERVAL parser fix

| Field | Value |
|-------|-------|
| Issue | [#4695](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4695) |
| Title | [parser] INTERVAL '5' DAY (PG 日期算术) parser 把 INTERVAL 当列名; LAG 函数完全不输出 |
| Labels | `v3.13-followup` |
| State (2026-09-04) | **open** — partial fix landed; remainder pending |
| Triage class | v3.13/defer (per `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3 — "Window function completion") |
| Owner | openclaw (OpenClaw fix-author) |
| Expiry | v3.13.0 GA milestone (>= 2027-06-30) |

## 1. Partial Fix Closed Scope

The issue body lists **two distinct symptoms**:

1. **INTERVAL '5' DAY parser** — parser treats `INTERVAL` as a column name; PG-style date arithmetic fails at parse time.
2. **LAG function returns empty result** — `LAG(...) OVER (...)` returns no rows.

Source-fix commit `e1b5f1131d` (on `develop/v3.12.0`) **only addresses symptom #2** (LAG/LEAD window functions).

| Symptom | Fixed by `e1b5f1131d`? |
|---------|------------------------|
| LAG/LEAD window-function empty-result | ✅ yes (commit `e1b5f1131d` in `crates/executor/src/expr_utils.rs` + `crates/executor/tests/issue_4695_lag_lead_test.rs`) |
| INTERVAL '5' DAY parser | ❌ no — still fails |

## 2. Close Conditions (NOT yet fully met)

Per Round-24 strict-close standards and V312-RC-GA §4, this issue is **CLOSED** only when ALL apply:

1. PR / commit merged with regression tests for **all** issue body symptoms
2. Per-PR body carries provenance (source_agent / source_run / commit SHA / branch name)
3. Verifier command output (real exit code + stdout + SHA-256) saved
4. Per-issue evidence doc carries full SHA chain
5. Gitea state transitioned via `PATCH /issues/{id}` to `state=closed`
6. Per Round-24 Anti-Pattern 10 §1: **closing partial fix = fabricates completion** — keep issue OPEN until ALL symptoms closed

Currently:
- ✅ Symptom #2 verified fixed by commit `e1b5f1131d` (commit message + test file present)
- ❌ Symptom #1 (INTERVAL parser) **NOT YET FIXED** in any reachable commit

## 3. Anti-Pattern — 10 禁止关闭条件

1. ❌ **Closing the issue with partial fix** — Round-24 §1 forbids. Issue body lists two symptoms; only one is fixed.
2. ❌ Closing with `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` — synthetic completion marker.
3. ❌ Closing with `SUBSTANTIALLY_COMPLETE` — Round-24 explicitly rejected.
4. ❌ Closing without a regression test for the second symptom (INTERVAL parser).
5. ❌ Closing without confirming INTERVAL-fix commit reachable in HEAD.
6. ❌ Direct Gitea API closure without PR/issue-content evidence.
7. ✅ Expiry (v3.13.0) is far away; not violated.
8. ❌ Marking `#[ignore]` on INTERVAL test to bypass.
9. ❌ Modify test data to mask INTERVAL bug.
10. ✅ No force-push after the LAG/LEAD merge.

**Verdict**: issue must REMAIN OPEN. This doc is honest-path documentation; it does NOT close the issue.

## 4. Honest Scope Disclosure

Per Round-24 Anti-Pattern #1, partial fix ≠ closure.

If/when a follow-up commit fixes INTERVAL parser, then § 1 conditions become fully
satisfied and the issue can be closed per the standard path:
- New commit message: `fix(v312-XX / #4695): INTERVAL '5' DAY parser — accept Postgres-style date arithmetic`
- New regression test in `tests/integration/parser/test_parse_interval.rs`
- Then close issue per §1 conditions + update this §4 to record full closure

Until then, this issue remains **active v3.13/defer** with the window-function
component partial-fixed and the parser component pending.

## 5. Verifier Commands (实跑 — must run, not just declare)

```bash
# 1. Symptom #2 (LAG/LEAD) verification — should PASS now
cargo test -p sqlrustgo-executor --test issue_4695_lag_lead_test --all-features

# 2. Symptom #1 (INTERVAL parser) verification — should still FAIL
#    No committed regression test exists. Manual smoke:
echo "SELECT INTERVAL '5' DAY;" | sqlrustgo-cli /tmp/test.db
# Expected (pre-fix status): parse error or column-name resolution failure
# Note: this manual probe is NOT in CI; per-interval-parser fix will add it.

# 3. Commit reachable
git merge-base --is-ancestor e1b5f1131d HEAD  # exit 0 ✓
```

## 6. Reviewer

- **Reviewer 1 (assigned)**: openclaw (OpenClaw fix-author; sign-off on LAG/LEAD scope)
- **Reviewer 2 (pending Phase 4 allocation)**: hermes-z6g4 / Codex 模式 — per Path B execution plan §4

## 7. Evidence Hash

| Artifact | SHA-256 | Source |
|----------|---------|--------|
| This doc `EVIDENCE.md` | (computed post-finalize) | `sha256sum` after final sync |
| Source-fix commit (LAG/LEAD) `e1b5f1131d` | (commit ref) | https://192.168.0.252:3000/openclaw/sqlrustgo/commit/e1b5f1131d |
| Regression test `crates/executor/tests/issue_4695_lag_lead_test.rs` | (file SHA) | grep + `sha256sum` |
| INTERVAL parser fix commit | **N/A — does not yet exist** | this is the pending gap |

## 8. Round-24 Partial-Scope Note (added 2026-09-04)

This document was created **as part of the orphan-batch audit pass**, not as a
closure. The audit identified that #4695 had a source-fix commit (`e1b5f1131d`)
reachable in HEAD, but **the fix did not cover the full issue body** (only LAG/LEAD,
not INTERVAL).

Per Round-24 Anti-Pattern standards, partial-fix orphan issues are documented
in their per-issue evidence file but **the Gitea issue remains open** until the
remaining symptoms receive a fix.

If a follow-up commit addresses INTERVAL parser, this doc should be updated to
record the full-fix SHA + close the issue via the standard path.

This is consistent with CLAIM_DOWNGRADE_MANIFEST §4: "Window function completion
(#4707, #4706, #4695, #4689)" — the original deferral scope was about window
function COMPLETION, not INTERVAL parser. The fact that issue #4695 lumps
both INTERVAL parser AND window functions together is itself unusual; if a
follow-up split occurs, sub-issues should be created separately.
