# V313-06: MEDIAN 聚合函数实现

## Why

V312-21 的 `median_unsupported.sql` fixture 记录了 MEDIAN() 的当前行为：解析器接受 MEDIAN 关键字，但执行器返回 NULL 而非正确的数值结果或明确的 UNSUPPORTED 错误。

MySQL 兼容层面要求：高级聚合函数（STDDEV_POP、VAR_POP、MEDIAN、GROUP_CONCAT）在尚未实现时必须返回明确的 UNSUPPORTED 错误，而非静默返回 NULL。当前实现不满足此契约：

- `SELECT MEDIAN(col) FROM t` → 返回 `NULL`（隐式降级）
- 期望行为 A：返回正确的中位数（若实现）
- 期望行为 B：返回 `UNSUPPORTED: aggregate not implemented`（若未实现）

本变更为 v3.13.0 的 MySQL 兼容能力补全，将 MEDIAN 从"隐式 NULL"提升为"正确实现"。

## What Changes

- **`crates/planner/src/lib.rs`**：在 `AggregateFunction` 枚举中添加 `Median` 变体
- **`crates/executor/src/expr/mod.rs`**：在 `eval_fn` 分发逻辑中添加 `MEDIAN` 分支，实现中位数计算
- **`crates/executor/src/parallel_group_by.rs`** 或等价文件：若聚合函数需要并行化感知，在 `AggregateCall::func` 匹配中添加 `Median` 分支
- **`tests/compat/mysql_v3_13/median_basic.sql`** + `.out`：基础 MEDIAN 测试（奇数元素、偶数元素、NULL 处理）
- **`tests/compat/mysql_v3_13/median_grouped.sql`** + `.out`：分组 MEDIAN 测试
- **`tests/compat/mysql_v3_12/median_unsupported.sql`**：更新为 `PASS` 状态

## Capabilities

### 新增能力

- `median-aggregate`：MEDIAN(col) 返回列的中位数值（奇数元素取中间值，偶数元素取中间两值平均）
- `median-grouped`：带 GROUP BY 的 MEDIAN 聚合

### 修改能力

- `mysql-compat-median-disposition`：从 DEFERRED 升级为 PASS，fixture 解除 deferred 状态

## Impact

- **修改文件**：`crates/planner/src/lib.rs`（AggregateFunction 枚举）、`crates/executor/src/expr/mod.rs`（eval_fn 分发）
- **新增文件**：`tests/compat/mysql_v3_13/median_basic.sql` + `.out`、`tests/compat/mysql_v3_13/median_grouped.sql` + `.out`
- **风险**：MEDIAN 需要全量数据排序，时间复杂度 O(n log n)；大数据集场景需评估性能影响
- **无新增外部 crate 依赖**
