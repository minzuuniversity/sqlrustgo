# Phase 1.1: ExecutionEngine Trait Proposal

> v3.7.0 Core Integrity Release - 执行引擎接口设计
> 状态: PROPOSAL - 待评审后实施

## 目标

收敛执行入口，定义统一 `ExecutionEngine` 接口和 `QueryContext`。

## 当前状态

| 执行器 | 问题 |
|--------|------|
| `LocalExecutor` | 直接调用 storage，无 txn/wal |
| `VolcanoExecutor` | executor.rs 缺少 WAL 集成 |
| `ParallelExecutor` | ISOLATED - 无主路径调用 |
| `ParallelVectorExecutor` | ISOLATED - 无主路径调用 |
| `TransactionalExecutor` | ✅ 已实现但未被主路径使用 |

## 已有资源

- `TransactionContext` (transaction/src/manager.rs) - 已存在
- `WalTransactionalExecutor` (transactional_executor.rs) - 已实现 WAL 支持
- `StorageEngine` trait (storage) - 已存在

## Proposal: 统一 QueryContext

```rust
pub struct QueryContext {
    session_id: SessionId,
    txn: Option<TxnContext>,      // None for read-only SELECT
    wal_enabled: bool,
    execution_mode: ExecutionMode,
}

pub enum ExecutionMode {
    Sequential,
    Parallel { worker_threads: usize },
}

impl QueryContext {
    pub fn new(session_id: SessionId) -> Self { ... }
    pub fn with_txn(txn: TxnContext) -> Self { ... }
    pub fn with_wal(self, enabled: bool) -> Self { ... }
    pub fn requires_txn(&self) -> bool { ... }
}
```

## Proposal: 统一 ExecutionEngine Trait

```rust
pub trait ExecutionEngine: Send + Sync {
    fn execute(
        &self,
        ctx: &QueryContext,
        plan: PhysicalPlan,
    ) -> Result<ExecutorResult, SqlError>;

    fn name(&self) -> &'static str;

    fn is_ready(&self) -> bool;
}
```

## Proposal: TransactionalExecutor 升级

```rust
pub struct TransactionalExecutor<S: StorageEngine> {
    storage: Arc<RwLock<S>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

impl<S: StorageEngine> ExecutionEngine for TransactionalExecutor<S> {
    fn execute(
        &self,
        ctx: &QueryContext,
        plan: PhysicalPlan,
    ) -> Result<ExecutorResult, SqlError> {
        // 必须使用 ctx.txn 进行 DML
        // 必须通过 WAL 进行持久化
    }
}
```

## 不在此阶段做的事情

- ❌ 大规模 executor 替换
- ❌ Parallel executor 集成
- ❌ Recovery pipeline 重构
- ❌ Storage engine 重构

## 只做

- ✅ 定义 QueryContext 结构
- ✅ 定义 ExecutionEngine 接口
- ✅ 迁移 LocalExecutor 到新接口（不修内部）
- ✅ 运行 check_mainline.sh 验证

## 验收条件

- [ ] QueryContext 定义完成
- [ ] ExecutionEngine trait 定义完成
- [ ] check_mainline.sh 无新增 CRITICAL
- [ ] 主路径使用统一接口

## 相关文档

- docs/EXECUTION_PATH.md
- docs/TRANSACTION_BOUNDARY.md
- docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md