## Why

Issue #4810: `DROP INDEX idx_name` 在 sqlrustgo 中报 `"index does not 
exist"` 错误,**即使索引存在**。

v312-91 PR #4789 已经实现 DROP INDEX executor,但可能有 regression
或与 v312-95 v2 rebases 后的 catalog schema 不兼容。Issue 报告者
观察到 DROP INDEX 失败,可能因为:

1. PR #4789 的 DROP INDEX executor 在后续 commit 中被覆盖 / revert。
2. Catalog 中存储的索引 metadata 与 DROP INDEX 路径读取的不一致
   (e.g., DROP 时按 `name` 查,DROP 后 storage 中残留孤儿元数据)。
3. IF EXISTS / IF NOT EXISTS 语法边界 case 处理错误。

SQL 标准 DROP INDEX 是 idempotent + atomic,失败模式属于静默问题
(索引存在但 DROP 误报 not exists,数据库长期累积未使用索引,泄漏
storage space)。

## What Changes

1. **Executor 验证**: `execute_drop_index` 在 PR #4789 后的状态:
     - 验证 `list_all_indexes` 返回正确的 name → table 映射。
     - 验证 `drop_index(table_name, index_name)` 真的从 catalog 移
       除索引。
     - 验证 IF EXISTS / 不存在时正确返回。
2. **Catalog schema consistency**: 如果 DROP INDEX 读到的 metadata
     schema 与 CREATE INDEX 写入的不一致,unify。
3. **Catalog cleanup**: DROP TABLE 时级联 drop indexes;CREATE INDEX
     时不污染其他表的索引列表。

## Capabilities

### New Capabilities

- `executor-drop-index-works`: `DROP INDEX idx` (索引存在) MUST 删
  除索引并返回 success。

## Out of Scope

- `DROP INDEX IF EXISTS`: 已在 PR #4789 实现,本 PR 验证仍正常。
- `DROP INDEX CONCURRENTLY` (PostgreSQL): 推迟。
- `ALTER INDEX ... RENAME`: 推迟。

## Verification

- 新测试 `tests/integration/sql/p3_drop_index_4810_test.rs`:
  - `drop_index_existing_succeeds` — issue anchor:`CREATE INDEX 
    idx_a ON t(col); DROP INDEX idx_a;` 后再 `DROP INDEX idx_a` 应该
    报 not exists。
  - `drop_index_if_exists_no_error` — `DROP INDEX IF EXISTS no_idx`。
  - `drop_index_nonexistent_errors` — `DROP INDEX no_idx` 报错。
  - `drop_index_then_select_works` — DROP INDEX 后相关列的查询仍
    正常(seq scan fallback)。
  - `drop_index_cascade_on_drop_table` — DROP TABLE 级联 drop 该
    表的所有索引。
  - `drop_index_with_compound_index` — 多列索引 DROP 正确。

Total: 6 tests。

- `cargo test --test p3_drop_index_4810_test` 6/6 PASS。
- 回归: drop_index_test (v312-91), parser_e2e_test。

## Risks / Trade-offs

- **PR #4789 已被合并**: 大概率本 issue 已经是 fixed-by-existing-code。
  验证步骤:
  1. 检查 `git log --grep "DROP INDEX"` 找 PR #4789 后续 commit。
  2. 跑现有 drop_index 测试,看是否全 PASS。
  3. 如果全 PASS,转 issue 为 "verified, no fix needed"。
  4. 如果有 FAIL,定位 regression commit,做 minimal fix。
- **Schema migration**: 如果 catalog metadata schema 在
  PR #4789 → v312-95 v2 间变化,旧 database file 可能不兼容。
  需要在 catalog load 时 graceful handle (e.g., 缺失 index 字段默认
  空 list)。