# V313-11: ALTER TABLE SET PARTITIONED BY 语法支持

## Why

DuckDB 的 `ALTER TABLE SET PARTITIONED BY` 是表重分区操作的核心语法，允许用户在不重建表的情况下更改表的分区策略。当前 SQLRustGo 解析器将 `ALTER TABLE t SET PARTITIONED BY (i)` 报错为 `Parse error: Expected ADD, DROP, MODIFY or RENAME`，导致以下问题：

1. 包含 DuckDB 分区语法的 SQL 脚本无法在 SQLRustGo 中执行
2. DuckDB 兼容性存在明显缺口
3. 用户无法通过 `SET PARTITIONED BY` 或 `RESET PARTITIONED BY` 修改表的分区方式

本变更将在解析器和执行层实现完整的 `SET PARTITIONED BY` / `RESET PARTITIONED BY` 支持。

## What Changes

- **`crates/parser/src/parser.rs`**：扩展 `AlterTableOperation` 枚举，增加 `SetPartitionedBy` 和 `ResetPartitionedBy` 变体；在解析逻辑中添加 `SET` / `RESET` 关键字分支
- **`crates/executor/src/ddl.rs`**：为新增操作变体实现执行逻辑（分区策略更新 / 重置）
- **`crates/sqlrustgo_sqllogictest/testdata/duckdb_full/alter__alter_table_set_partitioned_by.test`**：将 `statement error` 更新为 `statement ok`，验证语法解析成功
- **`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/alter_table_set_partitioned_by.test`**：同步更新
- **`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/case_insensitive_alter.test`**：验证大小写不敏感场景

## Capabilities

### 新增能力

- `alter-table-set-partitioned-by`：解析并执行 `ALTER TABLE t SET PARTITIONED BY (column_list)`
- `alter-table-reset-partitioned-by`：解析并执行 `ALTER TABLE t RESET PARTITIONED BY`
- `duckdb-compat-partition`：与 DuckDB 行为对齐的分区表语法

### 修改能力

- `alter-table-error-mode`：现有报错信息从解析错误升级为执行层不支持错误（而非解析失败）

## Impact

- **修改文件**：`crates/parser/src/parser.rs`（枚举 + 解析逻辑）、`crates/executor/src/ddl.rs`（执行路径）
- **修改测试文件**：`alter__alter_table_set_partitioned_by.test`、`alter_table_set_partitioned_by.test`、`case_insensitive_alter.test`
- **风险**：分区策略变更可能影响查询优化器路径；本变更优先覆盖语法解析和基础执行，存储层分区重排为后续迭代预留
- **无新增外部 crate 依赖**
