# v3.7.0 (2026-05-30) — GA 集成债务清算

> **状态**: Refactoring（非生产 GA）— 重构里程碑
>
> **注意**: v3.7.0 为集成债务清算重构版本。声称 GA Gate PASS 的门禁数据存在 SKIP 占位（R4/R5），
> 覆盖率差 0.01pp，INT-1~INT-4 未完全解决。WAL/并行/CBO 仍在整合至 mysql-server 路径中。
>
> **后续**: v3.8.0 为 Execution Architecture Consolidation，聚焦架构统一（Path A/B/C）。

## 一、版本定位

v3.7.0 是 SQLRustGo 的**集成债务清算**重构版本，旨在统一双链路执行路径（Path A/B/C）。

## 二、关键文档

| 文档 | 说明 |
|------|------|
| [ALPHA_GATE_REPORT.md](ALPHA_GATE_REPORT.md) | Alpha 门禁报告 |
| [BETA_GATE_REPORT.md](BETA_GATE_REPORT.md) | Beta 门禁报告 |
| [RC_GATE_REPORT.md](RC_GATE_REPORT.md) | RC 门禁报告 |
| [GA_GATE_REPORT.md](GA_GATE_REPORT.md) | GA 门禁报告（重构里程碑） |
| [BENCHMARK.md](BENCHMARK.md) | TPC-H 性能基准 |
| [COVERAGE_ANALYSIS_REPORT.md](COVERAGE_ANALYSIS_REPORT.md) | 覆盖率分析 |

## 三、门禁状态

| 门禁 | 状态 | 说明 |
|------|------|------|
| Alpha Gate | ✅ PASS | A1-A4 |
| Beta Gate | ✅ PASS | B1-B5（含 wal_tx_contract 15/22） |
| RC Gate | ⚠️ PASS (2 SKIP) | R4/R5 因网络问题 SKIP |
| GA Gate | ⚠️ **CONDITIONAL PASS** | 重构里程碑，非生产 GA |

### 覆盖率

| 指标 | 值 | 阈值 | 状态 |
|------|---|------|------|
| 平均覆盖率 | 84.99% | ≥85% | ⚠️ 差 0.01pp |
| L1 测试 | 547 tests | — | ✅ PASS |

## 四、已知缺陷（INT-1~INT-4）

| Issue | 缺陷 | 优先级 | 状态 |
|-------|------|--------|------|
| INT-1 | DML 不经过 WAL/TransactionManager | P0 | 持续修复中 |
| INT-2 | ParallelVolcanoExecutor 功能孤岛 | P0 | 持续修复中 |
| INT-3 | expr crate 孤岛 | P1 | 持续修复中 |
| INT-4 | mysql-server 双路径（Path A/B/C 未统一） | P1 | 持续修复中 |

> **说明**: v3.6.0 发现的 INT-1~INT-4 在 v3.7.0 中仍未完全解决。WAL/并行/CBO 等关键模块持续集成中。
> v3.8.0 为 Execution Architecture Consolidation，聚焦架构统一。

## 五、快速开始

```bash
git clone ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v3.7.0
cargo build --release
cargo test --all-features
```