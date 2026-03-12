# SQLRustGo 数据库内核架构设计 v2.1（集成 Observability）

> **版本**: v2.1
> **状态**: 架构蓝图
> **目标**: 集成 Observability 系统详细设计
> **更新日期**: 2026-03-13

---

## 目录

1. [系统总体架构](#1-系统总体架构)
2. [核心数据结构](#2-核心数据结构)
3. [模块详解](#3-模块详解)
4. [版本演进路径（含 observability 里程碑）](#4-版本演进路径含-observability-里程碑)
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
- **可观测性内建**：从第一天起集成日志、指标、追踪、profiler

---

## 2. 核心数据结构

（与 v2.0 一致，见 ARCHITECTURE_V2.md）

---

## 3. 模块详解

### 3.1 ~ 3.11

（与 v2.0 一致，见 ARCHITECTURE_V2.md）

### 3.12 Observability（可观测性）

#### 3.12.1 总体设计

Observability 是贯穿整个数据库内核的横向模块。

**crate 结构**：

```
crates/observability
├── logging.rs       # 结构化日志
├── metrics.rs       # 指标收集与暴露
├── tracing.rs       # 执行链路追踪
└── profiler.rs     # 查询级性能剖析
```

**统一接口**：

```rust
pub fn init_logging();
pub fn init_metrics(service_name: &str, exporter_addr: SocketAddr);
pub fn init_tracing(service_name: &str, jaeger_endpoint: Option<String>);

pub fn record_query_start(sql: &str, query_id: &str);
pub fn record_query_end(query_id: &str, duration: Duration, rows: u64, error: Option<&SqlError>);
```

#### 3.12.2 Logging（日志）

**技术栈**: tracing + tracing-subscriber

**日志级别**:
- ERROR: 不可恢复错误
- WARN: 潜在问题
- INFO: 关键事件
- DEBUG: 算子执行细节
- TRACE: 表达式求值

**JSON 输出示例**:
```json
{"timestamp":"2026-03-13T10:00:00Z","level":"INFO","message":"query_start","sql":"SELECT * FROM t","query_id":"abc123"}
```

#### 3.12.3 Metrics（指标）

**技术栈**: metrics + metrics-exporter-prometheus

**暴露方式**: HTTP 端口 8080 `/metrics`

**指标分类**:

| 类别 | 指标 | 类型 |
|------|------|------|
| Query | sqlrustgo_queries_total | Counter |
| Query | sqlrustgo_query_duration_seconds | Histogram |
| Executor | sqlrustgo_executor_operator_duration | Histogram |
| Storage | sqlrustgo_buffer_pool_hits_total | Counter |
| Transaction | sqlrustgo_txns_active | Gauge |
| Optimizer | sqlrustgo_optimizer_rules_fired_total | Counter |
| Network | sqlrustgo_connections_active | Gauge |

#### 3.12.4 Tracing（追踪）

**目的**: 追踪单个查询的执行流程

**技术栈**: tracing + tracing-opentelemetry

```rust
fn next(&mut self) -> Result<Option<RecordBatch>> {
    let span = tracing::info_span!("hash_join");
    let _guard = span.enter();
    // 执行逻辑
}
```

#### 3.12.5 Profiling（性能剖析）

**核心功能**: EXPLAIN ANALYZE 输出

**数据结构**:
```rust
pub struct QueryProfile {
    pub query_id: String,
    pub root: ProfileNode,
}

pub struct ProfileNode {
    pub operator: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub rows: u64,
    pub children: Vec<ProfileNode>,
}
```

**输出示例**:
```
HashJoin (actual time=12ms rows=1000)
  ├── SeqScan (actual time=3ms rows=5000)
  └── SeqScan (actual time=2ms rows=2000)
```

---

## 4. 版本演进路径（含 observability 里程碑）

| 版本 | 代号 | 时间 | 核心功能 | Observability 新增 |
|------|------|------|----------|-------------------|
| **v1.3** | Query Engine | 2026 Q2 | Volcano 执行器基础 | ✅ tracing 基础 + metrics 框架 |
| **v1.4** | SQL Completion | 2026 Q3 | GROUP BY, ORDER BY, RBO | ✅ EXPLAIN ANALYZE |
| **v1.5** | Storage Foundation | 2026 Q4 | 存储 + MVCC | ✅ 存储指标 + 事务指标 |
| **v1.6** | Statistics & Catalog | 2027 Q1 | 统计信息 | ✅ 统计指标 |
| **v1.7** | Indexing | 2027 Q2 | B+Tree | ✅ 索引指标 |
| **v1.8** | Planner Upgrade | 2027 Q3 | 计划器重构 | ✅ 优化器指标 |
| **v1.9** | Concurrency | 2027 Q4 | 完整 MVCC | ✅ 锁指标 |
| **v2.0** | Cost Optimizer | 2028 Q1 | Cascades | ✅ 优化器 tracing |
| **v2.1** | Statistics Enhance | 2028 Q2 | 直方图 | ✅ 丰富指标 |
| **v2.2** | Parallel Execution | 2028 Q3 | 并行执行 | ✅ 并行指标 |
| **v3.0** | Vectorized Engine | 2028 Q4 | 向量化 | ✅ 向量化指标 |
| **v3.1** | Columnar Storage | 2029 Q1 | 列存 | ✅ 列存 I/O |
| **v3.2** | Distributed | 2029 Q2 | 分布式 | ✅ 分布式 tracing |

**关键里程碑**:
- **v1.3**: 可观测性基础奠定
- **v1.4**: Query Profiler 上线
- **v2.0**: 优化器可观测

---

## 5. 工程实践与质量保障

### 5.1 测试策略
- 单元测试
- 集成测试
- 基准测试 (Criterion)
- 覆盖率门禁: 整体 ≥70%

### 5.2 CI/CD 自动化
- GitHub Actions: test, clippy, fmt, benchmark
- 分支保护

### 5.3 可观测性测试
- metrics 集成测试
- profiler 准确性测试

---

## 6. 附录：关键代码结构示例

### 6.1 Observability 模块初始化

```rust
// crates/observability/src/lib.rs
pub use logging::init_logging;
pub use metrics::{init_metrics, record_*};
pub use tracing::init_tracing;
pub use profiler::{QueryProfiler, QueryProfile};
```

### 6.2 Executor 集成 Profiler

```rust
impl Executor for HashJoinExec {
    fn next(&mut self) -> Result<Option<RecordBatch>> {
        let _guard = self.profiler.start_operator("HashJoin");
        // 执行逻辑
        self.profiler.record_operator_rows(batch.row_count());
        Ok(batch)
    }
}
```

### 6.3 Profiler 输出格式化

```rust
impl QueryProfile {
    pub fn format_tree(&self) -> String {
        let mut output = String::new();
        self.format_node(&self.root, 0, &mut output);
        output
    }
}
```

---

## 结语

通过集成完整的 Observability 系统，SQLRustGo 将具备现代数据库应有的可观测能力：从基础的日志和指标，到深入的执行追踪和查询剖析。

**下一步**: 基于此架构创建各版本开发计划，并在 crates/observability 中实现 v1.3 基础框架。

---

*更新日期: 2026-03-13*
