# PR-800: TransactionalFacade - 单一写入控制面

## 目标

引入 `TransactionalFacade` trait 作为系统的**唯一写入控制面（Single Write Authority）**，统一事务生命周期管理，消除当前多入口并存的控制权分裂问题。

## 背景问题

### 当前状态：控制权分裂

```
SqlExecutor         ← 多入口①
ExecutionEngine    ← 多入口②  
LocalExecutor      ← 多入口③
TransactionalExecutor ← 多入口④
WalTransactionalExecutor  ← 多入口⑤
```

**问题**：
- 任意入口可绕过事务边界
- WAL ordering 不可预测
- DriftDetector 只能事后检测，无法阻断

### 目标状态：单一控制面

```
SQL / CLI / API
        ↓
TransactionalFacade  ← 唯一写入入口
        ↓
ExecutionEngine (纯内核，无事务状态)
        ↓
Storage + WAL
```

---

## 设计

### 1. TransactionalFacade trait

```rust
use sqlrustgo_transaction::{TxId, TransactionContext};
use sqlrustgo_types::SqlResult;
use crate::execution::result::ExecutionResult;

/// 单一写入控制面 trait
/// 所有写操作必须经过此 trait，不允许存在其他写入口
pub trait TransactionalFacade: Send + Sync {
    // === 事务生命周期 ===
    
    /// 开始事务
    fn begin(&self) -> SqlResult<TxId>;
    
    /// 提交事务
    fn commit(&self) -> SqlResult<Option<u64>>;
    
    /// 回滚事务
    fn rollback(&self) -> SqlResult<()>;
    
    /// 检查是否在事务中
    fn is_in_transaction(&self) -> bool;
    
    /// 获取当前事务 ID
    fn current_tx_id(&self) -> Option<TxId>;
    
    // === 写操作（强制经过事务上下文） ===
    
    /// 执行写操作（INSERT/UPDATE/DELETE）
    /// 必须带 TxContext，不允许隐式事务
    fn execute_write(
        &self, 
        ctx: &TransactionContext, 
        op: WriteOp
    ) -> SqlResult<ExecutionResult>;
    
    /// 执行读操作（SELECT）
    fn execute_read(
        &self, 
        sql: &str
    ) -> SqlResult<ExecutionResult>;
    
    // === Drift Gate ===
    
    /// 验证操作是否违反事务约束
    fn validate_operation(&self, op: &WriteOp) -> Result<(), DriftViolation>;
}
```

### 2. WriteOp 枚举

```rust
/// 写操作类型
pub enum WriteOp {
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<sqlrustgo_types::Value>>,
    },
    Update {
        table: String,
        set: Vec<(String, sqlrustgo_types::Value)>,
        filter: String,
    },
    Delete {
        table: String,
        filter: String,
    },
}

impl WriteOp {
    pub fn table_name(&self) -> &str;
    pub fn operation_type(&self) -> &'static str;
}
```

### 3. DriftViolation 类型

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftViolation {
    pub violation_id: String,
    pub trace_id: String,
    pub violation_type: DriftViolationType,
    pub severity: DriftSeverity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriftViolationType {
    WalDrift,      // WAL 顺序违规
    TxnDrift,      // 事务边界违规
    GraphDrift,    // 图约束违规
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriftSeverity {
    Critical,  // 阻断级
    Medium,    // 降级级
    Low,       // 警告级
}
```

---

## 参考实现：WalTransactionalFacade

### 4.1 结构定义

```rust
use parking_lot::RwLock;
use sqlrustgo_storage::{StorageEngine, WalStorage};
use sqlrustgo_transaction::{TransactionManager, TxId, TransactionContext};
use std::sync::Arc;
use std::path::PathBuf;

/// WAL 支持的 TransactionalFacade 参考实现
/// 提供完整的事务管理和 WAL 日志
pub struct WalTransactionalFacade<S: StorageEngine> {
    storage: Arc<RwLock<WalStorage<S>>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
    drift_gate: DriftGate,
}
```

### 4.2 核心方法实现

```rust
impl<S: StorageEngine> WalTransactionalFacade<S> {
    
    pub fn new(inner: S, wal_path: PathBuf) -> SqlResult<Self> {
        let wal_storage = WalStorage::new(inner, wal_path)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        Ok(Self {
            storage: Arc::new(RwLock::new(wal_storage)),
            tx_manager: Arc::new(RwLock::new(TransactionManager::new())),
            drift_gate: DriftGate::new(),
        })
    }
    
    pub fn begin(&self) -> SqlResult<TxId> {
        // 1. 开启存储层事务
        let mut storage = self.storage.write();
        storage.begin_transaction()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 2. 开启事务管理器
        let tx_id = self.tx_manager.write().begin()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 3. 记录 WAL Begin
        storage.log_begin(tx_id)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        Ok(tx_id)
    }
    
    pub fn commit(&self) -> SqlResult<Option<u64>> {
        // 1. 验证无 pending drift
        let ctx = self.tx_manager.read().get_transaction_context()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 2. DriftGate pre-check
        if let Err(violation) = self.drift_gate.validate_pre_commit(&ctx) {
            return Err(SqlError::ExecutionError(
                format!("Drift violation blocked commit: {}", violation.description)
            ));
        }
        
        // 3. 提交存储层
        let mut storage = self.storage.write();
        storage.commit_transaction()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 4. 记录 WAL Commit
        storage.log_commit(ctx.tx_id)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 5. 提交事务管理器
        self.tx_manager.write().commit()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))
    }
    
    pub fn rollback(&self) -> SqlResult<()> {
        let mut storage = self.storage.write();
        storage.rollback_transaction()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        self.tx_manager.write().rollback()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))
    }
    
    pub fn execute_write(
        &self, 
        ctx: &TransactionContext, 
        op: WriteOp
    ) -> SqlResult<ExecutionResult> {
        // 1. Pre-check: DriftGate 验证
        if let Err(violation) = self.drift_gate.validate(&op, ctx) {
            return Err(SqlError::ExecutionError(
                format!("Drift violation: {}", violation.description)
            ));
        }
        
        // 2. 获取当前事务上下文
        let current_ctx = self.tx_manager.read()
            .get_transaction_context()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 3. 确保操作在事务上下文中
        if current_ctx.tx_id != ctx.tx_id {
            return Err(SqlError::ExecutionError(
                "Write operation must use current transaction context".to_string()
            ));
        }
        
        // 4. 执行写操作并记录 WAL
        let mut storage = self.storage.write();
        let affected = match &op {
            WriteOp::Insert { table, columns, values } => {
                storage.insert(table, columns, values)
            }
            WriteOp::Update { table, set, filter } => {
                storage.update(table, set, filter)
            }
            WriteOp::Delete { table, filter } => {
                storage.delete(table, filter)
            }
        }.map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        // 5. 记录 WAL 日志
        storage.log_mutation(ctx.tx_id, &op)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        
        Ok(ExecutionResult::new(vec![], affected))
    }
}
```

---

## DriftGate：事前阻断层

### 5.1 概念转变

| 旧角色 | 新角色 |
|--------|--------|
| DriftDetector | observability（事后检测） |
| DriftGate | enforcement（事前阻断） |

### 5.2 DriftGate 结构

```rust
pub struct DriftGate {
    trace_id: String,
    policy: GuardPolicy,
}

impl DriftGate {
    
    /// 执行前验证（返回 Result，而非收集后报告）
    pub fn validate(
        &self, 
        op: &WriteOp, 
        ctx: &TransactionContext
    ) -> Result<(), DriftViolation> {
        // 检查 WAL 顺序
        self.check_wal_ordering(op, ctx)?;
        
        // 检查事务边界
        self.check_txn_boundary(op, ctx)?;
        
        Ok(())
    }
    
    /// 提交前验证
    pub fn validate_pre_commit(
        &self, 
        ctx: &TransactionContext
    ) -> Result<(), DriftViolation> {
        // 确保所有 mutation 都有对应 WAL 记录
        self.check_pending_mutations(ctx)
    }
    
    fn check_wal_ordering(
        &self, 
        op: &WriteOp, 
        ctx: &TransactionContext
    ) -> Result<(), DriftViolation> {
        // WAL begin 必须在 mutation 之前
        if !ctx.wal_segment_open {
            return Err(DriftViolation::new(
                DriftViolationType::WalDrift,
                DriftSeverity::Critical,
                "Mutation without open WAL segment".to_string(),
            ));
        }
        Ok(())
    }
    
    fn check_txn_boundary(
        &self, 
        op: &WriteOp, 
        ctx: &TransactionContext
    ) -> Result<(), DriftViolation> {
        // 操作必须在活跃事务内
        if !ctx.is_active {
            return Err(DriftViolation::new(
                DriftViolationType::TxnDrift,
                DriftSeverity::Critical,
                "Mutation outside active transaction".to_string(),
            ));
        }
        Ok(())
    }
}
```

---

## 调用关系图

### 6.1 完整调用流

```
SQL: INSERT INTO orders VALUES (1, 100, '2026-01-01')
                    ↓
          SqlExecutor.execute_read()
                    ↓ (路由)
      TransactionalFacade.execute_write()
                    ↓
            [DriftGate.validate()]
                    ↓ (passed)
          ExecutionEngine.execute()
                    ↓
              Storage + WAL
```

### 6.2 Ownership Graph

```
┌─────────────────────────────────────────┐
│        TransactionalFacade              │
│  ┌─────────────────────────────────┐   │
│  │ WalTransactionalFacade          │   │
│  │  - storage: WalStorage          │   │
│  │  - tx_manager: TransactionManager│   │
│  │  - drift_gate: DriftGate        │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
                    │
                    ↓ calls
┌─────────────────────────────────────────┐
│       ExecutionEngine (PURE)            │
│  - NO begin/commit/rollback              │
│  - NO transaction state                │
│  - execute(PhysicalOp) only            │
└─────────────────────────────────────────┘
                    │
                    ↓ calls
┌─────────────────────────────────────────┐
│       StorageEngine + WAL              │
│  - StorageEngine trait                 │
│  - WalStorage wrapper                  │
└─────────────────────────────────────────┘
```

---

## Invariants（强制不变量）

### 7.1 写入路径不变量

```
I1: 所有写操作必须经过 TransactionalFacade.execute_write()
I2: execute_write() 必须携带有效的 TransactionContext
I3: TransactionContext.txn_id 必须匹配当前活跃事务
```

### 7.2 ExecutionEngine 不变量

```
I4: ExecutionEngine 不得暴露 begin/commit/rollback 方法
I5: ExecutionEngine 不得持有事务状态
I6: ExecutionEngine.execute() 必须是纯函数（无副作用）
```

### 7.3 WAL 不变量

```
I7: WAL begin 记录必须在第一个 mutation 之前
I8: WAL commit 记录必须在事务提交之前
I9: WAL mutation 记录必须在对应 Storage mutation 之前
```

---

## PR-801 前置依赖

### 8.1 ExecutionEngine 去事务化

**当前 ExecutionEngine trait：**
```rust
pub trait ExecutionEngine {
    fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError>;
    fn begin(&mut self) -> Result<u64, SqlError>;      // ← 删除
    fn commit(&mut self, txn: u64) -> Result<(), SqlError>; // ← 删除
    fn rollback(&mut self, txn: u64) -> Result<(), SqlError>; // ← 删除
}
```

**目标 ExecutionEngine trait：**
```rust
pub trait ExecutionEngine {
    fn execute(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult>;
    fn name(&self) -> &'static str;
    fn is_ready(&self) -> bool;
}
```

### 8.2 删除清单

| 要删除的项 | 位置 | 原因 |
|-----------|------|------|
| `ExecutionEngine::begin()` | engine.rs | facade 层负责 |
| `ExecutionEngine::commit()` | engine.rs | facade 层负责 |
| `ExecutionEngine::rollback()` | engine.rs | facade 层负责 |
| `QueryContext::txn_id` | context.rs | 由 TransactionContext 替代 |
| `QueryContext::requires_txn()` | context.rs | facade 层强制检查 |

---

## 影响范围扫描

### 9.1 需要修改的文件

```
crates/executor/src/
├── execution/
│   ├── engine.rs         [修改] - 删除 tx 方法
│   ├── facade.rs         [新增] - TransactionalFacade trait
│   ├── drift_gate.rs     [新增] - DriftGate 实现
│   └── context.rs        [修改] - 简化 QueryContext
├── transactional_executor.rs [修改] - 实现 TransactionalFacade
├── sql_executor.rs       [修改] - 降级为 read-only
├── local_executor.rs    [修改] - 移除 tx_manager
└── lib.rs                [修改] - 导出新 trait

crates/server/src/
├── openclaw_endpoints.rs [修改] - 使用 facade
└── teaching_endpoints.rs [修改] - 使用 facade

crates/sql-cli/src/
└── main.rs               [修改] - 使用 facade

crates/mysql-server/src/
└── lib.rs               [修改] - 使用 facade
```

### 9.2 不需要修改的文件（保持不变）

```
crates/transaction/       # 事务管理器（保持独立）
crates/storage/           # 存储引擎（facade 调用）
crates/bench/             # 基准测试（通过 facade）
```

---

## 测试策略

### 10.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_write_without_tx_fails() {
        let facade = create_facade();
        let op = WriteOp::Insert { /* ... */ };
        let ctx = TransactionContext::dummy(); // 无效 txn_id
        
        let result = facade.execute_write(&ctx, op);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_mutation_without_wal_fails() {
        // 测试 I7: WAL begin 必须在 mutation 之前
    }
    
    #[test]
    fn test_commit_with_violation_fails() {
        // 测试 DriftGate pre-commit check
    }
}
```

### 10.2 集成测试

```rust
#[test]
fn test_full_transaction_flow() {
    let facade = create_facade();
    
    // begin → insert → commit
    let tx_id = facade.begin().unwrap();
    let ctx = facade.get_transaction_context().unwrap();
    
    facade.execute_write(&ctx, insert_op).unwrap();
    facade.commit().unwrap();
}
```

---

## 风险与缓解

| 风险 | 级别 | 缓解措施 |
|------|------|----------|
| 现有代码大量修改 | 高 | 分 PR 执行，先 PR-800 再 PR-801 |
| 向后兼容断裂 | 中 | SqlExecutor 保留为 read-only 接口 |
| DriftGate 过度阻断 | 中 | 提供 bypass flag 用于迁移期 |

---

## 实施顺序

```
1. PR-800: TransactionalFacade trait + WalTransactionalFacade 实现
          ↓
2. PR-801: ExecutionEngine 去事务化
          ↓
3. PR-802: DriftDetector → DriftGate 升级
          ↓
4. PR-803: 所有入口（sql-cli, mysql-server, bench）迁移到 facade
```

---

## 结论

> **TransactionalFacade is the only write authority.
> Everything else is execution detail.**