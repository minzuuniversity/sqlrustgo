# v1.0.0 性能目标

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **目标环境**: CPU 16 核, RAM 32 GB, 无 GPU

---

## 检索性能目标

| 指标 | 目标 | 测量条件 |
|------|------|----------|
| 单查询 P50 (无 Reranker) | < 3s | search --all |
| 单查询 P50 (全链路) | < 5s | search --all --rerank |
| 单查询 P95 (全链路) | < 15s | search --all --rerank |
| 单查询 P99 (全链路) | < 30s | search --all --rerank |
| 10 并发 P50 | < 30s | bench --concurrent 10 |
| 索引构建 (1696 文档) | < 4h | index --source gmp-md/ |
| 矢量嵌入生成 (per chunk) | < 500ms | bge-m3 |

---

## 延迟分解（预估）

| 阶段 | P50 延迟 | 说明 |
|------|----------|------|
| 规则召回 | < 10ms | 内存 Hash 匹配 |
| 向量搜索 | 200-500ms | bge-m3 嵌入 + HNSW |
| 全文搜索 | 50-100ms | BM25 评分计算 |
| 图谱搜索 | 100-200ms | Cypher + BFS |
| RRF 融合 | < 5ms | 纯计算 |
| Reranker (per chunk) | 2-5s | bge-reranker-v2-m3 (CPU) |
| Reranker (20 chunks) | 40-100s | 实际只 rerank top-20 |
| **Reranker 优化** | **~2-5s** | 先 rerank top-5, 不够再扩大 |
| LLM 答案生成 | 2-8s | qwen2.5:14b |
| **总计** | **~5-14s** | 单查询全链路 |

---

## 391 场景目标

| 阶段 | 通过率目标 | 说明 |
|------|-----------|------|
| Alpha (Week 2) | ≥ 65% | Layer 2 四通道检索 |
| Beta (Week 3) | ≥ 78% | + Reranker + LLM |
| RC (Week 4) | ≥ 80% | 全链路 + 修复 |
| GA (Week 4) | ≥ 80% | 无退化 |

---

## 资源使用目标

| 指标 | 目标 |
|------|------|
| 二进制大小 | < 50MB |
| 内存占用 (闲置) | < 500MB |
| 内存占用 (查询) | < 4GB |
| 磁盘占用 (数据库) | < 5GB |
| 磁盘占用 (索引) | < 10GB |