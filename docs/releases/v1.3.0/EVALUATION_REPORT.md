# v1.3.0 评测报告

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta

---

## 一、评测状态

> ⚠️ **Beta 阶段说明**: P0 功能（向量索引、Layer 0 管线）尚未完成，以下数据为 v1.2.0 基线。v1.3.0 完整评测待 L0-Alpha Gate 完成后执行。

---

## 二、391 场景评测（基线）

> 数据来源: v1.2.0 最终评测

| 指标 | v1.2.0 结果 | 通过标准 | 状态 |
|------|-------------|---------|------|
| 场景通过率 | 96.08% (392/408) | ≥ 96.08% | ✅ |
| MRR | 0.497 | ≥ 0.497 | ✅ |
| 向量通道 recall@20 | 待填充 | ≥ 70% | 🔄 |
| FTS 通道 recall@20 | 待填充 | ≥ 60% | 🔄 |

---

## 三、v1.3.0 预期提升

### 3.1 向量索引就绪后

| 指标 | v1.2.0 (无向量) | v1.3.0 预期 | 提升来源 |
|------|-----------------|-------------|---------|
| 向量通道 recall@20 | N/A | ≥ 70% | HNSW 索引自动就绪 |
| 391 场景通过率 | 96.08% | ≥ 97% | 向量 + 图谱联合检索 |
| MRR | 0.497 | ≥ 0.50 | 向量语义增强 |

### 3.2 Layer 0 管线就绪后

| 指标 | 预期值 | 说明 |
|------|--------|------|
| NER 召回率 | ≥ 85% | 术语库 + LLM 辅助 |
| 术语覆盖度 | ≥ 90% | 600+ GMP 术语 |
| 图谱节点数 | > 5000 | Layer 0 管线扩展 |
| 图谱边数 | > 20000 | 关系抽取生成 |

---

## 四、评测命令

### 4.1 391 场景评测

```bash
# 评测命令
gmp-eval --test-set test-data/391_scenarios.json --output eval_v130.json

# 评估 MRR
gmp-eval --test-set test-data/391_scenarios.json --metrics mrr --output mrr_v130.json

# 评估 Recall@K
gmp-eval --test-set test-data/391_scenarios.json --metrics recall --k 10,20
```

### 4.2 向量通道专项评测

```bash
# 仅向量通道
gmp-cli search "洁净区级别划分" --channels vector --top-k 20

# 四通道对比
gmp-cli search "洁净区级别划分" --channels all --top-k 20 --output four_channel_eval.json
```

---

## 五、评测环境

| 组件 | 规格 |
|------|------|
| CPU | 待记录 |
| RAM | 待记录 |
| OS | 待记录 |
| Ollama 版本 | 待记录 |
| bge-m3 模型 | 待记录 |
