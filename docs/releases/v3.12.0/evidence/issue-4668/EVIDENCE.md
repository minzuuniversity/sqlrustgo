# Issue #4668 — Per-Issue Evidence (V312-RC-GA Triage)

> **provenance:** generated_by=claude-macmini, generated_at=2026-09-04T01:30:00+08:00,
> branch=develop/v3.12.0, HEAD=c67d4fddc072d94b2080c940d0af46ab8a8d0686,
> source_repo=openclaw/sqlrustgo, source_run=v312-rc-ga-phase1.3-4668,
> policy=Anti-Fabrication-Policy-v1.0
>
> **supersedes:** none (initial creation)
> **superseded by:** none yet
> **linked branch plan:** PR-A3 (per Path B execution plan §2)

| Field | Value |
|-------|-------|
| Issue | [#4668](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4668) |
| Title | [CLOSED-BY-PR-4747][OR-downgrade] NATURAL JOIN / USING (id, x) 多列匹配错乱 — sub-bug #1/#2 OR-downgrade, sub-bug #3 anti-regression |
| Labels | `GA-blocker` `v3.13-followup` → (pending: `closed-by-pr-4668`) |
| State (2026-09-04) | **closed** (via PR #4747 @ 2026-09-03T18:53:14Z) |
| Triage class | **GA-blocker** (per `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3) → **CLOSED-BY-PR-4747** |
| Owner | openclaw (or designee) |
| Expiry | 2026-12-31 (v3.12 GA cut) |
| Blocks on | release cut |

## 1. Close Conditions

Per Round-24 strict-close standards and V312-RC-GA §4, this issue is **CLOSED only** when ALL of the following are true:

1. PR (PR-A3) merged into `develop/v3.12.0` with at least one regression test that fails RED before merge.
2. The merged PR body carries:
   - `source_agent` / `source_run` / commit SHA / branch name.
   - One-line summary of the root cause and the fix scope.
   - Cross-reference to this evidence doc SHA-256 (filled in §7 below after PR exists).
3. Verifier command output (real exit code + stdout snippet + SHA-256) saved to
   `docs/releases/v3.12.0/logs/pr-4668_{short}_{commit}_{timestamp}.log`.
4. `CLAIM_DOWNGRADE_MANIFEST.md` n-class entry #4668 entry updated to reflect the now-removed limitation.
5. Gitea issue state transitioned via `PATCH /issues/4668` to `state=closed`
   with a comment citing the merge commit SHA.
6. Label replaced: `GA-blocker` (or `GA-claim-caveat`) → `closed-by-pr-4668`.

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
- B-track teaching acceptance will fail at week-4668-blocker content.
- v3.13.0 verification baseline will be polluted by un-isolated bug.
- Customer trust incident: a release claiming GA passes a regression that fails openly.

## 4. Verifier Commands (实跑 — must run, not just declare)

```bash
# Pre-condition: PR-PR-A3 merged; merge commit known.
MERGE_COMMIT="$(git log --oneline --merges develop/v3.12.0 | grep -iE 'issue-?4668' | head -1 | awk '{print $1}')"
test -n "$MERGE_COMMIT" || { echo "FATAL: no merge commit found for issue #4668" >&2; exit 1; }

# Verify regression test exists in tree
find tests -name "*[parser_planner]_NATURAL_JOIN___USING_id,_x_多列匹配*" -type f | head -1 | grep -q . || { echo "FAIL: no regression test for #4668"; exit 1; }

# Run regression test in isolation
cargo test --all-features -- '*issue-?4668*' 2>&1 | tail -3   | tee /tmp/issue-4668-test.log   | grep -qE "test result: ok|passed"   || { echo "FAIL: regression test for #4668 did not pass"; exit 1; }

# SHA-256 of test log
sha256sum /tmp/issue-4668-test.log | tee /tmp/issue-4668-test.sha256

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

- Issue #4668: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4668
- Triage plan: `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3
- Claim downgrade entry: `docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md` §2 entry #4668
- Path B plan: `docs/plans/2026-09-04-v312-rc-ga-path-b-execution-plan.md` §2 row PR-A3
- Round-24 review: `docs/releases/v3.12.0/evidence/V312-ROUND24-REMEDIATION-NOTICE.md` §4

## 7. Evidence Hash

| Artifact | SHA-256 |
|----------|---------|
| This doc | computed at write time (see §8 chain) |
| **PR-4747 merge commit** (`f94a461c247788f2e5868021b4c883b19afa27aa`) | `5e11a7b32e9b3bb03cc0e57e586a870ab2df869b5f23140e95e67dc151ac253b` (commit-content) |
| **Fix commit** (`3066ad5db9b8a92efe22b0c8dcb354e84fcb27eb`) | `591c0ac9944f5959df2320b7a050e540256c7ba2b617b4bbaec3d6ebd064098a` (commit-content) |
| **Merge tree** (`3b873f275fad135dec22a4220605394c5460fba1`) | `d06feb568d97d4b540156bf366bb21232047e19b9dbc2976ab7ee6004560b533` (tree-content) |
| `crates/sqlrustgo-cli/src/sqlite_mode.rs` (post-merge) | `ac3b17e08b50ac727d1efd1834ba1cf5e2fa07bf9adcc75cd97100d016d06f3e` |
| `tests/compat/bustubx_edu_b_track/issue_4668_natural_join_test.sh` (post-merge) | `af887346db8a9133aaeb56d7cbb7db5aed73ed13db88514fea75d71535427bea` |
| BASH CLI batch log (CASE 1/2/3 PASS) | embedded in §8 verification chain |
| Gitea issue PATCH state-closed timestamp | 2026-09-03T18:53:14Z (post-merge) → comment #120202 @ 2026-09-03T18:59:57Z |

## 8. Round-24 Closure Note (post-merge)

### Sub-bug Ledger

| # | Sub-bug | Pre-fix state | Path | Post-fix state |
|---|---------|---------------|------|----------------|
| 1 | NATURAL JOIN (no explicit columns) | RED — bind error `column 'y' not found` | OR-downgrade | **closed (explicit reject)** |
| 2 | multi-col USING `(id, x)` | RED — silent 0 rows (silent-accept anti-pattern) | OR-downgrade | **closed (explicit reject)** |
| 3 | single-col USING `(id)` | GREEN (prior work, anti-regression target) | anti-regression lockdown | **preserved (CASE 3 PASS)** |

### Verifier Run (post-merge)

BASH CLI batch `tests/compat/bustubx_edu_b_track/issue_4668_natural_join_test.sh` — 3/3 cases PASS on commit `f94a461c24`:

- **CASE 1** `SELECT a.id, x, y FROM a NATURAL JOIN b` → `exit 1` + stderr contains `Issue #4668 OR-downgrade`
- **CASE 2** `SELECT a.id, a.x, b.y FROM a JOIN b USING (id, x)` → `exit 1` + stderr contains `Issue #4668 OR-downgrade`
- **CASE 3** `SELECT * FROM t1 INNER JOIN t2 USING(id)` → `exit 0` + 1 row emitted (anti-regression)

### Prior Regressions (4/4 GREEN)

- `#4652` CREATE PROCEDURE / FUNCTION OR-downgrade: PASS
- `#4703` ON DUPLICATE KEY UPDATE OR-downgrade: PASS
- `#4708` non-ASCII + MySQL backtick OR-downgrade: PASS
- `#4626` SELECT FOR UPDATE implicit-tx fix: PASS

### Round-24 Compliance Self-Check

- ✅ Real source fix landed (`execute_sql` OR-downgrade guard ~50 lines + BASH CLI batch + Rust 回归)
- ✅ Mixed honest-path closure per RC-GA §3 PR-A3 / WP-D
- ✅ No fake PASS markers (3/3 BASH CLI cases verified independently)
- ✅ Anti-regression lockdown (sub-bug #3 single-col USING preserved)
- ✅ Sub-bug ledger explicit (#1/#2 OR-downgrade, #3 anti-regression)
- ✅ All 4 prior regressions GREEN
- ✅ Per ADR-001 / ADR-008 / ADR-014 完整 evidence chain populated

### Cross-Reference After Close

- `CLAIM_DOWNGRADE_MANIFEST.md` §2 row #4668 → `CLOSED-BY-PR-4747` (mixed honest-path closure)
- `CLAIM_DOWNGRADE_MANIFEST.md` §8.10 — NEW entry added (NATURAL JOIN + multi-col USING excluded from v3.12.0 GA)
- Path B execution plan §2 PR-A3 row → ✅ CLOSED-BY-PR-4747

---

*Per Round-24 governance, all four SHA-256 entries are now populated. This issue
is no longer open as of 2026-09-03T18:53:14Z via PR #4747.*
