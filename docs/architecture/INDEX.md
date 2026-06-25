# Architecture 文档索引

> **版本**: v3.9.0
> **最后更新**: 2026-06-17

---

## 一、文档目录

| 文档 | 说明 |
|------|------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | SQLRustGo 架构设计文档 - 核心模块详解 |
| [ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md) | 架构概览 - 系统总体架构图 |
| [ARCHITECTURE_EVOLUTION.md](ARCHITECTURE_EVOLUTION.md) | 架构演进历史 |
| [sqlrustgo_architecture.md](sqlrustgo_architecture.md) | SQLRustGo 架构详解 |

---

## 二、核心架构

| 文档 | 说明 |
|------|------|
| [EXECUTION_PATH.md](EXECUTION_PATH.md) | **唯一可信执行路径** - SQL → AST → Plan → Execution |
| [EXECUTION_ARCHITECTURE.md](EXECUTION_ARCHITECTURE.md) | 执行引擎架构 |
| [DIRECTORY_STRUCTURE.md](DIRECTORY_STRUCTURE.md) | 目录结构说明 |
| [MODULE_LIFECYCLE.md](MODULE_LIFECYCLE.md) | 模块生命周期管理 |

---

## 三、查询优化

| 文档 | 说明 |
|------|------|
| [CASCADES_OPTIMIZER.md](CASCADES_OPTIMIZER.md) | Cascades 查询优化器 |
| [cascades_optimizer_design.md](cascades_optimizer_design.md) | Cascades 优化器设计文档 |

---

## 四、事务与存储

| 文档 | 说明 |
|------|------|
| [TRANSACTION_BOUNDARY.md](TRANSACTION_BOUNDARY.md) | 事务边界设计 |

---

## 五、分布式架构

| 文档 | 说明 |
|------|------|
| [DISTRIBUTED_EXECUTION.md](DISTRIBUTED_EXECUTION.md) | 分布式执行架构 |
| [distributed_scheduler_design.md](distributed_scheduler_design.md) | 分布式调度器设计 |

---

## 六、开发方法论

| 文档 | 说明 |
|------|------|
| [SQL_SEMANTIC_AUDIT.md](SQL_SEMANTIC_AUDIT.md) | SQL 语义审计 |
| [SEMANTIC_DRIVEN_DEVELOPMENT.md](SEMANTIC_DRIVEN_DEVELOPMENT.md) | 语义驱动开发 |
| [EXECUTION_PIPELINE_REFACTORING.md](EXECUTION_PIPELINE_REFACTORING.md) | 执行流水线重构 |

---

## 七、关键架构原则

### 唯一可信执行路径

```
SQL Query
    ↓
Parser (SQL → AST)
    ↓
Planner (AST → LogicalPlan)
    ↓
Optimizer (LogicalPlan → PhysicalPlan)
    ↓
ExecutionEngine (PhysicalPlan → Result)
    ↓
TransactionManager (TxnContext, LockManager, MVCC)
    ↓
WAL (Write-Ahead Log → durability)
    ↓
StorageEngine (BufferPool → FileStorage)
```

### 核心模块

| 模块 | 位置 | 职责 |
|------|------|------|
| Lexer | lexer/ | SQL 词法分析 → Tokens |
| Parser | parser/ | Tokens → AST |
| Planner | planner/ | AST → LogicalPlan |
| Optimizer | optimizer/ | LogicalPlan → PhysicalPlan |
| Executor | executor/ | PhysicalPlan → Result |
| Transaction | transaction/ | WAL, TxManager, MVCC |
| Storage | storage/ | Page, BufferPool, B+ Tree |

---

*本文档由 Hermes Agent 维护*
*更新频率: 架构变更时更新*
