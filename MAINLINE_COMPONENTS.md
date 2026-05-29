# Mainline Components

> v3.7.0 Core Integrity Release - 主路径白名单
> 状态：production = 主路径使用 | migrating = 收敛中 | isolated = 孤岛

## 规则

**只有标记为 `production` 或 `migrating` 的 crate 才能被主路径依赖。**

**禁止反向依赖**：migrating 模块不能依赖 isolated 模块。

## 模块状态

### Phase 1: Core Execution (Production)

| Crate | 状态 | 说明 |
|-------|------|------|
| `parser` | production | SQL 解析 |
| `planner` | production | 逻辑计划 |
| `optimizer` | production | 物理优化 |
| `executor` | production | 查询执行 |
| `types` | production | 类型系统 |

### Phase 2: Transaction & Storage (Production)

| Crate | 状态 | 说明 |
|-------|------|------|
| `transaction` | production | 事务管理 (MVCC, Lock, Coordinator) |
| `storage` | production | 存储引擎 (BufferPool, FileStorage, WalStorage) |
| `catalog` | production | 元数据 |

### Phase 3: Network (Production)

| Crate | 状态 | 说明 |
|-------|------|------|
| `network` | production | TCP server, MySQL protocol |

### Phase 4: Utilities (Production)

| Crate | 状态 | 说明 |
|-------|------|------|
| `common` | production | 公共工具 |
| `information-schema` | production | INFORMATION_SCHEMA |

## 迁移模块 (Migrating)

> 这些模块正在收敛到主路径

| Crate | 状态 | 目标 | 相关 Issue |
|-------|------|------|-----------|
| `mysql-server` | migrating | 合并到 network | #2591, #2605 |
| `expr` | migrating | 合并表达式系统 | #2590, #2604 |

## 孤岛模块 (Isolated)

> 这些模块**未被主路径使用**，需要收敛或删除

| Crate | 问题 | 建议 |
|-------|------|------|
| `parallel_executor` | 63KB，但无主路径调用 | TASK-R2-2: 删除或合并 |
| `parallel_vector_executor` | 无主路径调用 | TASK-R2-2: 删除 |
| `local_executor_dml` | **PLACEHOLDER** - 未实现 | TASK-R1-2: 必须实现 |
| `expression` | 与 expr 重叠 | TASK-R3-1: 删除 |
| `qmd-bridge` | 不确定状态 | 评估 |
| `distributed` | 分布式实验 | **FROZEN** - P2 暂停 |
| `graph` | 图存储孤岛 | **FROZEN** - P2 暂停 |
| `vector` | 向量存储孤岛 | **FROZEN** - P2 暂停 |

## 已废弃 (Deprecated)

> 等待删除的模块

| Crate | 替代 | 删除版本 |
|-------|------|---------|
| `expr-legacy` | expr | v3.8.0 |

## 实验性 (Experimental)

> Feature-gated 实验功能

| Crate | Feature Flag | 说明 |
|-------|-------------|------|
| `vec_simd` | simd | SIMD 优化，v3.7.0 禁止新功能 |

## 生产支持工具 (Production Support)

> 不是主执行路径，但支持生产

| Crate | 说明 |
|-------|------|
| `wal-verification` | Verification tooling，不是主路径 |

## 主路径依赖图

```
mysql-client
    ↓
network (MySQL protocol handler)
    ↓
parser (SQL → AST)
    ↓
planner (AST → LogicalPlan)
    ↓
optimizer (LogicalPlan → PhysicalPlan)
    ↓
executor (PhysicalPlan → Result)
    ↓
transaction (TxnContext, TxnManager)
    ↓
storage (WalStorage, BufferPool, FileStorage)
```

## 唯一执行路径

```rust
// 主路径必须是:
QueryContext
  -> TransactionManager
      -> WalTransactionalExecutor
          -> StorageEngine
```

**禁止**:
```rust
// 禁止直接调用 storage.insert() 而不经过事务
storage.insert(&table, records)?;  // ❌

// 禁止使用 LocalExecutor 而不经过 TransactionManager
LocalExecutor::new().execute(...)  // ❌
```

## 验证

```bash
# 检查没有孤岛模块被主路径使用
cargo tree -p sqlrustgo-executor -i isolated_crate  # 应该无输出
```

## 相关 Issue

- #2599: v3.7.0 Master Issue
- #2601: Architecture Governance Issue
- #2603: R2 - 执行引擎统一