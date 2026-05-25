---
name: SQLRustGo Architecture
description: SQLRustGo 数据库引擎架构概览
created: 2026-04-24
source: 本地分析
tags: [sqlrustgo, database, rust, architecture]
---

# SQLRustGo 架构概览

## 项目概述

SQLRustGo 是一个用 Rust 实现的数据库引擎，支持 SQL-92 标准。

**版本**: v3.0.0 GA
**总 LOC**: ~244,767 (Rust)

## 核心模块

### 1. Parser (5,261 LOC)
SQL 解析模块，负责词法分析和语法解析。

**关键组件**:
- `lexer.rs` - 词法分析器
- `parser.rs` - 语法分析器
- `token.rs` - Token 定义

**关键函数**:
- `parse()` - 解析 SQL 语句
- `parse_create()` - 解析建表语句
- `parse_select()` - 解析查询语句

### 2. Executor (19,587 LOC)
查询执行引擎，处理 SQL 执行。

**关键功能**:
- Query Cache - 查询缓存
- Triggers - 触发器
- Hash Join 执行

### 3. Storage (15,752 LOC)
存储引擎，支持多种存储方式。

**支持的引擎**:
- MemoryStorage - 内存存储
- FileStorage - 文件存储
- ColumnarStorage - 列式存储

**特性**:
- Binlog 支持
- WAL (Write-Ahead Logging)
- MVCC

### 4. Transaction (4,796 LOC)
事务管理模块。

**隔离级别**:
- READ_UNCOMMITTED
- READ_COMMITTED
- REPEATABLE_READ
- SERIALIZABLE

**实现**: SSI (Serializable Snapshot Isolation)

### 5. Optimizer (6,509 LOC)
查询优化器。

**优化策略**:
- 成本优化 (CBO)
- Join 排序优化
- 规则优化

### 6. Distributed (11,920 LOC)
分布式特性。

**功能**:
- Sharding - 数据分片
- Raft 共识
- 2PC 两阶段提交
- 故障转移

## 执行流程

```
SQL String
    ↓
Lexer (Token Stream)
    ↓
Parser (AST)
    ↓
Planner (Logical Plan)
    ↓
Optimizer (Optimized Plan)
    ↓
Executor (Execution Result)
    ↓
Storage (Data Read/Write)
```

## 网络层

### MySQL Server (1,448 LOC)
- 端口: 3306
- 协议: MySQL Client/Server Protocol v10
- 兼容性: sysbench

### Server (7,787 LOC)
- gRPC 接口
- REST API

## 架构图

```
┌─────────────────────────────────────────┐
│           NETWORK LAYER                 │
│  mysql-server │ server │ network       │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│        QUERY PROCESSING                 │
│  parser → planner → optimizer          │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│           EXECUTION                     │
│  executor (Query Cache, Triggers)       │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│     STORAGE & TRANSACTION              │
│  storage │ transaction │ distributed    │
└─────────────────────────────────────────┘
```

## 测试状态

| 模块 | 测试数 | 状态 |
|------|--------|------|
| parser | 72 | ✅ 全部通过 |
| executor | 141 | ✅ 全部通过 |
| 总计 | 213+ | ✅ 全部通过 |

## 相关链接

- Parser 模块详解
- Executor 执行引擎
- Storage 存储引擎

---

*最后更新: 2026-04-24*