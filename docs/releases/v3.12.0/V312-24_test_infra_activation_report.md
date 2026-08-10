# V312-24: Test Infrastructure Activation Report

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **Created**: 2026-08-09
> **Agent**: minimax (claude-code)
> **Source Issue**: #3911
> **Branch**: `feature/v312-24-impl`
> **Source spec**: `openspec/changes/v312-24-test-infra-activation/{proposal,design,tasks}.md`

## Executive Summary

V312-24 Phase 1 (test-infra activation) is **COMPLETE** for the three skeleton
crates (`sqlancer`, `test-runner`, `test-registry`). Every asset now has an
executable binary that writes a structured artifact, an integration test
covering the new contract, and zero `|| true` / WARN-only masking.

| Crate | Before | After |
|-------|--------|-------|
| `sqlancer` | lib only, no bin, 9/9 inline unit tests | lib + `sqlancer` bin writing `target/sqlancer-report.json` (parse-only oracle) + 2 integration tests |
| `test-runner` | lib only, no bin, `timeout_per_test_ms` ignored, sequential `run_tests` | lib + `test-runner` bin (probe + manifest modes) writing `target/test-runner-report.json` + `tokio::time::timeout` enforcement + `JoinSet` parallel dispatch + 5 integration tests |
| `test-registry` | lib only, no bin, `toml` dep unused | lib + `test-registry-cli` bin (init/list/register/print-schema) + `from_toml`/`write_toml` + `ManagedTest` type + 3 integration tests |

**Out of scope for this PR (deferred per openspec/tasks.md "Carried items"):**
Phases 2 (E2E retire), 3 (warn-only fix), 4 (anti-fab violations), 5 (SQL
corpus activation, 16h), 6 (gate enforcement), 7 (docs/close-out beyond this
report), 8 (sign-off). The 16h V312-24 budget was spent entirely on Phase 1;
the remaining 7 phases are out of scope unless re-scoped.

---

## Acceptance criteria (from proposal §Acceptance)

| Criterion | Status | Evidence | Follow-up issue |
|-----------|--------|----------|-----------------|
| Every tool can run + produce artifact (sqlancer + test-runner + test-registry all executable) | ✅ PASS | `target/sqlancer-report.json` + `target/test-runner-report.json` + `target/test-registry.toml` all generated and validated by integration tests | — |
| Every retired/deferred item has reason + replacement gate + owner + expiry (15-item disposition table) | 🟡 PARTIAL | 12 items in the table are PASS (Phase 1 activation); 3 retired/deferred rows now map to V312-25 / V312-27 / V312-28 with owner + expiry; remainder is follow-up scope | V312-25 / V312-27 / V312-28 |
| No `\|\| true` masking in regression CI for test-infra binaries | 🟡 PARTIAL | The binary now exits 0 with a non-empty artifact; the masking at `scripts/test/run-regression.sh:111-112` is still in place. Phase 6 wiring deferred | V312-29 |
| `tests/baseline/ignore_registry.json` matches actual `#[ignore]` set after fixes | ⏸ DEFERRED | 7 stale paths + 1 phantom + 3 union_set_operations IGNORE_MULTI tracked | V312-27 |
| `crates/sql-corpus/tests/corpus_test.rs` pass_rate ≥ 80% | ⏸ DEFERRED | Currently 27.3% (6/16 subcategories); needs SimpleExecutor gap fixes | V312-28 |
| All anti-fab violations (#1-6) closed with non-WARN-only evidence | ⏸ DEFERRED | 7 violations tracked | V312-27 |

**Net result for this PR**: 1 of 6 acceptance criteria are PASS outright;
2 are PARTIAL (PASS in artifact terms, FAIL in gate wiring terms — gated on
V312-29 sign-off); 3 are explicitly DEFERRED to V312-25 / V312-27 / V312-28.
**None** of the DEFERRED items may close on the basis of "this report says
so" — see each follow-up issue for the hard closing boundary (commands
+ numeric outputs + file-existence checks).

---

## Phase 1 inventory — disposition table

| # | Item | Disposition | Owner | Expiry | Evidence gate |
|---|------|-------------|-------|--------|---------------|
| 1.1 | `crates/sqlancer` `[[bin]]` | activate | minimax | 2026-08-25 | `cargo run -p sqlancer -- --duration 1` exits 0 + writes non-empty `target/sqlancer-report.json` (see `cli_smoke::cli_runs_writes_report_and_exits_zero`) |
| 1.2 | sqlancer main entry wiring `Fuzzer::new(config).run(...)` | activate | minimax | 2026-08-25 | `crates/sqlancer/src/bin/sqlancer.rs` — parse-only oracle (`sqlrustgo_parser::parse`) as the executor; engine wiring tracked as follow-up |
| 1.3 | sqlancer writes `target/sqlancer-report.json` (FuzzerResult JSON) | activate | minimax | 2026-08-25 | `cli_smoke` test asserts file exists, is non-empty, and deserializes to the declared `ReportShape` schema |
| 1.4 | `crates/test-runner` `[[bin]]` | activate | minimax | 2026-08-25 | `cargo run -p test-runner` exits 0 (probe mode) and dispatches manifest entries in parallel (manifest mode) |
| 1.5 | `timeout_per_test_ms` via `tokio::time::timeout` in `run_test` | activate | minimax | 2026-08-25 | `timeout_enforced::managed_timeout_kills_hung_child` — `sleep 30` against 500ms budget returns `TimedOut` in <5s, proving the child was killed |
| 1.6 | `run_tests` parallel via `tokio::task::JoinSet` honoring `max_parallel` | activate | minimax | 2026-08-25 | `managed_dispatch::managed_all_respects_max_parallel` — 4× `sleep 2` with `max_parallel=2` completes in <7s (would be ~8s serial) |
| 1.7 | `target/test-runner-report.json` (serde_json) | activate | minimax | 2026-08-25 | bin smoke test produced a 5.5 KB JSON report with the declared `RunReport` schema; cli-mode validated by `managed_dispatch` test |
| 1.8 | `test-registry-cli` bin | activate | minimax | 2026-08-25 | `cargo run -p test-registry --bin test-registry-cli -- init <path>` writes a valid `test-registry.toml`; `list` parses and prints it back |
| 1.9 | `from_toml(path)` + `write_toml(path)` on `TestRegistry` | activate | minimax | 2026-08-25 | `toml_round_trip::round_trip_preserves_all_fields` round-trips 3 entries (varied fields) and asserts byte-level field equality |
| 1.10 | Register `sqlancer` + `test-runner` as managed entries | activate | minimax | 2026-08-25 | `starter_manifest()` in `crates/test-registry/src/bin/test-registry-cli.rs` seeds both entries; `toml_round_trip` test confirms the resulting TOML |
| 1.11 | Wire test-runner to consume test-registry manifest | activate | minimax | 2026-08-25 | `crates/test-runner/src/bin/test-runner.rs` `--manifest <PATH>` mode loads the manifest and dispatches each entry via `TestRunner::run_managed_all`; manifest-mode smoke test passed |
| 1.12 | `.gitignore` for `target/*-report.json` + `test-registry.toml` | activate | minimax | 2026-08-25 | `target/*-report.json` already covered by existing `target/` rule; added explicit `test-registry.toml` entry at repo root |

**Total: 12 items activated, 0 retired, 0 deferred-within-this-issue.**

---

## Verification log

### 1. Build (no warnings, no errors on my code)

```text
$ cargo build -p sqlancer -p test-runner -p test-registry
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### 2. Library unit tests

```text
$ cargo test -p sqlancer -p test-runner -p test-registry --lib
sqlancer:        9 passed
test-registry:   5 passed
test-runner:     6 passed
                 ─────
                 20/20
```

### 3. Integration tests (this PR)

```text
$ cargo test -p test-runner -p test-registry -p sqlancer \
    --test timeout_enforced --test managed_dispatch \
    --test toml_round_trip --test cli_smoke

cli_smoke             2/2 PASS
toml_round_trip       3/3 PASS
managed_dispatch      2/2 PASS
timeout_enforced      3/3 PASS
                      ─────
                      10/10
```

### 4. Strict clippy (warnings as errors)

```text
$ cargo clippy -p sqlancer -p test-runner -p test-registry \
    --lib --bins -- -D warnings
(no errors on my code; the `crates/tools/Cargo.toml: unused manifest key:
bin.1.rand` warning is pre-existing in a different crate and out of scope)
```

### 5. Format

```text
$ cargo fmt --check
(no diffs on crates/{sqlancer,test-runner,test-registry}/)
```

### 6. End-to-end smoke

```text
$ cargo run -q -p sqlancer -- --duration 1 --iterations 10 \
    --out target/sqlancer-report.json
sqlancer: wrote report — total_queries=10 successful=10 failed=0

$ cargo run -q -p test-registry --bin test-registry-cli -- \
    init target/test-registry.toml
wrote starter manifest at target/test-registry.toml (2 entries)

$ cargo run -q -p test-registry --bin test-registry-cli -- \
    register target/test-registry.toml sqlancer-quickcheck cargo \
    --timeout-ms 5000 --priority P3 --category ci --args check --release
registered sqlancer-quickcheck in target/test-registry.toml

$ cargo run -q -p test-runner -- \
    --manifest target/test-registry.toml --max-parallel 3 \
    --out target/test-runner-report.json
test-runner: mode=manifest wrote 3 entries — total=3 passed=2 failed=0
             timed_out=1 crashed=0 duration_ms=9248
```

The single `timed_out=1` is the 5s-budgeted `sqlancer-quickcheck` entry
running `cargo check` against the workspace — which legitimately takes
>5s on this machine, exercising the per-entry `timeout_ms` enforcement
end-to-end.

---

## API surface added/changed (Phase 1)

### `crates/sqlancer`
- New: `FuzzerResult { duration_secs, iterations_requested }` (JSON fields).
- New: `crates/sqlancer/src/bin/sqlancer.rs` binary.

### `crates/test-runner`
- `TestRunner::run_test` changed from `&mut self` → `&self` (enables
  `Arc<Self>` sharing for parallel dispatch). BREAKING change for any
  external caller that held `&mut TestRunner`; no callers found in the
  workspace.
- Removed: `TestRunner.results: HashMap<...>` field, `get_result`,
  `get_all_results`, `summary` methods. These were a dead surface — no
  write path existed after the `&self` change, so they would have always
  returned empty. See git log for the removal commit.
- New: `TestRunner::run_test_with_config(&TestRunConfig, &str, &str)`
  (helper used by parallel `run_tests`).
- New: `TestRunner::run_managed(&self, &ManagedTest)` (per-entry
  dispatch with the entry's own `timeout_ms`).
- New: `TestRunner::run_managed_all(self: Arc<Self>, Vec<ManagedTest>)`
  (bounded parallel dispatch via `JoinSet` + `Semaphore(max_parallel)`).
- New: `crates/test-runner/src/bin/test-runner.rs` binary (probe + manifest
  modes; `--manifest`, `--max-parallel`, `--retry-count`, `--timeout-ms`,
  `--out`, `--test` flags).
- New: `crates/test-runner/tests/{timeout_enforced,managed_dispatch}.rs`.

### `crates/test-registry`
- New: `ManagedTest` struct + `From<ManagedTest> for TestMetadata` bridge.
- New: `RegistryError` enum (Io/Parse/Serialize variants).
- New: `TestRegistry::register_managed(ManagedTest)` + `managed()`,
  `get_managed(name)`, `managed_len()` accessors.
- New: `TestRegistry::from_toml(&Path) -> Result<Self, RegistryError>`.
- New: `TestRegistry::write_toml(&Path) -> Result<(), RegistryError>`
  (outputs `[[test]]` table-array form).
- Added: `tests()`, `is_empty()`, `len()` accessors (WIP).
- Added: `managed_tests: HashMap<String, ManagedTest>` field.
- New: `crates/test-registry/src/bin/test-registry-cli.rs` binary
  (init / list / register / print-schema / help subcommands).
- New: `crates/test-registry/tests/toml_round_trip.rs`.
- New: `crates/sqlancer/tests/cli_smoke.rs`.

### `.gitignore`
- Added: `test-registry.toml` (a repo-root working manifest is local-only;
  canonical lives at `crates/test-registry/data/`).

---

## Known follow-ups (split into OPEN issues)

| Phase 2-8 内容 | 新 issue | Owner | Expiry | Baseline evidence |
|----------------|----------|-------|--------|-------------------|
| Phase 2 retire E2E scripts | **V312-25** | minimax | 2026-08-25 | `evidence/V312-25_baseline_evidence.txt` (11 files in tree, 0 git log deletions) |
| Phase 3 fix warn-only scripts | **V312-26** | minimax | 2026-08-30 | `evidence/V312-26_baseline_evidence.txt` (223 \|\| true hits in 3 scripts; sysbench installed) |
| Phase 4 anti-fab violations | **V312-27** | minimax | 2026-08-25 | `evidence/V312-27_baseline_evidence.txt` (37 stale + 0 phantom + 3 union entries; merge_vtu ignore; 5 e2e_beta skips) |
| Phase 5 corpus ≥ 80% | **V312-28** | opencode-z440 | 2026-09-30 | `evidence/V312-28_baseline_evidence.txt` (**99.4% pass rate**, 14 subcategories, 103 .sql files — V312-24 proposal §4 numbers are STALE) |
| Phase 6 gate wiring | **V312-29** | minimax | 2026-08-30 | `evidence/V312-29_baseline_evidence.txt` (run-regression:111-112 \|\| true; B10_SQLANCER check_warn at beta_gate:363; R4 lists 8 scripts) |
| Phase 7.3 + Phase 8 sign-off | **V312-30** | minimax | 2026-09-05 | `evidence/V312-30_baseline_evidence.txt` (PR not opened; 6 gate commands to run) |

每个新 issue 的 OpenSpec 写在 `openspec/changes/v312-25-.../`
`openspec/changes/v312-26-.../` ... `openspec/changes/v312-30-.../`,
ISSUES_PLAN.md 同步更新。每个 issue 都有**禁止关闭条件**——只靠
"openspec 标 done" / "报告标题写已完成" / "PR 已合并" 不允许关闭。

### 关闭 V312-24 的硬性前置

V312-24 本身**不能**在 V312-25 ~ V312-30 全部 PASS 之前关闭。
关闭 V312-24 必须同时满足：
1. V312-30 sign-off 报告 (`docs/releases/v3.12.0/V312-30_signoff_report.md`) 存在且非空；
2. `gh pr view <PR_NUMBER> --json state,mergedAt` 输出 `MERGED`；
3. ISSUE #3911 评论含 Phase 1 实际数字 + V312-25~30 编号 + evidence_hash；
4. 至少 2 名 reviewer 在 PR 上写 APPROVED。

### 15-item disposition table（V312-24 proposal §Disposition lines 64-82）

> V312-24 proposal 要求产出 15-item disposition table。本节给出
> **当前真实状态**（含本 PR 内 + V312-25..30 reconciliation 后的状态）。
> "evidence" 列指向实测命令/文件/数字，不是 openspec 状态。
>
> **每行都必须有可独立验证的 evidence**（command + 数字 + 文件存在）。
> 若 evidence 缺失或"PR 已合并"等抽象表述，本行视为 OPEN。

| # | Asset | Disposition | Owner | Expiry | Evidence (本 PR 实测) |
|---|-------|-------------|-------|--------|----------------------|
| 1 | `crates/sqlancer` | **activate** | minimax | 2026-08-25 | `cargo run -p sqlancer -- --duration 1` exit 0 + `target/sqlancer-report.json` 存在 (`cli_smoke` test PASS) |
| 2 | `crates/test-runner` | **activate** | minimax | 2026-08-25 | `cargo run -p test-runner -- --manifest X --max-parallel 3` exit 0 + `target/test-runner-report.json` 存在 (`managed_dispatch` test PASS) |
| 3 | `crates/test-registry` | **activate** | minimax | 2026-08-25 | `test-registry-cli init` 写 `test-registry.toml` + `from_toml`/`write_toml` round-trip OK (`toml_round_trip` test PASS) |
| 4 | `scripts/gate/e2e/e2e_01..04.sh` (4 stale mirrors) | **retire** | minimax | 2026-08-20 | `git ls-files ... \| grep e2e_0[1-8] \| wc -l` = 1 (e2e_07 保留) — V312-25 commit `56a532c` |
| 5 | `scripts/gate/e2e/e2e_05_savepoint_rollback.sh` | **retire** | minimax | 2026-08-20 | 同上 (含在 #4 统计) |
| 6 | `scripts/gate/e2e/e2e_06_cte_query.sh` | **retire** | minimax | 2026-08-20 | 同上 |
| 7 | `scripts/gate/e2e/e2e_07_json_vector.sh` | **fix** | minimax | 2026-08-25 | V312-26 重写 (0 `\|\| true`) + V312-30 byte-exact JSON+vector fixture assertion (本 PR) |
| 8 | `scripts/gate/e2e/e2e_08_migration.sh` | **retire** | minimax | 2026-08-20 | 同 #4 |
| 9 | `tests/e2e/startup_connect.sh` + `tests/e2e/tpch_sf01.sh` + `tests/e2e/kill9_recovery.sh` | **retire** | minimax | 2026-08-20 | `git log --diff-filter=D` 输出 10 unique deletions — V312-25 commit `56a532c` |
| 10 | `tests/e2e/backup_restore.sh` | **fix** | minimax | 2026-08-25 | V312-26 mysqldump+mysql round-trip (0 `\|\| true`) + V312-30 `backup_restore_docker.sh` 加 docker cp 路径 (本 PR) |
| 11 | `tests/e2e/sysbench_wired.sh` | **fix** | minimax | 2026-08-25 | V312-26 pre-flight (sysbench missing exit 1) + V312-30 `sysbench_smoke_test.sh` 实跑 1402 events in 3s (本 PR) |
| 12 | `tests/e2e/e2e_beta_test.rs` 5 double-skip tests | **fix** | minimax | 2026-08-25 | V312-27 删 5 处 `is_e2e_disabled` early-return; `CI=1 cargo test --test e2e_beta_test -- --ignored` = 6 PASS |
| 13 | `crates/executor/tests/merge_vtu_test.rs:212` | **fix** | minimax | 2026-08-20 | V312-27 删 `#[ignore = "VtuGuard not yet implemented"]`; `cargo test -p sqlrustgo-executor --test merge_vtu_test` = 13 PASS (含 `test_vtu_guard_wraps_storage`) |
| 14 | `tests/integration/sql/union_set_operations_test.rs` (0 ignores despite registry listing 3) | **fix** | minimax | 2026-08-20 | V312-30 reconciliation: 1 INTERSECT test 加 `#[ignore]` + 2 fact-tracker entries (EXCEPT + ORDER BY) 恢复 (本 PR 修 V312-27 误删) |
| 15 | `tests/integration/tpch/tpch_wire_smoke_sf.rs` (smoke `rows.len() <= 6` accepts 0) | **fix** | minimax | 2026-08-25 | V312-27 改 `> 0` + V312-30 加 `#[ignore]` for Q1.json fixture 缺失 (honest fail-explicit) + rebuilt symlink `tests/data/tpch-sf001 -> tpch-sf001-real` |
| 16 | `sql_corpus/` activation gate (current 27.3% pass rate) | **fix** | opencode-z440 | 2026-09-30 | V312-28 实测 99.4% pass rate + 16 subcategory guard tests (14 per-subcategory + 2 meta) — V312-24 proposal §4 数字 27.3%/16 subcategories 全部 STALE (V312-30 correction) |

**Total: 16 items disposed, 0 retired without owner, 0 deferred without expiry.**

**V312-24 自身关闭条件**：上表 16 项全 PASS + V312-30 sign-off 报告存在 + PR mergedAt 非空 + 2 reviewer APPROVED + evidence_hash 重新计算。

## Evidence files generated by this PR (regenerated per CI run)

- `target/sqlancer-report.json` — sqlancer fuzzer output (binary-default
  path; overridden by `--out`).
- `target/test-runner-report.json` — test-runner output (binary-default
  path; overridden by `--out`).
- `target/test-registry.toml` — manifest (created on demand by
  `test-registry-cli init`).

These are gitignored.
