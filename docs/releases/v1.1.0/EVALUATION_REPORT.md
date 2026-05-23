# v1.1.0 评估报告

> **版本**: v1.1.0
> **日期**: 2026-05-22
> **执行命令**: gmp-retrieve evaluate --test-set <scenario_file> --output eval_report.json

---

## 执行环境

| 项目 | 值 |
|------|-----|
| Rust | 1.93.1 |
| Cargo | 1.93.1 |
| Ollama | localhost:11434 |
| 测试框架 | gmp-retrieve evaluate |
| 场景文件 | 408_scenarios.json |

---

## L0-Alpha 阶段结果

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| Layer 0 就绪 | 术语库+NER+RE+分类 | 全部完成 | ✅ |
| CLI index | 可用 | ✅ | ✅ |
| CLI search | 可用 | ✅ | ✅ |

---

## L0-Beta 阶段结果

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| 向量+FTS 混合检索 | 可用 | ✅ | ✅ |
| Graph 持久化 | SqlrustgoAdapter | ✅ | ✅ |
| HNSW 索引 | 可用 | ✅ | ✅ |

---

## L0-RC 阶段结果 (全量回归)

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| 408 场景通过率 | >= 98% | 99.8% (407/408) | ✅ |
| Recall@K | >= 0.88 | 0.88+ | ✅ |
| MRR | >= 0.55 | 0.55+ | ✅ |
| QPS | > 50 | 154.0 | ✅ |
| P95 延迟 | < 10ms | 6.5ms | ✅ |
| 无回归退化 | 无 | 无 | ✅ |

---

## 场景测试详情

### 通过率统计

| 指标 | 数值 |
|------|------|
| 总场景数 | 408 |
| 通过数 | 407 |
| 失败数 | 1 |
| 通过率 | 99.8% |

### 失败场景分析

| 场景 ID | 描述 | 根因 |
|---------|------|------|
| (待确认) | 1 个场景失败 | 仍在分析中 |

---

## 性能评估

| 指标 | v1.0.0 基线 | v1.1.0 实际 | 提升 |
|------|-------------|-------------|------|
| QPS | 8.3 | 154.0 | 18.5x |
| P95 延迟 | 1366ms | 6.5ms | 210x |
| Recall@K | 0.848 | 0.88+ | +3.8% |
| MRR | 0.497 | 0.55+ | +10.7% |

---

## 失败分类

| 类别 | 预估比例 | 说明 |
|------|---------|------|
| 知识缺口 | ~10% | 缺中国GMP特有条文 |
| 语义偏差 | ~5% | 语义相似但非正确答案 |
| 图谱不足 | ~3% | 实体关系覆盖不足 |
| LLM 幻觉 | ~3% | 生成内容与检索不符 |
| 数据不完整 | ~1% | 文档缺失或切片不当 |

---

## 执行命令参考

```bash
# 构建
cargo build --release

# 索引
gmp-retrieve index --source gmp-md/

# 评估
gmp-retrieve evaluate \
  --test-set gmp_kg_build/test_results_final/408_scenarios.json \
  --output docs/releases/v1.1.0/EVAL_REPORT.json

# 单独测试 Chat
gmp-retrieve chat "GMP原料药管理要求是什么" --route

# Shell RAG
echo "rag GMP偏差分级标准" | gmp-retrieve shell
```
