# SQLRustGo v1.4.0 README

> **版本**: v1.4.0
> **发布日期**: 2026-03-18
> **代号**: CBO & Vectorization Ready
> **状态**: GA (正式发布)
> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## SQLRustGo v1.4.0

**查询优化与执行增强版**

SQLRustGo v1.4.0 专注于提升查询性能，通过 CBO 成本优化器和 SortMergeJoin 算子提供更智能的执行计划。

### 核心特性

| 特性 | 说明 |
|------|------|
| **CBO 成本优化器** | 基于代价的查询优化器框架 |
| **SortMergeJoin** | 新的连接算法 |
| **NestedLoopJoin** | 支持 Cross Join 和外连接 |
| **统计信息集成** | 从 ANALYZE 获取表统计信息 |
| **Prometheus 指标** | /metrics 端点 + Grafana Dashboard |
| **SIMD 向量化** | Vector 数据结构 + BatchIterator |

---

## 快速开始

```bash
# 构建
cargo build --all-features

# 运行 REPL
cargo run --bin sqlrustgo

# 运行测试
cargo test --all-features

# 运行 TPC-H 基准
bash scripts/gate/check_tpch.sh --sf1
```

---

## 文档

| 文档 | 说明 |
|------|------|
| [RELEASE_NOTES.md](./RELEASE_NOTES.md) | 发布说明 |
| [CHANGELOG.md](./CHANGELOG.md) | 变更日志 |
| [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) | 开发计划 |
| [TEST_PLAN.md](./TEST_PLAN.md) | 测试计划 |
| [TEST_MANUAL.md](./TEST_MANUAL.md) | 测试手册 |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 架构文档 |

---

## 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         v1.4.0 架构                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Query Input                                                    │
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Optimizer (CBO)                             │    │
│  │     Cost Model / Join Order / Index Selection            │    │
│  └─────────────────────────────────────────────────────────┘    │
│                           │                                        │
│                           ▼                                        │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Executor Layer                               │    │
│  │   SeqScan / IndexScan / HashJoin / SortMergeJoin       │    │
│  │   NestedLoopJoin / Projection / Filter                  │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 核心优化器规则

| 规则 | 说明 | 状态 |
|------|------|------|
| CostModel | 代价模型基础 | ✅ |
| Statistic Integration | 统计信息集成 | ✅ |
| Index Selection | 索引选择优化 | ✅ |
| Join Reordering | Join 顺序优化 | ✅ |

---

## Join 算法

| 算法 | 说明 | 状态 |
|------|------|------|
| HashJoin | 哈希连接 | ✅ |
| SortMergeJoin | 排序合并连接 | ✅ |
| NestedLoopJoin | 嵌套循环连接 | ✅ |
| Cross Join | 笛卡尔积 | ✅ |
| Outer Join | 左/右/全外连接 | ✅ |
| Semi/Anti Join | 半连接/反连接 | ✅ |

---

## 可观测性

| 端点 | 说明 | 状态 |
|------|------|------|
| /metrics | Prometheus 指标 | ✅ |
| /health/live | 存活探针 | ✅ |
| /health/ready | 就绪探针 | ✅ |
| /health | 综合健康检查 | ✅ |

---

## 性能基准

| 测试 | 说明 | 状态 |
|------|------|------|
| TPC-H SF=1 | 22/22 查询通过 | ✅ |
| CBO 基准 | 代价模型验证 | ✅ |
| 性能对比 | v1.3 vs v1.4 | ✅ |

---

## 质量门禁

| 检查项 | 状态 |
|--------|------|
| 编译通过 | ✅ |
| 测试通过 | ✅ |
| Clippy 检查 | ✅ |
| 格式检查 | ✅ |
| TPC-H SF=1 | ✅ |

---

## 资源

- **代码**: https://github.com/minzuuniversity/sqlrustgo
- **讨论**: https://github.com/minzuuniversity/sqlrustgo/discussions
- **问题**: https://github.com/minzuuniversity/sqlrustgo/issues

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）