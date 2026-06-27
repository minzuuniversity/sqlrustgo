# v3.5.0 Architecture Decisions

## ADR-0001: Volcano Executor Model

**编号**: ADR-0001  
**标题**: Volcano Volcano-style Iterator Model  
**状态**: ✅ 实施  
**日期**: 2022-Q1  
**模块**: executor

### 决策

采用 Volcano 风格的火山模型作为执行器架构基础，所有物理算子实现 `next()` 接口，支持流水线式执行。

### 影响模块

executor/iterator.rs, executor/pipeline.rs

---

## ADR-0002: MVCC Snapshot Isolation

**编号**: ADR-0002  
**标题**: Multi-Version Concurrency Control with Snapshot Isolation  
**状态**: ✅ 实施  
**日期**: 2022-Q2  
**模块**: transaction

### 决策

事务系统采用 MVCC Snapshot Isolation，通过版本链实现读写不阻塞。

### 影响模块

transaction/mvcc.rs, transaction/snapshot.rs

---

## ADR-0003: Cost-Based Optimizer (CBO)

**编号**: ADR-0003  
**标题**: Cost-Based Query Optimizer  
**状态**: ✅ 实施  
**日期**: 2022-Q3  
**模块**: optimizer

### 决策

查询优化器采用 CBO 模型，基于统计信息（行数、列基数）估算执行成本，选择最优计划。

### 影响模块

optimizer/cbo.rs, optimizer/statistics.rs

---

## ADR-0004: WAL Design

**编号**: ADR-0004  
**标题**: Write-Ahead Log Architecture  
**状态**: ✅ 实施  
**日期**: 2022-Q4  
**模块**: storage

### 决策

采用 WAL 机制保证事务持久性，支持 ROCKSDB 和内存模式双后端。

### 影响模块

storage/wal.rs, storage/wal_archive.rs

---

## ADR-0005: Clustered Index

**编号**: ADR-0005  
**标题**: Clustered B-Tree Index  
**状态**: ✅ 实施  
**日期**: 2023-Q1  
**模块**: storage

### 决策

表数据按主键顺序物理存储在 B-Tree 叶节点，实现主键索引与数据合一。

### 影响模块

storage/btree.rs, storage/clustered_index.rs

---

## ADR-0006: SSI Isolation Level

**编号**: ADR-0006  
**标题**: Serializable Snapshot Isolation (SSI)  
**状态**: ✅ 实施  
**日期**: 2023-Q2  
**模块**: transaction

### 决策

SSI 提供最强隔离级别，通过 SIREAD 锁检测危险场景并序列化冲突事务。

### 影响模块

transaction/ssi.rs, transaction/si_lock.rs

---

## ADR-0007: HNSW Vector Index

**编号**: ADR-0007  
**标题**: HNSW Approximate Nearest Neighbor Index  
**状态**: ✅ 实施  
**日期**: 2023-Q3  
**模块**: storage

### 决策

引入 HNSW 算法支持向量检索，支持 ANN 搜索用于 AI 向量检索增强。

### 影响模块

storage/hnsw.rs, storage/vector_index.rs

---

## ADR-0008: Trust Infrastructure

**编号**: ADR-0008  
**标题**: Trust Convergence and Gate Governance  
**状态**: ✅ 实施  
**日期**: 2024-Q1  
**模块**: governance

### 决策

建立完整的门禁体系（Alpha/Beta/RC/GA），Truthfulness 原则为最高优先级。

### 影响模块

scripts/gate/, docs/releases/

---

## ADR-0009: GMP Four-Layer Architecture

**编号**: ADR-0009  
**标题**: GMP Four-Layer Retrieval Architecture  
**状态**: ✅ 实施  
**日期**: 2024-Q2  
**模块**: gmp

### 决策

GMP 采用四通道检索架构：Rule + Vector + FTS + Graph，通过 RRF/Reranker/LLM 综合排序。

### 影响模块

gmp-retrieval/src/channel.rs

---

## ADR-0010: Parser Architecture

**编号**: ADR-0010  
**标题**: Recursive Descent SQL Parser  
**状态**: ✅ 实施  
**日期**: 2024-Q2  
**模块**: parser

### 决策

SQL 解析器采用递归下降解析，支持 MySQL 5.7/8.0 协议。

### 影响模块

parser/src/statement.rs, parser/src/expr.rs

---

## ADR-0011: Planner and Optimizer

**编号**: ADR-0011  
**标题**: Volcano-style Planner and Optimizer  
**状态**: ✅ 实施  
**日期**: 2024-Q3  
**模块**: optimizer

### 决策

优化器采用 Volcano 风格规则和代价模型结合，支持逻辑优化和物理优化两阶段。

### 影响模块

optimizer/planner.rs, optimizer/rule.rs

---

## ADR-0012: Network Protocol

**编号**: ADR-0012  
**标题**: MySQL Protocol Compatibility Layer  
**状态**: ✅ 实施  
**日期**: 2024-Q3  
**模块**: protocol

### 决策

网络层实现 MySQL 协议编解码，支持 5.7/8.0 协议版本。

### 影响模块**

protocol/src/decoder.rs, protocol/src/encoder.rs

---

## ADR-0013: Catalog System

**编号**: ADR-0013  
**标题**: Unified Catalog for Schemata, Tables, Columns  
**状态**: ✅ 实施  
**日期**: 2024-Q4  
**模块**: catalog

### 决策

统一 Catalog 管理元数据，支持 Database/Table/Column 三级结构，与统计信息联动。

### 影响模块

catalog/src/core.rs, catalog/src/statistics.rs