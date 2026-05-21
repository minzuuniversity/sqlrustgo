# GMP 内审检索系统 v1.0.0 用户手册

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro

---

## 1. 快速开始

### 1.1 安装

```bash
# 克隆项目
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform

# 编译
cargo build --release
# 二进制: target/release/gmp-retrieve

# 验证
./target/release/gmp-retrieve --version
```

### 1.2 前置要求

```bash
# 1. 确保 Ollama 运行并加载模型
ollama pull bge-m3:latest
ollama pull bge-reranker-v2-m3:latest
ollama pull qwen2.5:14b

# 2. 验证模型加载
curl http://localhost:11434/api/tags | grep -E "bge-m3|bge-reranker|qwen2.5"
```

---

## 2. 数据索引构建

### 2.1 首次索引

```bash
# 全量导入 (文档 + 向量化 + 图谱)
gmp-retrieve index --source gmp-md/ --source agent-knowledge/

# 预计耗时: 2-4 小时 (1696 文档, CPU)
# 产出: gmp.db (SQLRustGo 数据库文件)
```

### 2.2 验证索引

```bash
# 检查完整性
gmp-retrieve index --validate

# 输出示例:
# Documents:  1696/1696
# Chunks:     8742
# Vectors:    8742 (HNSW)
# Nodes:      723
# Edges:      1581
```

### 2.3 增量更新 (P1)

```bash
# 新增文档
gmp-retrieve index --source new_doc.md --append

# 重建向量索引
gmp-retrieve index --rebuild-vector
```

---

## 3. 检索查询

### 3.1 基本查询

```bash
# 全通道检索 (默认)
gmp-retrieve search "偏差分级标准"

# 指定通道
gmp-retrieve search "SOP起草流程" --channel fts      # 全文检索
gmp-retrieve search "CAPA实施" --channel vector       # 语义检索
gmp-retrieve search "质量标准" --channel rule          # 规则检索
gmp-retrieve search "混合机操作" --channel graph       # 图谱检索

# 启用 Reranker 重排
gmp-retrieve search "验证主计划" --rerank
```

### 3.2 调试模式

```bash
# 查看各通道分数
gmp-retrieve search "偏差与CAPA的关系" --all --explain

# 输出示例:
# [RuleChannel] score=2.0  "偏差管理规程"
# [VectorChannel] score=0.87  "CH05-偏差处理SOP.md" (cosine=0.87)
# [FtsChannel] score=3.2  "CAPA管理程序.md" (BM25)
# [GraphChannel] score=0.5  "纠正措施跟踪表.md" (2-hop)
# 
# [RRF Fused] Top-20:
#   1. "偏差管理规程.md" (2.156)
#   2. "CAPA管理程序.md" (1.892)
#   ...
```

### 3.3 检索结果解读

```text
Rank  Score    Channel      Document
---   -----    -------      --------
1     2.156    Rule+FTS     偏差管理规程.md
2     1.892    FTS+Vector   CAPA管理程序.md
3     1.743    Vector       纠正与预防措施SOP.md

Channel 标注:
  Rule    - 来自 agent-knowledge 规则库
  Vector  - 来自 bge-m3 语义匹配
  FTS     - 来自 BM25 全文匹配
  Graph   - 来自图谱邻居扩展
```

---

## 4. LLM 问答

### 4.1 交互式模式

```bash
gmp-retrieve shell

> 搜索: 如果发生重大偏差应该怎么处理？

Assistant:
根据相关文档，重大偏差的处理流程如下：

1. **立即报告**: 发现偏差后立即向质量部门报告 [偏差管理规程.md]
2. **偏差分类**: 根据影响程度分为重大/一般/微小 [偏差分级标准.md]
3. **启动 CAPA**: 重大偏差必须启动纠正与预防措施 [CAPA管理程序.md]
4. **调查记录**: 填写偏差调查表，记录根因分析 [偏差调查表模板.md]
...

[引用来源]:
- 偏差管理规程.md: §3.2 偏差处理流程
- 偏差分级标准.md: §2.1 分级定义
- CAPA管理程序.md: §4.1 CAPA启动条件
```

### 4.2 意图分类

系统自动识别 7 种查询意图：

| 意图 | 示例 | 行为 |
|------|------|------|
| RetrieveSop | "SOP起草流程" | 检索原文 |
| CheckCompliance | "这样操作合规吗" | 合规性判定 |
| CompareRegulation | "EU GMP vs 中国 GMP 偏差管理" | 跨法规对比 |
| TraceProcess | "偏差发现后的处理链路" | 流程追溯 |
| ClassifyIncident | "液体渗漏属于什么级别偏差" | 事件分类 |
| GenerateChecklist | "生成清洁验证检查清单" | 生成清单 |
| AnalyzeRootCause | "为什么CAPA执行率低" | 根因分析 |

---

## 5. 391 场景评估

### 5.1 批量评估

```bash
# 全量 391 场景评估
gmp-retrieve evaluate --test-set gmp_kg_build/test_results_final/391_scenarios.json --output results.json

# 快速抽查
gmp-retrieve evaluate --test-set 391_scenarios.json --limit 20

# 查看结果
# results.json:
{
  "total": 391,
  "correct": 251,
  "partial": 62,
  "incorrect": 60,
  "unanswerable": 18,
  "pass_rate": "80.1%"
}
```

### 5.2 按类别分析

```bash
# 查看各场景类别通过率
gmp-retrieve evaluate --test-set 391_scenarios.json --by-category

# 输出:
# SOP:     85/100 (85.0%)
# 偏差:     42/55  (76.4%)
# 验证:     35/45  (77.8%)
# CAPA:    28/35  (80.0%)
# 法规:     55/66  (83.3%)
# 设备:     31/50  (62.0%)  ← 需补充数据
# 其他:     37/40  (92.5%)
```

---

## 6. 性能压测

```bash
# 单查询性能测量
time gmp-retrieve search "偏差与CAPA" --all --rerank

# 批量性能测试
gmp-retrieve bench --rounds 100
# 输出: P50=4.2s, P95=12.8s, P99=28.3s

# 并发测试
gmp-retrieve bench --concurrent 10 --rounds 100
# 输出: P50=22.4s, P95=56.7s, 0 crashes
```

---

## 7. 常见问题

### Q: 索引构建失败怎么办？

```bash
# 检查数据文件
ls gmp-md/*.md | wc -l    # 应为 1696

# 清理旧索引
rm -f gmp.db && gmp-retrieve index --source gmp-md/
```

### Q: Ollama 连接失败？

```bash
# 检查服务
curl http://localhost:11434/api/tags

# 如果失败，启动 Ollama
ollama serve
```

### Q: 向量搜索返回空？

```bash
# 验证向量索引
gmp-retrieve index --validate vector
# 如果 chunks=0，需要重新导入
```

### Q: LLM 回答编造信息？

系统在 prompt 中要求 "严格基于材料回答"，但 14B 模型可能存在幻觉。如发现编造，请在答案中查找 [需人工确认] 标记。

---

## 8. 配置

### 8.1 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `OLLAMA_HOST` | `http://localhost:11434` | Ollama 服务地址 |
| `GMP_DB_PATH` | `./gmp.db` | 数据库路径 |
| `LLM_MODEL` | `qwen2.5:14b` | LLM 模型名 |
| `EMBED_MODEL` | `bge-m3:latest` | 嵌入模型名 |
| `RERANK_MODEL` | `bge-reranker-v2-m3:latest` | Reranker 模型名 |

### 8.2 检索配置 (YAML)

```yaml
# config.yaml
retrieval:
  rrf_k: 60
  channel_weights:
    rule: 2.0
    vector: 1.0
    fts: 1.0
    graph: 1.0
  top_k_candidates: 20

reranker:
  model: "bge-reranker-v2-m3"
  mode: "cpu"  # cpu | gpu
  top_k: 10
  fallback: "noop"  # noop | error

llm:
  model: "qwen2.5:14b"
  temperature: 0.3
  max_tokens: 1024
  retry: 3
  timeout_secs: 30
```