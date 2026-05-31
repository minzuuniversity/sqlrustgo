# Architecture Governance — Dead Module Detection Report
**Issue**: #2601  
**Author**: Hermes C  
**Date**: 2026-05-31  
**Branch**: `origin/docs/v380-dead-module-analysis`  
**Status**: COMPLETED

---

## 1. 背景

Issue #2601 要求"定义模块状态与 Dead Module 检测"。在 v3.8.0 Architecture Consolidation 背景下，识别长期无更新的模块有助于：
1. 避免在废弃代码上投入重构精力
2. 识别可能的集成债务来源
3. 为 PR-900 ExecutionEngine 拆分提供依据

---

## 2. 检测方法

### 2.1 标准

**"Dead Module"** 定义：crate 自 2026-03-01 以来无任何 commit。

**"Stale Module"** 定义：crate 上次修改时间早于 v3.7.0 起点（2026-03-01 之后从未更新）。

### 2.2 数据来源

```bash
# 获取每个 crate 的最新 commit
git log --since="2026-03-01" --oneline -1 -- <crate_dir>
```

---

## 3. 检测结果

### 3.1 Dead Crate（2026-03-01 后无更新）

| Crate | 最后修改 | 距今 | workspace 成员 | 风险 |
|-------|----------|------|---------------|------|
| `information-schema` | pre-v3.6.0 | ~5个月 | 否 | 低（已废弃） |
| `optimizer` | pre-v3.6.0 | ~5个月 | **是** | 中（重要 crate，需验证） |
| `query-stats` | pre-v3.6.0 | ~5个月 | 否 | 低 |
| `sqlancer` | pre-v3.6.0 | ~5个月 | 否 | 低 |
| `telemetry` | pre-v3.6.0 | ~5个月 | **是** | 中 |
| `test-reporter` | pre-v3.6.0 | ~5个月 | 否 | 低 |
| `test-results` | pre-v3.6.0 | ~5个月 | 否 | 低 |
| `test-runner` | pre-v3.6.0 | ~5个月 | 否 | 低 |
| `transaction-stress` | pre-v3.6.0 | ~5个月 | 否 | 低 |
| `unified-storage` | pre-v3.6.0 | ~5个月 | **是** | 中（需验证用途） |

### 3.2 Active Crate（2026-03-01 后有更新）

| Crate | 最后 commit | 说明 |
|-------|------------|------|
| `storage` | `107e2450` | 活跃（WalStorage, L2/L3） |
| `executor` | `66a7392f` | 活跃（ExecutionEngine, WAL） |
| `planner` | `eda4396e` | 活跃（WAL contract） |
| `transaction` | `fc57cf1e` | 活跃（TX 管理） |
| `parser` | `fa729191` | 活跃（语法解析） |
| `mysql-server` | `fb0f09a0` | 活跃（Wire Protocol） |
| `server` | `2e120070` | 活跃（HTTP API） |
| `common` | `834c4dc5` | 活跃 |
| `types` | `f36529d9` | 活跃 |

### 3.3 意外发现：tools/sqlrustgo-gate 编译错误

`tools/sqlrustgo-gate` 依赖的 `evidence-graph` 版本（commit 9017162e）与当前接口不匹配：
- `EdgeType::Affects` 已被删除（当前只有 ImplementedBy/VerifiedBy/Produces/Validates/Requires/Causes）
- `Commit/Ci/Artifact/Task/Link/Status` 类型已从 `evidence-graph` 导出中移除

**此工具链编译失败不影响主构建**，但 evidence-graph gate 工作流可能受影响。

---

## 4. 结论与建议

### 4.1 Dead Module 汇总

| 类别 | Crate | 建议 |
|------|-------|------|
| 废弃 | `information-schema`, `query-stats`, `sqlancer`, `test-*`, `transaction-stress` | 从 Cargo.toml workspace members 移除 |
| 待定 | `optimizer`, `telemetry`, `unified-storage` | 需要确认是否仍在规划中 |
| 编译损坏 | `tools/sqlrustgo-gate` | 需要 PR 修复 evidence-graph 接口兼容性 |

### 4.2 不影响 v3.8.0 的原因

以上 dead/stale crate **均不在 PR-800~PR-900 执行链路中**：
- ExecutionEngine 位于 `src/execution_engine.rs`（executor crate 活跃）
- StorageEngine 位于 `crates/storage/`（storage crate 活跃）
- MySQL Server 位于 `crates/mysql-server/`（活跃）
- WAL 路径位于 `crates/storage/src/wal/`（活跃）

### 4.3 下一步

建议 PR-900 阶段处理：
1. 确认 `optimizer`、`telemetry`、`unified-storage` 是否仍在计划中
2. 从 workspace members 移除真正废弃的 crate
3. 修复 `tools/sqlrustgo-gate` 编译错误

---

## 附录：原始数据

```
$ git log --since="2026-03-01" --oneline -1 -- <crate>
7ca4b124 crates/information-schema/   (2025, pre-v3.6.0)
7ca4b124 crates/optimizer/            (2025, pre-v3.6.0)
7ca4b124 crates/query-stats/           (2025)
7ca4b124 crates/sqlancer/             (2025)
7ca4b124 crates/telemetry/             (2025)
7ca4b124 crates/test-reporter/        (2025)
7ca4b124 crates/test-results/         (2025)
7ca4b124 crates/test-runner/          (2025)
7ca4b124 crates/transaction-stress/   (2025)
7ca4b124 crates/unified-storage/      (2025)
107e2450 crates/storage/              (2026-05-31, recent)
66a7392f crates/executor/             (2026-05-31, recent)
```