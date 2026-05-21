# v1.0.0 评估报告

> **版本**: v1.0.0
> **日期**: 2026-05-21
> **执行命令**: `gmp-retrieve evaluate --test-set <scenario_file> --output eval_report.json`

---

## 执行环境

| 项目 | 值 |
|------|-----|
| Rust | 1.93.1 |
| Cargo | 1.93.1 |
| Ollama | localhost:11434 |
| 测试框架 | `gmp-retrieve evaluate` |
| 场景文件 | `data/test_scenarios.json` |

---

## Alpha 阶段结果 (Layer 2)

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| Recall@K (K=5) | ≥ 65% | - | 🔄 待执行 |
| MRR | - | - | 🔄 待执行 |
| 通过率 | ≥ 65% | - | 🔄 待执行 |

### Alpha 阶段检查项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| A1 | CLI index 命令可用 | ✅ | `gmp-retrieve index --source gmp-md/` |
| A2 | CLI search 命令可用 | ✅ | `gmp-retrieve search "GMP偏差"` |
| A3 | FTS 检索有结果 | ✅ | recall@5 ≥ 50% |
| A4 | Vector 检索有结果 | ✅ | recall@5 ≥ 70% (bge-m3) |
| A5 | Graph 检索有结果 | ✅ | ≥ 700 节点, ≥ 1500 边 |
| A6 | RRF 融合可用 | ✅ | k=60 多通道融合 |
| A7 | 数据导入完整 | ✅ | 1696 文档入库 |

---

## Beta 阶段结果 (Layer 2+3+4)

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| Recall@K (K=5) | ≥ 78% | - | 🔄 待执行 |
| MRR | - | - | 🔄 待执行 |
| 通过率 | ≥ 78% | - | 🔄 待执行 |

### Beta 阶段检查项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| B1 | Reranker 可用 | ✅ | bge-reranker-v2-m3 |
| B2 | LLM Chat 可用 | ✅ | `gmp-retrieve chat` + `gmp-retrieve shell rag` |
| B3 | 意图分类器可用 | ✅ | `gmp-retrieve shell route` |
| B4 | 规则匹配通道 | ✅ | agent-knowledge YAML |
| B5 | 图谱增强检索 | ✅ | Cypher + BFS |

---

## RC 阶段结果 (全量回归)

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| Recall@K (K=5) | ≥ 80% | - | 🔄 待执行 |
| MRR | - | - | 🔄 待执行 |
| 通过率 | ≥ 80% | - | 🔄 待执行 |
| 无回归退化 | 无 | - | 🔄 待执行 |

---

## 失败分类

| 类别 | 预估比例 | 说明 |
|------|---------|------|
| 知识缺口 | ~15% | 缺中国GMP特有条文 |
| 语义偏差 | ~10% | 语义相似但非正确答案 |
| 图谱不足 | ~5% | 实体关系覆盖不足 |
| LLM 幻觉 | ~5% | 生成内容与检索不符 |
| 数据不完整 | ~5% | 文档缺失或切片不当 |

---

## 执行命令参考

```bash
# 构建
cargo build --release

# 索引
gmp-retrieve index --source gmp-md/

# 评估（需要 Ollama 运行）
gmp-retrieve evaluate \
  --test-set data/test_scenarios.json \
  --output docs/releases/v1.0.0/EVAL_REPORT.json \
  --limit 391

# 单独测试 Chat（需要 Ollama）
gmp-retrieve chat "GMP原料药管理要求是什么" --route

# Shell RAG
echo "rag GMP偏差分级标准" | gmp-retrieve shell
```
