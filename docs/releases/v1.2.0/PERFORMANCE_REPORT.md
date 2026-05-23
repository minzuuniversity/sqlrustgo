# GMP-Platform v1.2.0 性能测试报告

> **版本**: v1.2.0 | **日期**: 2026-05-23 | **分支**: `develop/v1.2.0`
> **测试环境**: macOS, Apple Silicon | **Ollama**: 192.168.0.252:11434

---

## 一、测试方法

### 1.1 测试层次

| 层级 | 测试类型 | 工具 |
|------|---------|------|
| L0 | 单元性能测试 | `cargo test` (内嵌) |
| L1 | 集成性能测试 | `cargo test --test` |
| L2 | 端到端场景测试 | `gmp-eval` + `eval_hybrid_search.py` |
| L3 | QPS 压测 | `gmp-eval --bench-qps N` |
| L4 | **LLM-as-Judge 语义评估** | `gmp-eval --judge-mode` (新增) |

### 1.2 测试环境

```
OS: macOS (Apple Silicon)
CPU: Apple M-series
Memory: 16GB+
Storage: SSD
Ollama: 192.168.0.252:11434 (默认)
```

---

## 二、基准对比

### 2.1 v1.1.0 vs v1.2.0 核心指标

| 指标 | v1.1.0 | v1.2.0 | 变化 | 目标 | 状态 |
|------|--------|--------|------|------|------|
| 场景通过率 | 99.8% | **TBD** | — | 99%+ | ⏳ |
| QPS | 154.0 | **TBD** | — | 200+ | ⏳ |
| P95 延迟 | 6.5ms | **TBD** | — | <5ms | ⏳ |
| 向量通道通过率 | 0/89 | **TBD** | — | 50%+ | ⏳ |
| 图查询通过率 | 0% | **TBD** | — | 60%+ | ⏳ |
| Recall@K | — | **TBD** | — | 0.88+ | ⏳ |
| MRR | — | **TBD** | — | 0.55+ | ⏳ |

### 2.2 QPS 提升路线图

| 版本 | QPS | P95 | 提升倍数 |
|------|-----|-----|---------|
| v1.0.0 | 8.3 | 1366ms | 1.0x (基线) |
| v1.1.0 | 154.0 | 6.5ms | 17.6x |
| **v1.2.0** | **TBD** | **TBD** | **TBD** |

**v1.2.0 QPS 提升预期来源**:
1. HNSW 向量索引完全集成，减少 Ollama HTTP 开销
2. NL → Cypher 转换器减少无效查询
3. gmp-api 连接池优化

---

## 三、场景测试结果

### 3.1 测试概要

| 项目 | 结果 |
|------|------|
| 测试日期 | 2026-05-23 (计划) |
| 场景总数 | 408+ |
| 通过数 | **TBD** |
| 通过率 | **TBD** |
| 基准对比 | v1.1.0: 99.8% |

### 3.2 预期改进

| 分类 | v1.1.0 通过率 | v1.2.0 预期 | 改进项 |
|------|-------------|-----------|--------|
| 图查询 (部门角色) | 0% | 60%+ | NL→Cypher 转换器 |
| 向量语义检索 | 0% | 50%+ | HNSW 完全集成 |
| 多跳因果推理 | 0% | 40%+ | Cypher 可变长度路径 |
| 交叉场景 | 待统计 | 待统计 | RRF 权重优化 |

---

## 四、单元测试结果

### 4.1 测试统计

| 阶段 | 通过 | 失败 | 跳过 | 总计 |
|------|------|------|------|------|
| v1.2.0 | **TBD** | 0 | **TBD** | **TBD** |

### 4.2 预期跳过测试 (Z440 延续)

| 测试 | 原因 | 关联 |
|------|------|------|
| `test_cypher_create_and_query` | Z440 (预计 v1.2.0 修复) | DiskGraphStore 持久化 |
| `test_cypher_with_where` | Z440 | 同上 |
| `test_cypher_disk_store_persistence` | Z440 | 同上 |
| `test_search_fts_result_scoring` | BM25 pre-existing issue | 评分算法 |

---

## 五、QPS 压测详情

### 5.1 测试命令

```bash
./target/release/gmp-eval --db data/gmp.db --test-set test-data/408_scenarios.json --bench-qps 5
```

### 5.2 报告格式

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

---

## 六、代码质量检查

### 6.1 Format

| 检查 | 状态 | 说明 |
|------|------|------|
| `cargo fmt --check` | ⏳ | 待执行 |

### 6.2 Clippy

| 检查 | 状态 | 说明 |
|------|------|------|
| `cargo clippy --workspace --exclude gmp-cli` | ⏳ | 待执行 |
| `cargo clippy` (全部) | ⚠️ 9 PRE-EXISTING | gmp-cli params! 宏 (非本次引入) |

### 6.3 Security Audit

| 检查 | 状态 | 结果 |
|------|------|------|
| `cargo audit --no-fetch` | ⏳ | 待执行 |

---

## 七、P0 性能优化分析

### 7.1 P0-1: 向量通道完全集成

| 优化项 | 实现方式 | 预期效果 |
|--------|---------|---------|
| HNSW → RRF fusion | 将 HnswVectorIndex 接入 RetrievalEngine | 向量通道通过率 0%→50%+ |
| 并行通道查询 | tokio::join! 并行执行 FTS+Vector+Cypher | 延迟 -30% |

### 7.2 P0-2: NL → Cypher 转换器

| 优化项 | 实现方式 | 预期效果 |
|--------|---------|---------|
| 自然语言解析 | LLM 辅助 Cypher 生成 | 图查询通过率 0%→60%+ |
| Cypher 执行优化 | 预编译 Cypher AST | 查询延迟 -40% |

### 7.3 P0-3: LLM-as-Judge 评估

| 优化项 | 实现方式 | 预期效果 |
|--------|---------|---------|
| 语义相似度评估 | Ollama 评估检索结果相关性 | 评估准确性 +20% |
| 误判率降低 | 替代精确 doc_id 匹配 | 场景通过率 +5% |

### 7.4 P0-4: gmp-api 端到端集成

| 优化项 | 实现方式 | 预期效果 |
|--------|---------|---------|
| 连接池 | sqlrustgo 连接池优化 | 并发能力 +50% |
| E2E 测试覆盖 | 完整 API 端点测试 | API 可靠性 ≥ 99% |

---

## 八、测试执行命令

```bash
# 运行单元测试
cargo test -p gmp-storage -p gmp-core -p gmp-llm -p gmp-reranker

# 运行场景测试
python3 eval_hybrid_search.py
cargo run -p gmp-eval -- --db data/gmp.db --test-set test-data/408_scenarios.json

# QPS 压测
./target/release/gmp-eval --bench-qps 5

# LLM-as-Judge 评估 (新增)
./target/release/gmp-eval --judge-mode

# 回归检测
bash scripts/benchmarks/regression_check.sh

# 代码质量
cargo fmt --check
cargo clippy --workspace --exclude gmp-cli
cargo audit --no-fetch
```

---

## 九、结论

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| 场景通过率 | 99%+ | **TBD** | ⏳ |
| QPS | 200+ | **TBD** | ⏳ |
| P95 | <5ms | **TBD** | ⏳ |
| 向量通道通过率 | 50%+ | **TBD** | ⏳ |
| 图查询通过率 | 60%+ | **TBD** | ⏳ |
| 单元测试 | 全部通过 | **TBD** | ⏳ |
| 安全扫描 | 无新漏洞 | **TBD** | ⏳ |

**v1.2.0 性能目标待测试验证。**

---

## 十、附录: 性能基线数据

### A. v1.1.0 基线数据 (参考)

| 指标 | v1.1.0 实测值 |
|------|--------------|
| 场景通过率 | 99.8% (407/408) |
| QPS | 154.0 |
| P95 延迟 | 6.5ms |
| 单元测试 | 72 passed / 4 ignored |
| 安全扫描 | 3 allowed warnings |

### B. 性能目标分解

| 子系统 | 当前 (v1.1.0) | 目标 (v1.2.0) | 改进方式 |
|--------|--------------|--------------|---------|
| FTS 通道 | 99.8% | 99.8% | 保持 |
| 向量通道 | 0% | 50%+ | HNSW 集成 |
| 图查询通道 | 0% | 60%+ | NL→Cypher |
| RRF 融合 | 基准 | 基准 | 参数调优 |
| API 延迟 | N/A | -30% | 连接池 |
