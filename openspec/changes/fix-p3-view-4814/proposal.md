## Why

Issue #4814: `CREATE VIEW v AS SELECT ...` 在 sqlrustgo 中报错
`create_view not supported by this storage engine`。Parser 接受,但 storage 层
完全未实现 CREATE VIEW 的执行路径,与 MySQL/SQLite 不兼容。

这是 P3 batch 中**唯一的"未实现功能"**,其他 7 个 issue 都是 silent
accept-and-ignore bug。VIEW 是 ORM 框架 (Django, Rails, Hibernate) 的中间层依赖,
完全不可用导致 ORM 集成阻塞。

## What Changes

1. **Parser**: `parse_create_view` 已经存在并接受 `CREATE VIEW name AS
   SELECT ...` 和列别名。无需改动。
2. **ExecutionEngine**: 新增 `execute_create_view` 处理函数:把视图定义
   (查询文本或结构化 AST) 存入 catalog 的 `views` 表。
3. **Catalog**: 在 `Catalog` 中新增 `views: HashMap<String, ViewDefinition>`
   字段,`ViewDefinition` 包含 view name、columns、underlying query AST、
   `is_materialized: bool`。
4. **Storage**: `MemoryStorage` 和 `FileStorage` 实现 `create_view`、
   `drop_view`、`get_view` 三个 trait method。`FileStorage` 的 view
   元数据持久化到独立的 `views.dat` 文件,与 table 数据分离。
5. **SELECT FROM view**: `resolve_table_reference` 把 view name 当作
   "可解析视图"展开为底层 SELECT 子查询,然后照常执行。递归 view (view
   引用 view) 用栈深度限制 ≤ 16 防无限递归。
6. **DROP VIEW**: parser 已支持 `DROP VIEW name`,executor 实现
   `execute_drop_view` 走 catalog 删除路径。

## Capabilities

### New Capabilities

- `executor-create-view`: `sqlrustgo` MUST 接受 `CREATE VIEW name AS
  SELECT ...` 并将视图定义存入 catalog,在随后的 `SELECT * FROM view`
  中正确解析并执行底层查询。
- `executor-drop-view`: `sqlrustgo` MUST 接受 `DROP VIEW name` 并从
  catalog 中移除视图。
- `storage-view-metadata`: `MemoryStorage` 和 `FileStorage` MUST 实现
  view 元数据的持久化/恢复,重启后视图依然可用。

### Modified Capabilities

- None. 现有 CREATE TABLE / SELECT 行为完全不变。

## Out of Scope

- **MATERIALIZED VIEW**: 解析接受(由 [[fix-v313-99-4692-writable-cte]]
  处理),但 executor 层 materialization (预计算 + 增量刷新) 不在本 PR。
  `is_materialized=true` 的 view 行为等同普通 view (lazy resolution)。
- **VIEW 权限管理**: 不实现 GRANT/REVOKE 对 VIEW 的访问控制。
- **WITH CASCADED / WITH LOCAL CHECK OPTION**: 推迟到 v3.14。
- **可更新 VIEW** (UPDATE/DELETE/INSERT INTO view): 不在本 PR。

## Verification

- `cargo build --all-features` clean。
- 新测试 `tests/integration/sql/p3_view_4814_test.rs`:
  - `create_view_basic` — 创建 view + SELECT * FROM view。
  - `create_view_with_column_aliases` — `CREATE VIEW v(id, doubled) AS
    SELECT id, val*2 FROM t`。
  - `create_view_with_aggregation` — view 内含 GROUP BY / aggregate。
  - `select_from_view_unqualified` — bare `SELECT * FROM v`。
  - `select_from_view_with_where` — `SELECT * FROM v WHERE id=1`。
  - `select_from_view_with_join` — view 内含 join。
  - `drop_view_then_select_fails` — DROP 后再 SELECT 报错。
  - `create_view_persists_across_restart` (FileStorage only) — 重启后
    view 仍可用。
  - `nested_view_depth_limit` — view 引用 view 深度 ≤ 16,超过报错。
  - `view_does_not_collide_with_table_name` — 同名 table+view 互不干扰。
- `cargo test -p sqlrustgo --test p3_view_4814_test` 10/10 PASS。
- 回归测试 `cargo test -p sqlrustgo --test cte_materialization --test
  parser_e2e_test --test repro_v312_93_cte_values_anchor` 全绿。

## Risks / Trade-offs

- **VIEW 元数据一致性**: FileStorage 重启后 view 必须在 table 之前加载
  (因为 view 可能引用 table)。需要保证 catalog 加载顺序:tables → indexes
  → views。
- **VIEW schema evolution**: ALTER TABLE 改 column 后,view 的底层 query
  可能失效。本 PR 不实现 auto-recompile,失效时报错 "view references
  missing column" (清晰但非自动修复)。
- **MATERIALIZED VIEW 占位**: `is_materialized=true` 当前行为等同普通
  view,可能在用户预期中造成 confusion。文档中明确说明。
