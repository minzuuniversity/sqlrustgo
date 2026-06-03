# Proposal: LocalExecutor 添加 engine 字段 (Issue #2811, G4)

## Why

`MergeExecutor::new()` 需要 `Arc<Mutex<dyn ExecutionEngine>>` 参数，但 `LocalExecutor` 结构体没有 `engine` 字段，导致 `MergeExecutor` 无法实例化（PR-870 死代码）。同时 `LocalExecutor` 当前持有 `storage: &'a dyn StorageEngine`（借用），无法传递给 `MergeExecutor::new()` 所需的 `Arc<RwLock<dyn StorageEngine>>` 形式。

本 PR 是 G3 (#2810) 的前置依赖：G3 会在 `local_executor.rs` 中调用 `MergeExecutor::new()`，需要 LocalExecutor 暴露 Arc-wrapped storage 和 engine 字段。

## What Changes

- **BREAKING (API 兼容)**: `LocalExecutor.storage` 从 `&'a dyn StorageEngine` 改为 `Arc<RwLock<dyn StorageEngine>>`
- **新增字段**: `LocalExecutor.engine: Arc<Mutex<dyn ExecutionEngine>>`
- **新增方法**: `LocalExecutor::new_with_engine(storage, engine)` 构造函数
- **保留**: `LocalExecutor::new(storage)` 现有构造函数（自动包装 storage + 构造默认 engine）
- **更新**: 所有 call sites（约 20 处）将 `&storage` 改为 `Arc::new(RwLock::new(storage))`

## Capabilities

### New Capabilities

- `local-executor-engine-field`: LocalExecutor 必须持有 `Arc<Mutex<dyn ExecutionEngine>>` 字段以支持 MergeExecutor 实例化和 VTU 路径

### Modified Capabilities

（无现有 spec 涉及此变更）

## Impact

### 代码影响
- `crates/executor/src/local_executor.rs`: 主要改动
- `crates/server/src/connection_pool.rs`: 2 处 `LocalExecutor::new` 调用
- `crates/executor/src/harness.rs`: 1 处调用
- `crates/executor/src/local_executor.rs` (mod tests): 17 处测试

### 依赖影响
- 无新增依赖
- 已有 `Arc<Mutex<...>>` 和 `ExecutionEngine` trait

### 范围限定
- 本 PR **仅重构 LocalExecutor 字段**
- **不**实现 `execute_merge()` 调用（这是 G3 #2810）
- **不**实现 MERGE 的实际执行（这是 G3）

### 后续路径
- 本 PR 后，G3 PR（独立）将：
  - 在 `local_executor.rs` 中检测 MERGE 关键字
  - 调用 `MergeExecutor::new(self.storage.clone(), self.engine.clone())`
  - 调用 `merge_executor.execute_merge(&merge_stmt)`
