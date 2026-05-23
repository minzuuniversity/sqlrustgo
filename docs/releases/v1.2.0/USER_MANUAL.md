# GMP-Platform v1.2.0 用户手册

> **版本**: v1.2.0 | **日期**: 2026-05-23

---

## 1. 概述

GMP-Platform 是 GMP 合规性检索系统，支持中文 GMP 文档的全文检索、向量检索和图谱查询。

**核心能力**:
- 全文检索 (FTS5 + LIKE n-gram fallback)
- 向量检索 (HNSW, bge-m3 embeddings) - v1.2.0 完全集成
- 图查询 (Cypher + NL→Cypher 转换器) - v1.2.0 新增
- 混合检索 (FTS + 向量 + 图谱融合)
- LLM-as-Judge 语义评估 - v1.2.0 新增

---

## 2. 快速开始

### 2.1 构建

```bash
cargo build --release -p gmp-eval
```

### 2.2 运行评估

```bash
# 完整场景测试
./target/release/gmp-eval \
  --db data/gmp.db \
  --test-set test-data/408_scenarios.json

# QPS 基准测试
./target/release/gmp-eval \
  --db data/gmp.db \
  --test-set test-data/408_scenarios.json \
  --bench-qps 5

# LLM-as-Judge 评估 (v1.2.0 新增)
./target/release/gmp-eval \
  --db data/gmp.db \
  --judge-mode
```

### 2.3 查询示例

```bash
# 单次查询
./target/release/gmp-eval \
  --db data/gmp.db \
  --query "偏差调查流程"

# Hybrid 模式 (FTS + 向量)
./target/release/gmp-eval \
  --db data/gmp.db \
  --enable-vector

# 图查询模式 (v1.2.0 新增)
./target/release/gmp-eval \
  --db data/gmp.db \
  --graph \
  --query "哪些部门负责偏差调查"

# 纯 FTS + LIKE
./target/release/gmp-eval \
  --db data/gmp.db \
  --no-hybrid
```

---

## 3. 检索策略

### 3.1 FTS5 全文检索

默认策略。使用 BM25 排名。

```sql
-- 直接 SQL 查询
SELECT doc_id, content FROM gmp_document_chunks
WHERE content MATCH '偏差调查';
```

### 3.2 LIKE n-gram fallback

当 FTS5 对中文多字查询召回为零时，自动触发 LIKE fallback。

```sql
-- 手动触发 LIKE 查询
SELECT doc_id, content FROM gmp_document_chunks
WHERE content LIKE '%偏差%' OR content LIKE '%调查%';
```

### 3.3 向量检索 (v1.2.0 完全集成)

使用 HNSW 索引进行语义相似度检索。

```bash
# 启用向量通道
./target/release/gmp-eval --enable-vector --query "偏差管理最佳实践"
```

### 3.4 图查询 (v1.2.0 新增 NL→Cypher)

自然语言自动转换为 Cypher 查询。

```bash
# 图查询模式
./target/release/gmp-eval --graph --query "偏差和CAPA有什么关系"

# 预期转换: MATCH (d:Deviation)-[:TRIGGERS]->(c:CAPA) RETURN d, c
```

### 3.5 混合检索

```bash
# 默认启用向量 + FTS + 图谱
./target/release/gmp-eval --enable-vector --graph
```

---

## 4. 知识图谱

### 4.1 GMP 术语查询

```bash
# 查询术语的同义词
./target/release/gmp-cli query-term "偏差"
```

### 4.2 图查询 (Cypher)

```bash
# 启动交互式 shell
./target/release/gmp-cli shell --db data/gmp.db

# Cypher 查询示例
MATCH (d:Deviation)-[:TRIGGERS]->(c:CAPA) RETURN d, c;

# 多跳查询
MATCH (d:Deviation)-[:CAUSES]->(c:CAPA)-[:VERIFIES]->(v:Verification) RETURN d, v;
```

### 4.3 图统计 (v1.2.0)

```bash
# 查看图数据统计
./target/release/gmp-cli graph-stats

# 预期输出:
# Nodes: 1234
# Edges: 5678
# Labels: [Deviation, CAPA, Document, Person, Department]
```

---

## 5. 配置

### 5.1 Ollama 配置

默认 Ollama 地址: `http://192.168.0.252:11434`

```bash
# 指定 Ollama URL
OLLAMA_BASE_URL=http://localhost:11434 ./target/release/gmp-eval ...

# 查看可用模型
curl http://localhost:11434/api/tags
```

### 5.2 数据库路径

```bash
./target/release/gmp-eval --db /path/to/gmp.db ...
```

### 5.3 通道权重配置

```yaml
# config.yaml
channels:
  rule:
    weight: 2.0
    enabled: true
  vector:
    weight: 1.0
    enabled: true
  fts:
    weight: 1.0
    enabled: true
  graph:
    weight: 1.0
    enabled: true

rrf:
  k: 60
```

---

## 6. 性能调优

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--top-k` | 10 | 返回结果数 |
| `--vector-top-k` | 10 | 向量检索数 |
| `--hybrid-rrf-k` | 60 | RRF 融合参数 |
| `--pool-size` | 10 | API 连接池大小 (v1.2.0) |

### 6.1 向量检索优化

```bash
# 增加向量检索数
./target/release/gmp-eval --vector-top-k 20 --query "偏差管理"

# 调整 HNSW 参数 (需编译时配置)
# ef_search: 搜索范围 (默认 100)
# m: 每个节点的连接数 (默认 16)
```

### 6.2 图查询优化

```bash
# 限制图查询深度
./target/release/gmp-cli shell --max-depth 3

# 启用图查询缓存
./target/release/gmp-cli shell --cache
```

---

## 7. LLM-as-Judge 评估 (v1.2.0 新增)

### 7.1 评估模式

```bash
# 启用 LLM 判断
./target/release/gmp-eval --judge-mode --test-set test-data/408_scenarios.json

# 纯语义评估（不检查 doc_id）
./target/release/gmp-eval --judge-only --query "偏差调查流程"
```

### 7.2 评估结果解读

| JudgeResult | 说明 |
|-------------|------|
| ExactMatch | doc_id 完全相等 |
| SemanticMatch | LLM 判断语义相关 |
| NoMatch | 不相关 |

---

## 8. API 使用 (v1.2.0)

### 8.1 REST API 端点

```bash
# 健康检查
curl http://localhost:8080/health

# 检索
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{"query": "偏差调查流程", "top_k": 10, "enable_graph": true}'

# 统计
curl http://localhost:8080/api/v1/stats
```

### 8.2 响应格式

```json
{
  "query": "偏差调查流程",
  "results": [
    {
      "doc_id": "GMP-2026-001",
      "content": "偏差调查应按照...",
      "score": 0.95,
      "channel": "hybrid",
      "references": ["[文档编号]"]
    }
  ],
  "metadata": {
    "total": 10,
    "latency_ms": 45,
    "channels_used": ["fts", "vector", "graph"]
  }
}
```

---

## 9. 故障排查

### 9.1 向量搜索返回空

原因: `gmp_embeddings` 表为空。

```bash
# 检查向量数据
sqlite3 data/gmp.db "SELECT COUNT(*) FROM gmp_embeddings;"

# 重建向量索引 (v1.2.0)
./target/release/gmp-eval index --rebuild-embeddings
```

### 9.2 图查询返回空

原因: NL→Cypher 转换失败或图数据为空。

```bash
# 检查图数据
./target/release/gmp-cli graph-stats

# 手动测试 Cypher
./target/release/gmp-cli shell --cypher "MATCH (n) RETURN count(n)"
```

### 9.3 检索结果不符合预期

运行场景测试查看具体失败用例:

```bash
./target/release/gmp-eval --db data/gmp.db --test-set test-data/408_scenarios.json --verbose

# LLM-as-Judge 调试
./target/release/gmp-eval --judge-mode --verbose --query "你的查询"
```

---

## 10. 升级说明

### 10.1 从 v1.1.0 升级

v1.2.0 完全兼容 v1.1.0，无需重建索引：

```bash
git checkout develop/v1.2.0
cargo build --release

# 验证
./target/release/gmp-eval --version
```

### 10.2 新功能一览

| 功能 | v1.1.0 | v1.2.0 |
|------|--------|--------|
| 向量通道 | 部分集成 | **完全集成** |
| NL→Cypher | ❌ | **新增** |
| LLM-as-Judge | ❌ | **新增** |
| 图统计 | ❌ | **新增** |
| API 连接池 | 简单连接 | **连接池优化** |
