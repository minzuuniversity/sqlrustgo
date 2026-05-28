# v3.6.0 开发计划 — Architecture Convergence Release

> **版本**: 1.0
> **状态**: Draft
> **分支**: `develop/v3.6.0`
> **起点**: v3.5.0 GA (commit `5720805b`)
> **目标**: 统一执行架构 + Evidence-based AI + Benchmark Gate
> **定位**: Architecture Convergence Release（非 Feature Release）

---

## 一、版本定位

### 为什么叫 Architecture Convergence

v3.5.0 有 4 套执行模型并行：
- Volcano Executor（行式）
- Parallel Executor（并行执行）
- Stored Proc Runtime（过程式）
- Vector Search（向量检索）

这导致：
- 测试复杂度爆炸
- executor coverage 无法提升
- spill/exchange 无法统一插入
- SIMD 无法真正发挥

**v3.6.0 目标**：一次统一执行架构，打完 v4.x 的所有基础。

---

## 二、战略目标

### SQLRustGo v3.6.0 核心目标

```
Unified Execution Architecture = 向量化 + Pipeline + Benchmark Gate + Evidence AI
```

### 五个技术突破点

| # | 突破点 | 当前状态 | v3.6.0 目标 |
|---|--------|---------|-------------|
| 1 | 统一执行模型 | 4 套并行 | Chunk-based Pipeline |
| 2 | SIMD 向量化 | 无 | SIMDExpr + Vectorized Agg |
| 3 | Benchmark Gate | stub | Regression CI + History |
| 4 | Evidence AI | RAG chatbot | SQL-Verified AI |
| 5 | 分布式地基 | 无 | Exchange Operator |

---

## 三、执行架构统一

### 3.1 当前问题

```
executor/
├── volcano/        # 行式火山模型
├── parallel/       # 并行执行器
├── stored_proc/    # 1748 行过程式 runtime
└── vector_search/  # 向量检索
```

4 套模型混杂导致：
- stored_proc.rs 1748 行无测试
- 无法插入 exchange operator
- SIMD 无法统一
- spill 无法做

### 3.2 统一目标架构

```
executor/
├── chunk/              # 新增：Chunk 数据结构
│   ├── chunk.rs        # Chunk = Vec<ColumnVector>
│   ├── column_vector.rs
│   └── selection_vector.rs
├── pipeline/           # 新增：Pipeline 执行器
│   ├── pipeline.rs
│   ├── task.rs
│   ├── driver.rs
│   └── exchange.rs
├── vectorized/         # 新增：SIMD 向量化
│   ├── simd_expr.rs
│   ├── vector_filter.rs
│   └── vector_agg.rs
├── spill/             # 新增：Spill Framework
│   ├── spill_manager.rs
│   └── partitioned_spill.rs
├── adapters/           # 新增：兼容适配层
│   └── row_to_chunk.rs
└── proc/              # 重构：stored_proc 拆分
    ├── parser.rs
    ├── vm.rs
    ├── runtime.rs
    ├── opcode.rs
    └── builtin.rs
```

### 3.3 迁移策略

**Phase 1（不破坏现有功能）**：
- 新增 `Chunk` 数据结构
- 新增 `RowToChunkAdapter`
- 现有 volcano/parallel/stored_proc 不变

**Phase 2（逐步迁移）**：
- SeqScan → ChunkScan
- Filter → VectorFilter
- HashJoin → ChunkHashJoin

**Phase 3（完成统一）**：
- 删除旧 volcano adapter
- 所有 operator 走 Pipeline

---

## 四、SIMD 向量化

### 4.1 目标

使用 `std::simd`（Rust 稳定 SIMD），不手写 AVX512。

### 4.2 实施模块

| 模块 | 功能 |
|------|------|
| `simd_expr.rs` | 表达式求值（predicates, projections） |
| `vector_filter.rs` | 向量过滤（WHERE clause） |
| `vector_agg.rs` | 向量聚合（SUM, COUNT, AVG） |
| `vector_cmp.rs` | 向量比较（JOIN 条件） |

### 4.3 优先级

1. `vector_filter.rs` — 收益最高，影响最广
2. `simd_expr.rs` — 表达式求值
3. `vector_cmp.rs` — Hash Join 优化
4. `vector_agg.rs` — 聚合优化

---

## 五、Benchmark Gate

### 5.1 为什么必须现在做

现在是"优化器时代"：
- CI 不会因为性能回退变红
- 用户会 silently fail
- 比 crash 更危险

### 5.2 Benchmark 场景

| 场景 | 指标 | CI Threshold |
|------|------|-------------|
| Point Select | QPS | 基准 ±15% |
| Range Scan | rows/sec | 基准 ±15% |
| Hash Join | rows/sec | 基准 ±20% |
| Aggregation | rows/sec | 基准 ±15% |
| WAL fsync | latency | ≤ 基准 ×1.3 |
| TPC-H SF=1 | total time | ≤ 180s |
| Concurrent TXN | TPS | 基准 ±20% |

### 5.3 CI 集成

```yaml
# .github/workflows/benchmark-gate.yml
- name: Benchmark Gate
  run: |
    cargo bench --save-baseline main
    cargo bench --fail-under-baseline 0.85
```

### 5.4 输出格式

```json
{
  "scenario": "TPC-H SF=1 Q1",
  "qps": 12500,
  "p50_ms": 12.3,
  "p95_ms": 45.6,
  "rss_mb": 512,
  "baseline": 12000,
  "delta_pct": 4.2
}
```

---

## 六、Evidence-based AI

### 6.1 当前问题

RAG Chatbot 会产生幻觉，GMP 场景不可接受。

### 6.2 SQL-Verified AI 架构

```
User Question
    ↓
Planner LLM → 生成 SQL
    ↓
SQLRustGo 执行 SQL
    ↓
返回 Evidence：
  - rows
  - audit_chain
  - timestamp
  - signature
    ↓
Verifier LLM → 验证 Evidence
    ↓
Final AI Answer（含 Evidence ID）
```

### 6.3 强制证据格式

```json
{
  "evidence_id": "uuid",
  "sql": "SELECT ...",
  "rows": [...],
  "audit_chain": "...",
  "document_hash": "sha256",
  "confidence": 0.93,
  "rule_version": "v3.6.0"
}
```

### 6.4 G20 Evidence Integrity Gate

所有 AI SQL 执行必须包含 evidence trace，否则 gate fail。

---

## 七、分布式地基

### 7.1 正确路线（不是现在做分布式）

```
v3.6: 单机 pipeline + exchange interface
v3.7: pseudo-distributed
v3.8: shard routing
v4.0: Raft + distributed txn
```

### 7.2 v3.6 分布式地基

Exchange Operator 接口（不实现真正的分布式）：

```rust
trait ExchangeOperator {
    fn exchange(&self, chunk: Chunk) -> ChunkStream;
    fn broadcast(&self, chunk: Chunk);
    fn shuffle(&self, chunk: Chunk, partitions: usize);
}
```

---

## 八、PostgreSQL 最小兼容

### 8.1 v3.6 目标（BI/Driver 兼容）

| 特性 | 优先级 |
|------|--------|
| startup/auth | P0 |
| simple query | P0 |
| extended query | P0 |
| prepared statement | P0 |
| parameter binding | P0 |
| binary format | P0 |
| pg_catalog basics | P0 |
| COPY | P1 |
| transaction status | P1 |

### 8.2 不做

- logical replication
- vacuum compatibility
- PL/pgSQL
- trigger

---

## 九、版本门禁概览

| Gate | 检查项 | 标准 | 状态 |
|------|--------|------|------|
| A1-A5 | 基础构建 | 同 v3.5.0 | — |
| A6 | Chunk 模型 | 编译通过 | — |
| A7 | Pipeline 基础 | 编译通过 | — |
| B1-B5 | Beta 基础 | 同 v3.5.0 | — |
| B6 | Pipeline 迁移 | 30% operator 迁移 | — |
| B7 | SIMD 向量化 | 3/4 模块完成 | — |
| B8 | Benchmark Gate | 6/8 场景完成 | — |
| R1-R8 | RC 基础 | 同 v3.5.0 | — |
| R9 | Exchange 接口 | 接口定义完成 | — |
| R10 | Evidence AI P0 | SQL-Verified 流程 | — |
| G1-G5 | GA 基础 | 同 v3.5.0 | — |
| G6 | Pipeline 100% | 所有 operator 迁移 | — |
| G7 | SIMD 完成 | 4/4 模块 | — |
| G8 | Benchmark 8/8 | 全部场景完成 | — |
| G9 | Evidence AI | G20 Evidence Gate | — |
| G10 | PG 兼容 P0 | 8/8 特性完成 | — |

---

## 十、关键 Issue

| Issue | 描述 | 优先级 | 阶段 |
|-------|------|--------|------|
| #v360-chunk | Chunk 数据结构 + ColumnVector | P0 | Alpha |
| #v360-pipeline | PipelineExecutor 核心 | P0 | Alpha |
| #v360-simd | SIMD 向量化表达式 | P0 | Alpha |
| #v360-spill | Spill Framework | P1 | Beta |
| #v360-exchange | Exchange Operator 接口 | P1 | Beta |
| #v360-bench | Benchmark Gate CI | P0 | Alpha |
| #v360-evidence | Evidence-based AI | P1 | RC |
| #v360-proc | stored_proc 拆分 VM 化 | P1 | Alpha |
| #v360-pg | PostgreSQL 兼容 P0 | P2 | RC |
| #v360-opt | Optimizer 优化 | P1 | Beta |

---

*文档版本: 1.0 | 状态: Draft | 待 Tech Lead 评审*
