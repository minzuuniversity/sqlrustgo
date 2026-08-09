# V312-24: Test Infrastructure Activation — proposal

> **Author**: minimax (claude-opus)
> **Date**: 2026-08-09 (Asia/Shanghai)
> **Branch**: `develop/v3.12.0` (HEAD at this writing)
> **Tracking issue**: #3911 (V312-24 Test Infrastructure Activation)
> **Source plan**: `docs/releases/v3.12.0/ISSUES_PLAN.md` lines 158-164 + `TESTING_SYSTEM_BETA_REPORT.md`

## Why

v3.12.0 ISSUES_PLAN §V312-24 mandates that every test-infrastructure asset at v3.10.0 baseline either be activated (runs + produces artifact) or carry explicit retirement/deferral with reason + replacement gate + owner + expiry. WARN-only masking of broken tests is forbidden.

The audit at develop/v3.12.0 HEAD found **four classes of failure** that must be dispositioned:

1. **Skeletons with no executable surface** — `crates/sqlancer`, `crates/test-runner`, `crates/test-registry` are library crates with no `[[bin]]` and zero callers. The regression CI tries `cargo run --release -p sqlancer -- --duration 120` but it has no binary target; failure is masked with `|| true` in `scripts/test/run-regression.sh:111-112` and gated as `B10_SQLANCER check_warn` in `scripts/gate/check_beta_gate.sh:363`.
2. **Dead-code E2E scripts** — `tests/e2e/` has 8 R4-named scripts; 4 are byte-identical stale mirrors of `scripts/gate/e2e/e2e_*.sh` (rename tasks 1.1-1.4 from `issue-3399-r4-e2e-scripts/tasks.md` never executed). `scripts/gate/e2e/e2e_05..08` are dead-code: not referenced by any gate and never run.
3. **WARN-only anti-fabrication masking** — 6 confirmed violations:
   - `tests/integration/tpch/tpch_wire_smoke_sf.rs` smoke asserts `rows.len() <= 6` (accepts 0), cargo reports PASS even with a broken engine.
   - `tests/e2e/e2e_beta_test.rs` 5 tests use double-skip (`#[ignore]` + runtime `if is_e2e_disabled() { return; }`); on CI the `CI` env var is always set so even `cargo test -- --ignored` returns immediately.
   - `tests/integration/sql/union_set_operations_test.rs` has 0 `#[ignore]` attributes despite `tests/baseline/ignore_registry.json` listing 3 IGNORE_MULTI items — registry drift, features implemented.
   - `tests/integration/sql/multi_statement_test.rs:136` cites PR #3635 — needs verification that the cited fix is merged (out of scope; reference only).
   - `crates/executor/tests/merge_vtu_test.rs:212` `#[ignore = "VtuGuard not yet implemented"]` — STALE. VtuGuard was CLOSED in v3.8.0 ARCH-3 (PRs #3152/#3787/#3790, debt-registry.yaml:207). The ignore masks a now-passing test.
   - `tests/integration/tpch/tpch_wire_smoke_sf.rs` and `tests/e2e/backup_restore.sh` and `tests/e2e/sysbench_wired.sh` all carry `2>/dev/null || true` or `if X; then PASS; fi` patterns that silently swallow failures.
4. **Inactivated corpus** — `sql_corpus/` has 103 SQL files / 19,002 lines / 16 subcategories, but the only test that reads it (`crates/sql-corpus/tests/corpus_test.rs::test_sql_corpus_all`) requires 80% pass rate; v3.12.0 baseline reports 27.3% (6/16 subcategories PASS). The gap is the activation gate.

## Current State (Inventory)

```
crates/sqlancer          → library only, no bin, 9/9 inline unit tests pass, regression CI masked || true
crates/test-runner       → library only, no bin, 6/6 inline unit tests pass, never enforces timeout/max_parallel
crates/test-registry     → library only, no bin, 5/5 inline unit tests pass, no TOML load/save (Cargo.toml has toml dep but src/lib.rs doesn't use it)
crates/sqlrustgo_sqllogictest → binary sqlrustgo-sqllogictest ACTIVE, 22 active testdata files, 6/16 smoke corpus pass (27.3%)
crates/sql-corpus          → active, SimpleExecutor 1240 LoC, 4 tests, gate threshold 80% (currently failing at 27.3%)
sql_corpus/                → 103 files / 19,002 lines, never executed as gate input
scripts/gate/e2e/         → 9 scripts: 4 stale mirrors + 3 dead-code + 1 warn-only + 1 active (e2e_runner_exec.sh)
tests/e2e/                 → 8 scripts: 4 stale mirrors + 2 active + 2 warn-only
tests/baseline/ignore_registry.json → 42 entries, multiple stale v3.9.0 paths from before 3e4cd4accc restructure
```

Total: **17 E2E scripts classified (6 active / 8 broken / 3 dead-code / 3 warn-only)** + **3 test-infra skeleton crates** + **6 anti-fab violations** + **103-file corpus unactivated**.

## Goals / Non-Goals

**Goals** (each must produce executable artifact + non-WARN-only evidence):

1. **sqlancer** — add `[[bin]]` + main entry that wires `Fuzzer::new(config).run(120)` and writes `target/sqlancer-report.json` artifact. Replace `|| true` masking with `check_fail` on regression CI exit code 0 + non-empty report.
2. **test-runner** — implement `timeout_per_test_ms` via `tokio::time::timeout`, parallelize `run_tests` via `tokio::task::JoinSet` honoring `max_parallel`, write `serde_json` results to `target/test-runner-report.json`. Add `[[bin]]` + `examples/run_smoke.rs`.
3. **test-registry** — implement `from_toml(path)` + `write_toml(path)` using the unused `toml` dep. Register `sqlancer` + `test-runner` binaries as managed entries. Add `[[bin]]` `bin/test-registry-cli`.
4. **E2E scripts consolidation** — de-duplicate `tests/e2e/` ↔ `scripts/gate/e2e/`, delete the 4 stale mirrors, retire 3 dead-code scripts with explicit disposition in `tests/baseline/ignore_registry.json`, fix the 2 warn-only scripts (`backup_restore.sh` must do real restore-not-reinsert; `sysbench_wired.sh` must run real sysbench or fail).
5. **Anti-fabrication enforcement** — fix the 6 violations listed above; update `tests/baseline/ignore_registry.json` to reflect the actual `#[ignore]` set after fixes; verify `tests/e2e/e2e_beta_test.rs` either deletes the `if is_e2e_disabled()` early-return OR sets `SQLRUSTGO_E2E_BETA_SKIP=0` in CI.
6. **Corpus activation** — produce evidence of 80% pass-rate on `sql_corpus/` (current 27.3%). Either (a) fix the 73% gap in SimpleExecutor, or (b) lower threshold with explicit owner + expiry + replacement-gate attestation per ISSUES_PLAN §V312-24 acceptance.

**Non-Goals**:

- Multi-language integration (Python/SQLite corpus downloads) — blocked by network per `V310-14b` record; deferred to v3.12.1+
- Persistent disk-backed SubqueryIndex — already deferred to v3.12.0 per ISSUES_PLAN §V311-17
- Refactoring `crates/test-runner/src/lib.rs` 344-line monolith into modules — deferred until activation proves the runner is wired
- Implementing real sysbench (100-statement read loop fallback is the de-facto smoke) — deferred; the new `sysbench_wired.sh` either runs sysbench (if installed) or fails explicitly with exit 1 + clear stderr message

## Disposition (per ISSUES_PLAN §V312-20 schema)

Every asset in the inventory carries one of {activate, retire, defer, fix}:

| Asset | Disposition | Owner | Expiry | Evidence gate |
|-------|-------------|-------|--------|----------------|
| `crates/sqlancer` | **activate** (add `[[bin]]` + main + report writer) | minimax | 2026-08-25 | regression CI exit 0 + non-empty `target/sqlancer-report.json` + ≥1 fuzz iteration |
| `crates/test-runner` | **activate** (add `[[bin]]`, timeout/parallel enforcement, results writer) | minimax | 2026-08-25 | `target/test-runner-report.json` non-empty + tokio timeout actually fires |
| `crates/test-registry` | **activate** (TOML load/save + cli) | minimax | 2026-08-25 | registry.toml round-trip + sqlite-backed persistence path (no-op is fine) |
| `scripts/gate/e2e/e2e_01..04.sh` (4 stale mirrors) | **retire** (duplicate of `tests/e2e/`) | minimax | 2026-08-20 | git rm + ignore-registry entry with reason "superseded by tests/e2e/" |
| `scripts/gate/e2e/e2e_05_savepoint_rollback.sh` | **retire** (dead-code, not in any gate) | minimax | 2026-08-20 | git rm + ignore-registry entry |
| `scripts/gate/e2e/e2e_06_cte_query.sh` | **retire** (dead-code, not in any gate) | minimax | 2026-08-20 | git rm + ignore-registry entry |
| `scripts/gate/e2e/e2e_07_json_vector.sh` | **fix** (warn-only assertion, label says JSON/vector but only JSON) | minimax | 2026-08-25 | grep assertion removed or replaced with byte-exact JSON+vector fixture |
| `scripts/gate/e2e/e2e_08_migration.sh` | **retire** (dead-code, two-table JOIN mislabeled as migration) | minimax | 2026-08-20 | git rm + ignore-registry entry |
| `tests/e2e/startup_connect.sh` + `tests/e2e/tpch_sf01.sh` + `tests/e2e/kill9_recovery.sh` | **retire** (byte-identical stale mirrors of e2e_01/04/03) | minimax | 2026-08-20 | git rm + ignore-registry entry |
| `tests/e2e/backup_restore.sh` | **fix** (warn-only re-insert fakery; must do real restore) | minimax | 2026-08-25 | `docker cp` + `mysql ... SELECT` round-trip evidence |
| `tests/e2e/sysbench_wired.sh` | **fix** (warn-only fallback; must run real sysbench) | minimax | 2026-08-25 | sysbench version check + fail-explicit if absent |
| `tests/e2e/e2e_beta_test.rs` 5 double-skip tests | **fix** (remove `if is_e2e_disabled() { return; }`) | minimax | 2026-08-25 | `cargo test -- --ignored` exercises assertions + produces non-trivial output |
| `crates/executor/tests/merge_vtu_test.rs:212` | **fix** (remove stale ignore; VtuGuard closed 2026-07-12) | minimax | 2026-08-20 | `merge_vtu_test` runs + passes |
| `tests/integration/sql/union_set_operations_test.rs` (0 ignores despite registry listing 3) | **fix** (remove stale registry entries) | minimax | 2026-08-20 | registry.json matches actual `#[ignore]` set (cross-checked via grep) |
| `tests/integration/tpch/tpch_wire_smoke_sf.rs` (smoke assertion `rows.len() <= 6` accepts 0) | **fix** (assert exact row count or `rows.len() > 0`) | minimax | 2026-08-25 | smoke test fails when engine broken |
| `sql_corpus/` activation gate (current 27.3% pass rate) | **fix** (raise to ≥80% per `crates/sql-corpus/tests/corpus_test.rs:64` threshold) | minimax | 2026-08-30 | corpus_test pass_rate ≥ 80% across 16 subcategories |

Total: **15 dispositioned items**, **0 retired without owner**, **0 deferred without expiry**.

## Acceptance criteria (from issue #3911 body)

- [ ] Every tool can run + produce artifact (sqlancer + test-runner + test-registry all executable)
- [ ] Every retired/deferred item has reason + replacement gate + owner + expiry (15-item disposition table)
- [ ] No `|| true` masking in regression CI for test-infra binaries
- [ ] `tests/baseline/ignore_registry.json` matches actual `#[ignore]` set after fixes (no stale v3.9.0 paths)
- [ ] `crates/sql-corpus/tests/corpus_test.rs` pass_rate ≥ 80%
- [ ] All anti-fab violations (#1-6) closed with non-WARN-only evidence

## Dependencies

- **#3904 V312-17 Coverage & Disabled-Test Debt Close-out** — sister issue; shares ignore-registry fix
- **#3898 V312-11 SQLite SQLLogicTest Oracle Gate** — sister issue; shares sqlrustgo_sqllogictest activation
- **#3906 V312-19 SQL Corpus / Architecture Invariant** — sister issue; corpus activation overlaps

## References

- `docs/releases/v3.12.0/ISSUES_PLAN.md` lines 158-164
- `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` lines 25-41 (canonical skeleton list)
- `docs/governance/{ANTI_FABRICATION_POLICY, AI_GENERATOR_AUDIT_CHECKLIST, DISABLED_TESTS_ANALYSIS}.md`
- `tests/baseline/ignore_registry.json`
- `scripts/test/run-regression.sh:111-112` (sqlancer `|| true` masking)
- `scripts/gate/check_beta_gate.sh:363` (B10_SQLANCER DEFERRED)
- `openspec/changes/issue-3399-r4-e2e-scripts/{proposal,tasks}.md` (R4 8/8 spec)
- `openspec/changes/v311-17-hash-anti-join/{proposal,tasks}.md` (canonical structure template)
