# SQLRustGo 数据库内核架构设计 v2.0

> **版本**: v2.0
> **状态**: 架构蓝图
> **目标**: 为 SQLRustGo 项目提供一个完整、可演进的数据库内核设计
> **更新日期**: 2026-03-13

---

## 目录

1. [系统总体架构](#1-系统总体架构)
2. [核心数据结构](#2-核心数据结构)
3. [模块详解](#3-模块详解)
4. [版本演进路径](#4-版本演进路径)
5. [工程实践与质量保障](#5-工程实践与质量保障)
6. [附录：关键代码结构示例](#6-附录关键代码结构示例)

---

## 1. 系统总体架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Client Layer                                   │
│  (CLI / MySQL Protocol / HTTP API)                                          │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Query Processing Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  ┌───────────┐  │
│  │  Parser  │→│  Binder  │→│  Planner │→│ Optimizer │→│ Executor  │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘  └───────────┘  │
│                                    │                                        │
│                                    ▼                                        │
│                              ┌──────────────┐                              │
│                              │   Catalog    │                              │
│                              └──────────────┘                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Storage & Transaction Layer                       │
│  ┌────────────┐  ┌──────────────┐  ┌────────────┐  ┌───────────────┐      │
│  │Buffer Pool │←│    Storage    │→│   Index    │  │   WAL         │      │
│  └────────────┘  │    Engine    │  └────────────┘  │ (Write-Ahead  │      │
│                  └──────────────┘                   │     Log)      │      │
│                         │                           └───────────────┘      │
│                         ▼                                                   │
│                   ┌────────────┐                                            │
│                   │   Disk     │                                            │
│                   └────────────┘                                            │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Observability Layer                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐                 │
│  │ Logging  │  │ Metrics  │  │ Tracing  │  │ Profiling   │                 │
│  └──────────┘  └──────────┘  └──────────┘  └─────────────┘                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

**设计原则**：

- **分层清晰**：各层职责单一，依赖单向
- **接口稳定**：核心 trait 尽早冻结，扩展通过组合
- **可插拔**：存储引擎、索引，执行器均可通过 trait 替换
- **可观测性内建**：从第一天起集成日志、指标、追踪

---

## 2. 核心数据结构

### 2.1 数据值 (Value)

```rust
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}
```

### 2.2 数据类型 (DataType)

```rust
pub enum DataType {
    Null,
    Boolean,
    Integer,
    Float,
    String,
}
```

### 2.3 表达式 (Expr)

```rust
pub enum Expr {
    Column(String),
    Literal(Value),
    Alias(Box<Expr>, String),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },
    Aggregate {
        func: AggregateFunction,
        args: Vec<Expr>,
        distinct: bool,
    },
}
```

### 2.4 逻辑计划节点 (LogicalPlan)

```rust
pub enum LogicalPlan {
    Scan { table_name: String, columns: Vec<String>, filters: Vec<Expr> },
    Projection { input: Box<LogicalPlan>, exprs: Vec<Expr> },
    Filter { input: Box<LogicalPlan>, predicate: Expr },
    Join { left: Box<LogicalPlan>, right: Box<LogicalPlan>, on: Expr, join_type: JoinType },
    Aggregate { input: Box<LogicalPlan>, group_exprs: Vec<Expr>, aggregate_exprs: Vec<Expr> },
    Sort { input: Box<LogicalPlan>, order_by: Vec<SortExpr> },
    Limit { input: Box<LogicalPlan>, limit: usize, offset: usize },
}
```

### 2.5 物理计划节点 (PhysicalPlan)

```rust
pub trait PhysicalPlan: Send + Sync {
    fn schema(&self) -> &Schema;
    fn children(&self) -> Vec<Arc<dyn PhysicalPlan>>;
    fn execute(&self) -> Result<Box<dyn Executor>>;
    fn statistics(&self) -> PlanStatistics;
}
```

### 2.6 执行器接口 (Executor)

```rust
pub trait Executor: Send {
    fn open(&mut self) -> Result<()>;
    fn next(&mut self) -> Result<Option<RecordBatch>>;
    fn close(&mut self) -> Result<()>;
}
```

### 2.7 批量数据 (RecordBatch)

```rust
pub struct RecordBatch {
    schema: Arc<Schema>,
    columns: Vec<Array>,
    row_count: usize,
}
```

---

## 3. 模块详解

### 3.1 Parser（语法分析器）

- **位置**: crates/parser
- **职责**: 将 SQL 文本转换为 AST
- **演进**: v1.3 已存在，后续扩展

### 3.2 Binder（绑定器）

- **位置**: crates/binder (v1.4 引入)
- **职责**: 表名/列名解析，语义检查
- **演进**: 初期简化，v1.8 后与 planner 整合

### 3.3 Planner（计划器）

- **位置**: crates/planner
- **职责**: 将绑定后的查询转换为逻辑计划
- **演进**: v1.3 简单查询，v1.4 加入 Aggregate/Sort/Limit

### 3.4 Optimizer（优化器）

- **位置**: crates/optimizer
- **职责**: 规则优化，生成物理计划
- **演进**: 
  - v1.4: 简单 RBO
  - v1.6: 统计信息集成
  - v2.0: Cascades 框架

### 3.5 Executor（执行器）

- **位置**: crates/executor
- **职责**: 执行物理计划
- **演进**:
  - v1.3: Volcano 模型 (Scan, Project, Filter, NLJ, HJ)
  - v1.4: Aggregate, Sort, Limit
  - v3.0: 向量化执行

### 3.6 Storage Engine（存储引擎）

- **位置**: crates/storage
- **职责**: 数据持久化、缓冲池
- **演进**:
  - v1.5: 基础页式存储
  - v1.6: WAL 集成
  - v3.1: 列式存储

### 3.7 Transaction Manager（事务管理器）

- **位置**: crates/transaction
- **职责**: MVCC、锁管理
- **演进**:
  - v1.5: MVCC 骨架
  - v1.6: WAL 集成
  - v1.9: 完整 MVCC

### 3.8 Write-Ahead Log（预写日志）

- **位置**: crates/wal
- **职责**: 崩溃恢复
- **演进**:
  - v1.6: 基础 WAL
  - v1.9: ARIES 风格恢复

### 3.9 Catalog（元数据管理）

- **位置**: crates/catalog
- **职责**: 表、列、索引元数据

### 3.10 Observability（可观测性）

- **位置**: crates/observability
- **演进**:
  - v1.3: tracing + metrics 基础
  - v1.4: EXPLAIN ANALYZE
  - v2.0: 优化器追踪

### 3.11 Index Engine（索引引擎）

- **位置**: crates/index
- **演进**: v1.7: B+Tree 索引

### 3.12 Utility（工具模块）

- **位置**: crates/common
- **内容**: 错误类型、配置、测试辅助

---

## 4. 版本演进路径

| 版本 | 代号 | 时间 | 核心功能 |
|------|------|------|----------|
| **v1.3** | Query Engine | 2026 Q2 | Volcano 执行器基础，表达式求值，测试框架 |
| **v1.4** | SQL Completion | 2026 Q3 | GROUP BY, ORDER BY, LIMIT, 聚合算子, 简单 RBO |
| **v1.5** | Storage Foundation | 2026 Q4 | 页式存储、缓冲池、基础 MVCC 骨架 |
| **v1.6** | Statistics & Catalog | 2027 Q1 | 统计信息收集、Catalog 持久化 |
| **v1.7** | Indexing | 2027 Q2 | B+Tree 索引、IndexScan |
| **v1.8** | Planner Upgrade | 2027 Q3 | 逻辑/物理计划分离、规则优化器增强 |
| **v1.9** | Concurrency | 2027 Q4 | 完整 MVCC、锁管理器、死锁检测 |
| **v2.0** | Cost Optimizer | 2028 Q1 | Cascades 框架、成本模型、Join 重排 |
| **v2.1** | Statistics Enhance | 2028 Q2 | 直方图、采样、多列统计 |
| **v2.2** | Parallel Execution | 2028 Q3 | 并行扫描、并行 HashJoin |
| **v3.0** | Vectorized Engine | 2028 Q4 | DataChunk 向量化、SIMD |
| **v3.1** | Columnar Storage | 2029 Q1 | 列存引擎、Parquet 导入导出 |
| **v3.2** | Distributed (Exp) | 2029 Q2 | Exchange 算子、远程分片查询 |

---

## 5. 工程实践与质量保障

### 5.1 测试策略

- **单元测试**: 每个模块独立功能测试
- **集成测试**: 端到端 SQL 测试
- **基准测试**: Criterion 测量核心算子性能
- **覆盖率门禁**: 整体 ≥70%，核心模块 ≥80%

### 5.2 CI/CD 自动化

- GitHub Actions: 测试、clippy、fmt、benchmark
- 分支保护: main、release/*、rc/* 必须 PR + 1 人审核

### 5.3 文档体系

- 用户指南: mdbook 构建
- 架构文档: 持续更新
- API 文档: cargo doc

### 5.4 错误处理规范

- 使用 thiserror 定义错误类型
- 核心函数返回 Result<T, SqlError>
- 避免 unwrap/expect

---

## 6. 附录：关键代码结构示例

### 6.1 Executor trait 实现示例（v1.3）

```rust
pub struct FilterExec {
    input: Box<dyn Executor>,
    predicate: Arc<dyn Expression>,
}

impl Executor for FilterExec {
    fn open(&mut self) -> Result<()> {
        self.input.open()
    }

    fn next(&mut self) -> Result<Option<RecordBatch>> {
        while let Some(batch) = self.input.next()? {
            let selected = self.predicate.evaluate_batch(&batch)?;
            if selected.row_count() > 0 {
                return Ok(Some(selected));
            }
        }
        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        self.input.close()
    }
}
```

### 6.2 表达式求值接口

```rust
pub trait Expression: Send + Sync {
    fn data_type(&self) -> &DataType;
    fn evaluate(&self, input: &RecordBatch) -> Result<Array>;
}
```

### 6.3 存储引擎 Buffer Pool

```rust
pub struct BufferPool {
    frames: Vec<Frame>,
    replacer: Box<dyn Replacer>,
    disk_manager: Arc<DiskManager>,
}

impl BufferPool {
    pub fn read_page(&mut self, page_id: PageId) -> Result<&mut Page>;
    pub fn write_page(&mut self, page_id: PageId) -> Result<()>;
}
```

---

## 结语

本文档为 SQLRustGo 提供了清晰的架构蓝图，覆盖从基础执行引擎到分布式分析引擎的完整演进路径。开发者可依据此文档，按版本逐步实现。

**下一步**: 基于此文档创建各版本的详细开发计划。

---

*更新日期: 2026-03-13*
