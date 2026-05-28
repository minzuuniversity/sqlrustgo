# v2.4.0 开发计划

> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## 一、版本目标与约束

### 1.1 核心目标

**v2.4.0 战略定位**：Graph Intelligence — 图引擎与智能查询版本。

**战略演进**：
```
v2.2.0: 架构优化
    ↓
v2.3.0: 并行执行（开发代号）
    ↓
v2.4.0: Graph Intelligence（正式发布）
    ↓
v2.5.0: Full Integration + GMP
```

### 1.2 核心价值

- **图引擎产品化**：将 Graph Engine 从研究项目变为生产可用
- **智能查询**：OpenClaw API 支持自然语言查询
- **存储优化**：列式存储压缩提升性能
- **CBO 增强**：基于成本的索引选择优化

### 1.3 版本约束

| 约束 | 说明 |
|------|------|
| 必须基于 v2.3.0 | 合并 v2.3.0 所有功能 |
| Graph Engine 独立部署 | 独立 crate，不破坏现有架构 |
| API 兼容性 | Iterator 返回类型，lazy evaluation |

---

## 二、功能范围

### 2.1 核心模块

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **Graph Engine** | 图引擎核心、GQL Parser、遍历执行 | P0 |
| **OpenClaw API** | HTTP Server、REST 端点 | P0 |
| **列式存储压缩** | LZ4/Zstd 压缩 | P1 |
| **CBO 索引选择** | Cost-based optimizer | P1 |
| **TPC-H SF=1** | 完整基准测试 | P1 |

### 2.2 Graph Engine（P0）

#### 功能列表

| 功能 | 说明 |
|------|------|
| Graph Engine 核心架构 | 独立 `graph-engine` crate |
| GQL Parser | Graph Query Language 解析器 |
| Graph Planning | 图查询规划器 |
| Graph Execution | 图遍历执行器 |
| 属性图模型 | 顶点、边、属性 |

#### 技术选型

| 组件 | 技术 | 说明 |
|------|------|------|
| 图语言 | GQL (ISO/IEC 39075) | 标准化图查询语言 |
| 执行模型 | Volcano-style | 与 SQL 引擎统一 |
| 存储 | DiskGraphStore | WAL 持久化 |

### 2.3 OpenClaw API（P0）

#### API 设计

```typescript
// Query API
GET  /api/v1/query          // SQL 查询执行
GET  /api/v1/nl_query       // 自然语言查询
GET  /api/v1/schema         // Schema introspection
GET  /api/v1/stats          // 执行统计

// Memory API
GET  /api/v1/memory/search  // 记忆搜索
POST /api/v1/memory/store   // 记忆存储
DELETE /api/v1/memory/:id   // 记忆删除
```

### 2.4 列式存储压缩（P1）

#### 压缩算法

| 算法 | 用途 | 压缩率 |
|------|------|--------|
| LZ4 | 高速压缩 | ~2x |
| Zstd | 高压缩率 | ~3x |

#### 实现

```rust
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: SqlType,
    pub compression: Option<CompressionType>, // LZ4, Zstd, None
}
```

### 2.5 CBO 索引选择（P1）

#### 功能

| 功能 | 说明 |
|------|------|
| Cost-based optimizer | 基于成本的优化 |
| Index selection | 索引选择启发式算法 |
| Query planning | 查询计划优化 |

---

## 三、Issue 规划

### 3.1 P0 Issue（必须完成）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1077 | P0 | Graph Engine 核心 | GQL Parser + Execution | 4 周 |
| #1078 | P0 | OpenClaw API | REST Server + Endpoints | 2 周 |

### 3.2 P1 Issue（Beta 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1302 | P1 | 列式存储压缩 | LZ4 + Zstd | 1 周 |
| #1303 | P1 | CBO 索引选择 | Cost-based optimizer | 2 周 |
| #1304 | P1 | TPC-H SF=1 | 22/22 queries | 1 周 |

---

## 四、技术架构

### 4.1 整体架构

```
┌──────────────────────────────────────────────────────────────┐
│                    SQLRustGo v2.4.0                            │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Graph Engine Layer (graph-engine)          │  │
│  │  GQL Parser │ Graph Planner │ Graph Executor           │  │
│  │  DiskGraphStore │ WAL Integration                      │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    OpenClaw API                         │  │
│  │  /query │ /nl_query │ /schema │ /stats │ /memory/*    │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────┐  │
│  │           Columnar Storage Layer                        │  │
│  │  LZ4/Zstd Compression │ BloomFilter │ CBO Index        │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────┐  │
│  │                 Core Engine (v2.3)                      │  │
│  │  Executor │ Optimizer │ Storage │ Transaction          │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 项目结构

```
crates/
├── graph-engine/           # 图引擎 (新)
│   ├── src/
│   │   ├── parser/        # GQL Parser
│   │   ├── planner/       # Graph Planner
│   │   ├── executor/      # Graph Executor
│   │   └── storage/       # DiskGraphStore
│   └── Cargo.toml
│
├── sqlrustgo/             # 主入口
├── sqlrustgo-executor/    # 执行器
├── sqlrustgo-optimizer/   # 优化器 (CBO)
├── sqlrustgo-storage/     # 存储 (压缩)
└── ...
```

### 4.3 依赖关系

```
v2.3.0 (并行执行)
    │
    ├─ Parallel Execution
    │   └─ RayonTaskScheduler
    │
    └─ v2.4.0 (Graph Intelligence)
        ├─ Graph Engine (新 crate)
        │   └─ 依赖 core executor
        │
        ├─ OpenClaw API (新功能)
        │   └─ 依赖 planner/executor
        │
        └─ Columnar Compression (存储增强)
            └─ 依赖 storage
```

---

## 五、测试计划

### 5.1 测试分层

| 层级 | 测试类型 | 执行环境 | 频率 |
|------|----------|----------|------|
| L0 | 编译 + 格式检查 | CI | 每次 PR |
| L1 | 单元测试 (Graph Engine) | CI | 每次 PR |
| L2 | 集成测试 (API) | CI | 每次 PR |
| L3 | TPC-H SF=1 | Z6G4 | Beta 前 |
| L4 | 性能基准测试 | Z6G4 | RC 前 |

### 5.2 测试命令

```bash
# 编译测试
cargo build --all-features

# 单元测试
cargo test -p graph-engine
cargo test --lib

# 集成测试
cargo test --test openclaw_api_test
cargo test --test graph_integration_test

# TPC-H 基准测试
cargo test --test tpch_sf1

# 覆盖率
cargo llvm-cov test --lib
```

---

## 六、门禁

### 6.1 Alpha Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| A1 | Build | `cargo build --all-features` | 编译通过 |
| A2 | Test | `cargo test --lib` | 全部通过 |
| A3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| A4 | Format | `cargo fmt -- --check` | 通过 |
| A5 | Graph Engine | `cargo test -p graph-engine` | 通过 |
| A6 | OpenClaw API | `curl /api/v1/query` | 通过 |

### 6.2 Beta Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| B1 | Full Integration | `cargo test --workspace` | 全部通过 |
| B2 | TPC-H SF=1 | `cargo test --test tpch_sf1` | 22/22 PASS |
| B3 | Coverage ≥75% | `cargo llvm-cov` | ≥75% |
| B4 | Compression | `cargo test --test compression` | 通过 |
| B5 | CBO Index | `cargo test --test cbo_test` | 通过 |

### 6.3 RC Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| R1 | Build | `cargo build --release` | 通过 |
| R2 | Test | `cargo test --all-features` | 全部通过 |
| R3 | Coverage ≥80% | `cargo llvm-cov` | ≥80% |
| R4 | SQL Compat | `bash scripts/gate/check_sql_compat.sh` | ≥80% |
| R5 | Performance | TPC-H SF=1 < 10s | 通过 |

---

## 七、发布计划

### 7.1 版本阶段

| 阶段 | 目标日期 | 说明 |
|------|----------|------|
| Alpha | 2026-04-05 | Graph Engine + OpenClaw 完成 |
| Beta | 2026-04-07 | 集成测试通过 |
| RC | 2026-04-09 | 预发布版本 |

### 7.2 发布检查清单

- [ ] 所有 P0 Issue 完成
- [ ] Graph Engine 独立测试通过
- [ ] OpenClaw API 所有端点验证
- [ ] TPC-H SF=1 22/22 PASS
- [ ] 压缩功能验证
- [ ] CBO 索引选择验证

---

## 八、风险与应对

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| Graph Engine 复杂度 | 开发周期延长 | 中 | 独立 crate 隔离 |
| OpenClaw API 设计 | 后期重构 | 中 | 提前设计评审 |
| 压缩性能 | 吞吐量下降 | 低 | 可选压缩，配置化 |

---

## 九、关联文档

| 文档 | 说明 |
|------|------|
| [CHANGELOG.md](CHANGELOG.md) | 变更记录 |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | 发布说明 |
| [INTEGRATION_TEST_REPORT.md](INTEGRATION_TEST_REPORT.md) | 集成测试报告 |
| [PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) | 性能报告 |

---

## 十、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-04-09 | 初始版本 |

---

*文档版本: 1.0*
*生成日期: 2026-05-29*
*Truthfulness Score: 60%*