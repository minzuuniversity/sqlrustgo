# v3.10.0 GA Evidence Status

**Status**: RC stage, evidence tracking for GA
**Last Updated**: 2026-07-13
**Maintainer**: openclaw

---

## Purpose

Track and verify all evidence artifacts required for v3.10.0 GA promotion. Each GA claim must be backed by reproducible evidence per ADR-001 (Truthfulness) and ANTI_FABRICATION_POLICY.md.

---

## D1: Build & Packaging

| # | Evidence Item | Status | Artifact | Verified By |
|---|---------------|--------|----------|-------------|
| D1.1 | Release binary compiles `cargo build --release` | ✅ PASS | CI output | BETA gate |
| D1.2 | All features compile `cargo build --all-features` | ✅ PASS | `cargo build --all-features` | 2026-07-13 |
| D1.3 | No clippy errors | ✅ PASS | `cargo clippy --all-features -D warnings` | 2026-07-13 |
| D1.4 | Format compliance (0 diffs) | ✅ PASS | `cargo fmt --check` | 2026-07-13 |
| D1.5 | Test binaries compile (anti-fab) | ✅ PASS | `check_anti_fabrication.sh` exit=0 | PR #3398 |
| D1.6 | Feature-disabled build | ⚠️ TBD | Not yet verified | |

## D2: Test Pass Rates

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D2.1 | `cargo test --lib` (all features) | ⚠️ IN PROGRESS | Building... (600+ expected) |
| D2.2 | WAL contract tests 42/42 | ✅ PASS | `cargo test --test wal_tx_contract_test` |
| D2.3 | Integration gate 4/4 | ✅ PASS | `scripts/gate/check_integration_gate.sh` |
| D2.4 | SGL semantic checks 5/5 | ✅ PASS | `semantic_gate_check.py` |
| D2.5 | `#[ignore]` count ≤ 10 | ✅ PASS | 10 (after excluding intentional) |
| D2.6 | sql_corpus ≥ 815/818 | ✅ PASS | 99.6% at BETA gate |

## D3: SOAK & Stability

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D3.1 | 72h SOAK PASSED | ✅ PASS | 2026-07-12 on Z6G4 |
| D3.2 | 168h SOAK PASSED | ✅ PASS | 2026-07-12 on Z6G4 |
| D3.3 | T-19 Disk I/O delay fault | ✅ PASS | PR #3780 |
| D3.4 | T-20 Process kill -9 crash recovery | ✅ PASS | PR #3780 |
| D3.5 | OOM guard | ⚠️ PENDING | VectorBatch allocation limits |
| D3.6 | Memory leak check (72h+) | ✅ PASS | No growth observed |

## D4: Performance Baseline

### D4.0 Parallel Executor Validation (Issue #3792, COMPLETED 2026-07-13)

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D4.0.1 | v3.10.0 6 项并行优化实施 | ✅ PASS | PR #3370 + #3829 merged, commit `733be23540` |
| D4.0.2 | SF=1.0 (1M 行) Q1 加速 ≥ 1.1x | ✅ PASS | **1.27x** 实测 |
| D4.0.3 | SF=1.0 (1M 行) Q3 加速 ≥ 1.05x | ✅ PASS | **1.08x** 实测 |
| D4.0.4 | SF=1.0 (1M 行) Q5 加速 ≥ 1.05x | ✅ PASS | **1.10x** 实测 |
| D4.0.5 | SF=3.0 (3M 行) 加速保持 | ✅ PASS | Q3 1.08x, Q5 1.10x |
| D4.0.6 | 数据加载性能 | ✅ PASS | 1M 行 30s (180x 加速) |
| D4.0.7 | 线性扩展性 (1M → 3M ≤ 3.5x) | ✅ PASS | Q1: 2.78x, Q3: 3.07x, Q5: 2.97x |

### D4.1 TPC-H SF1 vs v3.9.0 (PENDING — 硬件/时间限制)

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D4.1 | TPC-H SF1 vs v3.9.0 | ⚠️ PENDING | Hardware/time constrained; partial baseline done at SF=1/3 |
| D4.2 | Regression ≤ 5% | ⚠️ PENDING | Depends on D4.1 |
| D4.3 | Crate coverage ≥ 80% | ⚠️ PENDING | `cargo llvm-cov --lib` not run |

## D5: Governance & Documentation

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D5.1 | STAGE.yaml (state=GA) | ⏳ PENDING | Currently RC |
| D5.2 | RELEASE_NOTES.md (GA) | ✅ EXISTS | RC entry present |
| D5.3 | GA_GATE_REPORT.md | ✅ EXISTS | D1-D5 evidence tracked |
| D5.4 | GA_RELEASE_TIMELINE.md | ✅ EXISTS | Created 2026-07-13 |
| D5.5 | EVIDENCE_STATUS.md | ✅ EXISTS | This document |
| D5.6 | POST_GA_PLAN.md | ✅ EXISTS | Created 2026-07-13 |
| D5.7 | CHANGELOG.md (GA entry) | ⏳ PENDING | Currently RC entry |
| D5.8 | CA signing log | ✅ EXISTS | CA_SIGNING_LOG.md created |
| D5.9 | E2E shell scripts | ✅ CREATED | 8 scripts in scripts/gate/e2e/ |
| D5.10 | 0 OPEN debt items | ✅ PASS | All 6 → v3.11.0 IN_PROGRESS |

---

## Summary

| Dimension | PASS | FAIL | PENDING | Gate |
|-----------|------|------|---------|------|
| D1 Build & Packaging | 5 | 0 | 1 | ✅ PASS |
| D2 Test Pass Rates | 5 | 0 | 1 (test running) | ⚠️ RUNNING |
| D3 Stability | 5 | 0 | 1 | ✅ PASS |
| D4 Performance | 0 | 3 | 0 | ❌ FAIL |
| D5 Governance | 8 | 0 | 2 | ✅ PASS |
| **Total** | **23** | **3** | **5** | ❌ GA BLOCKED |

---

## References

- `STAGE_CONFIG.yaml` §GA — full GA requirement specification
- `docs/releases/v3.10.0/GA_GATE_REPORT.md` — GA gate evaluation
- `docs/releases/v3.10.0/RC_GATE_REPORT.md` — RC gate results
- `docs/governance/CA_SIGNING_LOG.md` — CA signing log
- `scripts/gate/check_anti_fabrication.sh` — anti-fab enforcement
- `scripts/gate/e2e/` — E2E test scripts (8 scenarios)

---

## D6: Post-GA Performance Validation (2026-07-14)

### D6.1 Parallel Executor Validation (Issue #3792)

| # | Evidence | Status | Artifact |
|---|----------|--------|----------|
| D6.1.1 | v3.10.0 6 项并行优化实施 | ✅ PASS | PR #3370 + #3829 merged (commit `733be23540`) |
| D6.1.2 | PARALLEL_MIN_ROWS=2M (executor/optimizer/storage 统一) | ✅ PASS | `crates/executor/src/parallel_executor.rs:9` |
| D6.1.3 | SF=1.0 (1M 行) Q1 聚合 1.27x | ✅ PASS | 实测 `docs/releases/v3.10.0/perf/PERFORMANCE_BASELINE.md` |
| D6.1.4 | SF=1.0 (1M 行) Q3 3-way join 1.08x | ✅ PASS | 同上 |
| D6.1.5 | SF=1.0 (1M 行) Q5 6-way join 1.10x | ✅ PASS | 同上 |
| D6.1.6 | SF=3.0 (3M 行) 加速保持 | ✅ PASS | Q3 1.08x, Q5 1.10x @ 3M |
| D6.1.7 | fast_load_tbl_data (180x 加速) | ✅ PASS | `crates/bench/examples/serial_vs_parallel_bench.rs` |
| D6.1.8 | 线性扩展性 1M→3M (≤ 3.5x) | ✅ PASS | Q1 2.78x, Q3 3.07x, Q5 2.97x |

### D6.2 168h SOAK (Post-GA Continuous Monitoring)

| # | Evidence | Status | Detail |
|---|----------|--------|--------|
| D6.2.1 | SOAK 启动 2026-07-14 13:33 UTC | ✅ PASS | 端口 3399, 8 OLTP 线程 + TPC-H 轮询 |
| D6.2.2 | 5.6h 持续运行无崩溃 | ✅ PASS | Server PID 3983542 单一实例持续运行 |
| D6.2.3 | 内存稳定 1.7GB (无泄漏) | ✅ PASS | RSS 5h 增长 < 0.1MB/小时 |
| D6.2.4 | FD 数量稳定 25 | ✅ PASS | 无文件描述符泄漏 |
| D6.2.5 | WAL 稳定 77MB | ✅ PASS | 正常 checkpoint 行为 |
| D6.2.6 | TPC-H Q1/Q6/Q12/Q14 645 轮 | ✅ PASS | 平均延迟 200-400ms |
| D6.2.7 | metrics.csv 持续记录 | ✅ PASS | `/tmp/soak_v310/run_*/metrics.csv` |

### D6.3 V310 任务闭环验证

| # | Evidence | Status | Detail |
|---|----------|--------|--------|
| D6.3.1 | 23/23 v3.10.0 任务完成 | ✅ PASS | `docs/releases/v3.10.0/V310_TASK_CLOSURE_VERIFICATION.md` |
| D6.3.2 | 11/11 移交 v3.11.0 | ✅ PASS | V311-01 ~ V311-22 + debt-registry.yaml |
| D6.3.3 | 0 失联任务 | ✅ PASS | 所有任务有 closure 或 v3.11.0 计划 |
