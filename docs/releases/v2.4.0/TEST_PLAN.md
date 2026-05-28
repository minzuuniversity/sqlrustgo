# v2.4.0 测试计划

> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## 一、版本概述

v2.4.0 定位为 **Graph Intelligence** — 图引擎与智能查询版本。

### 1.1 战略演进

```
v2.2.0: 架构优化
    ↓
v2.3.0: 并行执行（开发代号）
    ↓
v2.4.0: Graph Intelligence（正式发布）
    ↓
v2.5.0: Full Integration + GMP
```

---

## 二、测试范围

### 2.1 核心功能模块

| 模块 | 功能 | 状态 |
|------|------|------|
| graph-engine | GQL Parser, Graph Planner, Graph Executor | P0 |
| OpenClaw API | REST Server, /query, /nl_query, /schema | P0 |
| Columnar Storage | LZ4/Zstd Compression | P1 |
| CBO Optimizer | Cost-based Index Selection | P1 |
| TPC-H SF=1 | 完整基准测试 | P1 |

---

## 三、测试策略

### 3.1 测试分层

| 层级 | 范围 | 执行环境 | 目标 |
|------|------|----------|------|
| L0 | 编译 + 格式检查 | CI | 零错误 |
| L1 | 单元测试 (Graph Engine, API) | CI | ≥80% |
| L2 | 集成测试 (Graph + SQL) | CI | 通过 |
| L3 | TPC-H SF=1 | Z6G4 | 22/22 PASS |
| L4 | 性能基准测试 | Z6G4 | 达到目标 |

### 3.2 测试类型

| 类型 | 命令 | 目标 |
|------|------|------|
| 单元测试 | `cargo test --lib` | 全部通过 |
| Graph Engine | `cargo test -p graph-engine` | 全部通过 |
| 集成测试 | `cargo test --test openclaw_api_test` | 全部通过 |
| TPC-H SF=1 | `cargo test --test tpch_sf1` | 22/22 PASS |
| 覆盖率 | `cargo llvm-cov test --lib` | ≥75% |

---

## 四、P0 功能测试

### 4.1 Graph Engine 测试

| 功能 | 测试内容 | 测试方法 |
|------|----------|----------|
| GQL Parser | MATCH/WHERE/RETURN 解析 | 解析测试用例 |
| Graph Planner | 逻辑计划生成 | 计划验证 |
| Graph Executor | BFS/DFS 遍历 | 执行结果验证 |
| DiskGraphStore | WAL 持久化 | 崩溃恢复测试 |

### 4.2 OpenClaw API 测试

| 端点 | 测试内容 | 验收条件 |
|------|----------|----------|
| `/api/v1/query` | SQL 查询执行 | 返回正确结果 |
| `/api/v1/nl_query` | 自然语言查询 | NL → SQL 转换 |
| `/api/v1/schema` | Schema introspection | 返回 schema |
| `/api/v1/stats` | 执行统计 | 返回统计数据 |
| `/api/v1/memory/*` | 记忆管理 | 存储/检索/删除 |

---

## 五、P1 功能测试

### 5.1 列式存储压缩测试

| 功能 | 测试内容 | 验收条件 |
|------|----------|----------|
| LZ4 压缩 | 压缩/解压缩验证 | 数据一致性 |
| Zstd 压缩 | 压缩/解压缩验证 | 数据一致性 |
| 压缩比 | 压缩效果测量 | LZ4 ~2x, Zstd ~3x |
| 性能影响 | 压缩对查询的影响 | 可接受范围内 |

### 5.2 CBO 索引选择测试

| 功能 | 测试内容 | 验收条件 |
|------|----------|----------|
| Cost 计算 | 成本模型验证 | 合理成本估算 |
| 索引选择 | 选择最优索引 | 验证选择正确 |
| 查询计划 | 优化计划生成 | 性能提升 |

### 5.3 TPC-H SF=1 测试

| 查询 | 目标 | 验收条件 |
|------|------|----------|
| Q1-Q22 | 全部通过 | 22/22 PASS |
| SF=1 性能 | < 10s total | 总时间 < 10s |

---

## 六、性能测试用例

| 用例 | 场景 | 验收条件 |
|------|------|----------|
| GB-01 | Graph BFS 遍历 10000 节点 | < 100ms |
| GB-02 | Graph DFS 遍历 10000 节点 | < 100ms |
| GB-03 | OpenClaw API 响应时间 | < 200ms |
| CB-01 | LZ4 压缩 1GB 数据 | ~2x 压缩率 |
| CB-02 | Zstd 压缩 1GB 数据 | ~3x 压缩率 |
| TP-01 | TPC-H SF=1 Q1 | < 500ms |
| TP-02 | TPC-H SF=1 All | < 10s |

---

## 七、测试命令

```bash
# 编译测试
cargo build --all-features

# 单元测试 (Graph Engine)
cargo test -p graph-engine

# 单元测试 (所有 crates)
cargo test --lib

# 集成测试 (OpenClaw API)
cargo test --test openclaw_api_test

# 集成测试 (Graph + SQL)
cargo test --test graph_integration_test

# TPC-H SF=1
cargo test --test tpch_sf1

# 覆盖率
cargo llvm-cov test --lib

# 压缩测试
cargo test --test compression_test

# CBO 测试
cargo test --test cbo_index_test
```

---

## 八、测试统计

### 8.1 单元测试

| Crate | 测试数 | 状态 |
|-------|--------|------|
| graph-engine | TBD | - |
| sqlrustgo-executor | 35 | ✅ 100% |

### 8.2 集成测试

| 测试 | 通过 | 总数 | 状态 |
|------|------|------|------|
| OpenClaw API | 11 | 11 | ✅ 100% |
| Graph Integration | TBD | - | - |
| TPC-H SF=1 | 11 | 11 | ✅ 100% |

---

## 九、相关文档

| 文档 | 说明 |
|------|------|
| [DEV_PLAN.md](DEV_PLAN.md) | 开发计划（测试任务来源） |
| [CHANGELOG.md](CHANGELOG.md) | 变更记录 |
| [INTEGRATION_TEST_REPORT.md](INTEGRATION_TEST_REPORT.md) | 集成测试报告 |
| [PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) | 性能报告 |

---

*最后更新: 2026-04-09*
*Truthfulness Score: 60%*