# v2.3.0 测试计划

> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## 测试策略

v2.3.0 测试计划基于 [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) 中的测试任务制定。

---

## 测试范围

### P0 功能测试

| 功能 | 测试内容 | 测试方法 |
|------|----------|----------|
| ParallelAggregateExecutor | 聚合并行执行 | 并行聚合 100 万行数据 |
| COUNT/SUM/AVG 并行 | 分组聚合 | 多核分组聚合验证 |
| GROUP BY 并行 | 分组并行 | 多核分组执行验证 |

### P0 性能基准测试

| 功能 | 测试内容 | 验收条件 |
|------|----------|----------|
| 并行扫描 | 1000 万行扫描 | ≥ 3x 加速 (8 核) |
| 并行 HashJoin | 100 万行 join | ≥ 2.5x 加速 (8 核) |
| 并行 Aggregate | 分组聚合 | ≥ 2x 加速 (8 核) |

### P1 功能测试

| 功能 | 测试内容 | 测试方法 |
|------|----------|----------|
| ParallelFilterExecutor | 过滤并行执行 | 并行过滤大数据集 |
| 谓词下推 | 优化效果验证 | 谓词下推前后对比 |
| 自适应并行度 | 动态调整验证 | 不同负载下的并行度 |
| 数据局部性 | 核间数据移动验证 | NUMA 架构测试 |

---

## 测试层级

| 层级 | 范围 | 执行环境 | 目标 |
|------|------|----------|------|
| L0 | 编译 + 格式检查 | CI | 零错误 |
| L1 | 单元测试 (并行组件) | CI | ≥ 80% |
| L2 | 集成测试 (并行执行) | CI | 通过 |
| L3 | 性能基准测试 | Z6G4 | 达到加速比目标 |

---

## 测试命令

```bash
# 编译测试
cargo build --all-features

# 单元测试
cargo test --lib -p sqlrustgo-executor

# 集成测试
cargo test --test parallel_integration

# 性能基准测试
cargo test --test parallel_bench -- --nocapture
```

---

## 性能测试用例

| 用例 | 场景 | 验收条件 |
|------|------|----------|
| PB-01 | 并行扫描 1000 万行 | ≥ 3x 加速 (8 核) |
| PB-02 | 并行 HashJoin 100 万行 | ≥ 2.5x 加速 (8 核) |
| PB-03 | 并行 Aggregate 分组聚合 | ≥ 2x 加速 (8 核) |
| PB-04 | 小数据集自动降级 | 无并行开销 |

---

## 加速比验证方法

```bash
# 对比顺序执行 vs 并行执行
sequential_time=$(cargo test --test bench_seq -- --nocapture)
parallel_time=$(cargo test --test bench_parallel -- --nocapture)
speedup=$(echo "scale=2; $sequential_time / $parallel_time" | bc)
echo "Speedup: ${speedup}x"
```

---

## 相关文档

| 文档 | 说明 |
|------|------|
| [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) | 开发计划（测试任务来源） |
| [CHANGELOG.md](CHANGELOG.md) | 变更记录 |

---

*最后更新: 2026-04-09*
*Truthfulness Score: 60%*