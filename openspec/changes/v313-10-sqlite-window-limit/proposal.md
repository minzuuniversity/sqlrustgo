# V313-10: 支持 LIMIT 子句中使用窗口函数

## Why

SQLite sqllogictest 语料 `order__test_limit.test` 第 49 行包含以下测试用例：

```sql
SELECT a FROM test LIMIT row_number() OVER ()
```

当前返回错误：

```
Parse error: Invalid LIMIT: invalid digit found in string
```

根因分析：

1. **LIMIT 子句解析器**（`crates/parser/src/parser.rs:4718-4741`）仅支持 `Token::NumberLiteral` 和 `Token::Identifier` 两种字面量
2. `row_number() OVER ()` 是一个 `Function` 表达式节点，无法被解析为 `u64`
3. 当前代码没有调用 `parse_expression()` 来处理复杂 LIMIT 表达式

## What Changes

- **`crates/parser/src/parser.rs`**：扩展 LIMIT 子句解析逻辑，支持通用表达式
- **`crates/planner/src/`**：扩展查询规划器，支持将表达式类型的 LIMIT 值下推为 LimitExec 参数
- **`tests/compat/sqlite_v3_13/window_limit.sql`** + `.out`（新增）：验证 `LIMIT` 中使用 `ROW_NUMBER()`、`RANK()` 等窗口函数
- **`tests/compat/sqlite_v3_12/order__test_limit.test`**：将第 49 行 `# expect` 从 `statement error` 更新为 `query I` 并补充期望输出
- **`docs/releases/v3.13.0/evidence/sqlite_compat/`**：更新 SQLite 兼容性矩阵

## Capabilities

### 新增能力

- `sqlite-limit-window-function`：支持 `SELECT ... LIMIT <window_function_expr>` 语法
- `sqlite-limit-rank`：支持 `LIMIT RANK() OVER (...)` 场景
- `sqlite-limit-row-number`：支持 `LIMIT ROW_NUMBER() OVER (...)` 场景

### 修改能力

- `sqlite-order-limit-test`：将 `order__test_limit.test` 从 `excluded`（category: parser）升级为 `pass`

## Impact

- **修改文件**：
  - `crates/parser/src/parser.rs`（LIMIT 表达式解析）
  - `crates/planner/src/`（LimitExec 表达式参数支持）
  - `tests/compat/sqlite_v3_12/order__test_limit.test`（解除 exclusion）
- **新增文件**：
  - `tests/compat/sqlite_v3_13/window_limit.sql` + `.out`
- **风险**：LIMIT 表达式求值时机须与执行器协调，确保窗口函数在 LIMIT 之前完成计算
- **无新增外部 crate 依赖**
