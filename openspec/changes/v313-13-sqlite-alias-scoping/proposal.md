# V313-13: 修复 SQLite 方言下 SELECT 别名在 WHERE 子句中的错误引用

## Why

V313-12 SQLite 方言覆盖测试揭示了以下问题：

- **表面错误**：`binder__alias_error_10057.test` 期望查询失败，但实际执行成功
- **根因**：Binder 层在处理 SELECT 别名时，未正确限定该别名在 WHERE 子句中的可见性
- 错误查询：`select test_data.foobar as new_column from test_data where new_column is not null`
- WHERE 子句引用了 SELECT 列表中的列别名 `new_column`，这在 SQL-92 语义下是非法的（WHERE 在 SELECT 之前求值，此时别名尚未定义）
- Binder 应在语义分析阶段捕获此错误并返回相应错误码

本变更修复 Binder 中的别名作用域逻辑，确保 SQLite 方言下正确拒绝 WHERE 子句中对 SELECT 别名的引用。

## What Changes

- **`crates/sqlrustgo-binder/src/`**：定位并修复别名在 WHERE 子句中的作用域判定逻辑
- **`tests/compat/sqlite_v3_13/`**：新增 SQLite compat fixture 验证别名作用域错误被正确捕获
- **`crates/sqlrustgo_sqllogictest/testdata/duckdb_full/binder__alias_error_10057.test`**：解除 exclusion，恢复 `statement error` 预期行为
- **`docs/releases/v3.13.0/evidence/sqlite_compat/`**：更新 `SURFACE_DISPOSITION.md` 中相关行

## Capabilities

### 新增能力

- `sqlite-alias-where-error`：SQLite 方言下，WHERE 子句引用 SELECT 别名时正确报错

### 修改能力

- `deferred-binder-alias-error-10057`：将 `binder__alias_error_10057.test` 从 excluded 状态升级为 PASS

## Impact

- **修改文件**：`crates/sqlrustgo-binder/src/` 中别名解析和作用域相关代码
- **新增文件**：
  - `tests/compat/sqlite_v3_13/alias_where_error.sql` + `.out`
- **修改文件**：`crates/sqlrustgo_sqllogictest/testdata/duckdb_full/binder__alias_error_10057.test`（移除 exclusion）
- **风险**：Binder 别名逻辑修改可能影响其他语义分析路径；需在全面回归测试后合并
- **无新增外部 crate 依赖**
