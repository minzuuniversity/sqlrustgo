# PR-840 DML Transaction Interception 架构方案

> 状态: 研究中
> 日期: 2026-05-31
> 基于: `develop/v3.7.0` (commit 0c3ac7c64)

---

## 1. 背景

### 1.1 RC Gate 需求

RC Gate 需要以下功能完成:

| 功能 | 描述 | 状态 |
|------|------|------|
| RC-F1 | BEGIN/COMMIT/ROLLBACK 路由到 TransactionManager | 已部分实现 (ExecutionEngine 层面) |
| RC-F2 | DML staging through WriteBuffer (不直接到 StorageEngine) | **未实现** |
| RC-F3 | COMMIT flushes WriteBuffer → StorageEngine | **未实现** |
| RC-F4 | ROLLBACK discards WriteBuffer (无存储副作用) | **未实现** |

### 1.2 当前代码库状态

**当前 DML 执行路径** (`src/execution_engine.rs`):

```
execute_insert() / execute_update() / execute_delete()
    │
    ├── require_tx()?      // EEK v0: 检查 active transaction
    │
    ├── storage.write().insert()    // 直接写入 StorageEngine
    ├── storage.write().update()    // 直接写入 StorageEngine
    └── storage.write().delete()    // 直接写入 StorageEngine
```

**关键发现**:

1. `ExecutionEngine` 已有 `transaction_manager: TransactionManager` 字段 (第41行)
2. `execute_transaction()` 处理 BEGIN/COMMIT/ROLLBACK (第1351行)
3. `require_tx()` 检查 `current_tx_id` 是否存在 (第1443行)
4. DML (INSERT/UPDATE/DELETE) **直接调用** `storage.insert()/update()/delete()` — **未经过 WriteBuffer**

---

## 2. 当前 DML 执行路径分析

### 2.1 execute_insert() 路径 (第800行)

```rust
fn execute_insert(&self, insert: &InsertStatement) -> SqlResult<ExecutorResult> {
    self.require_tx()?;                    // 检查活跃事务
    // ... 触发器处理 ...
    let mut storage = self.storage.write().unwrap();
    storage.insert(&table_name, processed_records)?;  // ← 直接写入
}
```

### 2.2 execute_update() 路径 (第927行)

```rust
fn execute_update(&self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
    self.require_tx()?;
    // ... 扫描、过滤、更新 ...
    let mut storage = self.storage.write().unwrap();
    storage.delete(&table_name, &[])?;     // ← 直接操作
    storage.insert(&table_name, ...)?;    // ← 直接操作
}
```

### 2.3 execute_delete() 路径 (第1088行)

```rust
fn execute_delete(&self, delete: &DeleteStatement) -> SqlResult<ExecutorResult> {
    self.require_tx()?;
    // ... 扫描、过滤 ...
    let mut storage = self.storage.write().unwrap();
    storage.delete(&table_name, &[])?;    // ← 直接删除
    storage.insert(&table_name, rows_to_keep)?;  // ← 重新插入保留行
}
```

### 2.4 WalStorage 包装 (crates/storage/src/wal_storage.rs)

`WalStorage<S>` 在 mutation 前记录 WAL，但**仍然直接调用 inner storage**:

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    let table_id = Self::table_name_to_id(table);
    for record in &records {
        let key = Self::record_key(record);
        let data = Self::record_to_bytes(record);
        self.log_insert(table_id, key, data)?;  // WAL 记录
    }
    self.inner.insert(table, records)            // ← 仍然直接写入
}
```

---

## 3. WriteBuffer 候选设计方案

### 3.1 设计方案 A: TransactionManager 持有 WriteBuffer (推荐)

```
mysql-server/Session
  └── TransactionManager
        ├── active_txn: Option<ActiveTransaction>
        ├── write_buffer: WriteBuffer
        └── snapshot: SnapshotMeta
              │
              ├── stage_write(WriteOp)    // DML 暂存
              ├── commit() → flush()     // 写入 StorageEngine
              └── rollback() → clear()   // 丢弃
```

**WriteBuffer 结构**:

```rust
pub struct WriteBuffer {
    staging: Vec<WriteOp>,
    max_size: usize,
}

pub enum WriteOp {
    Insert { table: String, row: Vec<Value> },
    Update { table: String, key: Vec<Value>, new_row: Vec<Value> },
    Delete { table: String, key: Vec<Value> },
}
```

**优点**:
- 符合 v3.8.0 ARCHITECTURE.md 原则 (P1/P2/P3)
- TransactionManager 统一管理事务生命周期
- LocalExecutor 保持 stateless

**缺点**:
- 需要重构 ExecutionEngine DML 路径

### 3.2 设计方案 B: 独立 WriteBufferService

```
ExecutionEngine
  └── WriteBufferService (Arc<RwLock<WriteBuffer>>)
        ├── stage_write()
        ├── flush_to_storage()
        └── discard()
```

**缺点**: 引入新组件，增加复杂性

### 3.3 设计方案 C: StorageEngine 接口扩展

扩展 `StorageEngine` trait 添加 `stage_write()`:

```rust
trait StorageEngine {
    fn stage_write(&mut self, op: WriteOp) -> SqlResult<()>;
    fn commit_staged(&mut self) -> SqlResult<()>;
    fn rollback_staged(&mut self) -> SqlResult<()>;
    // ... existing methods
}
```

**缺点**: 违反 v3.8.0 原则 — StorageEngine 不应知道事务

---

## 4. TransactionManager 集成方案

### 4.1 目标架构

```
COM_QUERY (mysql-server)
  │
  └── Session::execute()
        │
        ├── BEGIN → txn_manager.begin()
        ├── DML   → txn_manager.stage_write(write_op)
        ├── COMMIT → txn_manager.commit() → flush to StorageEngine
        └── ROLLBACK → txn_manager.rollback() → discard WriteBuffer
```

### 4.2 当前 TransactionManager 接口 (crates/transaction/src/transaction_manager.rs)

```rust
pub struct TransactionManager {
    ssi_detector: SsiDetectorSync,
    active_transactions: HashMap<TxId, ActiveTransaction>,
    next_tx_id: u64,
}

impl TransactionManager {
    pub fn begin_transaction(&mut self, isolation: IsolationLevel) -> Result<TxId, SsiError>
    pub fn record_read(&mut self, tx_id: TxId, key: Vec<u8>) -> Result<(), SsiError>
    pub fn record_write(&mut self, tx_id: TxId, key: Vec<u8>) -> Result<(), SsiError>
    pub fn commit(&mut self, tx_id: TxId) -> Result<(), SsiError>
    pub fn rollback(&mut self, tx_id: TxId) -> Result<(), SsiError>
    pub fn abort(&mut self, tx_id: TxId) -> Result<(), SsiError>
}
```

### 4.3 需要的扩展

```rust
impl TransactionManager {
    // 新增 WriteBuffer 相关
    pub fn stage_write(&mut self, tx_id: TxId, op: WriteOp) -> Result<(), TxError>
    pub fn get_staged_writes(&self, tx_id: TxId) -> Vec<WriteOp>
    pub fn clear_staged_writes(&mut self, tx_id: TxId)
    
    // 新增 snapshot 相关 (for read-your-writes)
    pub fn get_snapshot(&self, tx_id: TxId) -> Option<Snapshot>
    pub fn create_snapshot(&mut self, tx_id: TxId) -> Result<Snapshot, SsiError>
}
```

### 4.4 ActiveTransaction 扩展

```rust
pub struct ActiveTransaction {
    pub tx_id: TxId,
    pub snapshot: Snapshot,
    pub state: TransactionState,
    pub read_keys: Vec<Vec<u8>>,
    pub write_keys: Vec<Vec<u8>>,
    pub write_buffer: Vec<WriteOp>,  // 新增
    pub snapshot_created_at: u64,   // 新增: for read-your-writes
}
```

---

## 5. Rollback 语义保证

### 5.1 RC-F4 需求

> ROLLBACK discards WriteBuffer (no storage side effects)

### 5.2 实现策略

**当前问题**: DML 直接调用 `storage.delete()/insert()` 会产生存储副作用，Rollback 无法撤销。

**解决方案**:

1. **DML staging**: 所有 DML 操作先写入 `write_buffer`，不直接操作 StorageEngine
2. **Rollback clear**: `rollback()` 时清空 `write_buffer`，StorageEngine 数据不变
3. **Commit flush**: `commit()` 时将 `write_buffer` 批量写入 StorageEngine

### 5.3 Rollback 流程

```
ROLLBACK
  │
  ├── txn_manager.rollback(tx_id)
  │     │
  │     ├── active_txn.state = Aborted
  │     ├── clear write_buffer (tx_id)   // 丢弃所有 staged writes
  │     └── ssi_detector.release(tx_id)
  │
  └── StorageEngine 无变化 (WAL log_rollback 仅为恢复用)
```

### 5.4 存储副作用防止

| 操作 | 存储副作用 | 防止方式 |
|------|-----------|---------|
| INSERT | 写入数据页 | commit 前不调用 storage.insert() |
| UPDATE | 修改数据页 | commit 前不调用 storage.update() |
| DELETE | 删除数据页 | commit 前不调用 storage.delete() |

---

## 6. 实施步骤

### Phase 1: WriteBuffer 定义 (RC-F2 前提)

```
步骤 1.1: 在 crates/transaction/src/ 新建 write_buffer.rs
步骤 1.2: 定义 WriteOp enum (Insert/Update/Delete)
步骤 1.3: 实现 WriteBuffer struct with staging Vec<WriteOp>
步骤 1.4: 添加 stage(), clear(), flush(), is_empty() 方法
```

### Phase 2: TransactionManager 集成 WriteBuffer

```
步骤 2.1: 在 ActiveTransaction 添加 write_buffer 字段
步骤 2.2: TransactionManager 实现 stage_write(tx_id, op)
步骤 2.3: TransactionManager 实现 get_staged_writes(tx_id)
步骤 2.4: TransactionManager 实现 clear_staged_writes(tx_id)
```

### Phase 3: ExecutionEngine DML 重构 (RC-F2 核心)

```
步骤 3.1: ExecutionEngine 添加 &TransactionManager 引用
步骤 3.2: 重构 execute_insert() → stage_write(Insert)
步骤 3.3: 重构 execute_update() → stage_write(Update)
步骤 3.4: 重构 execute_delete() → stage_write(Delete)
步骤 3.5: 保留触发器逻辑在 staging 阶段执行
```

### Phase 4: COMMIT flush (RC-F3)

```
步骤 4.1: TransactionManager 实现 commit() 扩展
步骤 4.2: commit() 时调用 flush_to_storage()
步骤 4.3: flush_to_storage() 批量写入 StorageEngine
步骤 4.4: 写入后清空 write_buffer
```

### Phase 5: ROLLBACK discard (RC-F4)

```
步骤 5.1: TransactionManager 实现 rollback() 扩展
步骤 5.2: rollback() 时调用 clear_staged_writes()
步骤 5.3: 确认无 StorageEngine 副作用
```

### Phase 6: 集成测试

```
步骤 6.1: BEGIN → INSERT → ROLLBACK → SELECT (验证无数据)
步骤 6.2: BEGIN → INSERT → COMMIT → SELECT (验证有数据)
步骤 6.3: 并发事务 SSI 检测回归测试
步骤 6.4: TPC-H SF=1 回归测试
```

---

## 7. 风险分析

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| DML 重构破坏现有逻辑 | 中 | 高 | 隔离修改 Extensive 集成测试 |
| WriteBuffer 内存增长 | 低 | 中 | 配置 max_size; OOM 保护 |
| COMMIT flush 失败处理 | 中 | 高 | 保留 WAL; 两阶段提交考虑 |
| 触发器执行时机变化 | 中 | 中 | 确保触发器在 staging 时执行 |

---

## 8. 相关文档

- `docs/releases/v3.8.0/ARCHITECTURE.md` — Server-Level Transaction Model
- `docs/releases/v3.8.0/ROADMAP.md` — v3.8.0 路线图
- `crates/transaction/src/transaction_manager.rs` — 现有 TransactionManager
- `crates/storage/src/wal_storage.rs` — WAL 包装参考
- `crates/executor/src/transactional_executor.rs` — 事务执行器参考实现

---

## 9. 结论

PR-840 DML Transaction Interception 需要:

1. **引入 WriteBuffer** 在 TransactionManager 层
2. **重构 ExecutionEngine DML 路径** 从直接写入改为 staging
3. **扩展 commit()/rollback()** 支持 flush/discard WriteBuffer
4. **遵循 v3.8.0 架构原则** — TransactionManager 持有事务状态，LocalExecutor 保持 stateless

推荐采用**设计方案 A**，与 v3.8.0 ARCHITECTURE.md 保持一致。