# v3.9.0 真测试基础设施整改 (E2E / 性能 / 稳定性) - 跟踪

> **创建日期**: 2026-06-06
> **作者**: Hermes Agent
> **状态**: OPEN
> **Priority**: P0 (blocker for v3.9.0-ga)
> **相关 audit**:
> - `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md` (Hermes)
> - `b77c2fcb` docs(v3.9.0): TPC-H real-effectiveness audit (Macmini)
> - `docs/discovery/2026-06-05-tpch-22-mysql-server-comprehensive-report.md`
> **Related issues**:
> - #3215 TPC-H Q22 parse error (已修 by PR-3213)
> - #3216 TPC-H Q8 6-table join
> - #3217 TPC-H Q9 complex join

---

## 1. 概述

v3.9.0-rc2 (HEAD = 76efe391) 切标签 **不应** 在 production-equivalent 覆盖率 35% 时. 必须先整改 6 个 categories 的测试基础设施, 达到 ≥80% 才考虑 GA.

## 2. 必须整改的 6 categories

### 2.1 🔴 E2E L3 acceptance (15 tests) - 0/15 跑

**文件**: `tests/e2e_canonical_subprocess.rs` (15 tests, all `#[ignore]` "L3 acceptance — implementation pending")

**整改路径**:
- 实施 L3 acceptance binary (canonical subprocess + MySQL client)
- 15 tests unignore 顺序: DDL/DML → DQL → TX → SHOW → EXEC/BENCH
- 估 8-12h

**Owner**: TBD (priority P0)

### 2.2 🔴 L3 acceptance binary test (1 test) - 0/1 跑

**文件**: `tests/l3_canonical_binary.rs::l3_canonical_binary_serve_handshake_auth_and_query` (`#[ignore]` "L3 acceptance — implementation pending")

**整改路径**: 同 2.1, L3 实施时一起 unignore

### 2.3 🔴 Long stability (14 tests) - 0/14 跑

**文件**:
- `tests/long_run_stability_test.rs` (10 tests, all `#[ignore]` ">1h, run with --ignored, dedicated test env")
- `tests/long_run_stability_72h_test.rs` (4 tests, all `#[ignore]` "72h long-running")

**整改路径**:
- 1h+ real wall-clock stability (后台跑)
- 24h/72h/168h 真实 wall-clock (不能 simulated)
- 估 24-168h 跑 + 1h 写 analysis
- 需要专用 long-running CI env

### 2.4 🔴 Soak tests "10/10 PASS" = SIMULATED (not real)

**文件**: `tests/soak_test.rs` (10 tests, all "PASS" but simulated per `SOAK_72H_REPORT.md` §5)

**整改路径**:
- 用 `soak_runner` binary 跑真实 TPC-H queries (1 q/s for 72h = 259,200 real queries)
- 估 72h 跑 + 4h 写 RC_72H_REAL_SOAK_REPORT.md
- 资源 monitor: RSS, FD, lock count, p99 latency

### 2.5 🔴 QPS / Perf bench (18 tests) - 0/18 跑

**文件**:
- `tests/qps_benchmark_test.rs` (10 tests, all `#[ignore]` "long runtime")
- `tests/bench_v380_point_agg.rs` (6 tests, all `#[ignore]` "performance benchmark")
- `tests/perf_eng_batched_insert_test.rs` (2 tests, all `#[ignore]` "performance gate")

**整改路径**:
- 在 Z6G4 (192.168.0.252) 跑真实 perf measurement
- 填 `docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md` (目前全 TBD)
- 估 4-8h 跑 + 1h 写报告

### 2.6 🟡 TPC-H 真 bug 剩余 2 (Q8, Q9)

**来源**: Macmini `b77c2fcb` audit + issues #3216, #3217

**整改路径**:
- **Q8 6-table join** ("Join condition must reference one column from each side") - parser / executor 改
- **Q9 complex join** (同 Q8) - 同
- 估 4-8h

## 3. 整改时间线 (估)

| Task | Owner | 估时 | 阻塞 GA? |
|------|-------|------|----------|
| L3 acceptance binary 实施 | TBD | 8-12h | 🔴 yes |
| L3 unignore 15 E2E tests | TBD | 4-6h | 🔴 yes |
| Long stability 1h+ real | TBD | 24-168h | 🔴 yes |
| Real 72h soak | TBD | 72h + 4h | 🔴 yes |
| Perf bench real Z6G4 | TBD | 4-8h | 🔴 yes |
| Perf baseline 填实 | TBD | 1h | 🔴 yes |
| Q8 6-table join | TBD | 2-4h | 🟡 nice-to-have |
| Q9 complex join | TBD | 2-4h | 🟡 nice-to-have |

**总估**: 200-300h 工程 (1-2 month)

## 4. 决策点

### 4.1 选 A: 撤回 v3.9.0-rc1/rc2, 改 pre-rc1 (推荐)

理由:
- L3 没实施, E2E 0% 跑
- Perf 全 TBD
- Long stability 0% 跑
- Soak 是模拟

**实施**: 把 `v3.9.0-rc1` 和 `v3.9.0-rc2` 标签都改为 `v3.9.0-pre-rc1` (or `v3.9.0-beta`), 加 audit disclaimer 到 release notes.

### 4.2 选 B: 接受现状, 标记 v3.9.0-rc1/rc2 = "in-process 22/22 only"

理由:
- TPC-H 22/22 (in-process) 真实
- 其余用 "WIP" / "Pending" 标注

**风险**: 用户/投资人误以为 production-ready.

### 4.3 选 C: 立即修 server LOAD DATA perf + L3 acceptance binary

理由:
- 估 12-20h, 1-2 个工作日
- 修完 wire 22/22 + E2E 15 unignore = 真覆盖率从 35% → 60%

## 5. 关联文档

- `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md` (本 audit)
- `docs/discovery/2026-06-05-tpch-22-mysql-server-comprehensive-report.md` (Hermes 2026-06-05)
- `b77c2fcb` (Macmini TPC-H real-effectiveness audit)
- `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` (soak simulated 声明)
- `docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md` (全 TBD)
- `tests/tpch_hashes_v380.json` (PENDING placeholder)

## 6. 标签

- `ai-task` - AI 整改任务
- `governance` - 治理
- `priority/p0` - 阻塞 GA
- `role/parser`, `role/executor`, `role/storage`, `role/infra`

## 7. 验收标准

v3.9.0-ga 可切标签当:
- [ ] L3 acceptance 实施, 15/15 E2E tests pass
- [ ] 1h+ real wall-clock stability ≥ 1 test pass
- [ ] 24h real soak ≥ 1 test pass  
- [ ] 72h real soak ≥ 1 test pass
- [ ] Z6G4 真实 QPS/TPS 测量, PERFORMANCE_BASELINE.md 全填实
- [ ] TPC-H 22/22 wire 跑得动 (server LOAD DATA perf blocker 修)
- [ ] Q8, Q9 真 bug 修 (optional)
- [ ] TPC-H hash baseline `tpc_h_hash_sha256` 真值, 不是 0000...

**当前**: 0/8 满足. **真 production-equivalent 覆盖率 ≈ 35%**.

## 8. 参考

- 李哥 2026-06-05 22:46: "我目前严重怀疑你完成的各种测试的真实性, 我需要你用 gitnexus 工具, 对所有的集成测试和性能测试的实现进行分析和确认"
- 李哥 2026-06-06 (今): "1, 3, 4 这样的顺序执行" (撤回标签 → 修 server perf → 实施 L3)
