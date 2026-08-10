# V313-14: SQLite CREATE TABLE AS 行为修复

## Why

`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/create_as.test` 中的测试用例揭示了当前 SQLRustGo 对 `CREATE TABLE AS SELECT`（CTAS）的实现与 SQLite/sqllogictest 期望行为存在偏差。测试报错 `query result mismatch`，表明查询结果的列名或列数与预期不符。

SQLite 的 CTAS 语义如下：
- 列名取自 SELECT 列表项的别名（`SELECT 1 AS f` → 列名 `f`）；无别名时取原生表达式文本（`SELECT 1` → 列名 `1`）
- 列类型取自值的 affinity（`SELECT 1` → INTEGER affinity，`SELECT 'hello'` → TEXT affinity）
- `CREATE TABLE t(col1, col2) AS SELECT ...` 中指定的列名覆盖 SELECT 的列名；若 SELECT 结果列数少于指定列数，剩余列填充 NULL

当前 SQLRustGo 可能存在以下问题：
1. 未正确解析 `AS` 关键字后的 SELECT 语句，导致 CTAS 被错误地当作普通建表处理
2. 列名推断逻辑缺失，无别名时使用默认列名（如 `column_0`）而非 SQLite 兼容的原生表达式文本
3. 列类型 affinity 推断不正确
4. `WITH NO DATA` / `WITH DATA` 子句未实现

本变更将修复 CTAS 的解析和执行，使其符合 SQLite/sqllogictest 的行为。

## What Changes

- **`crates/parser/src/sql.y`** 或等价语法文件：扩展 `CREATE TABLE` 语法，支持 `CREATE TABLE name[(col1, col2, ...)] AS SELECT ... [WITH (NO)? DATA]`
- **`crates/parser/src/ast.rs`**：扩展 `CreateTableStatement`，增加 `as_query: Option<SelectStatement>`、`with_data: bool` 等字段
- **`crates/executor/src/`**（DDL 执行路径）：实现 CTAS 执行逻辑：
  - 从 SELECT 结果推断列名和类型
  - 创建表 schema
  - 将 SELECT 结果写入新表
- **`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/create_as.test`**：验证 CTAS 行为与 SQLite/sqllogictest 兼容
- **`tests/compat/sqlite_v3_13/create_table_as.sql`** + `.out`：新增 SQLite 兼容 fixture

## Capabilities

### 新增能力

- `sqlite-compat-create-table-as`：支持 `CREATE TABLE AS SELECT` 完整语法
- `sqlite-compat-ctas-column-names`：正确推断列名（别名优先，否则使用表达式文本）
- `sqlite-compat-ctas-with-data`：支持 `WITH NO DATA` / `WITH DATA` 子句

### 修改能力

- `sqlite-compat-ctas`：修复现有 CTAS 实现，使其与 SQLite 行为对齐

## Impact

- **修改文件**：`crates/parser/src/`（语法定义 + AST）、`crates/executor/src/`（DDL 执行）
- **新增文件**：`tests/compat/sqlite_v3_13/create_table_as.sql` + `.out`
- **风险**：CTAS 涉及 SELECT 执行和 DDL 写入的联动，需确保事务边界正确；`WITH NO DATA` 需要在 schema 创建后不执行数据插入
- **无新增外部 crate 依赖**
