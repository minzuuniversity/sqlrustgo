# V313-09: 实现 EXCEPT ALL / INTERSECT ALL 多重集合语义

## Why

`setops__test_except.test` 和 `setops__test_setops.test` 揭示了以下问题：

- **表面错误**：`INTERSECT ALL` / `EXCEPT ALL` 返回错误结果
- **根因**：`crates/executor/src/stored_proc.rs` 中 `execute_cte_subquery` 对 `INTERSECT ALL` 和 `EXCEPT ALL` 的实现为占位符（placeholder），未实现正确的多重集合语义
  - `INTERSECT ALL`：当前实现为 `left.chain(right).sort().dedup()`，仅返回不同值，丢失了多重性信息
  - `EXCEPT ALL`：当前实现为简单移除第一匹配项，未按出现次数正确抵消

正确行为（SQL-92 规范）：
- `INTERSECT ALL`：多重集合交集，每行结果保留 min(count_in_left, count_in_right) 次
- `EXCEPT ALL`：多重集合差集，每行结果保留 max(0, count_in_left - count_in_right) 次

本变更实现正确的多重集合语义，使 SQLite sqllogictest corpus 中的 EXCEPT ALL / INTERSECT ALL 测试通过。

## What Changes

- **`crates/executor/src/stored_proc.rs`**：实现 `INTERSECT ALL` / `EXCEPT ALL` 的多重集合算法
- **`crates/executor/src/`**：新增 `multiset.rs` 工具模块（多重集合交/差运算）
- **`tests/sqlrustgo_executor/`**：新增单元测试验证多重集合语义边界条件

## Capabilities

### 新增能力

- `setops-except-all`：EXCEPT ALL 多重集合差集语义
- `setops-intersect-all`：INTERSECT ALL 多重集合交集语义

### 修改能力

- `setops-except-all-deferred`：将 V3.12 的 deferred 状态升级为 PASS
- `setops-intersect-all-deferred`：将 V3.12 的 deferred 状态升级为 PASS

## Impact

- **修改文件**：`crates/executor/src/stored_proc.rs`、`crates/executor/src/multiset.rs`（新增）
- **风险**：多重集合算法需处理嵌套子查询、NULL 值、类型混合等边界情况；需在全面测试后合并
- **无新增外部 crate 依赖**
