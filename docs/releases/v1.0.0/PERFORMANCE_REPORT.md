# GMP-Platform v1.0.0 性能测试报告

> 日期: 2026-05-21
> 测试环境: macOS, Apple Silicon

---

## 一、测试方法

### 1.1 测试层次

| 层级 | 测试类型 | 工具 |
|------|---------|------|
| L0 | 单元性能测试 | `cargo test` (内嵌) |
| L1 | 集成性能测试 | `cargo test --test` |
| L2 | 端到端场景测试 | `gmp evaluate` |
| L3 | 并发压测 | `gmp bench` |

### 1.2 测试环境

```
OS: macOS (Apple Silicon)
CPU: Apple M-series
Memory: 16GB+
Storage: SSD
```

---

## 二、单元性能测试结果

### 2.1 存储层 (gmp-storage)

| 测试 | 数据规模 | 结果 | 阈值 |
|------|---------|------|------|
| `test_search_fts_performance_100docs` | 100 docs | ✅ PASS | < 5s |
| `test_index_throughput_50docs` | 50 docs | ✅ PASS | < 10s |
| `test_search_result_ordering` | 1 doc | ✅ PASS | — |
| `test_concurrent_storage_instances` | 2 instances | ✅ PASS | — |

### 2.2 cosine_sim 性能

| 测试用例 | 向量维度 | 结果 |
|---------|---------|------|
| identical vectors | 3D | 0.0ms |
| orthogonal vectors | 3D | 0.0ms |
| 45-degree vectors | 2D | 0.0ms |
| multi-dimensional | 1536D | < 1ms |

### 2.3 增量索引

| 场景 | 第一次索引 | 第二次索引(无变化) | 增量效果 |
|------|-----------|-----------------|---------|
| 50 docs | ~2-3s | ~50ms | 跳过 50 docs |
| 100 docs | ~4-5s | ~100ms | 跳过 100 docs |

---

## 三、集成测试结果

| 测试 | 描述 | 结果 |
|------|------|------|
| `test_full_index_and_search_cycle` | 索引→搜索完整流程 | ✅ |
| `test_incremental_reindex_detects_changes` | 增量检测变化 | ✅ |
| `test_validate_with_multiple_docs` | 多文档验证 | ✅ |
| `test_search_fts_result_scoring` | 评分排序 | ✅ |
| `test_empty_directory_handling` | 空目录处理 | ✅ |
| `test_document_count_consistency` | 文档计数一致性 | ✅ |
| `test_search_vectors_empty_then_populated` | 向量搜索空→有数据 | ✅ |

---

## 四、`gmp bench` 基准测试

### 4.1 命令

```bash
gmp bench --concurrent 4 --rounds 10
```

### 4.2 预定义测试查询

| # | 查询 |
|---|------|
| 1 | GMP原料药管理要求 |
| 2 | 洁净区环境监测 |
| 3 | 偏差调查流程 |
| 4 | 变更控制程序 |
| 5 | 供应商审计 |
| 6 | 批记录审核 |
| 7 | 验证主计划 |
| 8 | 召回程序 |

### 4.3 报告格式

```
========== BENCHMARK REPORT ==========
Total operations: {count}
Total time: {ms} ms
QPS: {qps}
Latency (ms):
  Min:    {min}
  Avg:    {avg}
  P50:    {p50}
  P95:    {p95}
  P99:    {p99}
  Max:    {max}
=====================================
```

### 4.4 预期性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| QPS | > 50 | 每秒查询数 |
| P50 | < 20ms | 中位数延迟 |
| P95 | < 100ms | 95分位延迟 |
| P99 | < 200ms | 99分位延迟 |

> 注：实际性能取决于 Ollama 服务响应时间和文档规模。

---

## 五、性能优化建议

### 5.1 当前瓶颈

1. **FTS 搜索**: SQLite FTS5 在大规模文档下性能有限
2. **向量搜索**: 全量遍历计算 cosine similarity，无索引
3. **增量索引**: 每次仍需扫描所有文件（glob）

### 5.2 优化方向

| 问题 | 优化方案 | 优先级 |
|------|---------|--------|
| FTS 性能 | 分片索引 + 缓存 | P2 |
| 向量搜索 | 引入向量索引 (FAISS/SQNS) | P1 |
| 增量索引 | 文件watch监控 | P2 |

---

## 六、测试覆盖统计

| 类别 | 测试数 | 状态 |
|------|--------|------|
| 单元测试 (L0) | 117 | ✅ |
| 集成测试 (L1) | 7 | ✅ |
| 场景测试 (L2) | 0 | 🔄 待执行 |
| 性能压测 (L3) | CLI可用 | ✅ |
| **总计** | **124** | |

---

## 七、测试执行命令

```bash
# 运行所有测试
cargo test

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --tests

# 运行性能测试
cargo test --lib -- test_performance

# 运行 benchmark CLI
gmp bench --concurrent 4 --rounds 10 --db ./data/gmp.db

# 运行端到端场景测试 (需 Ollama)
gmp evaluate --db ./data/gmp.db --ollama-url http://localhost:11434
```
