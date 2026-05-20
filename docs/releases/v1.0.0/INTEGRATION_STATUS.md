# v1.0.0 功能集成状态

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **阶段状态**: Alpha (开发启动)

---

## 集成状态总览

| 模块 | 状态 | 依赖 | 阻塞 |
|------|------|------|------|
| SQLRustGo 增量补全 | ✅ 已完成 | Hermes PR 已合并 | 无 |
| 项目脚手架 | ⬜ 待开始 | Rust ≥ 1.80 | 无 |
| 数据导入 | ⬜ 待开始 | SQLRustGo storage crate | 无 |
| 向量索引 | ⬜ 待开始 | SQLRustGo vector crate + bge-m3 | 无 |
| 图谱构建 | ⬜ 待开始 | SQLRustGo graph crate | 无 |
| RuleChannel | ⬜ 待开始 | agent-knowledge 数据集 | 无 |
| VectorChannel | ⬜ 待开始 | bge-m3 ollama | 无 |
| FtsChannel | ⬜ 待开始 | SQLRustGo FTS BM25 | 无 |
| GraphChannel | ⬜ 待开始 | 图谱构建完成 | 图谱构建 |
| RRF 融合 | ⬜ 待开始 | 四通道可用 | 四通道 |
| Reranker | ⬜ 待开始 | bge-reranker Ollama | 无 |
| LLM 问答 | ⬜ 待开始 | qwen2.5 Ollama | 无 |
| CLI | ⬜ 待开始 | clap crate | 无 |
| 391 测试 | ⬜ 待开始 | 全管线可用 | 全管线 |

---

## 外部依赖状态

| 依赖 | 状态 | 版本 | 备注 |
|------|------|------|------|
| SQLRustGo | ✅ | v3.2.0 + Hermes PR | 统一数据底座 |
| Ollama | ⬜ | latest | 待部署模型 |
| bge-m3 | ⬜ | latest | 1024d 嵌入 |
| bge-reranker-v2-m3 | ⬜ | latest | Cross-Encoder |
| qwen2.5:14b | ⬜ | 14B | LLM 推理 |
| Rust toolchain | ⬜ | ≥ 1.80 | 编译环境 |

---

## 数据集状态

| 数据集 | 状态 | 规模 | 路径 |
|--------|------|------|------|
| gmp-md 文档集 | ✅ | 1696 MD | `gmp-md/` |
| agent-knowledge 规则 | ✅ | 55+ YAML | `agent-knowledge/` |
| 391 测试场景 | ✅ | 391 JSON | `gmp_kg_build/test_results_final/` |
| 中国 GMP 条文 | ❌ | 缺失 | P2 补充 |

---

> 图例: ✅ 就绪 | ⬜ 待就绪 | ❌ 缺失