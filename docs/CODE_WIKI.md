# SQLRustGo Code Wiki

> **项目版本**: v3.3.0 (develop/v3.3.0)
> **文档维护**: AI Code Assistant
> **最后更新**: 2026-05-18
> **Git 远程**: https://gitcode.com/BreavHeart/sqlrustgo

---

## 目录

1. [项目概述](#1-项目概述)
2. [整体架构](#2-整体架构)
3. [核心模块详解](#3-核心模块详解)
4. [关键数据结构](#4-关键数据结构)
5. [模块依赖关系](#5-模块依赖关系)
6. [项目运行方式](#6-项目运行方式)
7. [关键命令参考](#7-关键命令参考)
8. [Workspace Crates 完整列表](#8-workspace-crates-完整列表)

---

## 1. 项目概述

### 1.1 项目简介

SQLRustGo 是一个用 Rust 语言实现的 SQL-92 兼容数据库系统，采用模块化架构设计（Workspace 结构），支持以下核心功能：

| 功能模块 | 说明 |
|---------|------|
| SQL 解析 | 完整的 SQL 词法分析和语法解析 |
| 查询规划 | 逻辑计划和物理计划生成 |
| 查询优化 | 基于成本模型 (CBO) 的查询优化 |
| 执行引擎 | 向量化执行和 SIMD 加速 |
| 存储引擎 | Buffer Pool、WAL、B+Tree 索引、列式存储 |
| 事务管理 | MVCC + SSI 隔离级别支持 |
| 分布式支持 | 分片、XA 两阶段提交、Raft 共识 |
| GIS 支持 | 空间数据存储和 R-Tree 索引 |
| 向量索引 | HNSW、IVF、PQ 等向量相似度检索 |
| 图数据库 | 属性图模型和 Cypher 查询 |
| GMP 追溯 | 药品生产质量管理规范追溯系统 |

### 1.2 技术栈

- **编程语言**: Rust (Edition 2021)
- **异步运行时**: Tokio
- **序列化**: Protobuf (Tonic/Prost)
- **列式存储**: Apache Arrow / Parquet
- **测试框架**: Criterion (基准测试), Proptest (属性测试)

### 1.3 版本信息

- **当前版本**: v3.0.0 (workspace 定义)
- **开发分支**: develop/v3.3.0
- **主分支**: main

---

## 2. 整体架构

### 2.1 架构分层图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            应用层 (Application)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────┐  ┌─────┐ │
│  │  SQL CLI    │  │  Bench CLI  │  │   AgentSQL  │  │Server │  │ Tools│ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └───────┘  └─────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│                       查询处理层 (Query Processing)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────┐ │
│  │   Parser    │→ │   Planner   │→ │  Optimizer  │→ │ Unified Query  │ │
│  │  (词法/语法) │  │  (逻辑计划)  │  │  (CBO优化)  │  │  (统一查询层)   │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│                       执行引擎层 (Execution Engine)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────┐ │
│  │  Executor   │  │    Expr     │  │  Vectorized │  │  Parallel Exec │ │
│  │  (算子执行)  │  │  (表达式)    │  │  (向量化)    │  │  (并行执行)     │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│                         存储引擎层 (Storage Engine)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────┐  ┌─────┐ │
│  │  Buffer Pool│  │  WAL日志    │  │  B+Tree索引 │  │Column │  │ FTS │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  │ar     │  │     │ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │(列式) │  │     │ │
│  │  Spill     │  │  Replication│  │  PITR      │  └───────┘  └─────┘ │
│  │  (外部排序) │  │  (主从复制)  │  │  (时间点恢复)│                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                   │
├─────────────────────────────────────────────────────────────────────────┤
│                         事务管理层 (Transaction)                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────┐ │
│  │    MVCC     │  │    SSI      │  │ Lock Manager│  │   XA/DTC      │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  │ (分布式事务)    │ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  └────────────────┘ │
│  │ Idempotency │  │ Deadlock    │  │ Savepoint   │                    │
│  │ (幂等性)     │  │ (死锁检测)  │  │ (保存点)     │                    │
│  └─────────────┘  └─────────────┘  └─────────────┘                    │
├─────────────────────────────────────────────────────────────────────────┤
│                          扩展功能层 (Extensions)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────┐  ┌─────┐ │
│  │   Vector    │  │   Graph     │  │    GIS      │  │  GMP  │  │ RAG │ │
│  │  (向量索引)  │  │  (图数据库)  │  │  (空间数据)  │  │(追溯)  │  │     │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └───────┘  └─────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│                         分布式层 (Distributed)                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────┐ │
│  │ Shard Router│  │ Raft Leader │  │Group Rep    │  │  Read/Write   │ │
│  │  (分片路由)  │  │  (Raft选主) │  │(组复制)     │  │  Split        │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  │ (读写分离)     │ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  └────────────────┘ │
│  │  Failover   │  │ XA Coord    │  │  Semi-Sync  │                    │
│  │  (故障转移)  │  │ (XA协调器)   │  │ (半同步复制) │                    │
│  └─────────────┘  └─────────────┘  └─────────────┘                    │
├─────────────────────────────────────────────────────────────────────────┤
│                         可观测性 (Observability)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────┐ │
│  │  Metrics    │  │  Telemetry  │  │  Slow Query │  │  Performance  │ │
│  │  (指标)      │  │  (遥测)     │  │  Log        │  │  Schema       │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 查询执行流程

```
SQL Query
    │
    ▼
┌─────────┐   Token   ┌─────────┐   AST    ┌─────────┐  逻辑计划  ┌─────────┐
│  Lexer  │──────────→│  Parser │─────────→│ Planner │──────────→│Optimizer│
└─────────┘           └─────────┘          └─────────┘            └─────────┘
                                                                         │
                                                                         ▼
                                                             ┌─────────────────┐
                                                             │ Unified Query   │
                                                             │ (统一查询入口)   │
                                                             └────────┬────────┘
                                                                      │
                                          ┌───────────────────────────────┼───────────────────────────────┐
                                          ▼                               ▼                               ▼
                                  ┌───────────────┐           ┌───────────────┐           ┌───────────────┐
                                  │  Vector Index  │           │  Graph Store   │           │  Storage Eng  │
                                  │  (向量检索)    │           │  (图查询)      │           │  (关系存储)   │
                                  └───────────────┘           └───────────────┘           └───────────────┘
                                          │                               │                               │
                                          └───────────────────────────────┼───────────────────────────────┘
                                                                      ▼
SQL Result ◀──── Row Data ◀──── Execution ◀──── 物理计划 ◀─────────────────────┘
```

---

## 3. 核心模块详解

### 3.1 Parser 模块 (`crates/parser`)

**职责**: SQL 词法分析和语法解析

| 文件 | 作用 |
|------|------|
| `src/lexer.rs` | 词法分析器 |
| `src/parser.rs` | 语法解析器 |
| `src/token.rs` | Token 定义 (SQL 关键字) |
| `src/transaction.rs` | 事务语句解析 |

**核心导出**:

```rust
pub use parser::parse;
pub use parser::{
    SelectStatement, InsertStatement, UpdateStatement, DeleteStatement,
    CreateTableStatement, DropTableStatement, AlterTableStatement,
    Statement, Expression, JoinClause, SelectColumn,
};
```

**支持的语句类型**:

- **DDL**: `CREATE`, `DROP`, `ALTER`, `TRUNCATE`
- **DML**: `INSERT`, `UPDATE`, `DELETE`, `MERGE`
- **DQL**: `SELECT`, `WITH` (CTE), `UNION`
- **事务**: `BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`
- **其他**: `EXPLAIN`, `VACUUM`, `CHECK`, `EVENT`

**解析器 trait**:

```rust
pub trait Parser {
    fn parse(&mut self) -> ParseResult<Vec<Statement>>;
    fn parse_statement(&mut self) -> ParseResult<Statement>;
}
```

---

### 3.2 Planner 模块 (`crates/planner`)

**职责**: 查询逻辑计划和物理计划生成

**核心文件**:

| 文件 | 作用 |
|------|------|
| `src/logical_plan.rs` | 逻辑计划定义 |
| `src/physical_plan.rs` | 物理计划算子定义 |
| `src/planner.rs` | 计划器实现 |
| `src/optimizer.rs` | 优化器接口 |

**核心类型**:

```rust
// 逻辑计划
pub enum LogicalPlan {
    TableScan { table_name: String, projection: Vec<String> },
    Selection { predicate: Expr, input: Box<LogicalPlan> },
    Projection { expr: Vec<Expr>, input: Box<LogicalPlan> },
    Aggregate { group_by: Vec<Expr>, aggr_funcs: Vec<Expr>, input: Box<LogicalPlan> },
    Sort { order_by: Vec<SortExpr>, input: Box<LogicalPlan> },
    Join { join_type: JoinType, left: Box<LogicalPlan>, right: Box<LogicalPlan>, condition: Expr },
    Limit { limit: usize, offset: Option<usize>, input: Box<LogicalPlan> },
    Window { window_funcs: Vec<WindowFunction>, input: Box<LogicalPlan> },
    Union { left: Box<LogicalPlan>, right: Box<LogicalPlan> },
}
```

**物理计划算子**:

```rust
pub struct SeqScanExec { pub table_name: String, pub projection: Vec<String> }
pub struct IndexScanExec { pub index_name: String, pub key_expr: Expr }
pub struct FilterExec { pub predicate: Expr, pub input: Box<dyn PhysicalPlan> }
pub struct ProjectionExec { pub exprs: Vec<Expr>, pub input: Box<dyn PhysicalPlan> }
pub struct AggregateExec { pub group_by: Vec<Expr>, pub aggr_funcs: Vec<Expr>, pub input: Box<dyn PhysicalPlan> }
pub struct HashJoinExec { pub join_type: JoinType, pub left_keys: Vec<Expr>, pub right_keys: Vec<Expr>, pub left: Box<dyn PhysicalPlan>, pub right: Box<dyn PhysicalPlan> }
pub struct SortExec { pub order_by: Vec<SortExpr>, pub input: Box<dyn PhysicalPlan> }
pub struct LimitExec { pub limit: usize, pub offset: Option<usize>, pub input: Box<dyn PhysicalPlan> }
pub struct WindowExec { pub window_funcs: Vec<WindowFunction>, pub input: Box<dyn PhysicalPlan> }
pub struct DeleteExec { pub table_name: String, pub predicate: Option<Expr> }
```

**表达式系统**:

```rust
pub enum Expr {
    Column(Column),                              // 列引用
    Literal(Value),                             // 字面量
    BinaryExpr { left: Box<Expr>, op: Operator, right: Box<Expr> },
    UnaryExpr { op: Operator, expr: Box<Expr> },
    AggregateFunction { func: AggregateFunction, args: Vec<Expr>, distinct: bool },
    WindowFunction { func: WindowFunction, args: Vec<Expr>, partition_by: Vec<Expr>, order_by: Vec<SortExpr>, frame: Option<WindowFrame> },
    CaseWhen { conditions: Vec<WhenClause>, else_result: Option<Box<Expr>> },
    In { expr: Box<Expr>, subquery: Box<LogicalPlan> },
    Exists(Box<LogicalPlan>),
    Substring { expr: Box<Expr>, start: Box<Expr>, length: Option<Box<Expr>> },
    Cast { expr: Box<Expr>, data_type: DataType },
}
```

---

### 3.3 Optimizer 模块 (`crates/optimizer`)

**职责**: 基于成本模型 (CBO) 的查询优化

**核心子模块**:

| 子模块 | 功能 |
|-------|------|
| `cost.rs` | 简单成本模型 |
| `stats.rs`, `stats_collector.rs`, `stats_registry.rs` | 表/列统计信息 |
| `rules.rs` | 优化规则 |
| `graph_cost.rs` | 图查询成本估算 |
| `network_cost.rs` | 分布式网络成本估算 |
| `vector_cost.rs` | 向量检索成本估算 |
| `unified_cost.rs`, `unified_plan.rs` | 统一成本模型和计划 |
| `projection_pushdown.rs` | 投影下推 |
| `index_selector.rs` | 索引选择 |
| `path_selector.rs` | 执行路径选择 |

**优化规则**:

```rust
pub use rules::{
    ConstantFolding,      // 常量折叠
    PredicatePushdown,    // 谓词下推
    ProjectionPruning,    // 投影裁剪
    FilterJoinReorder,     // Filter-Join 重排序
    AggregateSimplify,     // 聚合简化
};
```

**成本模型 trait**:

```rust
pub trait CostModel {
    fn estimate_cost(&self, plan: &dyn PhysicalPlan, stats: &StatisticsProvider) -> f64;
    fn estimate_cardinality(&self, plan: &dyn PhysicalPlan) -> f64;
}
```

**统计信息类型**:

```rust
pub struct TableStats {
    pub row_count: u64,
    pub page_count: u64,
    pub total_size: u64,
}

pub struct ColumnStats {
    pub null_count: u64,
    pub distinct_count: u64,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub histogram: Vec<HistogramBucket>,
}
```

---

### 3.4 Executor 模块 (`crates/executor`)

**职责**: 查询执行和结果返回

**核心子模块**:

| 子模块 | 文件/目录 | 功能 |
|-------|----------|------|
| `executor.rs` | 主执行器 | 算子执行框架 |
| `scan.rs` | 表扫描 | 顺序扫描 |
| `index_scan.rs` | 索引扫描 | 索引扫描执行 |
| `filter.rs` | 过滤执行 | 条件过滤 |
| `merge.rs` | Merge 执行 | MERGE INTO 语句 |
| `vectorization.rs`, `vector_executor.rs` | 向量化执行 | 向量化算子 |
| `vec_simd.rs` | SIMD 加速 | SIMD 指令加速 |
| `parallel_executor.rs`, `parallel_vector_executor.rs` | 并行执行 | 多线程并行 |
| `window_executor.rs` | 窗口函数 | 窗口函数执行 |
| `explain.rs` | EXPLAIN | 执行计划输出 |
| `trigger.rs`, `trigger_eval/` | 触发器 | 触发器执行 |
| `audit_logger.rs` | 审计日志 | DML 审计 |
| `query_cache.rs`, `query_cache_config.rs` | 查询缓存 | 查询结果缓存 |
| `ddl_executor.rs` | DDL 执行 | DDL 语句执行 |
| `stored_proc.rs` | 存储过程 | 存储过程执行 |
| `session_config.rs` | 会话配置 | 会话级配置 |

**执行器 trait**:

```rust
pub trait Executor {
    fn execute(&mut self) -> ExecutorResult<Vec<Record>>;
    fn schema(&self) -> Schema;
    fn name(&self) -> &str;
}
```

**执行上下文**:

```rust
pub struct ExecutorContext {
    pub catalog: Arc<Catalog>,
    pub storage: Arc<dyn StorageEngine>,
    pub tx_manager: Arc<TransactionManager>,
    pub query_cache: Arc<QueryCache>,
    pub config: SessionConfig,
}
```

---

### 3.5 Storage 模块 (`crates/storage`)

**职责**: 数据持久化和索引管理

**核心组件**:

| 组件 | 文件/目录 | 功能 |
|------|----------|------|
| `src/engine.rs` | 存储引擎 | StorageEngine trait, MemoryStorage |
| `src/buffer_pool.rs` | Buffer Pool | LRU 页面缓存 |
| `src/wal.rs` | WAL | 预写日志 |
| `src/file_storage.rs` | 文件存储 | 底层文件 I/O |
| `src/page.rs` | Page | 页面结构 |
| `src/bplus_tree/` | B+Tree | B+Tree 索引实现 |
| `src/clustered_index/` | 聚簇索引 | 聚簇索引实现 |
| `src/row_format/` | 行格式 | 行编码/解码 |
| `src/fts/` | 全文索引 | FTS 倒排索引 |
| `src/columnar/` | 列式存储 | Arrow/Parquet 支持 |
| `src/parquet/` | Parquet | Parquet 读写 |
| `src/recovery/` | 恢复 | PITR, WAL Chain, Checkpoint |
| `src/backup.rs`, `backup_storage.rs` | 备份 | 备份恢复 |
| `src/replication.rs`, `binlog_*` | 复制 | 主从复制 |
| `src/predicate.rs` | 谓词 | 谓词下推支持 |
| `src/spill.rs` | Spill | 溢出处理 |

**关键类型**:

```rust
// 存储引擎 trait
pub trait StorageEngine {
    fn create_table(&mut self, table_info: TableInfo) -> Result<TableId>;
    fn drop_table(&mut self, table_id: TableId) -> Result<()>;
    fn insert(&mut self, table_id: TableId, record: Record) -> Result<RowId>;
    fn update(&mut self, table_id: TableId, row_id: RowId, record: Record) -> Result<()>;
    fn delete(&mut self, table_id: TableId, row_id: RowId) -> Result<()>;
    fn scan(&mut self, table_id: TableId, predicate: Option<&Expr>) -> Result<Box<dyn Iterator<Item=Record>>>;
    fn create_index(&mut self, index_info: IndexInfo) -> Result<IndexId>;
}

// 页面结构
pub struct Page {
    pub page_id: PageId,
    pub data: Vec<u8>,
    pub page_type: PageType,
    pub checksum: u32,
}

pub enum PageType {
    DataPage,
    IndexPage,
    OverflowPage,
    WalPage,
}
```

**Buffer Pool 接口**:

```rust
impl BufferPool {
    pub fn fetch_page(&mut self, page_id: PageId) -> Result<FrameId>;
    pub fn unpin_page(&mut self, frame_id: FrameId);
    pub fn flush_page(&mut self, page_id: PageId) -> Result<()>;
    pub fn flush_all(&mut self) -> Result<()>;
    pub fn get_buffered_pages(&self) -> Vec<PageId>;
}
```

---

### 3.6 Transaction 模块 (`crates/transaction`)

**职责**: 事务管理和并发控制

**核心组件**:

| 组件 | 文件/目录 | 功能 |
|------|----------|------|
| `src/mvcc.rs` | MVCC | 多版本并发控制 |
| `src/ssi.rs` | SSI | 可串行化快照隔离检测 |
| `src/transaction_manager.rs` | 事务管理器 | 事务生命周期管理 |
| `src/lock_manager.rs` | 锁管理器 | 行锁、表锁 |
| `src/idempotency/` | 幂等性 | 幂等性保证 |
| `src/deadlock.rs` | 死锁 | 死锁检测 |
| `src/savepoint.rs` | 保存点 | SAVEPOINT 支持 |
| `src/coordinator.rs`, `src/dtc.rs` | DTC | 分布式事务协调 |
| `src/participant.rs` | 参与者 | XA 参与者 |
| `src/audit/` | 审计 | 事务审计 |
| `src/recovery.rs` | 恢复 | 事务恢复 |

**事务状态**:

```rust
pub enum TransactionState {
    Active,
    Prepared,
    Committed,
    Aborted,
    RolledBack,
}

pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Snapshot,        // 快照隔离
    Serializable,    // 可串行化
}
```

**MVCC 快照**:

```rust
pub struct Snapshot {
    pub tx_id: TxId,                    // 当前事务 ID
    pub min_id: TxId,                   // 活跃事务最小 ID
    pub active_txs: HashSet<TxId>,     // 活跃事务集合
    pub commit_ts: TxId,               // 快照时间戳
}
```

**SSI 检测**:

```rust
pub struct SerializationGraph {
    pub nodes: HashMap<TxId, Vec<SerializationEdge>>,
}

pub struct SsiDetector {
    pub graph: SerializationGraph,
    pub threshold: u64,
}

impl SsiDetector {
    pub fn detect_conflict(&mut self, tx1: TxId, tx2: TxId, edge: SerializationEdge) -> bool;
    pub fn is_serializable(&self) -> bool;
}
```

---

### 3.7 Catalog 模块 (`crates/catalog`)

**职责**: 数据库元数据管理

**核心组件**:

| 组件 | 文件 | 功能 |
|------|------|------|
| `src/catalog.rs` | Catalog | 目录服务 |
| `src/schema.rs` | Schema | 模式管理 |
| `src/table.rs` | Table | 表定义 |
| `src/column.rs` | Column | 列定义 |
| `src/data_type.rs` | DataType | 数据类型 |
| `src/index.rs` | Index | 索引信息 |
| `src/stored_proc.rs` | StoredProc | 存储过程 |
| `src/auth.rs`, `src/auth_rls.rs` | Auth | 认证授权、RLS |
| `src/event.rs` | Event | 事件调度器 |
| `src/migration/` | Migration | DDL 迁移 |
| `src/system_tables/` | System Tables | MySQL 系统表 |

**核心类型**:

```rust
pub struct Catalog {
    pub schemas: HashMap<String, Schema>,
    pub current_schema: String,
}

pub struct Table {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub indexes: Vec<Index>,
    pub constraints: Vec<TableConstraint>,
    pub triggers: Vec<TriggerInfo>,
}

pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<Expr>,
    pub primary_key: bool,
}
```

---

### 3.8 Types 模块 (`crates/types`)

**职责**: 核心数据类型定义

**值类型**:

```rust
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
    Date(i32),          // YYYY*512 + MM*32 + DD
    Timestamp(i64),     // Microseconds since epoch
}
```

**错误处理**:

```rust
pub enum SqlError {
    ParseError(String),
    EvaluationError(String),
    StorageError(String),
    TransactionError(String),
    CatalogError(String),
}

pub type SqlResult<T> = Result<T, SqlError>;
```

---

### 3.9 Expr 模块 (`crates/expr`)

**职责**: 统一的表达式求值系统

**架构设计**:

```
Parser ──→ Expr AST ──→ Planner/CBO ──→ Executor eval()
```

**核心类型**:

```rust
pub use expr::Expr;
pub use eval::EvalContext;

pub struct EvalContext {
    pub row: HashMap<String, Value>,    // "table.column" -> value
    pub params: Vec<Value>,            // 绑定参数
    pub functions: FunctionRegistry,
}
```

**求值方法**:

```rust
impl Expr {
    pub fn eval(&self, ctx: &EvalContext) -> SqlResult<Value>;
    pub fn eval_predicate(&self, ctx: &EvalContext) -> SqlResult<bool>;
    pub fn eval_as_bool(&self, ctx: &EvalContext) -> SqlResult<TriBool>;
    pub fn to_literal(&self) -> Option<Value>;
}
```

---

### 3.10 Vector 模块 (`crates/vector`)

**职责**: 向量相似度检索

**索引算法**:

| 算法 | 类 | 特点 |
|------|-----|------|
| 暴力搜索 | `FlatIndex` | O(n) 精确搜索 |
| 倒排文件 | `IvfIndex` | IVF + k-means |
| HNSW | `HnswIndex` | 分层导航小世界图 |
| PQ | `IvfpqIndex` | 产品量化压缩 |
| IVF-PQ | `IvfpqIndex` | IVF + PQ 组合 |

**距离度量**:

```rust
pub enum DistanceMetric {
    Cosine,        // 余弦相似度
    Euclidean,     // 欧氏距离
    DotProduct,    // 点积
    Manhattan,     // 曼哈顿距离
}
```

**核心 trait**:

```rust
pub trait VectorIndex {
    fn insert(&mut self, id: u64, vector: &[f32]) -> VectorResult<()>;
    fn search(&self, query: &[f32], k: usize) -> VectorResult<Vec<(u64, f32)>>;
    fn build_index(&mut self) -> VectorResult<()>;
}
```

**高级功能**:

- `batch_writer.rs`: 批量写入
- `gpu_accel.rs`: GPU 加速
- `parallel_knn.rs`: 并行 KNN
- `sharded_index.rs`: 分片索引
- `sql_vector_hybrid.rs`: SQL + 向量混合查询

---

### 3.11 Graph 模块 (`crates/graph`)

**职责**: 属性图数据库

**核心概念**:

- **Node**: 实体节点 (Batch, Device, SOP)
- **Edge**: 关系边
- **Label**: 类型标识
- **Property**: 属性键值对

**核心文件**:

| 文件/目录 | 功能 |
|----------|------|
| `src/cypher/` | Cypher 查询解析和执行 |
| `src/model/` | 图数据模型 |
| `src/store/` | 图存储 (磁盘存储) |
| `src/traversal/` | 图遍历算法 (BFS, DFS) |

**GMP 追溯链**:

```
Batch → Device → Calibration → Regulation
```

**Cypher 支持**:

```rust
pub mod cypher {
    pub struct CypherQuery {
        pub match_clause: Vec<MatchPattern>,
        pub where_clause: Option<Expr>,
        pub return_clause: Vec<ReturnItem>,
        pub order_by: Vec<SortExpr>,
        pub limit: Option<usize>,
    }
}
```

---

### 3.12 GIS 模块 (`crates/gis`)

**职责**: 空间数据存储和查询

**核心组件**:

| 组件 | 功能 |
|------|------|
| `src/geometry.rs` | 几何类型 (Point, LineString, Polygon) |
| `src/rtree.rs` | R-Tree 空间索引 |
| `src/spatial.rs` | 空间关系判断 |
| `src/wkt.rs` | WKT 格式解析/序列化 |
| `src/wkb.rs` | WKB 格式解析/序列化 |

**空间关系**:

```rust
pub enum SpatialRelation {
    Intersects,
    Within,
    Contains,
    Touches,
    Crosses,
    Overlaps,
    Disjoint,
}
```

---

### 3.13 GMP 模块 (`crates/gmp`)

**职责**: 药品生产质量管理规范 (GMP) 追溯系统

**核心功能**:

| 功能 | 说明 |
|------|------|
| `audit.rs` | 审计日志记录 |
| `document.rs` | 文档管理 (SOP, 验证报告) |
| `evidence.rs` | 证据链存储和验证 |
| `correction.rs` | 偏差和 CAPA 记录 |
| `provenance.rs` | 数据溯源追踪 |
| `workflow/` | 审批工作流引擎 |
| `signature/` | 数字签名 (ECDSA, Ed25519) |
| `hsm/` | 硬件安全模块集成 |
| `mobile/` | 移动设备信任验证 |
| `sop/` | SOP 管理和培训绑定 |
| `calibration/` | 设备校准管理 |
| `audit_chain.rs` | 防篡改审计链 |

**审计链**:

```rust
pub struct AuditChain {
    pub entries: Vec<AuditChainEntry>,
    pub genesis_hash: [u8; 32],
}

pub struct AuditChainEntry {
    pub timestamp: i64,
    pub operation: String,
    pub operator: String,
    pub checksum: [u8; 32],
    pub prev_hash: [u8; 32],
}
```

---

### 3.14 Distributed 模块 (`crates/distributed`)

**职责**: 分布式数据库支持

**核心组件**:

| 组件 | 文件 | 功能 |
|------|------|------|
| `src/shard_router.rs` | 分片路由 | 数据分片路由 |
| `src/shard_manager.rs` | 分片管理 | 分片元数据管理 |
| `src/partition.rs` | 分区策略 | 分区策略 |
| `src/raft.rs` | Raft 共识 | Raft 协议实现 |
| `src/group_replication.rs`, `group_replication_raft.rs` | 组复制 | MySQL Group Replication 风格复制 |
| `src/replication.rs` | 复制 | 主从复制 |
| `src/semisync.rs` | 半同步复制 | 半同步复制 |
| `src/read_write_splitter.rs` | 读写分离 | 读写分离路由 |
| `src/xa_coordinator.rs` | XA 协调器 | XA 分布式事务 |
| `src/failover_manager.rs` | 故障转移 | 自动故障转移 |
| `src/distributed_lock.rs` | 分布式锁 | 跨节点锁 |
| `src/grpc_client.rs`, `grpc_server.rs` | gRPC | RPC 通信 |

---

### 3.15 Unified Query 模块 (`crates/unified-query`)

**职责**: 统一查询入口 (支持多数据源)

**核心组件**:

| 组件 | 功能 |
|------|------|
| `src/engine.rs` | 统一查询引擎 |
| `src/adapters/` | 数据源适配器 (Storage, Vector, Graph) |
| `src/router.rs` | 查询路由 |
| `src/fusion.rs` | 结果融合 |
| `src/cache.rs` | 查询缓存 |

---

### 3.16 其他核心模块

| 模块 | 路径 | 职责 |
|------|------|------|
| `common` | `crates/common` | 公共工具 (日志、指标、连接池) |
| `server` | `crates/server` | 数据库服务器入口、HTTP API |
| `sql-cli` | `crates/sql-cli` | SQL 命令行工具 |
| `bench`, `bench-cli` | `crates/bench*` | 性能基准测试 |
| `sql-corpus` | `crates/sql-corpus` | SQL 兼容性测试语料 |
| `sqlancer` | `crates/sqlancer` | SQL 模糊测试 |
| `sync` | `crates/sync` | 数据同步 (OT 引擎) |
| `spill` | `crates/spill` | 溢出处理 (外部排序) |
| `rag` | `crates/rag` | RAG 检索增强生成 |
| `observability` | `crates/observability` | 可观测性 (Performance Schema) |
| `security` | `crates/security` | 安全 (防火墙、TLS) |
| `mysql-server` | `crates/mysql-server` | MySQL 协议兼容服务器 |

---

## 4. 关键数据结构

### 4.1 查询执行相关

```rust
// 逻辑计划节点
pub enum LogicalPlan {
    TableScan { table_name: String, projection: Vec<String> },
    Selection { predicate: Expr, input: Box<LogicalPlan> },
    Projection { expr: Vec<Expr>, input: Box<LogicalPlan> },
    Aggregate { group_by: Vec<Expr>, aggr_funcs: Vec<Expr>, input: Box<LogicalPlan> },
    Sort { order_by: Vec<SortExpr>, input: Box<LogicalPlan> },
    Join { join_type: JoinType, left: Box<LogicalPlan>, right: Box<LogicalPlan>, condition: Expr },
    Limit { limit: usize, offset: Option<usize>, input: Box<LogicalPlan> },
}

// 物理计划算子
pub trait PhysicalPlan {
    fn execute(&mut self) -> Result<Vec<Record>>;
    fn schema(&self) -> Schema;
    fn name(&self) -> &str;
}

// 记录
pub struct Record {
    pub values: Vec<Value>,
    pub row_id: Option<RowId>,
}
```

### 4.2 存储相关

```rust
// WAL 条目
pub struct WalEntry {
    pub tx_id: TxId,
    pub operation: WalOperation,
    pub table_id: TableId,
    pub page_id: PageId,
    pub slot_id: SlotId,
    pub before_image: Option<Vec<u8>>,
    pub after_image: Option<Vec<u8>>,
}

pub enum WalOperation {
    Insert,
    Update,
    Delete,
    Commit,
    Rollback,
}
```

### 4.3 事务相关

```rust
// MVCC 事务
pub struct Transaction {
    pub tx_id: TxId,
    pub start_ts: TxId,
    pub isolation: IsolationLevel,
    pub status: TransactionState,
    pub snapshot: Snapshot,
}

// XA 事务
pub struct XaTransaction {
    pub gtrid: String,
    pub bqual: String,
    pub format_id: i32,
    pub state: XaState,
}
```

---

## 5. 模块依赖关系

### 5.1 依赖层次图

```
┌────────────────────────────────────────────────────────────────┐
│                        sqlrustgo (bin)                         │
│              主入口，聚合所有子模块                               │
└───────────────────────────┬────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ sql-cli       │   │ server       │   │ bench         │
│ (CLI工具)     │   │ (HTTP服务器)  │   │ (基准测试)     │
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            ▼
┌────────────────────────────────────────────────────────────────┐
│                    核心依赖 (Core Dependencies)                  │
├───────────────┬───────────────┬───────────────┬─────────────────┤
│   executor   │   optimizer   │   planner    │   transaction   │
│  (执行引擎)   │   (查询优化)   │   (查询计划)  │   (事务管理)    │
├───────────────┼───────────────┼───────────────┼─────────────────┤
│   expr       │   catalog     │   storage    │   network       │
│  (表达式)     │   (元数据)     │   (存储)      │   (分布式)      │
├───────────────┼───────────────┼───────────────┼─────────────────┤
│  unified-     │   distributed │   types      │   common        │
│  query        │   (分布式)     │   (类型)      │   (公共)        │
├───────────────┴───────────────┴───────────────┴─────────────────┤
│                       types (基础类型)                           │
├──────────────────────────────────────────────────────────────────┤
│                      common (公共工具)                           │
└──────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────────┐
│                       第三方依赖                                 │
├───────────────┬───────────────┬───────────────┬─────────────────┤
│    tokio      │    arrow      │   parking_lot │   prost/tonic   │
│  (异步运行时)  │   (列式存储)   │   (同步锁)     │   (gRPC)        │
└───────────────┴───────────────┴───────────────┴─────────────────┘
```

### 5.2 Crate 依赖矩阵

| Crate | 依赖的 Crates |
|-------|--------------|
| `executor` | types, storage, transaction, expr, catalog |
| `optimizer` | planner, storage, expr |
| `planner` | parser, types, expr |
| `storage` | types, transaction, common |
| `transaction` | types, storage, common |
| `catalog` | types, storage |
| `vector` | types |
| `graph` | types, storage |
| `gis` | types, storage |
| `gmp` | types, storage, transaction, vector |
| `distributed` | storage, transaction, network |
| `network` | types |
| `server` | executor, optimizer, planner, storage, transaction, distributed, network |
| `unified-query` | executor, vector, graph, storage |

---

## 6. 项目运行方式

### 6.1 开发环境要求

- **Rust**: Edition 2021
- **Cargo**: 最新稳定版
- **系统**: Windows / Linux / macOS

### 6.2 构建项目

```bash
# 克隆项目
git clone https://gitcode.com/BreavHeart/sqlrustgo.git
cd sqlrustgo

# 编译 (全特性)
cargo build --all-features

# 快速检查 (不链接)
cargo check --all-features

# 构建指定 crate
cargo build -p sqlrustgo-executor --all-features
```

### 6.3 运行 SQL CLI

```bash
# 运行 REPL
cargo run --bin sqlrustgo

# 执行 SQL 文件
cargo run --bin sqlrustgo -- data/users.json < queries/q1.sql
```

### 6.4 运行测试

```bash
# 运行所有测试
cargo test --all-features

# 运行单个测试
cargo test <test_name> --all-features

# 运行特定 crate 测试
cargo test -p sqlrustgo-executor --all-features

# 文档测试
cargo test --doc
```

### 6.5 代码质量检查

```bash
# Clippy 检查 (必须零警告)
cargo clippy --all-features -- -D warnings

# 格式检查
cargo fmt --check --all

# 格式修复
cargo fmt --all
```

### 6.6 性能基准测试

```bash
# TPC-H 基准
cargo bench --bench tpch_bench

# SQL 操作基准
cargo bench --bench sql_operations

# Lexer 基准
cargo bench --bench lexer_bench

# Parser 基准
cargo bench --bench parser_bench

# 向量索引基准
cargo bench --bench vector_benchmark

# 图数据库基准
cargo bench --bench graph_benchmark
```

---

## 7. 关键命令参考

### 7.1 构建命令

| 命令 | 说明 |
|------|------|
| `cargo build --all-features` | 全特性构建 |
| `cargo build -p <crate>` | 构建指定 crate |
| `cargo check --all-features` | 编译检查 |
| `cargo run --bin sqlrustgo` | 运行主程序 |

### 7.2 测试命令

| 命令 | 说明 |
|------|------|
| `cargo test --all-features` | 运行所有测试 |
| `cargo test -p <crate>` | 测试指定 crate |
| `cargo test <pattern>` | 运行匹配测试 |
| `cargo test --doc` | 文档测试 |

### 7.3 质量检查命令

| 命令 | 说明 |
|------|------|
| `cargo clippy --all-features` | Lint 检查 |
| `cargo fmt --check` | 格式检查 |
| `cargo audit` | 安全审计 |

### 7.4 门禁检查脚本

```bash
# 文档链接检查
bash scripts/gate/check_docs_links.sh

# 覆盖率检查
bash scripts/gate/check_coverage.sh

# L0 Smoke 测试
bash scripts/gate/check_l0_smoke.sh

# SQL 兼容性检查
bash scripts/gate/check_sql_compat.sh

# Alpha Gate
bash scripts/gate/check_alpha_v310.sh

# Beta Gate
bash scripts/gate/check_beta_v310.sh

# RC Gate
bash scripts/gate/check_rc_v310.sh
```

### 7.5 常用环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `RUST_LOG` | 日志级别 | `info` |
| `SQLRUSTGO_DATA_DIR` | 数据目录 | `./data` |
| `SQLRUSTGO_WAL_DIR` | WAL 目录 | `./wal` |
| `SQLRUSTGO_THREADS` | 工作线程数 | CPU 核心数 |

---

## 8. Workspace Crates 完整列表

### 8.1 核心 crates (Core)

| Crate | 路径 | 说明 |
|-------|------|------|
| `sqlrustgo-parser` | `crates/parser` | SQL 解析器 |
| `sqlrustgo-planner` | `crates/planner` | 查询计划器 |
| `sqlrustgo-optimizer` | `crates/optimizer` | 查询优化器 |
| `sqlrustgo-executor` | `crates/executor` | 执行引擎 |
| `sqlrustgo-expr` | `crates/expr` | 表达式求值 |
| `sqlrustgo-storage` | `crates/storage` | 存储引擎 |
| `sqlrustgo-transaction` | `crates/transaction` | 事务管理 |
| `sqlrustgo-catalog` | `crates/catalog` | 元数据管理 |
| `sqlrustgo-types` | `crates/types` | 核心类型 |
| `sqlrustgo-common` | `crates/common` | 公共工具 |

### 8.2 扩展 crates (Extensions)

| Crate | 路径 | 说明 |
|-------|------|------|
| `sqlrustgo-vector` | `crates/vector` | 向量索引 |
| `sqlrustgo-graph` | `crates/graph` | 图数据库 |
| `sqlrustgo-gis` | `crates/gis` | GIS 支持 |
| `sqlrustgo-gmp` | `crates/gmp` | GMP 追溯 |
| `sqlrustgo-rag` | `crates/rag` | RAG 支持 |

### 8.3 分布式 crates (Distributed)

| Crate | 路径 | 说明 |
|-------|------|------|
| `sqlrustgo-distributed` | `crates/distributed` | 分布式支持 |
| `sqlrustgo-sync` | `crates/sync` | 数据同步 |
| `sqlrustgo-network` | `crates/network` | 网络通信 |
| `sqlrustgo-unified-storage` | `crates/unified-storage` | 统一存储 |
| `sqlrustgo-unified-query` | `crates/unified-query` | 统一查询 |

### 8.4 工具 crates (Tools)

| Crate | 路径 | 说明 |
|-------|------|------|
| `sqlrustgo-server` | `crates/server` | HTTP 服务器 |
| `sqlrustgo-sql-cli` | `crates/sql-cli` | SQL CLI |
| `sqlrustgo-bench` | `crates/bench` | 基准测试 |
| `sqlrustgo-bench-cli` | `crates/bench-cli` | 基准测试 CLI |
| `sqlrustgo-tools` | `crates/tools` | 工具集 |
| `sqlrustgo-sql-corpus` | `crates/sql-corpus` | SQL 语料库 |
| `sqlrustgo-sqlancer` | `crates/sqlancer` | SQL 模糊测试 |

### 8.5 支持 crates (Support)

| Crate | 路径 | 说明 |
|-------|------|------|
| `sqlrustgo-telemetry` | `crates/telemetry` | 遥测 |
| `sqlrustgo-security` | `crates/security` | 安全 |
| `sqlrustgo-observability` | `crates/observability` | 可观测性 |
| `sqlrustgo-fuzz` | `crates/fuzz` | 模糊测试 |
| `sqlrustgo-spill` | `crates/spill` | 溢出处理 |
| `information-schema` | `crates/information-schema` | Information Schema |

### 8.6 测试 crates (Testing)

| Crate | 路径 | 说明 |
|-------|------|------|
| `test-registry` | `crates/test-registry` | 测试注册 |
| `test-runner` | `crates/test-runner` | 测试运行器 |
| `test-results` | `crates/test-results` | 测试结果 |
| `test-reporter` | `crates/test-reporter` | 测试报告 |
| `transaction-stress` | `crates/transaction-stress` | 事务压力测试 |

---

## 附录

### A. 目录结构速查

```
sqlrustgo/
├── crates/                 # Workspace 成员 (46 个 crate)
│   ├── parser/            # SQL 解析器
│   ├── planner/           # 查询计划
│   ├── optimizer/         # 查询优化
│   ├── executor/          # 执行引擎
│   ├── storage/          # 存储引擎
│   ├── transaction/       # 事务管理
│   ├── catalog/           # 元数据
│   ├── types/             # 类型系统
│   ├── expr/              # 表达式
│   ├── vector/            # 向量索引
│   ├── graph/             # 图数据库
│   ├── gis/               # GIS 支持
│   ├── gmp/               # GMP 追溯
│   ├── distributed/        # 分布式支持
│   ├── unified-query/     # 统一查询
│   ├── network/           # 网络通信
│   ├── server/            # HTTP 服务器
│   ├── observability/      # 可观测性
│   ├── security/          # 安全
│   └── ...
├── src/                   # 根 crate (主入口)
├── tests/                 # 集成测试
├── benches/               # 基准测试
├── scripts/               # 工具脚本
│   └── gate/             # 门禁脚本
└── docs/                 # 文档
```

### B. 相关文档

- [架构文档](./architecture.md)
- [开发指南](./dev_guide_1823.md)
- [测试计划](./TEST_PLAN.md)
- [版本路线图](./ROADMAP.md)
- [分支策略](./BRANCH_GOVERNANCE.md)

---

## 9. 源码验证与精确信息（基于 develop/v3.3.0 实际代码分析）

> 以下内容基于 2026-05-18 对 develop/v3.3.0 分支（commit 0f9d74ff）的完整源码读取验证，修正了概要描述中的偏差。

### 9.1 ExecutionEngine 实际架构

`src/execution_engine.rs` 是整个系统的核心胶水层，**不是**简单的 SQL 执行入口，而是一个全功能执行引擎：

```rust
pub struct ExecutionEngine<S: StorageEngine> {
    storage: Arc<RwLock<S>>,
    catalog: Option<Arc<RwLock<Catalog>>>,
    stats: Arc<RwLock<ExecutionStats>>,
    cbo_enabled: bool,
    stats_enabled: bool,
    transaction_manager: TransactionManager,
    current_tx_id: Option<TxId>,
    default_isolation: TmIsolationLevel,
    current_role: Option<String>,
    current_user: Option<UserIdentity>,
    query_cache: Arc<RwLock<QueryCache>>,
    cache_config: QueryCacheConfig,
    workflow_engine: WorkflowEngine,
    prepared_inserts: HashMap<String, InsertStatement>,
}
```

**关键发现**：
- ExecutionEngine 内嵌了 `TransactionManager`，直接管理事务生命周期
- 集成了 `WorkflowEngine`（GMP 工作流），不是独立调用
- 集成了 `QueryCache` + `QueryCacheConfig`，查询缓存是引擎级特性
- 支持 CBO（`cbo_enabled`）和统计信息收集（`stats_enabled`）
- 使用 `ObservabilityState` 实现可观测性集成
- CTE 支持通过 `CteContext` 实现，CTE 结果物化为临时表

### 9.2 Transaction 模块实际结构

实际代码比文档描述更丰富：

| 子模块 | 功能 |
|--------|------|
| `mvcc.rs` | MVCC 核心（Snapshot, Transaction, TxId） |
| `ssi.rs` | SSI 冲突检测（SerializationGraph, SsiDetector, SireadLock） |
| `transaction_manager.rs` | 事务管理器（含幂等性支持：IdempotentBeginResult） |
| `lock.rs` | 锁定义（LockMode, LockTarget） |
| `lock_manager.rs` | 分布式锁管理器（DistributedLockManager） |
| `deadlock.rs` | 死锁检测 |
| `savepoint.rs` | 保存点支持 |
| `idempotency.rs` | 幂等性保证 |
| `audit.rs` | 事务审计 |
| `version_chain.rs` | 版本链 |
| `wait_for_graph.rs` | 等待图（死锁检测用） |
| `mvcc_storage.rs` | MVCC 存储适配 |
| `dtc.rs` | 分布式事务协调 |
| `gid.rs` | 全局事务 ID（NodeId） |

**关键导出**：

```rust
pub use mvcc::{Snapshot, Transaction, TxId, INVALID_TX_ID};
pub use ssi::{SerializationGraph, SireadLock, SsiDetector, SsiDetectorSync, SsiError};
pub use transaction_manager::{
    ActiveTransaction, IdempotencyOrSsiError, IdempotentBeginResult,
    IsolationLevel, TransactionManager, TransactionState,
};
pub use lock::{LockMode, LockTarget};
pub use lock_manager::DistributedLockManager;
pub use gid::NodeId;
```

### 9.3 Storage 模块实际结构

实际代码包含更多子模块：

| 子模块 | 功能 |
|--------|------|
| `engine.rs` | StorageEngine trait + MemoryStorage |
| `buffer_pool.rs` | LRU Buffer Pool + EncryptedBufferPool |
| `file_storage.rs` | 文件存储（47k 行，最大文件之一） |
| `wal.rs` | WAL 预写日志（58k 行，最大文件） |
| `page.rs` | Page 结构 |
| `bplus_tree/` | B+Tree 索引 |
| `clustered_index/` | 聚簇索引（ClusteredLeafPage, ClusteredWalManager） |
| `row_format.rs` | 行编码/解码（VarLenSlot, OverflowPage） |
| `fts/` | 全文索引（SimpleTokenizer, ChineseTokenizer, MultiLanguageTokenizer） |
| `recovery.rs` | 恢复（PITR, WalChainState, RecoveryVerifier, PageChecksum） |
| `backup.rs` + `backup_storage.rs` | 备份恢复 |
| `binary_format.rs` + `binary_storage.rs` | 二进制格式存储 |
| `encryption.rs` | AES 加密（AesEncryptionManager, Crypt） |
| `encrypted_file.rs` | 加密文件存储 |
| `key_manager.rs` | 密钥管理（BasicKeyManager, KeyManager） |
| `storage_tier.rs` + `tier_manager.rs` | 分层存储（热/温/冷） |
| `read_write_split.rs` | 读写分离 |
| `predicate.rs` | 谓词下推 |
| `page_access_tracker.rs` | 页面访问追踪 |

**关键发现**：
- 支持加密存储（AES + 密钥管理）
- 分层存储（StorageTier + TierManager）
- 完整的 PITR（Point-In-Time Recovery）支持
- 行格式支持溢出页（OverflowPage）和变长字段（VarLenSlot）

### 9.4 GMP 模块实际结构

GMP 模块远比文档描述的复杂，是一个完整的 GMP 合规框架：

| 子模块 | 功能 | 合规标准 |
|--------|------|----------|
| `audit.rs` | 审计日志 | ALCOA+ |
| `audit_chain.rs` | 防篡改审计链 | 21 CFR Part 11 |
| `audit_chain_tamper.rs` | 篡改检测 | 数据完整性 |
| `audit_chain_wal.rs` | 审计链 WAL 集成 | 持久化 |
| `electronic_signature.rs` | 电子签名 | 21 CFR Part 11 |
| `signature.rs` | 数字签名（ECDSA, Ed25519） | eIDAS |
| `immutable_record.rs` | 不可变记录 | ALCOA+ |
| `correction.rs` + `correction_chain.rs` | 修正链 | ALCOA+ |
| `provenance.rs` + `provenance_lineage.rs` | 数据溯源 | ISO 8000 |
| `evidence.rs` + `evidence_api.rs` + `evidence_storage.rs` + `evidence_verification.rs` | 证据链系统 | EBR |
| `hsm/` | HSM/TPM/KMS 集成 | ISO 27001 |
| `mobile/` | 移动端可信采集 | FDA 21 CFR Part 11 |
| `sop/` | SOP/培训绑定 | GMP SOP |
| `calibration/` | 设备校准 | GMP 校准 |
| `workflow/` | 审批工作流引擎 | GMP Workflow |
| `document.rs` | 文档管理 | GMP 文档 |
| `embedding.rs` + `vector_search.rs` + `semantic_embedding.rs` | 向量搜索 | - |
| `compliance.rs` | 合规检查 | 多标准 |
| `report.rs` | 报告生成（审计/偏差/CAPA） | GMP 报告 |
| `sql_api.rs` | SQL API（GmpExecutor） | - |
| `persist_sqlite.rs` | SQLite 持久化 | - |
| `scenarios.rs` | GMP 场景测试 | - |

### 9.5 AgentSQL 模块实际结构

AgentSQL 是一个 AI 辅助 SQL 模块：

| 子模块 | 功能 |
|--------|------|
| `nl2sql.rs` | 自然语言转 SQL（Nl2SqlService） |
| `schema.rs` | Schema 服务（表/视图元数据） |
| `stats.rs` | 统计信息服务 |
| `memory.rs` | Agent 记忆服务（会话/查询/长期记忆） |
| `explain.rs` | 执行计划解释服务 |
| `optimizer.rs` | SQL 优化建议服务 |
| `policy_engine.rs` | 访问策略引擎 |
| `column_masking.rs` | 列级数据脱敏 |
| `gateway.rs` | SQL 网关 |

### 9.6 Network 模块实际结构

Network 模块基于 Protobuf + gRPC，实现了 DTC（分布式事务协调器）协议：

```protobuf
// 关键消息类型
message PrepareRequest { gid, coordinator_node_id, changes[] }
message VoteResponse { gid, node_id, vote, reason }
message CommitRequest { gid }
message RollbackRequest { gid, reason }
message ExecutionResponse { gid, node_id, success, affected_rows, error }
message Change { table, operation, key, value }
message Notification { gid, node_id, type, details }
```

- 使用 `prost` 进行 Protobuf 序列化
- 支持 VoteCommit/VoteAbort 投票
- 支持 NodePing/NodePong 心跳

### 9.7 RAG 模块实际结构

RAG 模块实现了完整的检索增强生成管线：

| 组件 | 功能 |
|------|------|
| `RAGPipeline` | 核心检索管线（倒排索引 + 文档管理） |
| `KnowledgeBase` | 知识库（版本化文档 + 多源支持） |
| `OpenClawClient` | OpenClaw API 客户端 |
| `InvertedIndex` | 倒排索引 |
| `Tokenizer` | 分词器（Simple/Chinese/MultiLanguage） |

**关键特性**：
- 支持文档 CRUD + 版本追踪
- 支持知识库序列化/反序列化（JSON）
- 支持 SQL 存储集成（`save_to_storage` / `load_from_storage`）
- 支持 SQL LIKE 查询和按源过滤

### 9.8 Distributed 模块实际结构

分布式模块功能非常完整：

| 子模块 | 功能 |
|--------|------|
| `partition.rs` | 分区策略（Hash/Range） |
| `shard_manager.rs` + `shard_router.rs` | 分片管理和路由 |
| `raft.rs` | Raft 共识协议 |
| `group_replication.rs` + `group_replication_raft.rs` | 组复制 |
| `replication.rs` | 主从复制（Binlog + GTID） |
| `semisync.rs` | 半同步复制（AFTER_SYNC/AFTER_COMMIT） |
| `two_phase_commit.rs` | 两阶段提交 |
| `xa_coordinator.rs` + `xa_mvcc_bridge.rs` | XA 分布式事务 |
| `distributed_lock.rs` | 分布式锁 |
| `failover_manager.rs` | 故障转移 |
| `read_write_splitter.rs` | 读写分离 |
| `grpc_client.rs` + `grpc_server.rs` | gRPC 通信 |
| `cross_shard_query.rs` | 跨分片查询 |
| `consensus.rs` | 共识管理 |

---

## 10. GitCode Wiki 对比分析

### 10.1 GitCode Wiki 现状

**GitCode Wiki 地址**: https://gitcode.com/BreavHeart/sqlrustgo/wiki

**实际情况**: GitCode Wiki **无法访问**（返回"页面找不到或无权限"）。可能原因：
1. Wiki 功能未启用
2. 权限设置为私有
3. Wiki 内容尚未创建

### 10.2 本地 Wiki 知识库对比

项目在内部 Gitea（192.168.0.252:3000）维护了 LLM-Wiki 和 GBrain 知识库：

| 知识库 | 地址 | 用途 |
|--------|------|------|
| **LLM-Wiki** | `ssh://git@192.168.0.252:222/openclaw/sqlrustgo-wiki.git` | 项目文档、架构决策记录 |
| **GBrain** | `ssh://git@192.168.0.252:222/openclaw/ai-brain.git` | 可检索知识图谱、规则、模式库 |

本地知识库结构：
```
~/wiki/gbrain/sqlrustgo/
├── rules/           # 治理规则、编码规范
├── patterns/        # 重复问题模式
├── architecture/    # 架构决策记录 (ADR)
└── decisions/       # 具体决策备忘
```

### 10.3 本文档 vs GitCode Wiki 对比

| 维度 | 本文档 (CODE_WIKI.md) | GitCode Wiki |
|------|----------------------|--------------|
| **可访问性** | ✅ 随代码仓库分发 | ❌ 不可访问 |
| **架构描述** | ✅ 7层架构 + 查询流程图 | ❌ 无 |
| **模块详解** | ✅ 16+ 模块深度分析 | ❌ 无 |
| **源码验证** | ✅ 基于 commit 0f9d74ff 实际代码 | ❌ 无 |
| **数据结构** | ✅ 关键 struct/trait/enum | ❌ 无 |
| **依赖关系** | ✅ 层次图 + 依赖矩阵 | ❌ 无 |
| **运行方式** | ✅ 完整命令参考 | ❌ 无 |
| **GMP 合规** | ✅ 详细模块分析 | ❌ 无 |
| **分布式** | ✅ 完整组件列表 | ❌ 无 |
| **门禁体系** | ✅ 脚本 + 检查项 | ❌ 无 |

### 10.4 建议

1. **启用 GitCode Wiki**: 将本文档内容同步到 GitCode Wiki，提升外部可见性
2. **Wiki 结构建议**:
   - Home → 项目概述
   - Architecture → 架构设计
   - Modules → 模块详解（每个模块一个页面）
   - API → 公开接口文档
   - Deployment → 部署和运行
   - GMP-Compliance → GMP 合规框架
   - Distributed → 分布式架构
3. **同步机制**: 建议在 CI 中自动将 `docs/CODE_WIKI.md` 同步到 GitCode Wiki

---

## 11. 项目质量指标（v3.2.0 RC 实测）

| 指标 | 值 |
|------|-----|
| L1 覆盖率 | 85.81% |
| TPC-H SF=1 | 22/22 PASS |
| Point Select QPS | ≥30K |
| UPDATE QPS | ≥58K |
| DELETE QPS | ≥62K |
| 并发连接 | 200+ |
| GMP 测试 | 354+ tests PASS |
| RC Gate | 28/32 PASS |
| Clippy | 零警告 |
| Format | 通过 |

---

## 12. v3.3.0 新增内容

### 12.1 新增文件

| 文件 | 说明 |
|------|------|
| `docs/releases/v3.3.0/README.md` | v3.3.0 版本说明 |
| `docs/releases/v3.3.0/CHANGELOG.md` | v3.3.0 变更日志 |
| `docs/releases/v3.3.0/DEV_PLAN.md` | v3.3.0 开发计划（AI 辅助开发 + 多模型协作策略） |
| `scripts/gate/check_alpha_v330.sh` | v3.3.0 Alpha Gate 脚本 |

### 12.2 v3.3.0 开发方向

根据 DEV_PLAN.md，v3.3.0 的核心方向是：
- AI 辅助开发策略
- 多模型协作（DeepSeek + ChatGPT + Claude）
- AgentSQL 增强
- 性能持续优化

---

## 更新日志

| 日期 | 版本 | 更新内容 |
|------|------|---------|
| 2026-05-18 | v3.3.0 | 初始版本，增加 v3.3.0 新模块文档 |
| 2026-05-18 | v3.3.0-r2 | 源码验证补充（Section 9-12）：ExecutionEngine/Transaction/Storage/GMP/AgentSQL/Network/RAG/Distributed 实际结构；GitCode Wiki 对比分析；质量指标；v3.3.0 新增内容 |

---

*本文档由 AI 自动生成，基于 develop/v3.3.0 分支（commit 0f9d74ff）实际源码分析验证。*
