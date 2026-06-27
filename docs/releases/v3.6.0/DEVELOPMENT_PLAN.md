# v3.6.0 Development Plan

## 1. 概述与目标

- 版本: v3.6.0
- 目标: 代码知识图谱集成 + WAL验证 + SIMD加速 + 覆盖率提升
- 成功定义: Beta Gate B1-B8 全部 PASS

## 2. 功能列表

| 级别 | 功能 | Issue | 状态 |
|------|------|-------|------|
| P0 | Alpha Gate PASS | I#2560 | ✅ |
| P0 | WALVerifier 生产集成 (TI-3) | I#2561 | ✅ |
| P0 | SIMD 向量化聚合加速 | I#2563 | ✅ |
| P1 | Parser 覆盖率 ≥75% | I#2567.B3 | ⚠️ 47.16% |
| P1 | Executor 覆盖率 ≥75% | I#2567.B4 | ⚠️ 72.04% |
| P1 | mysql-server tests2 修复 | I#2567.B5 | ❌ |
| P2 | FULL OUTER JOIN/MERGE executor | — | 延迟至 v3.7.0 |
| P2 | 分布式执行路径 | — | 延迟至 v3.7.0 |

## 3. 技术任务

- WALVerifier 生产代码路径集成 ✓
- vec_simd.rs 模块 + compute_aggregate 阈值调度 ✓
- event.rs / merge.rs 执行器测试 ✓

## 4. 测试策略

Alpha: cargo test --lib --exclude mysql-server (PASS)
Beta:  cargo test --workspace (mysql-server 修复后 PASS)

## 5. 门禁计划

| 阶段 | 入口条件 | 状态 |
|------|----------|------|
| Alpha | develop/v3.6.0 分支 | ✅ CONDITIONAL PASS |
| Beta  | Alpha PASS + 5 items | ❌ 3 items 待完成 |
| RC    | Beta PASS | ❌ |
| GA    | RC PASS | ❌ |

## 6. 版本延续任务

| 任务 | 来源版本 | 当前状态 |
|------|----------|----------|
| mysql-server tests2 重写 | v3.5.0 | 未完成 |
| Parser 覆盖率提升 | v3.5.0 (47%) | 未完成 |

## 7. 风险评估

| 风险 | 级别 | 缓解 |
|------|------|------|
| parser 结构性缺陷（嵌套测试） | 🔴 | 需重写测试模块结构 |
| mysql-server 测试债务 | 🟡 | 可重新基于 v3.5.0 修复 |
| Z6G4 编译资源 | 🟡 | 使用 CARGO_TARGET_DIR=/tmp |

## 8. 里程碑

| 日期 | 事件 |
|------|------|
| 2026-05-29 | Alpha Gate ✅ |
| 2026-06-15 | Beta Gate (目标) |
| 2026-06-30 | GA Release (目标) |

## 9. 附录

- GATE_CONTRACT: docs/releases/v3.6.0/GATE_CONTRACT_v3.6.0.md
- ALPHA_REPORT: docs/releases/v3.6.0/ALPHA_GATE_REPORT_v3.6.0.md
- Knowledge OS: hermes-ops-wiki Knowledge-OS-Integration-Guide
