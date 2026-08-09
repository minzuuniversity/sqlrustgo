# V312-24: Test Infrastructure Activation — tasks

> **Status**: 🟡 PHASE 1 COMPLETE (2026-08-09, minimax) — phases 2-8 split into V312-25 ~ V312-30
> **Author**: minimax
> **Source run**: V312-24
> **Branch**: `feature/v312-24-impl`
> **Issue**: #3911
> **Acceptance**: per `openspec/changes/v312-24-test-infra-activation/proposal.md` §Acceptance criteria
> **Activation report**: `docs/releases/v3.12.0/V312-24_test_infra_activation_report.md`
> **Follow-up issues**: V312-25 (Phase 2), V312-26 (Phase 3), V312-27 (Phase 4), V312-28 (Phase 5), V312-29 (Phase 6), V312-30 (Phase 7.3 + 8)

## Phase 1: Activate test-infra skeletons (24h estimate) — ✅ COMPLETE

- [x] 1.1 Add `[[bin]] name = "sqlancer"` to `crates/sqlancer/Cargo.toml`
- [x] 1.2 Add `crates/sqlancer/src/bin/sqlancer.rs` with `fn main()` that wires `Fuzzer::new(config).run(...)`
- [x] 1.3 Make sqlancer write `target/sqlancer-report.json` (FuzzerResult JSON)
- [x] 1.4 Add `[[bin]] name = "test-runner"` to `crates/test-runner/Cargo.toml`
- [x] 1.5 Implement `timeout_per_test_ms` via `tokio::time::timeout` in `run_test` (and `run_managed`)
- [x] 1.6 Parallelize `run_tests` via `tokio::task::JoinSet` honoring `max_parallel`; also `run_managed_all`
- [x] 1.7 Write `serde_json` results to `target/test-runner-report.json`
- [x] 1.8 Add `crates/test-registry/src/bin/test-registry-cli.rs` for TOML read/write
- [x] 1.9 Implement `from_toml(path)` + `write_toml(path)` in `crates/test-registry/src/lib.rs` (uses existing `toml` dep)
- [x] 1.10 Register `sqlancer` + `test-runner` binaries as managed entries in test-registry (via `test-registry-cli init` + `starter_manifest()`)
- [x] 1.11 Wire test-runner to consume test-registry manifest (`--manifest <PATH>` mode in `test-runner` bin)
- [x] 1.12 Add `target/sqlancer-report.json` + `target/test-runner-report.json` + `target/test-registry.toml` to `.gitignore` (target/ already covered; added `test-registry.toml` at root)

Integration tests added (10/10 PASS):
- `crates/sqlancer/tests/cli_smoke.rs` — 2 tests
- `crates/test-registry/tests/toml_round_trip.rs` — 3 tests
- `crates/test-runner/tests/timeout_enforced.rs` — 3 tests
- `crates/test-runner/tests/managed_dispatch.rs` — 2 tests
## Phase 2: Retire dead-code E2E scripts (4h estimate)

- [ ] 2.1 `git rm tests/e2e/startup_connect.sh` (byte-identical stale mirror of e2e_01)
- [ ] 2.2 `git rm tests/e2e/tpch_sf01.sh` (mirror of e2e_04)
- [ ] 2.3 `git rm tests/e2e/kill9_recovery.sh` (mirror of e2e_03)
- [ ] 2.4 `git rm scripts/gate/e2e/e2e_01_basic_crud.sh` (mirror of startup_connect)
- [ ] 2.5 `git rm scripts/gate/e2e/e2e_02_tx_commit_rollback.sh` (mirror of rollback_mvcc)
- [ ] 2.6 `git rm scripts/gate/e2e/e2e_03_wal_crash_recovery.sh` (mirror of kill9_recovery)
- [ ] 2.7 `git rm scripts/gate/e2e/e2e_04_parallel_executor.sh` (mirror of tpch_sf01)
- [ ] 2.8 `git rm scripts/gate/e2e/e2e_05_savepoint_rollback.sh` (dead-code, not in any gate)
- [ ] 2.9 `git rm scripts/gate/e2e/e2e_06_cte_query.sh` (dead-code)
- [ ] 2.10 `git rm scripts/gate/e2e/e2e_08_migration.sh` (dead-code, mislabeled)

## Phase 3: Fix warn-only E2E scripts (8h estimate)

- [ ] 3.1 Rewrite `scripts/gate/e2e/e2e_07_json_vector.sh` — replace `grep -q name` with byte-exact JSON+vector fixture assertion
- [ ] 3.2 Rewrite `tests/e2e/backup_restore.sh` — replace `|| true` + data-reinsert with real `mysqldump` + `mysql` restore round-trip
- [ ] 3.3 Rewrite `tests/e2e/sysbench_wired.sh` — explicit `sysbench --version` check + exit 1 with clear stderr if absent
- [ ] 3.4 Verify all `|| true` patterns in retired scripts are gone (run `grep -rn '|| true' scripts/gate/e2e tests/e2e`)

## Phase 4: Fix anti-fabrication violations (8h estimate)

- [ ] 4.1 `crates/executor/tests/merge_vtu_test.rs:212` — remove `#[ignore = "VtuGuard not yet implemented"]` (stale; ARCH-3 closed VtuGuard)
- [ ] 4.2 `tests/integration/tpch/tpch_wire_smoke_sf.rs` — change `assert!(rows.len() <= 6)` to `assert!(rows.len() > 0, "smoke engine broken")` (or exact expected count)
- [ ] 4.3 `tests/e2e/e2e_beta_test.rs` lines 52-141 — remove `if is_e2e_disabled() { return; }` early-return guards from 5 tests
- [ ] 4.4 `tests/baseline/ignore_registry.json` — remove stale v3.9.0 paths (tests/tx_wal_contract_tests.rs, tests/soak_test.rs, tests/dml_integration_test.rs, tests/stored_proc_catalog_test.rs, tests/tpch_q9_audit.rs, tests/small_executor_modules_test.rs, tests/boundary_test.rs) — paths moved in 3e4cd4accc
- [ ] 4.5 `tests/baseline/ignore_registry.json` — remove stale entry for `crates/parser/src/parser.rs:7299` (phantom — no `#[ignore]` at that line)
- [ ] 4.6 `tests/baseline/ignore_registry.json` — remove 3 IGNORE_MULTI entries for `tests/integration/sql/union_set_operations_test.rs` (features implemented; no `#[ignore]` attributes in file)
- [ ] 4.7 Cross-check: `python3 -c "import json; d=json.load(open('tests/baseline/ignore_registry.json')); import re; src=open('crates/parser/src/parser.rs').read(); print('phantom:', any(f.get('file','').endswith('parser.rs') and '7299' in f.get('reason','') for f in d.get('files',[])))"` (assert False)

## Phase 5: Activate SQL corpus (16h estimate, hardest item)

- [ ] 5.1 Run baseline: `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all -- --nocapture > /tmp/corpus_baseline.txt 2>&1`
- [ ] 5.2 Triage failing subcategories (target 73% gap = ~75 subcategory failures)
- [ ] 5.3 Fix SimpleExecutor gaps: GROUP BY ... WITH ROLLUP, multi-column IN, INTERSECT/EXCEPT, INSERT ... ON DUPLICATE KEY UPDATE, recursive CTE, JSON_TABLE
- [ ] 5.4 Add 1 inline test per fixed subcategory to prevent regression
- [ ] 5.5 Re-run baseline, confirm pass_rate ≥ 80%
- [ ] 5.6 If 80% not achievable in budget: lower gate threshold with explicit owner + expiry + replacement-gate attestation per ISSUES_PLAN §V312-24 acceptance

## Phase 6: Wire gate enforcement (8h estimate)

- [ ] 6.1 `scripts/test/run-regression.sh:111-112` — remove `|| true` masking for sqlancer CLI invocation; assert non-empty `target/sqlancer-report.json` on exit 0
- [ ] 6.2 `scripts/gate/check_beta_gate.sh:363` — promote B10_SQLANCER from `check_warn` to `check_fail`; require `cargo run --release -p sqlancer -- --duration 30` to exit 0 with non-empty `target/sqlancer-report.json`
- [ ] 6.3 `scripts/gate/check_rc_gate_v3.10.0.sh:132-135` — update R4 substring match list to reflect post-retirement E2E script set (8 → 4 active: alter_rename, rollback_mvcc, union_set_ops, + e2e_runner_exec)
- [ ] 6.4 `scripts/gate/check_gate_test_integrity.sh` (P16) — extend scan to flag `|| true` masking after `cargo test` invocations in `scripts/gate/*.sh` (currently scans `cargo test … --test X` references only)

## Phase 7: Documentation + close-out (4h estimate) — partial (7.2 done)

- [x] 7.1 V312-24 not in `debt-registry.yaml` (debt-registry tracks F-XX debt, not v3.12.0 issue items); V312-24 closure recorded in this `tasks.md` and the activation report instead.
- [x] 7.2 `docs/releases/v3.12.0/V312-24_test_infra_activation_report.md` — inventory table (12 items) + before/after coverage + test counts + gate output
- [ ] 7.3 Comment on #3911 with closure summary + evidence_hash — requires PR merge SHA; pending until PR opens
- [ ] 7.4 If any items still deferred at GA: append to `docs/releases/v3.12.0/historical-backlog-disposition.yml` — required at GA-time once phases 2-8 are dispositioned

## Phase 8: Self-audit + sign-off (4h estimate)

- [ ] 8.1 `cargo test --workspace --no-fail-fast` (verify no regressions introduced)
- [ ] 8.2 `cargo clippy --all-features -- -D warnings` (P2 gate)
- [ ] 8.3 `cargo fmt --check` (P1 gate)
- [ ] 8.4 `bash scripts/gate/check_anti_fabrication.sh` (P16 gate)
- [ ] 8.5 `bash scripts/gate/check_beta_gate.sh` (verify B10_SQLANCER now exit 0 + report)
- [ ] 8.6 2 reviewer approvals (per V312 master acceptance)

---

## Estimate breakdown

| Phase | Description | Hours |
|-------|-------------|-------|
| 1 | Activate test-infra skeletons | 24h |
| 2 | Retire dead-code E2E scripts | 4h |
| 3 | Fix warn-only E2E scripts | 8h |
| 4 | Anti-fabrication violations | 8h |
| 5 | SQL corpus activation (hardest) | 16h |
| 6 | Gate enforcement wiring | 8h |
| 7 | Documentation + close-out | 4h |
| 8 | Self-audit + sign-off | 4h |
| **Total** | | **76h** |

Per V312_DAG_ANALYSIS budget 16h for V312-24 — we will defer Phase 5 (corpus activation) to V312-24-corpus as follow-up if budget exceeded; remaining 7 phases fit ~60h but aggressive parallel execution achievable.

## Carried items (V312-24 → V312-XX follow-up)

- Phase 5 SQL corpus activation (16h) — defer to V312-24-corpus if Phase 1-4 + 6-8 take > 8 days; expires 2026-09-30 with owner opencode-z440
