# V313-12: 修复 SQLite 兼容层 NOT NULL 约束强制执行

## Why

V313-12 的 `constraints__test_not_null.test` 和 `test_constraint_with_updates.test` fixture 揭示了 SQLite 兼容层的一个关键缺陷：

- **表面错误**：`INSERT INTO t VALUES (NULL)` 和 `UPDATE t SET col=NULL` 在 NOT NULL 列上执行成功，但按 SQL 标准应返回错误
- **根因**：SQLite 兼容层的 INSERT/UPDATE 执行器在写入数据前未检查 NOT NULL 约束
- `ColumnDefinition::nullable` 字段已正确解析（来自 `CREATE TABLE` 的 `NOT NULL` 修饰符），但执行层未使用此信息进行约束验证

本变更在 INSERT/UPDATE 执行路径中添加 NOT NULL 约束检查，确保违反约束的操作被拒绝并返回明确错误。

## What Changes

- **`crates/executor/src/`**：在 `InsertExecutor` 和 `UpdateExecutor`（或等价执行器）中添加 NOT NULL 约束验证
- **`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/test_constraint_with_updates.test`**：更新预期结果，验证 NOT NULL 检查在 UPDATE 场景正常工作
- **`crates/sqlrustgo_sqllogictest/testdata/constraints__test_not_null.test`**：确认现有 fixture PASS
- **`docs/releases/v3.13.0/evidence/`**：更新 SQLite 兼容层相关证据（若存在）

## Capabilities

### 新增能力

- `sqlite-notnull-insert-enforcement`：INSERT 语句正确强制执行 NOT NULL 约束，违反时返回 `SqlError::ConstraintViolation`
- `sqlite-notnull-update-enforcement`：UPDATE 语句正确强制执行 NOT NULL 约束，违反时返回 `SqlError::ConstraintViolation`

### 修改能力

- `sqlite-constraint-fixture`：将 `constraints__test_not_null.test` 和 `test_constraint_with_updates.test` 的状态从 `DEFERRED` 或 `FAIL` 升级为 `PASS`

## Impact

- **修改文件**：`crates/executor/src/` 中 INSERT/UPDATE 执行器代码
- **风险**：约束检查可能引入轻微性能开销（每行一次 NULL 检查），但对典型工作负载可忽略
- **无新增外部 crate 依赖**
- **向后兼容**：现有行为仅在违反 NOT NULL 时改变（从静默接受改为报错），符合 SQL 标准
