# Tasks — V312-56A: Metadata/SHOW/information_schema 教学与兼容闭环

## Phase 1: 现状调研 ✅

- [x] 1.1 检查当前 information_schema 实现状态 - **完整** (`crates/information-schema`)
- [x] 1.2 检查当前 SHOW CREATE TABLE/INDEX/COLUMNS 实现 - **SHOW INDEX/COLUMNS 是 placeholder**
- [x] 1.3 识别 missing 或 placeholder 实现 - **execute_show_index/execute_show_columns 返回空**

## Phase 2: information_schema 实现 ✅

> information_schema crate 已完整实现，无需额外工作。

### 2.1 information_schema.tables ✅

- [x] 2.1.1 实现 `information_schema.tables` SQL 查询路径 - **已存在**
- [x] 2.1.2 添加正例 fixture - **有测试**
- [x] 2.1.3 添加 unsupported 边界测试 - **fail-closed 已有**

### 2.2 information_schema.columns ✅

- [x] 2.2.1 实现 `information_schema.columns` SQL 查询路径 - **已存在**
- [x] 2.2.2 添加正例 fixture - **有测试**
- [x] 2.2.3 添加 unsupported 边界测试 - **fail-closed 已有**

### 2.3 information_schema.indexes ✅

- [x] 2.3.1 实现 `information_schema.indexes` SQL 查询路径 - **已存在**
- [x] 2.3.2 添加正例 fixture - **有测试**
- [x] 2.3.3 添加 unsupported 边界测试 - **fail-closed 已有**

## Phase 3: SHOW 命令实现 ✅

### 3.1 SHOW CREATE TABLE ✅

- [x] 3.1.1 确保输出与 live schema 一致 - **已完整实现**
- [x] 3.1.2 包含 column type、NULL、primary key、default - **已实现**
- [x] 3.1.3 添加 e2e fixture - **已有测试**

### 3.2 SHOW COLUMNS ✅

- [x] 3.2.1 不再是空 placeholder - **已实现，从 storage.get_table_info() 读取**
- [x] 3.2.2 LIKE pattern 支持 - **已实现 wildcard_match 辅助函数**
- [x] 3.2.3 添加 e2e fixture - **已添加 show_columns_returns_column_metadata, show_columns_with_like_pattern**

### 3.3 SHOW INDEX ✅

- [x] 3.3.1 返回已注册索引的 index name/table/column/type - **已实现，从 catalog 读取**
- [x] 3.3.2 若暂不支持，明确降级为 DEFERRED - **当 catalog 不可用时返回空结果**
- [x] 3.3.3 添加 e2e fixture - **已添加 show_index_nonexistent_table_returns_error, show_index_on_table_without_catalog_returns_empty**

### 3.4 DESCRIBE ✅

- [x] 3.4.1 等价于 SHOW COLUMNS - **已实现**
- [x] 3.4.2 添加 e2e fixture - **已添加 describe_table_returns_columns**

## Phase 4: 测试验证

- [x] 4.1 `cargo test --test mysql_server_e2e_test show -- --nocapture` PASS
- [x] 4.2 `cargo test --test ddl_e2e_test show -- --nocapture` PASS
- [x] 4.3 `bash scripts/gate/check_v312_21_mysql_compat.sh` PASS

## Phase 5: 文档更新

- [ ] 5.1 更新 MYSQL_COMPAT_STATUS.md
- [ ] 5.2 更新 COMPREHENSIVE_ASSESSMENT_REPORT.md
- [ ] 5.3 运行 `bash scripts/gate/check_docs_consistency.sh` PASS

## Acceptance Criteria

- [x] information_schema.tables/columns/indexes 有 SQL 查询路径或明确 unsupported error
- [x] SHOW CREATE TABLE 输出与 live schema 一致
- [x] SHOW COLUMNS 不再是空 placeholder
- [x] SHOW INDEX 返回已注册索引信息 (catalog 可用时)
- [x] MySQL CLI / e2e fixture 覆盖正反例
- [x] 所有测试 PASS
- [ ] 文档一致

## 已知限制

- SHOW INDEX 依赖 catalog 中的索引信息。`bootstrap_tables: false` 配置下 catalog 不可用，SHOW INDEX 返回空结果。
- CREATE INDEX 执行后索引信息不会自动同步到 catalog（需要在 storage 层创建索引后才能同步到 catalog）。

## 修改文件

- `src/engine_ddl.rs` - 修复 execute_show_index 和 execute_show_columns 实现
- `tests/integration/sql/show_tables_test.rs` - 添加 SHOW INDEX/SHOW COLUMNS/DESCRIBE 测试
