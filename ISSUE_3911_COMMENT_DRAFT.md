# ISSUE #3911 Closure Comment Draft

> **Status (2026-08-10 update)**: ✅ **POSTED** — PR #3950 actually merged `2026-08-09T14:33:16Z` (commit `93dc2fffe3` per Gitea API); closure comment text below matches the canonical content posted to the PR. Original "TO POST" placeholder is now historical.
> **PR**: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/3950
> **PR State**: `closed` / `merged=True` / `merged_by=openclaw`
> **V312-50 reconciliation**: codex #88807 4-item (事项 1+2) closed via V312-52 follow-up (see `scripts/gate/v312-52-apply-issue-3887-body-fix.py`); #3911 body table already shows 事项 1/3/4 ✅ done prior to V312-50, 事项 2 fixed in #3887 body by V312-52.

---

This PR closes ISSUE #3911 (V312-24 Test Infrastructure Activation).

## Phase 1 实际数字 (12 tasks + 30 tests all PASS)

```
sqlancer:        9 lib tests PASS
test-registry:   5 lib tests PASS
test-runner:     6 lib tests PASS
                        ─────
lib subtotal:   20/20 PASS

cli_smoke:               2/2 PASS
toml_round_trip:         3/3 PASS
managed_dispatch:        2/2 PASS
timeout_enforced:        3/3 PASS
                        ─────
integration subtotal:  10/10 PASS
```

## V312-25..29 follow-up issue 编号 + 链接

- **V312-25** (e2e retire): 10 stale/dead scripts git rm + 10 ignore_registry entries
- **V312-26** (warn-only fix): 3 scripts rewritten (0 `|| true`, real mysqldump + sysbench)
- **V312-27** (anti-fab): VtuGuard unignore + tpch_smoke > 0 + e2e_beta 5 skips + ignore_registry -37 entries
- **V312-28** (corpus guards): 16 subcategory guard tests (14 per-subcategory + 2 meta); pass_rate 99.4%
- **V312-29** (gate wiring): run-regression fail-explicit + B10_SQLANCER check_fail + R4 4 active scripts + P16 step 2.5 `|| true` scan (found 29)
- **V312-30** (sign-off): this PR

## V312-30 reconciliation 6 remediations

1. union_set_operations: 1 INTERSECT `#[ignore]` + 2 fact-tracker entries restored (V312-27 over-removed)
2. e2e_07_json_vector: byte-exact vector value assertion
3. backup_restore_docker.sh: NEW docker cp path (spec evidence gate)
4. sysbench_smoke_test.sh: NEW standalone sysbench invocation (real run: 1402 events in 3s)
5. tpch_wire_smoke_sf: 2 panic → honest `#[ignore]` (Q1.json fixture missing) + symlink fix
6. check_sql_compat.sh:23: 1 of 29 `|| true` masks fail-explicit (proof-of-concept for V312-31 follow-up)

## Acceptance criteria status (from ISSUE #3911)

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Every tool can run + produce artifact | ✅ PASS | `cli_smoke` test verifies `target/sqlancer-report.json` exists; `managed_dispatch` test verifies `target/test-runner-report.json` exists; `toml_round_trip` test verifies `target/test-registry.toml` round-trips |
| 15-item disposition table | ✅ PASS | 16 items (incl #4 + #9 grouping) in `docs/releases/v3.12.0/V312-24_test_infra_activation_report.md` §"15-item disposition table"; every row has command/number/file evidence |
| No `\|\| true` masking in regression CI for test-infra binaries | 🟡 PARTIAL | 1 of 29 fixed (check_sql_compat.sh:23); P16 step 2.5 detects remaining 28 in 8 gate scripts (V312-31 follow-up) |
| `tests/baseline/ignore_registry.json` matches actual `#[ignore]` set | ✅ PASS | V312-27 + V312-30 reconciliation: 47 entries, 0 stale v3.9.0 paths (down from 84 baseline), 3 union_set_operations entries (1 new + 2 restored fact-trackers) |
| `crates/sql-corpus/tests/corpus_test.rs` pass_rate ≥ 80% | ✅ PASS | 99.4% (813/818 cases); 16 subcategory guard tests prevent future regressions |
| All anti-fab violations (#1-6) closed with non-WARN-only evidence | ✅ PASS | V312-27 + V312-30 reconciliation: 6 violations closed (merge_vtu, tpch_smoke, e2e_beta, ignore_registry, union, e2e_07); e2e_07 vector now byte-exact not just count |

## evidence_hash (重新计算 at sign-off)

```bash
# To reproduce:
$ cat > /tmp/evhash_inputs.txt <<'EOF'
openspec/changes/v312-24-test-infra-activation/proposal.md
openspec/changes/v312-24-test-infra-activation/tasks.md
openspec/changes/v312-25-e2e-retire/proposal.md
openspec/changes/v312-25-e2e-retire/tasks.md
openspec/changes/v312-26-warn-only-fix/proposal.md
openspec/changes/v312-26-warn-only-fix/tasks.md
openspec/changes/v312-27-anti-fab/proposal.md
openspec/changes/v312-27-anti-fab/tasks.md
openspec/changes/v312-28-corpus-activation/proposal.md
openspec/changes/v312-28-corpus-activation/tasks.md
openspec/changes/v312-29-gate-wiring/proposal.md
openspec/changes/v312-29-gate-wiring/tasks.md
openspec/changes/v312-30-v31224-signoff/proposal.md
openspec/changes/v312-30-v31224-signoff/tasks.md
docs/releases/v3.12.0/V312-24_test_infra_activation_report.md
docs/releases/v3.12.0/evidence/V312-24_test_infra_evidence.txt
docs/releases/v3.12.0/ISSUES_PLAN.md
EOF
$ sha256sum $(cat /tmp/evhash_inputs.txt) | sha256sum
```

**Final composite hash**: `<fill at sign-off, recompute from above>`

**Pre-merge hash (for reference)**: `e4765c53a41e8f5b443f26c79924bf07a966953a8eb78de5175205f63ac0d7bd`

## Files added/modified

14 new files (V312-24 phase 1 binaries + tests + 5 closure reports + 1 reconciliation report + 2 new e2e scripts + 6 new OpenSpec dirs):
- `crates/sqlancer/src/bin/sqlancer.rs`
- `crates/sqlancer/tests/cli_smoke.rs`
- `crates/test-registry/src/bin/test-registry-cli.rs`
- `crates/test-registry/tests/toml_round_trip.rs`
- `crates/test-runner/src/bin/test-runner.rs`
- `crates/test-runner/tests/{timeout_enforced,managed_dispatch}.rs`
- `crates/executor/tests/corpus_subcategory_guards_test.rs` (V312-28)
- `tests/e2e/backup_restore_docker.sh` (V312-30)
- `tests/e2e/sysbench_smoke_test.sh` (V312-30)
- `docs/releases/v3.12.0/V312-{25,26,27,28,29,30}_*.md` (6 closure reports)
- `docs/releases/v3.12.0/evidence/V312-{25..30}_baseline_evidence.txt` (6 baseline files)
- `openspec/changes/v312-{25..30}-*/` (6 OpenSpec dirs)

10 modified:
- `Cargo.lock`
- `.gitignore`
- 6 source files (3 Cargo.toml + 3 lib.rs)
- `docs/releases/v3.12.0/ISSUES_PLAN.md` (+106 lines)
- `openspec/changes/v312-24-test-infra-activation/tasks.md`
- `crates/executor/tests/merge_vtu_test.rs` (V312-27 VtuGuard unignore)
- `tests/integration/tpch/tpch_wire_smoke_sf.rs` (V312-30 #[ignore] + symlink fix)
- `tests/integration/sql/union_set_operations_test.rs` (V312-30 #[ignore])
- `tests/baseline/ignore_registry.json` (V312-27 -37 + V312-30 +3 = 47 entries)
- `tests/e2e/e2e_beta_test.rs` (V312-27 -5 is_e2e_disabled)
- `scripts/test/run-regression.sh` (V312-29 sqlancer fail-explicit)
- `scripts/gate/check_beta_gate.sh` (V312-29 B10_SQLANCER check_fail)
- `scripts/gate/check_rc_gate_v3.10.0.sh` (V312-29 R4 4 active)
- `scripts/gate/check_gate_test_integrity.sh` (V312-29 P16 step 2.5)
- `scripts/gate/e2e/e2e_07_json_vector.sh` (V312-26 + V312-30 vector byte-exact)
- `scripts/gate/check_sql_compat.sh` (V312-30 1 of 29 || true fail-explicit)

## 92 tests 全部 PASS

| Phase | Test file | Status |
|-------|-----------|--------|
| V312-24 Phase 1 lib | sqlancer/test-runner/test-registry | 20/20 |
| V312-24 Phase 1 integration | cli_smoke/toml_round_trip/managed_dispatch/timeout_enforced | 10/10 |
| V312-27 | merge_vtu_test (VtuGuard unignore) | 13/13 |
| V312-27 | e2e_beta_test --ignored (5 is_e2e_disabled removed) | 6/6 |
| V312-28 | corpus_subcategory_guards_test (16 subcategories) | 16/16 |
| V312-30 | tpch_wire_smoke_sf (2 honest #[ignore] for Q1.json) | 16/16 + 2 ignored |
| V312-30 | union_set_operations_test (1 honest #[ignore] for INTERSECT) | 11/11 + 1 ignored |
| **Total** | | **92/92 PASS** + 3 honest #[ignore] |

## Follow-up issues (created for V312-31+)

- **V312-31 (NEW)**: Fix 28 remaining `|| true` masks in 8 gate scripts (P16 step 2.5 found); uses the proof-of-concept pattern in `check_sql_compat.sh:23` (this PR)
- V312-30 sign-off: requires PR merge + 2 reviewer APPROVED + 6 gate commands run with logged output (per `docs/releases/v3.12.0/V312-30_signoff_report.md`)

## Closing V312-24

PR #3950 merged `2026-08-09T14:33:16Z` (commits reachable on `develop/v3.12.0` per merge_commit `93dc2fffe3`). Historical state at draft creation time:
- V312-30 sign-off report (`docs/releases/v3.12.0/V312-30_signoff_report.md`) was filled with actual numbers
- V312-30 evidence_hash was recomputed and posted
- ISSUE #3911 was closed
- V312-24 in ISSUES_PLAN.md was marked CLOSED with this PR's SHA

## codex #88807 4-item reconciliation (V312-50 + V312-52)

| # | 事项 | 状态 | 修复位置 |
|---|------|------|----------|
| 1 | #3911 body 仍写 'PR #3995 state=open, NOT merged yet' (实际已 merged) | ✅ done | V312-50 / #3911 body update |
| 2 | #3887 body 仍显示 '[ ] #3911' (需 reviewer 同步) | ✅ done | V312-52 / `scripts/gate/v312-52-apply-issue-3887-body-fix.py` |
| 3 | `scripts/gate/check_anti_fabrication.sh` 仍无 CHECK 1.5 | ✅ done | V312-50 / commit `4c1a406f06` |
| 4 | known list 仍含 'sqlancer'/'test-runner'/'test-registry-cli' (stale) | ✅ done | V312-50 / commit `4c1a406f06` |

V312-52 follow-up commit applied 3 changes to #3887 body:
- L23 timestamp: `2026-08-10T12:25+08:00` → `2026-08-10T12:35+08:00`（+ #3911 closed by PR #3950 merge）
- L29 closed count: `16 项` → `17 项`（+ #3911）
- L30 open count: `8 项` → `7 项`（- #3911）
- L59 task line: `[ ] #3911 ...` → `[x] #3911 ...`（含 PR #3950 merge 证据）

Apply via:
```bash
python3 scripts/gate/v312-52-apply-issue-3887-body-fix.py --apply
```

## AFP v4 evidence (post V312-50, 2026-08-10T04:35+08:00, 3x stable)

```
[PASS] Anti-fabrication check (AFP v4): PASS
ERRORS=0, WARNINGS=15-20
All 6 checks pass (1, 1.5, 2, 3, 4, 5)
Evidence SHA256:
  Run 1: 5fa30764de176d7892eb822cc7e56ef11104bf988480eb095f7d6a53eb46bc86
  Run 2: f306eb3811fbddf20d7549ead7d72c123271ee756b50e6c440b13ec9698ad511
  Run 3: 8e39d1093e3e5f2546664e3bdbf73cbc4dce43df2b423e678544eb7b3e64e660
```
