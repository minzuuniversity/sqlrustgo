# Execution Path

> v3.7.0 Core Integrity Release - 唯一可信执行路径
> 这是数据库内核的"宪法"，所有代码必须遵守

## 唯一合法执行路径

```
SQL Query
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 1. Parser (crates/parser/)                                   │
│    SQL text → AST (Abstract Syntax Tree)                     │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Planner (crates/planner/)                                 │
│    AST → LogicalPlan ( Relational Algebra )                  │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Optimizer (crates/optimizer/)                             │
│    LogicalPlan → PhysicalPlan (operators)                    │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. ExecutionEngine (crates/executor/)                        │
│    PhysicalPlan → Result                                     │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. TransactionManager (crates/transaction/)                │
│    TxnContext, LockManager, MVCC                           │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 6. WAL (crates/storage/wal.rs)                              │
│    Write-Ahead Log → durability                             │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│ 7. StorageEngine (crates/storage/)                         │
│    BufferPool → FileStorage (persistence)                  │
└─────────────────────────────────────────────────────────────┘
```

## 路径规则

### 强制规则

| 规则 | 说明 |
|------|------|
| R1 | DML 必须经过 TransactionManager |
| R2 | DML 必须先写 WAL，再修改 storage |
| R3 | 所有层传递 QueryContext |
| R4 | 禁止绕过 ExecutionEngine 直接调用 storage |
| R5 | SELECT 可以跳过 WAL（read-only） |

### 禁止模式

```rust
// ❌ 禁止：直接 storage.insert 不经过 txn/wal
storage.insert(&table, records)?;

// ✅ 必须：
txn_manager.begin()?;
wal.append(insert_log)?;
storage.insert(&table, records)?;
txn_manager.commit()?;
```

## QueryContext 传递

```rust
pub struct QueryContext {
    session_id: SessionId,
    txn: Option<TxnContext>,  // Some for DML, None for read-only SELECT
    config: QueryConfig,
    wal_enabled: bool,
}

impl ExecutionEngine {
    fn execute(&self, ctx: &QueryContext, plan: PhysicalPlan) -> Result<DataSet> {
        // ctx.txn 必须存在对于 DML
        // ctx.wal_enabled 必须为 true 对于 DML
    }
}
```

## ExecutionContext 结构

```rust
pub struct ExecutionContext {
    pub query_ctx: Arc<QueryContext>,
    pub executor_config: ExecutorConfig,
    pub worker_pool: Arc<WorkerPool>,
}

pub enum ExecutionMode {
    Sequential,
    Parallel { worker_threads: usize },
}
```

## 模块接口约束

### Executor → Transaction

```rust
pub trait TransactionalExecutor {
    fn execute_dml(&self, ctx: &QueryContext, op: DmlOperation) -> Result<TxId>;
    fn execute_query(&self, ctx: &QueryContext, plan: PhysicalPlan) -> Result<DataSet>;
}
```

### Transaction → WAL

```rust
pub trait WalWriter {
    fn append(&self, entry: WalEntry) -> Result<Lsn>;
    fn flush(&self) -> Result<()>;
}
```

### WAL → Storage

```rust
pub trait StorageEngine {
    fn begin_txn(&self) -> TxnContext;
    fn insert(&self, txn: &TxnContext, table: &str, records: Records) -> Result<()>;
    fn commit(&self, txn: &TxnContext) -> Result<()>;
    fn rollback(&self, txn: &TxnContext) -> Result<()>;
}
```

## 验证命令

```bash
# 验证主路径调用链
cargo tree -p sqlrustgo-executor -e normal | grep storage
# 应该看到: sqlrustgo-transaction -> sqlrustgo-storage

# 验证无孤岛调用
grep -r "storage.insert" crates/executor/src/
# 应该无输出（应该通过 txn.wal 后再 insert）
```

## 违反后果

违反此路径的代码将导致：

1. **无 crash recovery** - 数据可能丢失
2. **无事务保证** - 数据不一致
3. **架构分裂** - 主路径/孤岛路径并存

## 相关文档

- `TRANSACTION_BOUNDARY.md` - 事务边界定义
- `ARCHITECTURE_RULES.yaml` - 架构规则详情
- `MAINLINE_COMPONENTS.md` - 主路径组件列表