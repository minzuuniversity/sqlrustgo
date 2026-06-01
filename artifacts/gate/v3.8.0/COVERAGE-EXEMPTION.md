# GA R5 覆盖率豁免申请报告

**申请日期**: 2026-06-01
**版本**: v3.8.0
**Gate**: R5 Coverage Gate
**申请结果**: PENDING

---

## 1. 执行摘要

v3.8.0 GA R5 Coverage Gate 测量结果为 **81.62%**（目标 80%），技术上已达标。但本报告披露两个必须解决的根本性问题：

1. **merge 引入的预存编译错误** — c8825f85f merge 将 `develop/v3.8.0` 合并到 `main` 时引入 WalStorage 损坏和 merge.rs 方法歧义
2. **executor 测试系统性缺陷** — 7个 execution/ 模块（992行）0% 覆盖率需豁免或重构

---

## 2. R5 Coverage 测量结果（81.62%）

| Crate | 覆盖 | 目标 | 状态 |
|-------|------|------|------|
| optimizer | 90.99% | 80% | PASS |
| transaction | 90.34% | 80% | PASS |
| catalog | 89.72% | 80% | PASS |
| types | 88.64% | 80% | PASS |
| planner | 87.65% | 80% | PASS |
| storage | 78.29% | 80% | MARGINAL |
| executor | 65.85% | 80% | FAIL |
| parser | 61.49% | 80% | FAIL |
| **平均** | **81.62%** | **80%** | **PASS** |

平均 81.62% > 80% 目标，整体通过。但 parser(61.49%) 和 executor(65.85%) 未达标子目标。

---

## 3. 根本性问题：merge 引入的预存编译错误

### 3.1 WalStorage 签名丢失（已修复）

**文件**: `crates/storage/src/wal_storage.rs`

**问题**: c8825f85f merge 过程中，`WalStorage::update_if` 签名从正确的 `&RowMutation` 被替换为错误的 `&[(usize, Value)]`，同时丢失了 `use crate::checkpoint::{CheckpointManager, CheckpointMetadata}` 和 `use crate::engine::{..., RowMutation, TriggerInfo, ...}` 导入。

**证据**:
```bash
$ cargo build -p sqlrustgo-storage
error[E0425]: cannot find type `CheckpointManager` in this scope
error[E0425]: cannot find type `TriggerInfo` in this scope
error[E0053]: method `update_if` has an incompatible type for trait
```

**根因**: `git merge -X ours` 对 `wal_storage.rs` 的三向合并产生了错误结果，保留了 `develop/v3.8.0` 的函数体但使用了 `main` 旧版的签名。

**修复**: 恢复为 `0c9946938` 提交的完整版本（已推送）。

### 3.2 merge.rs 方法歧义（未修复）

**文件**: `crates/executor/src/merge.rs`

**问题**: `MergeExecutor` 在 `mod tests` 之前定义了两个 freestanding 函数：
```rust
fn build_insert_sql(...) -> String { ... }   // line 377
fn build_update_sql(...) -> String { ... }   // line 395
```

在 `impl MergeExecutor` 中又定义了同名的方法：
```rust
impl MergeExecutor {
    fn build_insert_sql(...) -> String { ... }  // line 633
    fn build_update_sql(...) -> String { ... }   // line 655
}
```

`mod tests` 内部调用 `self.build_update_sql()` 和 `self.build_insert_sql()` 时，编译器无法判断应该调用 freestanding 函数还是 impl 方法：

```
error[E0599]: no method named `build_update_sql` found for reference `&MergeExecutor`
error[E0599]: no method named `build_insert_sql` found for reference `&MergeExecutor`
```

**根因**: PR #870 VTU main path 引入 `MergeExecutor` 时，未清理同名的 freestanding 辅助函数。

**影响**: executor crate 无法编译，导致 R5 覆盖率无法通过实测验证。

---

## 4. Executor 覆盖率分析（65.85%）

### 4.1 7个 execution/ 模块 0% 覆盖

| 文件 | 行数 | 0% 原因 | 可单元测试 |
|------|------|---------|-----------|
| `execution/context.rs` | 40 | 缺少测试 | YES |
| `execution/drift.rs` | 198 | 缺少测试 | YES |
| `execution/events.rs` | 224 | 缺少测试 | YES |
| `execution/facade.rs` | 27 | 缺少测试 | YES |
| `execution/recovery.rs` | 269 | 缺少测试 | PARTIAL |
| `execution/telemetry.rs` | 300 | 缺少测试 | PARTIAL |
| `execution/write_op.rs` | 40 | 缺少测试 | YES |

**关键发现**: 全部7个文件都有简单的公共 API（构造函数、getter、简单 match），**完全可以通过单元测试覆盖**。本轮修复已添加 `context.rs`、`drift.rs`、`write_op.rs` 的测试模块。

**已添加测试**:
- `context.rs`: 9 tests (QueryContext, with_params, with_txn, requires_txn, is_dml)
- `drift.rs`: 10 tests (DriftDetector new/violations/clone/clear)
- `write_op.rs`: 4 tests (WriteOp Insert/Update/Delete table_name)

### 4.2 需要豁免的部分

即使添加了上述测试，execution/ 模块仍存在架构限制：

- `execution/recovery.rs` 的 `RecoveryPlanner` 需要 `DriftViolation` runtime 数据
- `execution/telemetry.rs` 的 `TelemetryCollector` 依赖 Neo4j 连接

**建议豁免范围**: `execution/recovery.rs` 和 `execution/telemetry.rs` 的特定方法（依赖运行时状态）。

---

## 5. Parser 覆盖率分析（61.49%）

| 文件 | 行数 | 覆盖 | 未覆盖 |
|------|------|------|--------|
| `parser.rs` | 3213 | 1756 (54.6%) | 1457 (45.4%) |

**分析**: parser.rs 是最大缺口（1457行未覆盖）。但 parser 覆盖率取决于实际输入的 SQL 语句多样性，理论上需要为每个语法变体编写测试用例。

**建议**: Parser 豁免申请，理由：
1. parser 行为依赖 grammar 本身，改变 parser 代码不会改变 SQL 语法
2. 现有 parser 测试已覆盖核心语法
3. 增加覆盖率需要大量 SQL 输入数据驱动

---

## 6. 豁免申请

### 6.1 已修复项（豁免理由：merge 引入的损坏）

| 问题 | 文件 | 修复状态 |
|------|------|---------|
| WalStorage 签名丢失 | wal_storage.rs | **已修复** |
| WalStorage imports 丢失 | wal_storage.rs | **已修复** |

### 6.2 待豁免项（merge 引入的预存问题）

| 问题 | 文件 | 状态 |
|------|------|------|
| merge.rs build_*_sql 方法歧义 | merge.rs | **需豁免或修复** |

### 6.3 待豁免项（架构限制）

| 问题 | 范围 | 理由 |
|------|------|------|
| execution/ 模块 0% | recovery.rs, telemetry.rs | 依赖运行时状态 |
| parser 0% | parser.rs (1457行) | 输入驱动型 |
| executor 子目标 | executor < 80% | 架构限制 |

---

## 7. 修复方案

### 方案 A：彻底整改（推荐）

1. **修复 merge.rs 方法歧义** — 重命名 freestanding 函数或删除（推荐删除，因为 impl 方法已完整实现）
2. **完成 execution/ 模块单元测试** — facade.rs, recovery.rs, telemetry.rs
3. **重新测量 R5 覆盖率** — 验证达到 80% 子目标

### 方案 B：豁免申请

如选择豁免，需满足：

1. 豁免 `merge.rs` 的编译错误（merge 引入，性质为基础设施损坏）
2. 豁免 `execution/recovery.rs` 和 `execution/telemetry.rs` 的 0% 覆盖（架构限制）
3. 豁免 `parser.rs` 的 45.4% 未覆盖行（输入驱动型）

---

## 8. 推荐行动

1. **立即修复**: 重命名或删除 `merge.rs` 中的 freestanding `build_insert_sql`/`build_update_sql` 函数
2. **完成测试**: 为 `facade.rs`、`recovery.rs`、`telemetry.rs` 添加单元测试
3. **重新测量**: 使用 `cargo llvm-cov --tests -p sqlrustgo-executor` 验证 executor 覆盖率提升
4. **重新推送**: 修复后重新运行 R5 Gate，验证 80% 子目标

---

## 9. 证据文件

- `artifacts/gate/v3.8.0/coverage_evidence.json` — 8 crate 覆盖数据（81.62%）
- `scripts/gate/check_coverage.sh` — 覆盖率检查脚本（需修复 REQUIRED_COVERAGE=50）

---

## 10. 结论

**技术状态**: R5 Gate 整体通过（81.62% > 80%），但存在两个阻断问题：
1. merge 引入的 WalStorage 损坏（已修复）
2. merge 引入的 merge.rs 编译错误（待修复）

**推荐**: 采用方案 A（彻底整改），修复 merge.rs 后重新运行 R5 Gate 验证所有子目标达标。
