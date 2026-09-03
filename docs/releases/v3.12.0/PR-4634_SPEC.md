# PR-4634 SPEC — issue batch #4610..#4623 fixes

> **PR**: PR-4634 `fix(v312-62 / #4610..#4623): issue batch — executor / parser / CLI bug fixes`
> **Author**: Claude (claude-z6g4)
> **Created**: 2026-09-02
> **Status**: MERGED (commit `a8aaf8e30`)
> **Target**: `develop/v3.12.0`

---

## 1. 范围（Scope）

本 PR 一次性修复 9 个独立 ISSUE（#4610, #4611, #4612, #4613, #4618, #4619, #4620, #4622, #4623），
覆盖 executor、parser、CLI 三个子系统。每个 ISSUE 是独立的 bug，但都属于 v3.12.0 收尾阶段的回归修复。

**推迟 ISSUE**（不在本 PR 范围）：
- #4617 IndexScan 选择规则 — 需 optimizer 整体改造
- #4621 COUNT(\*) 覆盖索引 — 需 AggregateFunction AST 扩展

---

## 2. 修复接口与改动面

### 2.1 Executor — `crates/executor/src/expr/mod.rs`

| Issue | 函数 | 接口变化 |
|---|---|---|
| #4610 | `parse_lit(s: &str) -> Value` | Float literal 不再被截断为 Integer |
| #4611 | `eval_fn("LENGTH" / "LEN", args)` | 返回 codepoint 计数，非字节数 |
| #4612 | `compare_values(Value::Text, Value::Text) -> i32` | 默认 BINARY collation（移除 RTRIM） |
| #4613 | `eval_fn("ROUND", args)` | `d > 0` 时返回 Float |
| #4623 | `group_concat(args: &[Value]) -> Value` | 剥离 parser sentinel literals |

### 2.2 Parser — `crates/parser/src/parser.rs`

| Issue | 函数 | 接口变化 |
|---|---|---|
| #4618 | `parse_savepoint_statement()` | `ROLLBACK TO <name>` 简写形式接受 |
| #4620 | `parse_alter_table()` | `MODIFY` / `ADD CONSTRAINT` 在 parse 期报错 |

### 2.3 Executor CTE — `crates/executor/src/stored_proc.rs`

| Issue | 类型 / 函数 | 接口变化 |
|---|---|---|
| #4622 | `ProcedureContext` | 新增 `cte_columns: HashMap<String, Vec<ColumnDefinition>>` |
| #4622 | `WithSelect` 分支 + `execute_cte_subquery` | 嵌套 CTE 可解析 schema |

### 2.4 CLI batch — `crates/sqlrustgo-cli/src/sqlite_mode.rs`

| Issue | 类型 / 函数 | 接口变化 |
|---|---|---|
| #4619 | `SqliteMode` | 新增 `tx_depth: u32` |
| #4619 | `dispatch_one()` | 事务内出错自动 ROLLBACK |

---

## 3. 不在范围（Out of Scope）

1. **#4617** `IndexScan` 优化器规则——`crates/optimizer/src/index_selector.rs` 已有 analyzer
   但未接入 planner 的 `optimize()` pass。需要：
   - 在 `optimize()` 中调用 `index_selector::analyze_predicate_for_index`
   - 在物理计划中发出 `IndexScan` 节点
   - 实现 `IndexScanExecutor`
2. **#4621** `COUNT(*)` 覆盖索引——需要：
   - 在 `AggregateFunction` enum 添加 `GroupConcat` 变体
   - parser 中将 `GROUP_CONCAT` 分类为 aggregate（而非 FunctionCall）
   - `compute_aggregates` 中注册聚合路径
3. **#4612 风险项**：原 RTRIM 行为由 PR #4492 为 TPC-H Q14 兼容性添加。
   本次移除后需回归 TPC-H Q14 测试套件。
4. **#4622 限制**：`cte_columns` 只为显式命名列的 CTE 存储 schema，
   `SELECT *` 投影的子查询仍依赖 `lookup_table_columns` 推断。

---

## 4. 依赖与约束

- **构建依赖**：所有修改使用现有 crate 依赖，无新增 crate
- **API 兼容**：parser 错误信息新增，executor 行为变更——属于 bug fix 级别，非 breaking change
- **存储影响**：#4619 仅影响 CLI batch 模式的执行流程；存储层接口不变

---

## 5. 验收口径（Acceptance Criteria）

详见 `PR-4634_ACCEPTANCE.md`。
