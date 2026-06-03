# Proposal: LocalExecutorDml MERGE dispatch (Issue #2810, G3)

## Why

`MergeExecutor::execute_merge()` 在 `crates/executor/src/merge.rs:36` 已定义但**从未被调用**。生产路径上的 `LocalExecutorDml` (`crates/executor/src/local_executor_dml.rs`) 完全不处理 MERGE 语句（连错误返回都没有，因为它是一个 45 行的 placeholder）。

前置依赖已完成：
- ✅ G1 (PR #2842): FileStorage 包装 WalStorage
- ✅ G2 (PR #2849): Parser 支持 MERGE 语法（`Statement::Merge(MergeStatement)` AST）
- ✅ G4 (PR #2862): LocalExecutorDml 添加 `engine: Arc<Mutex<dyn ExecutionEngine>>` 字段

现在 G3 可以将这三个能力串起来：检测 MERGE → 解析 → 转换 AST → 构造 MergeExecutor → 调用 execute_merge。

## What Changes

- `crates/executor/src/local_executor_dml.rs`:
  - 添加 `storage: Arc<RwLock<dyn StorageEngine>>` 字段（之前仅 engine）
  - 添加 `new_with_storage(storage, engine)` 构造函数（同时设置 storage 和 engine）
  - 修改 `new()` 自动使用 `NoopExecutionEngine` + 占位 `MemoryStorage`（保持向后兼容）
  - 添加 `execute_dml(sql: &str) -> Result<ExecutorResult, SqlError>` 方法
  - 在 `execute_dml` 中检测 MERGE 关键字
  - 用 G2 parser 解析 MERGE
  - 添加私有 `convert_parser_merge_to_planner` 函数（lossy: parser → planner）
  - 构造 `MergeExecutor::new(self.storage.clone(), self.engine.clone())`
  - 调用 `merge_executor.execute_merge(&planner_merge)`

## Capabilities

### New Capabilities

- `merge-dispatch-g3`: `LocalExecutorDml.execute_dml` 必须检测 MERGE 关键字并调用 `MergeExecutor::execute_merge`，而非返回错误

### Modified Capabilities

（无现有 spec 涉及此变更）

## Impact

### 代码影响
- `crates/executor/src/local_executor_dml.rs`: 唯一改动文件（约 +120, -10）

### 依赖影响
- 新增依赖: `sqlrustgo-parser`（已存在，作为 dev-dep 或 workspace）
- 新增依赖: `sqlrustgo-planner`（已存在）
- 新增依赖: `sqlrustgo-storage`（已存在）
- 新增依赖: `sqlrustgo-types`（已存在）

### 范围限定
- 本 PR **仅 dispatcher 集成**
- **不**实现完整 MERGE DML 路径（INSERT/UPDATE/DELETE 仍为后续）
- **不**修改 `MergeExecutor::execute_merge` 本身
- type 转换 lossy：parser::MergeStatement 支持多 WHEN + DELETE，planner::MergeStatement 仅 matched/not_matched 单个 clause

### 后续路径
- 本 PR 后，MERGE 语句会通过 `execute_dml` 路径，调用 `execute_merge`
- 后续可优化：完整 DML 执行、错误处理、性能

## Known Limitations

1. **Lossy conversion**: parser `Vec<MergeWhenClause>` → planner `Option<MergeClause>` × 2（仅 first matched + first not_matched 被保留）
2. **DELETE not supported in MERGE**: parser 支持，planner 不支持
3. **Subquery source not fully wired**: parser 解析 OK，planner 期望单表名
4. **No `execute_dml` caller yet**: 需要在 REPL/server 端接入 SQL 字符串入口

## Migration Plan

### Deploy
1. PR 合并到 `develop/v3.8.0`
2. CI 验证
3. P0 任务全部完成

### Rollback
- 单 commit revert
- 现有调用 `LocalExecutorDml::new()` 的代码（仅测试）保持兼容
