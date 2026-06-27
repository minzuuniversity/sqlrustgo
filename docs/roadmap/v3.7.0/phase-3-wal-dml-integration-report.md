# Phase 3 WAL DML 集成技术分析报告

> 日期: 2026-05-30
> 版本: v3.6.0
> 作者: Sisyphus AI Agent

## 执行摘要

本报告记录对 SQLRustGo Phase 3（WAL DML 集成）的技术分析，为 3.7.0 版本综合修复提供依据。

## 一、现有基础设施

### 1.1 WAL 模块 (`crates/storage/src/`)

| 文件 | 职责 |
|------|------|
| `wal.rs` | WAL 条目定义、序列化、LSN 管理 |
| `wal_storage.rs` | WAL 封装器，为 StorageEngine 添加 WAL 支持 |

**WalEntry 结构：**
```rust
pub struct WalEntry {
    pub tx_id: u64,           // 事务 ID
    pub entry_type: WalEntryType, // Begin/Insert/Update/Delete/Commit/Rollback
    pub table_id: u64,        // 表 ID
    pub key: Option<Vec<u8>>, // 行键
    pub data: Option<Vec<u8>>, // 行数据
    pub lsn: u64,             // 日志序列号
    pub timestamp: u64,        // 时间戳
}
```

**WalEntryType 枚举：**
```rust
pub enum WalEntryType {
    Begin = 1,
    Insert = 2,
    Update = 3,
    Delete = 4,
    Commit = 5,
    Rollback = 6,
    Checkpoint = 7,
    Prepare = 8,
}
```

### 1.2 WalStorage 实现

`WalStorage<S: StorageEngine>` 为存储引擎添加 WAL 支持：

```rust
pub fn begin_transaction(&mut self) -> SqlResult<u64> {
    if self.current_tx_id != 0 {
        return Err(...);  // 防止嵌套事务
    }
    let tx_id = self.generate_tx_id();
    if self.wal_enabled {
        self.wal.log_begin(tx_id)?;  // 记录 Begin
    }
    self.current_tx_id = tx_id;
    Ok(tx_id)
}

pub fn commit_transaction(&mut self) -> SqlResult<()> {
    let tx_id = self.current_tx_id;
    if self.wal_enabled {
        self.wal.log_commit(tx_id)?;  // 记录 Commit
        self.wal.sync()?;             // 强制刷盘
    }
    self.current_tx_id = 0;
    Ok(())
}
```

**DML WAL 日志记录：**
```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    let table_id = Self::table_name_to_id(table);
    for record in &records {
        let key = Self::record_key(record);
        let data = Self::record_to_bytes(record);
        self.log_insert(table_id, key, data)?;  // WAL 记录
    }
    self.inner.insert(table, records)  // 执行实际插入
}

fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
    let table_id = Self::table_name_to_id(table);
    let key = format!("{:?}", filters).into_bytes();
    self.log_delete(table_id, key)?;  // WAL 记录
    self.inner.delete(table, filters)
}

fn update(&mut self, table: &str, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize> {
    let table_id = Self::table_name_to_id(table);
    let key = format!("{:?}", filters).into_bytes();
    let data = format!("{:?}", updates).into_bytes();
    self.log_update(table_id, key, data)?;  // WAL 记录
    self.inner.update(table, filters, updates)
}
```

### 1.3 事务模块 (`crates/transaction/src/`)

| 文件 | 职责 |
|------|------|
| `transaction_manager.rs` | 事务生命周期管理 |
| `mvcc.rs` | MVCC 快照和版本链 |
| `ssi.rs` | 可串行化快照隔离冲突检测 |

**TransactionManager 核心 API：**
```rust
pub fn begin_transaction(&mut self, isolation: IsolationLevel) -> Result<TxId, SsiError>
pub fn commit_transaction(&mut self) -> Result<Option<u64>, TransactionError>
pub fn rollback_transaction(&mut self) -> Result<(), TransactionError>
```

### 1.4 执行器模块 (`crates/executor/src/`)

| 文件 | 状态 | 说明 |
|------|------|------|
| `executor.rs` | ✅ | Executor trait 定义 |
| `local_executor.rs` | ⚠️ | 存在但缺少依赖无法编译 |
| `transactional_executor.rs` | ⚠️ | 存在但缺少依赖无法编译 |
| `local_executor_dml.rs` | ⚠️ | 占位符 |

### 1.5 Planner 模块 (`crates/planner/src/`)

DML 执行器定义：

```rust
pub struct DeleteExec {
    table_name: String,
    predicate: Option<Expr>,
    schema: Schema,
}
// 注意: InsertExec, UpdateExec 在 physical_plan.rs 中未找到完整实现
```

## 二、核心问题

### 2.1 孤儿模块问题

以下模块存在于代码库但无法编译：

```
orphan modules (缺少 workspace 依赖):
├── transactional_executor.rs
│   需要: sqlrustgo_transaction, parking_lot, WalStorage
├── local_executor.rs
│   需要: parking_lot, query_stats, operator_profile, sql_normalizer
└── local_executor_dml.rs
    仅占位符，无实际实现
```

### 2.2 DML 实现差距

| DML 操作 | Planner 定义 | Executor 实现 | WAL 日志 |
|---------|-------------|--------------|---------|
| INSERT | ⚠️ 缺失 | ⚠️ 未实现 | ✅ WalStorage 有 log_insert |
| UPDATE | ⚠️ 缺失 | ⚠️ 未实现 | ✅ WalStorage 有 log_update |
| DELETE | ✅ DeleteExec | ✅ execute_delete | ❌ 未调用 WAL |

**当前 execute_delete 实现：**
```rust
fn execute_delete(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
    // 直接调用 storage.delete()，无 WAL 记录
    let deleted = self.storage.delete(table_name, &[])?;
    Ok(ExecutorResult::new(vec![], deleted))
}
```

### 2.3 两层事务边界

当前存在**两套独立的事务管理**：

1. **WalStorage 事务边界** (存储层)
   - `begin_transaction()` → 记录 WAL Begin
   - `commit_transaction()` → 记录 WAL Commit + sync
   - `rollback_transaction()` → 记录 WAL Rollback

2. **TransactionManager 事务边界** (执行层)
   - MVCC 快照管理
   - SSI 冲突检测
   - 读写键跟踪

**问题**：两层事务边界**未协调**，可能导致：
- WAL 记录了但 MVCC 未提交
- MVCC 回滚了但 WAL 已刷盘

## 三、架构设计建议

### 3.1 WalLocalExecutor 结构

建议新增 `WalLocalExecutor`，整合存储层 WAL 和执行层事务管理：

```rust
pub struct WalLocalExecutor {
    storage: Arc<RwLock<WalStorage<FileStorage>>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
    cache: Arc<RwLock<QueryCache>>,
}

impl Executor for WalLocalExecutor {
    fn execute(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
        match plan.name() {
            "Insert" => self.execute_insert(plan),
            "Update" => self.execute_update(plan),
            "Delete" => self.execute_delete(plan),
            _ => self.execute_query(plan),  // 复用 LocalExecutor 查询逻辑
        }
    }
}
```

### 3.2 DML 事务流程

```rust
impl WalLocalExecutor {
    fn execute_insert(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
        // 1. 自动开启事务（如果未开启）
        if !self.is_in_transaction() {
            self.begin()?;
        }
        
        // 2. 执行插入（WAL 记录由 WalStorage 自动完成）
        let affected = self.storage.write().insert(table_name, records)?;
        
        // 3. 自动提交（单语句事务）
        self.commit()?;
        
        Ok(ExecutorResult::new(vec![], affected))
    }
}
```

### 3.3 两层事务协调

```rust
pub struct TransactionCoordinator {
    wal_storage: Arc<RwLock<WalStorage<FileStorage>>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

impl TransactionCoordinator {
    pub fn begin(&mut self) -> Result<TxId, TransactionError> {
        // 同时开启 WAL 和 MVCC 事务
        self.wal_storage.write().begin_transaction()?;
        self.tx_manager.begin()
    }
    
    pub fn commit(&mut self) -> Result<Option<u64>, TransactionError> {
        // 先提交 MVCC，再提交 WAL
        let commit_ts = self.tx_manager.commit()?;
        self.wal_storage.write().commit_transaction()?;
        Ok(commit_ts)
    }
    
    pub fn rollback(&mut self) -> Result<(), TransactionError> {
        // 先回滚 WAL，再回滚 MVCC
        self.wal_storage.write().rollback_transaction()?;
        self.tx_manager.rollback()
    }
}
```

## 四、关键文件清单

### 4.1 需要修改的文件

```
crates/
├── executor/
│   ├── Cargo.toml          # 添加缺失依赖
│   └── src/
│       ├── lib.rs          # 导出 WalLocalExecutor
│       ├── transactional_executor.rs  # 修复编译问题
│       └── local_executor.rs         # 修复编译问题
└── planner/
    └── src/
        └── physical_plan.rs  # 添加 InsertExec, UpdateExec
```

### 4.2 需要新增的文件

```
crates/
└── executor/
    └── src/
        └── wal_local_executor.rs  # 新增 WalLocalExecutor
```

### 4.3 依赖添加到 workspace

```toml
# crates/executor/Cargo.toml
[dependencies]
parking_lot = { workspace = true }
sqlrustgo-transaction = { workspace = true }
tempfile = { workspace = true }

# 如果需要慢查询日志:
# query_stats = "0.1"
```

## 五、实现步骤（3.7.0）

### Phase 1: 依赖修复
1. 清理 `local_executor.rs` 中不存在的导入
2. 修复 `transactional_executor.rs` 编译错误
3. 添加必要依赖到 `executor/Cargo.toml`

### Phase 2: DML Planner 定义
1. 实现 `InsertExec` in `physical_plan.rs`
2. 实现 `UpdateExec` in `physical_plan.rs`
3. 实现对应的 `PhysicalPlan` trait

### Phase 3: WalLocalExecutor
1. 创建 `wal_local_executor.rs`
2. 实现 `execute_insert/update/delete`
3. 添加事务边界自动管理

### Phase 4: 测试验证
1. WAL 恢复测试
2. 事务提交/回滚测试
3. 崩溃恢复测试

## 六、风险评估

| 风险 | 级别 | 缓解措施 |
|------|------|---------|
| 修改 workspace 依赖影响其他 crate | 高 | 使用 `--all-features` 分阶段测试 |
| 两层事务边界不一致 | 高 | 实现 `TransactionCoordinator` 统一管理 |
| 孤儿模块是废弃代码 | 中 | 与团队确认后清理 |
| DML 并发执行性能 | 中 | 添加适当锁策略 |

## 七、结论

Phase 3 WAL DML 集成需要：

1. **依赖修复** - 让 `transactional_executor.rs` 可编译
2. **DML 定义** - 在 planner 中添加 `InsertExec`, `UpdateExec`
3. **Executor 实现** - 创建 `WalLocalExecutor` 整合 WAL + 事务
4. **架构统一** - 通过 `TransactionCoordinator` 协调两层事务边界

建议在 **3.7.0 版本**综合处理此问题。

---

## 相关 Issue

- Issue #2587: [Phase 3] WAL DML 集成研究报告 - 3.7.0 综合修复建议
- PR #2586: Phase 2 - WAL 基础设施

## 附录：关键代码片段

### A. WalStorage DML 实现

```rust
// crates/storage/src/wal_storage.rs

fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    let table_id = Self::table_name_to_id(table);
    for record in &records {
        let key = Self::record_key(record);
        let data = Self::record_to_bytes(record);
        self.log_insert(table_id, key, data)?;  // WAL 记录已插入
    }
    self.inner.insert(table, records)
}

fn log_insert(&mut self, table_id: u64, key: Vec<u8>, data: Vec<u8>) -> SqlResult<()> {
    let entry = WalEntry {
        tx_id: self.current_tx_id,
        entry_type: WalEntryType::Insert,
        table_id,
        key: Some(key),
        data: Some(data),
        lsn: self.wal.next_lsn(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    self.wal.write_entry(&entry)
}
```

### B. LocalExecutor execute_delete

```rust
// crates/executor/src/local_executor.rs (line 1036)

fn execute_delete(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
    use sqlrustgo_planner::DeleteExec;
    
    let delete_exec = plan.as_any().downcast_ref::<DeleteExec>();
    
    match delete_exec {
        Some(delete_plan) => {
            let table_name = delete_plan.table_name();
            
            // 无 WAL，无事务边界
            let deleted = self.storage.delete(table_name, &[])?;
            Ok(ExecutorResult::new(vec![], deleted))
        }
        None => Ok(ExecutorResult::empty()),
    }
}
```

### C. TransactionalExecutor 事务边界

```rust
// crates/executor/src/transactional_executor.rs (line 105-127)

pub fn begin(&self) -> Result<TxId, TransactionError> {
    let mut storage = self.storage.write();
    storage.begin_transaction()?;  // WAL begin
    self.tx_manager.write().begin()  // MVCC begin
}

pub fn commit(&self) -> Result<Option<u64>, TransactionError> {
    let mut storage = self.storage.write();
    storage.commit_transaction()?;  // WAL commit
    self.tx_manager.write().commit()  // MVCC commit
}

pub fn rollback(&self) -> Result<(), TransactionError> {
    let mut storage = self.storage.write();
    storage.rollback_transaction()?;  // WAL rollback
    self.tx_manager.write().rollback()  // MVCC rollback
}
```