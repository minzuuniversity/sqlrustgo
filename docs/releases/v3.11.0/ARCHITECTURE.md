# SQLRustGo v3.11.0 架构说明

## 概览

SQLRustGo 是一个关系数据库系统，实现 SQL-92 子集，并采用 Volcano 风格执行模型。v3.11.0 的架构重点是把前序版本中的索引、权限、执行器优化、存储可靠性和 MySQL wire protocol 能力推进到主路径。

## 系统架构

```text
Clients: MySQL CLI / JDBC / mysql-client / sqlrustgo-cli
        |
        v
sqlrustgo-mysql-server
        |
        +-- Connection Manager, MySQL Wire Protocol
        +-- Parser: SQL -> AST -> Relational Algebra
        v
sqlrustgo-planner
        |
        +-- Predicate pushdown
        +-- Join reordering
        +-- Projection elimination
        v
sqlrustgo-optimizer
        |
        +-- CBO Cost Estimator
        +-- Hash Semi Join / Hash Anti Join
        +-- Decorrelation
        v
sqlrustgo-executor
        |
        +-- VolcanoExecutor
        +-- Hash Join / Sort Merge Join / Aggregation
        +-- Table Scan / Index Scan
        +-- Storage Engine facade
        v
sqlrustgo-storage
        |
        +-- Page Manager
        +-- FileStorage / MemoryStorage
        +-- B+ Tree Index
        +-- Compression: LZ4 / zstd
        +-- WAL + Double-Write Buffer
```

## v3.11.0 新增或强化的组件

| 组件 | 说明 | 生产边界 |
|---|---|---|
| Clustered Index (V311-01) | 主键数据与索引结构更紧密结合，提升点查能力 | 需要继续补充 crash/recovery 场景 |
| Adaptive Hash Index (V311-02) | 对热点数据访问建立自适应 hash 加速路径 | 需要更多 eviction 和并发行为测试 |
| Change Buffer / Double-Write Buffer | 提升写入路径可靠性，降低页面损坏风险 | v3.12 仍需 fault injection 与 restore 验证 |
| Row-Level Security / Column Privileges | 为 GMP 和多角色访问控制提供基础 | v3.12 需扩展到 vector/graph/RAG 路径 |
| Hash Semi/Anti Join 与 Decorrelation | 改善复杂子查询和 TPC-H 查询执行能力 | TPC-H correctness 仍需跨引擎校验 |
| MySQL Wire Protocol | 支持 MySQL 客户端接入和 E2E 路径 | prepared statement、error packet、TLS/compression 需补强 |

## 架构判断

v3.11.0 的架构已经可以支撑受控 SQL 场景和 GMP 内审检索原型，但不能直接宣称完整替代 MySQL 5.7。面向 v3.12.0，应优先补齐 SQLLogicTest、TPC-H correctness、wire protocol、LOAD DATA、backup/restore、upgrade/downgrade 和 crash recovery 证据。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# SQLRustGo v3.11.0 Architecture

## Overview

SQLRustGo is a relational database system implementing a SQL-92 subset with a Volcano-style execution model.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Clients                                 │
│   (MySQL CLI, JDBC, mysql-client, sqlrustgo-cli)                 │
└────────────────────────────┬────────────────────────────────────┘
                             │ MySQL Wire Protocol
┌────────────────────────────▼────────────────────────────────────┐
│                    sqlrustgo-mysql-server                        │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              Connection Manager (port 3307)              │  │
│   └──────────────────────────────────────────────────────────┘  │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │                   Parser (lalrpop)                       │  │
│   │         SQL → AST → Relational Algebra                   │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                    sqlrustgo-planner                            │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              Logical Optimizer                           │  │
│   │   - Predicate pushdown                                 │  │
│   │   - Join reordering                                    │  │
│   │   - Projection elimination                             │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   sqlrustgo-optimizer                          │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              CBO Cost Estimator                          │  │
│   │   - Hash Semi Join                                     │  │
│   │   - Hash Anti Join                                    │  │
│   │   - Decorrelation                                     │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   sqlrustgo-executor                           │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │               VolcanoExecutor                            │  │
│   │   - Hash Join / Hash Semi Join / Hash Anti Join        │  │
│   │   - Sort + Merge Join                                  │  │
│   │   - Aggregation (Hash + Sort)                         │  │
│   │   - Table Scan / Index Scan                           │  │
│   └──────────────────────────────────────────────────────────┘  │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              Storage Engine                             │  │
│   │   - Clustered Index (V311-01)                         │  │
│   │   - Adaptive Hash Index (V311-02)                     │  │
│   │   - Buffer Pool (LRU)                                 │  │
│   │   - WAL + Double-Write Buffer                         │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   sqlrustgo-storage                            │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │               Page Manager                              │  │
│   │   - FileStorage / MemoryStorage                         │  │
│   │   - B+ Tree Index                                     │  │
│   │   - Compression (LZ4/zstd)                            │  │
│   └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## New Components in v3.11.0

### Clustered Index (V311-01)

Primary key data stored with the index structure for fast point lookups.

### Adaptive Hash Index (V311-02)

Automatically built for frequently accessed pages. Reduces B+ tree traversal for hot data.

### Hash Semi Join (V311-15)

Optimized for correlated EXISTS/IN subqueries:
```sql
SELECT * FROM orders o WHERE EXISTS (SELECT 1 FROM customers c WHERE c.id = o.customer_id)
```

### Hash Anti Join (V311-17)

Optimized for NOT EXISTS/NOT IN:
```sql
SELECT * FROM orders o WHERE NOT EXISTS (SELECT 1 FROM invalid_customers ic WHERE ic.id = o.customer_id)
```

## Execution Flow

1. Client sends SQL via MySQL protocol
2. `mysql-server` parses SQL to AST
3. `planner` converts AST to logical plan
4. `optimizer` applies CBO transformations
5. `executor` runs VolcanoIterator model
6. `storage` reads/writes pages via Buffer Pool
7. WAL ensures durability
