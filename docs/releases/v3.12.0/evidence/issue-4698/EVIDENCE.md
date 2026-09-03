# Issue #4698 — Per-Issue Evidence (V312-RC-GA Triage)

> **provenance:** generated_by=claude-macmini, generated_at=2026-09-04T01:30:00+08:00,
> branch=develop/v3.12.0, HEAD=c67d4fddc072d94b2080c940d0af46ab8a8d0686,
> source_repo=openclaw/sqlrustgo, source_run=v312-rc-ga-phase1.3-4698,
> policy=Anti-Fabrication-Policy-v1.0
>
> **supersedes:** none (initial creation)
> **superseded by:** none yet
> **linked branch plan:** PR-B2 (per Path B execution plan §2)

| Field | Value |
|-------|-------|
| Issue | [#4698](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4698) |
| Title | [executor] GREATEST/LEAST 与 MOD/POWER/LOG/EXP/SQRT 等数学函数不输出 + sqlite_stat1 系统表不存在 + DEFAULT (express |
| Labels | `GA-claim-caveat` |
| State (2026-09-04) | open |
| Triage class | **GA-claim-caveat** (per `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §{3-claim}) |
| Owner | openclaw (or designee) |
| Expiry | v3.13.0 GA milestone (>= 2027-06-30) |
| Blocks on | v3.13 release GA cut |

## 1. Close Conditions

Per Round-24 strict-close standards and V312-RC-GA §4, this issue is **CLOSED only** when ALL of the following are true:

1. PR (PR-B2) merged into `develop/v3.12.0` with at least one regression test that fails RED before merge.
2. The merged PR body carries:
   - `source_agent` / `source_run` / commit SHA / branch name.
   - One-line summary of the root cause and the fix scope.
   - Cross-reference to this evidence doc SHA-256 (filled in §7 below after PR exists).
3. Verifier command output (real exit code + stdout snippet + SHA-256) saved to
   `docs/releases/v3.12.0/logs/pr-4698_{short}_{commit}_{timestamp}.log`.
4. `CLAIM_DOWNGRADE_MANIFEST.md` n-class entry #4698 entry updated to reflect the now-removed limitation.
5. Gitea issue state transitioned via `PATCH /issues/4698` to `state=closed`
   with a comment citing the merge commit SHA.
6. Label replaced: `GA-blocker` (or `GA-claim-caveat`) → `closed-by-pr-4698`.

## 2. Anti-Pattern — 10 禁止关闭条件

Per Round-24 Anti-Fabrication-Policy-v1.0 and `V313-ROUND24-EVIDENCE-MANIFEST.md`:

1. ❌ Closing the issue without a merged PR.
2. ❌ Closing with `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` language.
3. ❌ Closing with `SUBSTANTIALLY_COMPLETE` language (Round-24 explicitly rejected).
4. ❌ Closing without a RED regression test present in `tests/integration/` or equivalent.
5. ❌ Closing without a SQLite oracle diff (when the bug has oracle-applicable semantics).
6. ❌ Closing via direct Gitea API without a PR reference.
7. ❌ Closing before expiry unless superseded by code fix.
8. ❌ Marking `#[ignore]` on failing tests to bypass the gate.
9. ❌ Modifying test expected values to mask the bug rather than fixing the parser/executor.
10. ❌ Reverting the fix and then closing the issue (force-push after-mark).

## 3. Failure Scenario (GPT-style reverse argument)

If the issue is left open at GA cut without a proper closure:

- v3.12.0 GA claims cannot include this capability.
- Users running production SQL on v3.12.0 will reproduce the defect.
- B-track teaching acceptance will fail at week-4698-caveat content.
- v3.13.0 verification baseline will be polluted by un-isolated bug.
- Customer trust incident: a release claiming GA passes a regression that fails openly.

## 4. Verifier Commands (实跑 — must run, not just declare)

```bash
# Pre-condition: PR-PR-B2 merged; merge commit known.
MERGE_COMMIT="$(git log --oneline --merges develop/v3.12.0 | grep -iE 'issue-?4698' | head -1 | awk '{print $1}')"
test -n "$MERGE_COMMIT" || { echo "FATAL: no merge commit found for issue #4698" >&2; exit 1; }

# Verify regression test exists in tree
find tests -name "*[executor]_GREATEST_LEAST_与_MOD_POWER_LOG_EXP_SQRT*" -type f | head -1 | grep -q . || { echo "FAIL: no regression test for #4698"; exit 1; }

# Run regression test in isolation
cargo test --all-features -- '*issue-?4698*' 2>&1 | tail -3   | tee /tmp/issue-4698-test.log   | grep -qE "test result: ok|passed"   || { echo "FAIL: regression test for #4698 did not pass"; exit 1; }

# SHA-256 of test log
sha256sum /tmp/issue-4698-test.log | tee /tmp/issue-4698-test.sha256

# Diff against SQLite oracle (where applicable)
sqlite3 :memory: "SELECT 1;" > /tmp/sqlite-baseline.log 2>&1
sqlrustgo :memory: -c "SELECT 1;" > /tmp/sqlrustgo-baseline.log 2>&1
diff /tmp/sqlite-baseline.log /tmp/sqlrustgo-baseline.log
```

## 5. Reviewer

- **Reviewer 1 (assigned)**: openclaw (源作者 / PR author) — must sign off on regression coverage.
- **Reviewer 2 (pending Phase 4 allocation)**: hermes-z6g4 / Codex 模式 — per Path B execution plan §4.
- **Sign-off criteria**: 12 items (expanded from prior 7); see V312-RC-GA Path B §5.

## 6. Cross-References

- Issue #4698: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4698
- Triage plan: `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3
- Claim downgrade entry: `docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md` §3 entry #4698
- Path B plan: `docs/plans/2026-09-04-v312-rc-ga-path-b-execution-plan.md` §2 row PR-B2
- Round-24 review: `docs/releases/v3.12.0/evidence/V312-ROUND24-REMEDIATION-NOTICE.md` §4

## 7. Evidence Hash

| Artifact | SHA-256 |
|----------|---------|
| This doc (pending first close) | TBD |
| PR PR-B2 merge commit (after merge) | TBD |
| Regression test log (after fix) | TBD |
| Oracle diff log (after fix) | TBD |

*When PR-B2 lands and the four artifacts are filled, this hash section is updated,
and the issue is closable under §1.*

---

*Per Round-24 governance, this document MUST NOT be downgraded or rewritten to
forget the open state before all four SHA-256 entries are populated.*
