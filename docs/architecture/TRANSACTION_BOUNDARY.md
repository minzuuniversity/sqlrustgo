# Transaction Boundary

> v3.7.0 Core Integrity Release - 事务边界定义
> 定义哪些操作必须经过事务，哪些可以跳过

## 操作分类

### T1: 必须事务 (Transactional)

| 操作 | 必须经过 | 原因 |
|------|---------|------|
| `INSERT` | Txn + WAL | 写数据，需要 durability |
| `UPDATE` | Txn + WAL | 修改数据，需要 undo log |
| `DELETE` | Txn + WAL | 删除数据，需要 undo log |
| `CREATE INDEX` | Txn | DDL 元数据变更 |
| `DROP TABLE` | Txn | DDL 元数据变更 |

### T2: 可选事务 (Read-Only Safe)

| 操作 | 说明 |
|------|------|
| `SELECT` | 可以跳过 WAL，但不能跳过 MVCC snapshot |

### T3: 禁止 (Illegal in v3.7.0)

| 操作 | 状态 |
|------|------|
| `CREATE DATABASE` | 禁止（v3.7.0 是稳态版本） |
| `ALTER TABLE ADD COLUMN` | 禁止（架构冻结） |
| 跨 storage engine 操作 | 禁止（单引擎约束） |

## DML 执行流

### 正确流程

```
INSERT INTO users VALUES (1, 'alice')
    ↓
QueryContext { wal_enabled: true }
    ↓
TransactionManager.begin()
    ↓
WalWriter.append(BeginLog { tx_id })
    ↓
WalWriter.append(InsertLog { tx_id, table: 'users', records: [...] })
    ↓
StorageEngine.insert(txn, 'users', records)
    ↓
TransactionManager.commit()
    ↓
WalWriter.append(CommitLog { tx_id, commit_ts })
    ↓
StorageEngine.commit(txn)
```

### 错误流程（禁止）

```
INSERT INTO users VALUES (1, 'alice')
    ↓
StorageEngine.insert('users', records)  // ❌ 直接写，无 txn，无 WAL
```

## MVCC Snapshot 隔离

### SELECT 行为

```rust
pub fn execute_select(&self, ctx: &QueryContext, plan: PhysicalPlan) -> Result<DataSet> {
    let snapshot = ctx.txn.get_snapshot();  // 获取 MVCC snapshot
    let visible_records = storage.scan(snapshot, &plan.filter)?;
    Ok(DataSet::from_records(visible_records))
}
```

### 一致性保证

| 隔离级别 | 支持状态 | 说明 |
|---------|---------|------|
| READ_UNCOMMITTED | ❌ | v3.7.0 不支持 |
| READ_COMMITTED | ✅ | 已实现 |
| REPEATABLE_READ | ✅ | MVCC 实现 |
| SERIALIZABLE | ❌ | v3.7.0 不支持 |

## 事务状态机

```
BEGIN
    ↓
┌─────────────────────────────────────────┐
│          IN_TRANSACTION                 │
│  - 可以执行 DML                          │
│  - 可以执行 SELECT                       │
│  - 修改被隔离                            │
└─────────────────────────────────────────┘
    ↓                ↓
COMMIT           ROLLBACK
    ↓                ↓
┌─────────┐    ┌─────────────┐
│COMMITTED│    │  ROLLED_BACK│
└─────────┘    └─────────────┘
```

## Lock 边界

### 锁类型

| 锁 | 持有时间 | 用于 |
|---|---------|------|
| SHARED | 事务期间 | SELECT |
| EXCLUSIVE | DML 期间 | INSERT/UPDATE/DELETE |
| INTENTION_SHARED | 表级 | DDL |
| INTENTION_EXCLUSIVE | 表级 | DDL |

### 死锁检测

```rust
pub struct DeadlockDetector {
    timeout_ms: u64,
    wait_graph: WaitGraph,
}

impl DeadlockDetector {
    pub fn detect(&self) -> Result<Option<DeadlockCycle>>;
    pub fn resolve(&self, cycle: &DeadlockCycle) -> TxId;  // rollback youngest
}
```

## 验证命令

```bash
# 验证 INSERT 经过事务
grep -rn "storage.insert\|storage.update\|storage.delete" \
    crates/executor/src/ \
    --include="*.rs" | grep -v "txn\|transaction\|wal"

# 应该无输出
```

## 违反后果

| 违反类型 | 后果 |
|---------|------|
| DML 无事务 | 崩溃后数据丢失 |
| DML 无 WAL | 无法 replay recovery |
| SELECT 无 MVCC | 脏读/不可重复读 |
| 跨引擎操作 | 架构分裂风险 |

## 相关文档

- `EXECUTION_PATH.md` - 执行路径详情
- `crates/transaction/` - 事务实现
- `crates/storage/wal.rs` - WAL 实现