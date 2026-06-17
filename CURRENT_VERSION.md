# 当前版本状态

v3.9.0-rc7 (develop/v3.9.0)

## 阶段信息

- **版本**: v3.9.0-rc7 (Release Candidate)
- **类型**: Production Readiness Release (工程化版本, 非功能版本)
- **当前里程碑**: Sprint 8 GA Gap Closure (PR #3465 merged)
- **分支**: develop/v3.9.0
- **当前 SHA**: `fb0e77758` (last merge: PR #3466)
- **关键合并**: `edcc3e20d` (PR #3465 — Sprint 8 Q8 + ADR-006 Phase 3 + soak_runner)
- **GA 目标**: 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
- **关键 Issue**: [#2778](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/2778) (启动) + #3225/#3265/#3266/#3229 (real soak) + #3312 (Q8 cell_diff, separate)

## 版本概述

v3.9.0 将 SQLRustGo 从 v3.8.0 的 **TransactionManager 集成**升级为 **Production-Ready GA Candidate**，核心战略：

> **核心问题反转**: 不是"支持多少 SQL"，而是"数据库死了以后还能不能回来"。

- 资源分配: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / **新 SQL 0%**
- 16 任务 / 451h / 6 Phase
- 新门禁 G1-G10 (取代 L1-L6)
- 详细计划: `docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md`

## v3.9.0 核心任务 (G1-G10 状态)

### Phase 1 - 架构债 (W1-2)

| 门禁 | 状态 | Issue |
|------|------|-------|
| G3 INT-3 关闭 | TBD | #3146 |
| G4 ARCH-3 关闭 | TBD | #3109 (P1) |

### Phase 2 - 集成债 (W3-4)

| 门禁 | 状态 | Issue |
|------|------|-------|
| G2 INT-2 关闭 | TBD | #3108 (P0) |
| G5 SEM-1 关闭 | TBD | #3146 |

### Phase 3 - 备份/恢复 (W5-6)

| 门禁 | 状态 | Issue |
|------|------|-------|
| G6 Backup/Restore 100+ 场景 | TBD | 待创建 |
| G8 Crash Matrix 100+ 场景 | TBD | 待创建 |

### Phase 4 - 长期稳定性 (W7-8)

| 门禁 | 状态 | Issue |
|------|------|-------|
| **G7 24h Soak** | 🟡 **INFRA DONE** (PR #3465: `sqlrustgo-mysql-server soak` 子命令). **Run PENDING** (needs Z6G4). | #3225/#3265/#3266/#3229 |
| G9 Upgrade Test 50+ 路径 | TBD | 待创建 |

### Phase 5 - 审计 (W9-10)

| 门禁 | 状态 | Issue |
|------|------|-------|
| G10 Audit + Time Travel 40+ tests | TBD | 待创建 |

### Phase 6 - 性能优化 + GA (W11-12)

| 门禁 | 状态 | Issue |
|------|------|-------|
| **G1 22/22 TPC-H 保持** | ✅ **22/22 PASS** (PR #3465, Q8 cartesian→hash 165,000× 加速到 0.18ms) | — |

## Sprint 8 GA Gap Closure (PR #3465, merged 2026-06-17)

### Track A — Q8 Sprint 8
- **Q8 perf**: 33s → **0.18ms** (165,000× 加速)
- **22/22 TPC-H**: PASS in 19.73ms (verified via `cargo test --release --test tpch_full_22_test`)
- 新算法 `extract_comma_join_keys` + JoinKey::All hash join fast path

### Track B — ADR-006 Phase 3 (5 meta-gates 全部 PASS)
- **V5**: `check_full_gate_verification.sh` 不再接受 DRIFT as PASS
- **V6**: `check_g_correctness_v390.sh` `|| true` 移除
- **V8**: 9 gate scripts `set -o pipefail` + `$?`/`PIPESTATUS` 检查
- **V2**: `tests/baseline/ignore_registry.json` 重生成 (42 + 1 marker)
- 5 meta-gate (P11/P12/P13/P14/P15): 全部 ✅ PASS

### Track C — `sqlrustgo-mysql-server soak` 子命令
- Real wall-clock 长期浸泡 binary
- 资源监控: RSS, FD, lock count, p99 latency
- JSONL time-series + Markdown report
- Graceful SIGTERM/SIGINT
- Long-stability tests 分析: `docs/releases/v3.9.0/LONG_STABILITY_TESTS_ANALYSIS.md`
- 26 long-running `#[ignore]` tests 已分析，Z6G4 验证步骤明确

## v3.9.0 vs v3.8.0

| 方面 | v3.8.0 | v3.9.0 |
|------|---------|---------|
| 类型 | TransactionManager 集成 | Production-Ready GA Candidate |
| 战略 | 事务生命周期管理 | 工程化 (可靠性 + 可恢复性 + 可审计性) |
| 新门禁 | L1-L6 | G1-G10 |
| Q8 perf | 0.2s (Sprint 6 alias-aware) | **0.18ms** (Sprint 8 hash join) |
| Soak infra | compressed 1,440× | **real wall-clock** binary |
| Meta-governance | ADR-007 (P16) | ADR-006 P11-P15 (V5/V6/V8/V2 done) |

## 相关文档

- [v3.9.0 文档入口](docs/releases/v3.9.0/README.md)
- [v3.9.0 专用 CHANGELOG](docs/releases/v3.9.0/CHANGELOG.md)
- [v3.9.0 路线图](docs/releases/v3.9.0/ROADMAP.md)
- [v3.9.0 Sprint 8 收敛跟踪](CONVERGENCE_TRACKER.md) (本仓库根)
- [v3.9.0 Q8 plan](docs/plans/2026-06-11-tpch-q8-cartesian-join-fix.md)
- [v3.9.0 Long-Stability Analysis](docs/releases/v3.9.0/LONG_STABILITY_TESTS_ANALYSIS.md)
- [ADR-006 Meta-Governance](docs/governance/adr/ADR-006-meta-governance.md)
- [v3.8.0 历史](docs/releases/v3.8.0/)

## 变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 2.0 | 2026-04-22 | 创建 v2.8.0 开发分支，基于 v2.7.0 GA |
| 3.0 | 2026-05-28 | 创建 v3.8.0 开发分支，基于 v3.7.0 GA |
| 4.0 | 2026-06-05 | 创建 v3.9.0 开发分支 (develop/v3.9.0)，基于 v3.8.0 GA |
| 5.0 | 2026-06-17 | **Sprint 8 GA Gap Closure**: PR #3465 merged (Q8 hash join + ADR-006 Phase 3 + soak_runner). 本版本状态文件从 v3.8.0-alpha 迁移至 v3.9.0-rc7. |
