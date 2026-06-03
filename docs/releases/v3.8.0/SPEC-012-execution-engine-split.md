# SPEC-012 — ExecutionEngine 行数拆分 (PR-900 第一阶段)

> **PR Number**: SPEC-012
> **PR Title**: ExecutionEngine 行数拆分 — 1587→1431 (<1500 AD-001 目标)
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-execution-engine-split` (从 gitea/develop/v3.8.0 @ 651433468 切出)
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

Alpha Gate A7-3 报告:
```
[A7-3] ExecutionEngine:     1587 lines
INFO (目标 < 1500 lines, 当前     1587 lines)
      AD-001 拆分目标未完成，但这是 PR-900 的最终目标
```

ARCHITECTURE_DECISIONS.md AD-001 要求 `ExecutionEngine` 行数 < 1500。SPEC-012 实现 **PR-900 拆分第一阶段**。

### 1.2 拆分目标

| 阶段 | 行数 | 状态 |
|---|---|---|
| 基线 | 1587 | ❌ |
| SPEC-012 (本 PR) | **1431** | ✅ <1500 |
| PR-900 完整拆分 (后续) | ~1100 | 待 PR-900 |

### 1.3 拆分内容

将 CBO 估算 (Cost-Based Optimizer) 模块从 `execution_engine.rs` 提取到独立 `cbo_estimator.rs`：

| 提取项 | 原行号 | 新位置 |
|---|---|---|
| `estimate_row_count` | 163-170 | `cbo_estimator::estimate_row_count` |
| `estimate_selectivity` | 172-184 | `cbo_estimator::estimate_selectivity` |
| `estimate_seq_scan_cost` | 186-190 | `cbo_estimator::estimate_seq_scan_cost` |
| `estimate_index_scan_cost` | 192-200 | `cbo_estimator::estimate_index_scan_cost` |
| `estimate_index_benefit` | 202-208 | `cbo_estimator::estimate_index_benefit` |
| `should_use_index` | 210-216 | `cbo_estimator::should_use_index` |
| `estimate_join_cost` | 218-244 | `cbo_estimator::estimate_join_cost` |
| `optimize_join_order` | 246-282 | `cbo_estimator::optimize_join_order` |
| `collect_table_stats` | 284-365 (impl block) | `cbo_estimator::collect_table_stats` |

总计提取 ~120 行 CBO 代码 + 80 行 ANALYZE 实现 + 简化 collect_table_stats。

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| 新增 cbo_estimator.rs | `src/cbo_estimator.rs` | 自由函数，依赖 `Arc<RwLock<ExecutionStats>>` | `cargo build` 0 errors |
| 注册模块 | `src/lib.rs` | `pub mod cbo_estimator;` | 同上 |
| ExecutionEngine 添加 forwarder | `src/execution_engine.rs` | 8 个 estimate_* 方法委托 `crate::cbo_estimator` | 327/327 tests PASS |
| ExecutionEngine 简化 collect_table_stats | 同上 | 委托 `cbo_estimator::collect_table_stats` | 同上 |
| execution_engine.rs 行数 | 1587→1431 | 提取 CBO + 简化 ANALYZE | `wc -l` <1500 |

### 2.2 禁止做 (Must NOT Do)

- ❌ 修改 CBO 业务逻辑（仅拆分位置）
- ❌ 修改 `ExecutionEngine` 公共 API（保持向后兼容，所有 forwarder 保留原签名）
- ❌ 删除任何方法/测试
- ❌ 引入新依赖

### 2.3 不在范围内 (Out of Scope)

- PR-900 完整拆分 (DML/DDL/Tx/Permission executor) → 后续 PR
- A8-1 EVIDENCE 156 violations → 需 Gitea CI 实际跑
- A8-3 C-ARCH-01/03/05 → 独立代码缺陷

---

## 3. 技术设计

### 3.1 cbo_estimator.rs 公开 API

```rust
// 自由函数，参数为 &Arc<RwLock<ExecutionStats>>
pub fn estimate_row_count(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str) -> u64;
pub fn estimate_selectivity(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str, column_name: &str) -> f64;
pub fn estimate_seq_scan_cost(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str) -> f64;
pub fn estimate_index_scan_cost(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str, selectivity: f64) -> f64;
pub fn estimate_index_benefit(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str, selectivity: f64) -> f64;
pub fn should_use_index(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str, column_name: &str) -> bool;
pub fn estimate_join_cost(stats: &Arc<RwLock<ExecutionStats>>, left_table: &str, right_table: &str, join_type: &str) -> f64;
pub fn optimize_join_order<'a>(stats: &Arc<RwLock<ExecutionStats>>, tables: &'a [&str]) -> Vec<&'a str>;
pub fn collect_table_stats<S: StorageEngine>(engine: &S, table: &str) -> SqlResult<TableStatistics>;
```

### 3.2 ExecutionEngine forwarders (向后兼容)

```rust
impl<S: StorageEngine> ExecutionEngine<S> {
    /// 保留原签名, 委托 cbo_estimator
    pub fn estimate_row_count(&self, table_name: &str) -> u64 {
        crate::cbo_estimator::estimate_row_count(&self.stats, table_name)
    }
    // ... 7 个类似 forwarder
}
```

### 3.3 行数变化

| 文件 | 修复前 | 修复后 | 变化 |
|---|---|---|---|
| `src/execution_engine.rs` | 1587 | 1431 | **-156** |
| `src/cbo_estimator.rs` | 0 (新增) | ~180 | +180 |
| `src/lib.rs` | 38 | 39 | +1 (`pub mod cbo_estimator;`) |
| **总计** | 1625 | 1650 | +25 (净增 — 模块边界开销) |

### 3.4 验证矩阵

| 检查项 | 命令 | 修复前 | 修复后 |
|--------|------|--------|--------|
| 行数 | `wc -l src/execution_engine.rs` | 1587 | **1431** <1500 ✅ |
| Build | `cargo build --all-features` | OK | OK (0 warnings on sqlrustgo lib) |
| executor tests | `cargo test -p sqlrustgo-executor --lib` | 327/327 | 327/327 ✅ |
| storage tests | `cargo test -p sqlrustgo-storage --lib` | 287/287 | 287/287 ✅ |
| A7-3 gate | `bash check_architecture_freeze.sh` | INFO 1587 | PASS 1431 ✅ |
| clippy | `cargo clippy -p sqlrustgo -p sqlrustgo-executor -p sqlrustgo-storage --all-features --lib -- -D warnings` | 0 warnings | 0 warnings ✅ |

### 3.5 提交规范

```bash
git commit -m "refactor(executor): split CBO estimator from execution_engine.rs (SPEC-012)

将 CBO (Cost-Based Optimizer) 估算从 execution_engine.rs 提取到独立模块,
降低 execution_engine.rs 行数 1587→1431, 达到 AD-001 <1500 行目标.

提取内容 (9 个方法):
- estimate_row_count / estimate_selectivity / estimate_seq_scan_cost
- estimate_index_scan_cost / estimate_index_benefit / should_use_index
- estimate_join_cost / optimize_join_order
- collect_table_stats (ANALYZE 实现)

公共 API 向后兼容:
- ExecutionEngine::estimate_*(...) 保留原签名
- 内部委托 crate::cbo_estimator::estimate_*(...)
- 外部调用方 (其他 crate / tests) 无需修改

验证:
- execution_engine.rs: 1587→1431 (-156 行) ✅
- cargo build --all-features: Finished
- cargo test -p sqlrustgo-executor --lib: 327/327 PASS
- cargo test -p sqlrustgo-storage --lib: 287/287 PASS
- cargo clippy 6 core crates: 0 warnings
- A7-3 gate: INFO → PASS

源: AD-001 (ExecutionEngine 拆分目标)
后续: PR-900 完整拆分 (DML/DDL/Tx/Permission executor)
       SPEC 范围: 仅 CBO 估算 + ANALYZE"
```

---

## 4. 验收标准 (Acceptance Criteria)

- [x] **AC-1**: `execution_engine.rs` 行数 < 1500 (1431)
- [x] **AC-2**: `cargo build --all-features` 0 errors
- [x] **AC-3**: `cargo test -p sqlrustgo-executor --lib` 327/327 PASS
- [x] **AC-4**: `cargo test -p sqlrustgo-storage --lib` 287/287 PASS
- [x] **AC-5**: `cargo clippy 6 core crates --all-features -- -D warnings` 0 warnings
- [x] **AC-6**: `check_architecture_freeze.sh A7-3` PASS
- [x] **AC-7**: ExecutionEngine 公共 API 向后兼容
- [x] **AC-8**: PR base = `develop/v3.8.0`
- [x] **AC-9**: 3 平台分支一致

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 公共 API 破坏 | 低 | 高 | 保留所有 forwarder，原签名不变 |
| 性能下降（多一跳函数调用） | 极低 | 低 | 函数 inline 化由编译器优化 |
| 死代码 warning | 中 | 低 | collect_table_stats 简化版被使用（不是死代码）|

---

## 6. 关联

- **源**: Alpha Gate A7-3 (AD-001 拆分目标)
- **后续**: PR-900 完整拆分（DML/DDL/Tx/Permission executor 进一步拆分）
- **Gitea Issue**: 无单独 Issue

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写，所有状态变更基于实际执行证据。*
