# Proposal — V312-56A: Metadata/SHOW/information_schema 教学与兼容闭环

## Why

Issue #4251: information_schema、SHOW CREATE TABLE、SHOW INDEX、SHOW COLUMNS、DESCRIBE 的真实能力未整理为 MySQL 教学和基础客户端可用的受控子集。现有实现可能是 placeholder，与 live schema 不一致，导致文档与实现不匹配。

## What Changes

### 1. information_schema 能力矩阵

实现并文档化:
- `information_schema.tables` - SQL查询路径返回 schema 中的表
- `information_schema.columns` - SQL查询路径返回列信息
- `information_schema.indexes` - 索引元数据查询
- 若暂不支持，返回明确的 unsupported error (fail-closed)

### 2. SHOW 命令完整实现

- `SHOW CREATE TABLE` - 输出与 live schema 一致，包含 column type、NULL、primary key、default
- `SHOW COLUMNS` - 不再是空 placeholder；若暂不支持 LIKE pattern，fail-closed 并文档化
- `SHOW INDEX` - 返回已注册索引的 index name/table/column/type
- `DESCRIBE` - 等价于 `SHOW COLUMNS`

### 3. 教学 Fixtures

每个命令需要:
- **正例**: 正常输出的 SQL 查询
- **反例**: 不支持的 pattern / 错误语义的用例
- **边界**: unsupported 时的 fail-closed 行为

### 4. MySQL CLI / e2e 覆盖

确保 mysql client 或等效工具能展示这些元数据命令的输出。

## Capabilities

### New Capabilities

- **information_schema SQL 查询路径** - 支持标准 SQL 查询元数据
- **完整的 SHOW 命令实现** - 与 MySQL 语义对齐
- **fail-closed 边界处理** - 不支持的功能明确报错，不静默忽略

### Modified Capabilities

- 现有 placeholder SHOW 命令 → 完整实现

## Non-goals

- 不实现完整 MySQL information_schema (仅限.tables/.columns/.indexes)
- 不实现 SHOW TABLE STATUS (延期)
- 不实现 COLUMNS 的 FIELD/COLLATION 等扩展字段

## Acceptance Criteria

- [ ] `information_schema.tables`、`information_schema.columns`、`information_schema.indexes` 至少有 SQL 查询路径或明确 unsupported error
- [ ] `SHOW CREATE TABLE` 输出与 live schema 一致
- [ ] `SHOW COLUMNS` 不再是空 placeholder
- [ ] `SHOW INDEX` 至少返回已注册索引信息
- [ ] MySQL CLI / e2e fixture 覆盖上述正反例
- [ ] 运行 `cargo test --test mysql_server_e2e_test show -- --nocapture` PASS
- [ ] 运行 `cargo test --test ddl_e2e_test show -- --nocapture` PASS
- [ ] 运行 `bash scripts/gate/check_v312_21_mysql_compat.sh` PASS

## Issue Reference

Issue #4251 (V312-56A)
