## Context

Issue #4814 — 当前 `execute_create_view` 路径在 storage engine 层
返回 `NotImplemented`,完全没有 view 概念。Parser 接受 `CREATE VIEW`
是因为 `parse_create_view` 已经实现,但 executor 把 view 定义丢弃,
不进入任何 catalog。

`crates/storage/src/` 中 `StorageEngine` trait 也没有 view 相关的
method (`create_view`/`drop_view`/`get_view`),需要扩展 trait。

VIEW 在 SQL 标准中的语义:`CREATE VIEW v AS SELECT ...` 把命名查询存
储为可重用的关系,`SELECT * FROM v` 等价于把 v 替换为底层查询并执行。
视图不存储数据,只是查询的别名。

## Approach

### A1. Catalog 扩展

在 `crates/storage/src/catalog.rs` 中新增模块:

```rust
pub struct ViewDefinition {
    pub name: String,
    pub columns: Vec<String>,        // 显式列别名(可选)
    pub query: Statement,            // 底层 SELECT AST
    pub is_materialized: bool,       // 始终 false (本 PR 不实现 matview)
    pub created_at: SystemTime,
}

pub struct Catalog {
    pub tables: HashMap<String, TableInfo>,
    pub indexes: HashMap<String, IndexInfo>,
    pub views: HashMap<String, ViewDefinition>,  // NEW
}
```

### A2. StorageEngine trait 扩展

在 `crates/storage/src/engine.rs`:

```rust
pub trait StorageEngine {
    // ... existing methods ...

    fn create_view(&mut self, view: ViewDefinition) -> SqlResult<()>;
    fn drop_view(&mut self, name: &str) -> SqlResult<()>;
    fn get_view(&self, name: &str) -> Option<ViewDefinition>;
    fn list_views(&self) -> Vec<String>;
}
```

### A3. MemoryStorage + FileStorage 实现

`MemoryStorage`: HashMap-backed,直接读写。

`FileStorage`: 在 `data/views.dat` 中用 bincode 序列化 `Vec<ViewDefinition>`。
启动时从文件加载,每次 `create_view`/`drop_view` 后整体重写文件。

### A4. ExecutionEngine 集成

`src/execution_engine.rs`:

```rust
fn execute_create_view(&self, stmt: &CreateViewStatement) -> SqlResult<...> {
    let view = ViewDefinition {
        name: stmt.view_name.clone(),
        columns: stmt.columns.clone(),
        query: stmt.query.clone(),
        is_materialized: stmt.is_materialized,
        created_at: SystemTime::now(),
    };
    self.storage.write().create_view(view)?;
    Ok(ExecutorResult::empty())
}

fn execute_drop_view(&self, stmt: &DropViewStatement) -> SqlResult<...> {
    if !self.storage.read().get_view(&stmt.view_name).is_some() {
        return Err(SqlError::NotFound(format!("view {}", stmt.view_name)));
    }
    self.storage.write().drop_view(&stmt.view_name)?;
    Ok(ExecutorResult::empty())
}
```

### A5. SELECT FROM view 解析

在 `src/engine_select.rs::resolve_table_reference` 中:

```rust
// Check views FIRST (before table lookup), since view name shadows table
if let Some(view) = self.storage.read().get_view(table_name) {
    // Recursive view depth check via thread-local or counter field
    if self.view_resolution_depth >= 16 {
        return Err(SqlError::Runtime("view nesting depth exceeds 16".into()));
    }
    let new_depth = self.view_resolution_depth + 1;
    return Ok(self.execute_view_recursive(&view, alias, depth=new_depth));
}
```

`execute_view_recursive` 把 view 的 `query` 当子查询执行,返回 rows +
columns。

### A6. View 嵌套防环

存储在 `ExecutionEngine` 上的 `view_resolution_depth: Cell<u32>`(单线程)
或 `AtomicU32`(多线程)。每次进入 `execute_view_recursive` 自增,
离开时自减。超过 16 报错。

### A7. View 重启持久化

`FileStorage::open` 启动时调用 `self.load_views_from_disk()`,
把 `data/views.dat` 反序列化到 `self.views: HashMap`。`save_views`
在 `create_view`/`drop_view` 后同步刷新。

## Files Changed

| File | Lines | Purpose |
|------|-------|---------|
| `crates/storage/src/catalog.rs` | +40 | 新增 ViewDefinition 类型 |
| `crates/storage/src/engine.rs` | +20 | StorageEngine trait 新增 5 个 method |
| `crates/storage/src/memory.rs` | +60 | MemoryStorage view 实现 |
| `crates/storage/src/file_storage.rs` | +120 | FileStorage view 持久化 + 加载 |
| `src/execution_engine.rs` | +80 | execute_create_view / execute_drop_view |
| `src/engine_select.rs` | +60 | view 解析 + 递归执行 + 深度限制 |
| `src/engine_dml.rs` | +20 | DROP VIEW 走 execute_drop_view |
| `tests/integration/sql/p3_view_4814_test.rs` | +280 | 10 个集成测试 |
| `Cargo.toml` | +5 | 注册新 test target |

Total: ~685 lines, ~30 files touched.

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_view_4814_test` | 10/10 PASS |
| `cte_materialization_test` | 9/9 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS (no regression) |
| `repro_v312_93_cte_values_anchor` | 4/4 PASS |
| `repro_v313_96_4717_insert_cte_subquery` | 3/3 PASS |
| `cargo test -p sqlrustgo --lib` | no regression vs baseline |

## Non-Goals

- MATERIALIZED VIEW executor (materialization + refresh + auto-recompile)
- VIEW 权限管理
- WITH CASCADED / WITH LOCAL CHECK OPTION
- 可更新 VIEW (UPDATE/DELETE/INSERT INTO view)
- 视图依赖追踪 (auto-invalidate view when underlying table ALTERed)

## Difficulty Tier

**🔴 HARD** — 唯一"未实现功能"issue,需要新增 5 个 trait method、
storage 层持久化、递归解析与深度限制、view/table shadow 处理。
预计 3-5 天工作量。
