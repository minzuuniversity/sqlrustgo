# Design: LocalExecutor engine 字段重构

## Context

### 背景
`MergeExecutor::new()` 签名是 `(storage: Arc<RwLock<dyn StorageEngine>>, engine: Arc<Mutex<dyn ExecutionEngine>>)`。但当前 `LocalExecutor` 持有 `storage: &'a dyn StorageEngine`（借用），且完全没有 `engine` 字段。这意味着任何想从 LocalExecutor 创建 MergeExecutor 的代码（如 G3）都无法做到。

### 当前状态
```rust
pub struct LocalExecutor<'a> {
    storage: &'a dyn StorageEngine,
    cache: Arc<RwLock<QueryCache>>,
    ...
}

impl<'a> ExecutionEngine for LocalExecutor<'a> {
    fn execute(&mut self, ...) -> ... { self.execute_dml(ctx) }
    ...
}
```

### 约束
- 20+ call sites 使用 `LocalExecutor::new(&storage)` 或 `LocalExecutor::new(&*storage_ptr)`，必须保持兼容
- `LocalExecutor` 已实现 `ExecutionEngine` trait，新字段不能破坏此实现
- `Arc<Mutex<dyn ExecutionEngine>>` 需要一个 trait object，但 LocalExecutor 自身引用了 `self`（循环依赖）

## Goals / Non-Goals

### Goals
- ✅ 添加 `engine: Arc<Mutex<dyn ExecutionEngine>>` 字段
- ✅ 改造 `storage` 为 `Arc<RwLock<dyn StorageEngine>>`
- ✅ 提供 `new_with_engine` 构造函数
- ✅ 保持 `new` 向后兼容
- ✅ 所有现有测试通过

### Non-Goals
- ❌ 不实现 `execute_merge()` 调用（G3 任务）
- ❌ 不重写 `impl ExecutionEngine for LocalExecutor` 的实现（保持 `self.execute_dml(ctx)` 行为）
- ❌ 不改变其他 executor crate 的实现

## Decisions

### Decision 1: storage 改为 Arc<RwLock<dyn StorageEngine>>

**选择**: 
```rust
pub struct LocalExecutor {
    storage: Arc<RwLock<dyn StorageEngine>>,
    engine: Arc<Mutex<dyn ExecutionEngine>>,
    ...
}
```

移除 `<'a>` 生命周期参数。

**理由**:
- 必须有 owned storage 才能 clone 给 MergeExecutor
- `Arc<RwLock<>>` 与 MergeExecutor 期望的类型完全一致
- 移除生命周期简化类型签名

**备选**:
- ❌ 保留 `&'a` + 添加 `Arc<Mutex<>>` 克隆：需要 unsafe 转换或 `Arc::clone(&self.storage_arc)` 模式
- ❌ 双重 storage（一个 borrowed + 一个 arc）：冗余

### Decision 2: engine 字段使用 self-referencing adapter

**选择**: 使用 `LocalExecutorEngine` adapter 实现 `ExecutionEngine` trait

```rust
struct LocalExecutorEngine {
    storage: Arc<RwLock<dyn StorageEngine>>,
    cache: Arc<RwLock<QueryCache>>,
    cache_config: QueryCacheConfig,
}

impl ExecutionEngine for LocalExecutorEngine {
    fn execute(&mut self, ctx: &mut QueryContext) -> Result<...> {
        // Delegate to LocalExecutor::execute_dml logic
    }
    // ... other trait methods
}
```

然后构造：
```rust
let engine = Arc::new(Mutex::new(LocalExecutorEngine { 
    storage: storage.clone(),
    cache: cache.clone(),
    cache_config: cache_config.clone(),
}));
```

**理由**:
- LocalExecutor 已有 self-referential 结构（storage + cache + engine 互相引用）
- Adapter 模式将"engine 功能"封装为独立结构
- Adapter 持有 Arc 引用，独立于 LocalExecutor 的 lifetime

**备选**:
- ❌ `Arc::new_cyclic` 创建 self-referential：复杂且易出错
- ❌ LocalExecutor 自身作为 engine（用 `Arc<Mutex<Self>>`）：需要 Self: Send + Sync + 移除所有借用

### Decision 3: new() 自动构造默认 engine（向后兼容）

**选择**:
```rust
impl LocalExecutor {
    /// Backward-compat constructor
    pub fn new<S: StorageEngine + 'static>(storage: S) -> Self {
        let storage_arc = Arc::new(RwLock::new(storage));
        let engine = Arc::new(Mutex::new(LocalExecutorEngine::new(
            storage_arc.clone(),
            ...
        )));
        Self { storage: storage_arc, engine, ... }
    }

    /// New explicit constructor
    pub fn new_with_engine(
        storage: Arc<RwLock<dyn StorageEngine>>,
        engine: Arc<Mutex<dyn ExecutionEngine>>,
        ...
    ) -> Self { ... }
}
```

**理由**:
- 现有 20+ call sites 只需 `LocalExecutor::new(storage)` 不变
- 新的代码可使用 `new_with_engine` 传入显式 engine

**备选**:
- ❌ 强制所有 call sites 更新：breaking change 太大

### Decision 4: 保留 LocalExecutor::execute() 和 execute_dml() 行为

**选择**: 不改变 `impl ExecutionEngine for LocalExecutor` 的 `execute` 方法行为

```rust
impl ExecutionEngine for LocalExecutor {
    fn execute(&mut self, ctx: &mut QueryContext) -> Result<...> {
        self.execute_dml(ctx)
    }
    // ...
}
```

**理由**:
- 保持现有 DML 路径不变
- engine 字段主要用于 G3 调用 MergeExecutor，不影响 LocalExecutor 自身 execute 行为

## Risks / Trade-offs

| 风险 | 缓解 |
|------|------|
| 20+ call sites 需要更新 | `new()` 保持兼容，传入 owned storage 而非 borrowed |
| `LocalExecutor<'a>` 移除生命周期参数 | grep 所有引用点统一更新 |
| Self-referencing engine 复杂度 | Adapter 模式隔离，避免 Arc::new_cyclic |
| 测试代码风格 `&storage` | 改为 owned `storage`（local var） |

## Migration Plan

### Deploy
1. PR 合并到 `develop/v3.8.0`
2. CI 验证
3. 后续 G3 PR 依赖本 PR

### Rollback
- 单 commit revert
- LocalExecutor API 兼容性回退（旧 `&'a` 签名）

## Open Questions

- **Q1**: LocalExecutor 移除 `<'a>` 后是否需要 `LocalExecutor` 默认实现 Send + Sync？  
  现状: `impl Send + Sync` 在某些代码中依赖 `'a`  
  解决: 移除 `<'a>` 后 Send + Sync 自动实现（因为所有字段是 Send + Sync 的）

- **Q2**: `LocalExecutorEngine` adapter 是否需要单独 crate？  
  现状: 与 LocalExecutor 同 crate 即可  
  本设计: 同 crate（`local_executor.rs` 内私有 struct）
