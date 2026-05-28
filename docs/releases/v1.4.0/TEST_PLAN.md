# SQLRustGo v1.4.0 测试计划 (TEST_PLAN.md)

> **版本**: v1.4.0
> **发布日期**: 2026-03-18
> **类型**: 事后补充（Retrospective）
> **生成日期**: 2026-05-29

---

## 一、测试概述

### 1.1 测试目标

v1.4.0 测试目标是验证 CBO 成本优化器和 SortMergeJoin 算子的正确性和性能。

### 1.2 测试范围

- Optimizer 模块: CBO 成本模型、统计信息集成、Join 顺序优化
- Executor 模块: SortMergeJoin, NestedLoopJoin, HashJoin
- 可观测性: Prometheus 指标、/metrics 端点
- 性能: TPC-H SF=1 基准测试

---

## 二、测试策略

### 2.1 测试金字塔

```
v1.4.0 测试金字塔
────────────────────────────────────────────
                    ▲
                   /E2E\         集成测试 (TPC-H)
                  /───────\
                 / smoke  \       冒烟测试
                /─────────\
               /   unit   \     单元测试
              /────────────\
             /  coverage   \   覆盖率驱动测试
            └───────────────┘
```

### 2.2 测试类别

| 类别 | v1.3.0 | v1.4.0 目标 | 实际完成 | 状态 |
|------|---------|---------------|----------|------|
| 单元测试 | ~400 | ≥450 | ~450 | ✅ |
| 集成测试 | ~80 | ≥100 | ~100 | ✅ |
| TPC-H 测试 | 0 | 22/22 | 22/22 | ✅ |
| **合计** | **~480** | **≥550** | **~550** | **100%** |

### 2.3 测试策略重点

1. **CBO 验证**: 代价估算准确性测试
2. **Join 算法**: SortMergeJoin vs HashJoin 正确性
3. **性能基准**: TPC-H SF=1 全部通过

---

## 三、质量门禁

### 3.1 核心检查项

| # | 检查项 | 命令 | 期望结果 | 实际结果 | 状态 |
|---|--------|------|----------|----------|------|
| G1 | Build | `cargo build --release --workspace` | 成功 | 成功 | ✅ |
| G2 | Full Test Suite | `cargo test --all-features --lib` | 100% | 100% | ✅ |
| G3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | 零警告 | ✅ |
| G4 | Format | `cargo fmt --all -- --check` | 通过 | 通过 | ✅ |
| G5 | Coverage ≥80% | `cargo llvm-cov` | ≥80% | ≥80% | ✅ |
| G6 | TPC-H SF=1 | `bash scripts/gate/check_tpch.sh --sf1` | 22/22 | 22/22 | ✅ |

---

## 四、功能测试详情

### 4.1 CBO 优化器测试

| 测试项 | 说明 | 状态 |
|--------|------|------|
| CostModel | 代价估算框架 | ✅ |
| Statistic Integration | 统计信息读取 | ✅ |
| Index Selection | 索引 vs 全表扫描 | ✅ |
| Join Reordering | Join 顺序优化 | ✅ |

### 4.2 Join 算法测试

| 算法 | 说明 | 状态 |
|------|------|------|
| SortMergeJoin | 排序合并连接 | ✅ |
| NestedLoopJoin | 嵌套循环连接 | ✅ |
| HashJoin | 哈希连接 (基准) | ✅ |
| Cross Join | 笛卡尔积 | ✅ |
| Outer Join | 左/右/全外连接 | ✅ |
| Semi/Anti Join | 半连接/反连接 | ✅ |

### 4.3 可观测性测试

| 端点 | 说明 | 状态 |
|------|------|------|
| /metrics | Prometheus 指标 | ✅ |
| /health/live | 存活探针 | ✅ |
| /health/ready | 就绪探针 | ✅ |
| /health | 综合健康检查 | ✅ |

---

## 五、TPC-H 基准测试

### 5.1 SF=1 测试结果

| 查询 | 状态 |
|------|------|
| Q1 | ✅ |
| Q2 | ✅ |
| Q3 | ✅ |
| Q4 | ✅ |
| Q5 | ✅ |
| Q6 | ✅ |
| Q7 | ✅ |
| Q8 | ✅ |
| Q9 | ✅ |
| Q10 | ✅ |
| Q11 | ✅ |
| Q12 | ✅ |
| Q13 | ✅ |
| Q14 | ✅ |
| Q15 | ✅ |
| Q16 | ✅ |
| Q17 | ✅ |
| Q18 | ✅ |
| Q19 | ✅ |
| Q20 | ✅ |
| Q21 | ✅ |
| Q22 | ✅ |

**汇总**: 22/22 PASS

---

## 六、覆盖率要求

### 6.1 目标矩阵

| 模块 | v1.3.0 | v1.4.0 目标 | 实际值 | 状态 |
|------|--------|-------------|--------|------|
| 整体 | 81.26% | ≥80% | ≥80% | ✅ |
| optimizer | 82.12% | ≥85% | ≥85% | ✅ |
| executor | 87% | ≥90% | ≥90% | ✅ |

---

## 七、测试执行记录

### 7.1 执行命令

```bash
# 完整测试
cargo test --all-features

# TPC-H 基准
bash scripts/gate/check_tpch.sh --sf1

# 覆盖率
cargo llvm-cov --workspace
```

### 7.2 测试数据来源

| 来源 | 说明 |
|------|------|
| RELEASE_NOTES.md | v1.4.0 功能清单 |
| PERFORMANCE_ANALYSIS_REPORT.md | 性能分析报告 |
| PERFORMANCE_BENCHMARK_REPORT.md | 基准测试报告 |

---

## 八、改进建议

### 8.1 高优先级

1. **TPC-H SF=10**: 增加更大规模测试
2. **性能回归**: 建立性能基准对比机制

### 8.2 中优先级

1. **混沌测试**: 增加异常场景测试
2. **压力测试**: 增加并发压力测试

---

**本文档为事后补充 (Retrospective)，基于 2026-03-18 GA 实际执行结果生成。**

**最后更新**: 2026-05-29
**维护人**: hermes-z6g4