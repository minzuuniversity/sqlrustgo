# Tasks — Issue #4814: CREATE VIEW / DROP VIEW / SELECT FROM view

## 1. Catalog + Storage Engine

- [ ] 1.1 在 `crates/storage/src/catalog.rs` 新增 `ViewDefinition` struct:
      name / columns / query / is_materialized / created_at。
- [ ] 1.2 在 `crates/storage/src/catalog.rs` 给 `Catalog` 添加
      `views: HashMap<String, ViewDefinition>` 字段。
- [ ] 1.3 在 `crates/storage/src/engine.rs` 的 `StorageEngine` trait
      添加 5 个 method:
      `create_view(&mut self, view: ViewDefinition) -> SqlResult<()>`
      `drop_view(&mut self, name: &str) -> SqlResult<()>`
      `get_view(&self, name: &str) -> Option<ViewDefinition>`
      `list_views(&self) -> Vec<String>`
      `view_exists(&self, name: &str) -> bool`
- [ ] 1.4 在 `crates/storage/src/memory.rs` 实现上述 5 个 method
      (HashMap-backed,内存中操作即可)。
- [ ] 1.5 在 `crates/storage/src/file_storage.rs` 实现上述 5 个 method +
      `load_views_from_disk()` / `save_views_to_disk()` 持久化路径
      (`data/views.dat` 用 bincode 序列化 `Vec<ViewDefinition>`)。
- [ ] 1.6 在 `FileStorage::open` 启动流程里 `load_views_from_disk`,
      在 `create_view` / `drop_view` 后 `save_views_to_disk`。

## 2. ExecutionEngine DDL

- [ ] 2.1 在 `src/execution_engine.rs` 新增
      `fn execute_create_view(&self, stmt: &CreateViewStatement)
      -> SqlResult<ExecutorResult>`:
      - 校验 view name 不与现有 table 冲突(可选 warning,允许同名)。
      - 构造 `ViewDefinition`,写入 `storage.write().create_view()`。
      - 返回 `ExecutorResult::empty()`。
- [ ] 2.2 新增 `fn execute_drop_view(&self, stmt: &DropViewStatement)
      -> SqlResult<ExecutorResult>`:
      - `IF EXISTS` 语义:不存在则 warning + 空结果。
      - 否则 `storage.write().drop_view()`。
- [ ] 2.3 在 DDL dispatch 表 (主 execute 函数) 中注册 CREATE VIEW /
      DROP VIEW → 上述两个 method。

## 3. SELECT FROM view 解析

- [ ] 3.1 在 `src/engine_select.rs::resolve_table_reference` 中,**先**
      检查 `storage.read().get_view(table_name)`,命中则走 view 解析路径。
- [ ] 3.2 新增 `fn execute_view_recursive(&self, view: &ViewDefinition,
      alias: Option<String>, depth: u32) -> SqlResult<(Vec<Vec<Value>>,
      TableInfo)>`:
      - `if depth >= 16 { return Err(...); }` 防递归爆栈。
      - 把 view.query (Statement::Select) 包装为内部 SELECT,执行
        `execute_select_inner`。
      - 返回 rows + TableInfo(columns = view.columns 或 query columns)。
- [ ] 3.3 把 view 解析产生的 rows 注入外层 FROM 流程 (与 `from_subquery`
      同路径),正确处理 alias / columns / 类型推导。
- [ ] 3.4 处理 view 与 table 同名:view 优先 (如果 table 同名则 shadow),
      文档说明此行为。

## 4. Tests (10 tests)

- [ ] 4.1 `create_view_basic` — `CREATE VIEW v AS SELECT * FROM t` +
      `SELECT * FROM v` 返回 t 的全部行。
- [ ] 4.2 `create_view_with_column_aliases` — `CREATE VIEW v(id, doubled)
      AS SELECT id, val*2 FROM t`。
- [ ] 4.3 `create_view_with_aggregation` — view 含 `SELECT dept, COUNT(*)
      FROM t GROUP BY dept`。
- [ ] 4.4 `select_from_view_unqualified` — `SELECT * FROM v`。
- [ ] 4.5 `select_from_view_with_where` — `SELECT * FROM v WHERE id=1`。
- [ ] 4.6 `select_from_view_with_join` — view 内含 join,验证 view query
      可用全部 SELECT 语法。
- [ ] 4.7 `drop_view_then_select_fails` — DROP VIEW 后 SELECT 报错。
- [ ] 4.8 `create_view_persists_across_restart` (FileStorage only) —
      创建 view → 关闭 → 重启 → SELECT 仍可用。
- [ ] 4.9 `nested_view_depth_limit` — view A 引用 view B 引用 view C...
      深度 17 时报 "view nesting depth exceeds 16"。
- [ ] 4.10 `view_does_not_collide_with_table_name` — 同名 table + view
      不冲突,view 优先。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test -p sqlrustgo --test p3_view_4814_test` 10/10 PASS。
- [ ] 5.3 `cargo test --all-features --lib` no regression。
- [ ] 5.4 `openspec validate fix-p3-view-4814 --strict` valid。

## 6. Commit + Memory

- [ ] 6.1 Commit message:
      `fix(P3 / #4814): implement CREATE VIEW / DROP VIEW / SELECT FROM view`
- [ ] 6.2 在 `memory/` 下新增
      `p3-view-4814-storage-view-closure.md` 记录 view 存储设计 + 嵌套
      深度限制权衡。
- [ ] 6.3 `openspec archive fix-p3-view-4814` after merge。

## 7. Out of Scope (deferred)

- [ ] 7.1 MATERIALIZED VIEW executor (materialization + refresh) — 推到
      v3.14,parser 已经接受 `is_materialized=true` 但行为等同普通 view。
- [ ] 7.2 VIEW 权限管理 (GRANT/REVOKE) — 推到 v3.14。
- [ ] 7.3 WITH CASCADED / WITH LOCAL CHECK OPTION — 推到 v3.14。
- [ ] 7.4 可更新 VIEW (UPDATE/DELETE/INSERT INTO view) — 推到 v3.15。
