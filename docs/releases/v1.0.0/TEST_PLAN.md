# GMP 内审检索系统 v1.0.0 测试计划

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **阶段状态**: Alpha

> **关联门禁规范**: [docs/governance/GATE_SPEC.md](../../governance/GATE_SPEC.md)

---

## 一、测试概述

### 1.1 测试目标

验证 GMP 内审检索系统 v1.0.0 的四层架构在 391 个 GMP 合规审核场景中达到 ≥ 80% 通过率，且性能满足 P50 < 5s、P95 < 15s。

### 1.2 测试范围

| 方面 | 包含 | 排除 |
|------|------|------|
| 功能 | 四通道检索, RRF 融合, Reranker 重排, LLM 问答, CLI 命令 | Web API |
| 数据 | 1696 gmp-md + 55+ agent-knowledge 规则 + 391 场景 | 中国 GMP 条文 |
| 性能 | P50/P95/P99 延迟, 10 并发压测 | 分布式/大规模集群 |
| 稳定性 | 连续 50 次查询无崩溃 | 7×24 长稳测试 |
| 安全 | cargo-audit, API Key 泄露检查 | 渗透测试 |

### 1.3 与 v22 基线的测试差异

| 测试类型 | v22 基线 | v1.0.0 | 变更 |
|----------|----------|--------|------|
| 检索通道 | 1 (FTS5 BM25) | 4 (规则/向量/全文/图谱) | +3 通道 |
| 391 测试 | 44.5% | ≥ 80% | +35.5% |
| 答案形式 | 片段返回 | RAG 生成 + 引用 | 全新 |
| 图谱测试 | 无 | BFS/Cypher 邻居召回 | 全新 |

### 1.4 成功标准

- [ ] 391 场景通过率 ≥ 80%
- [ ] 四通道各通道 recall@K 达标
- [ ] RRF 融合输出正确格式
- [ ] Reranker 重排后 Top-1 相关性 > 基线
- [ ] LLM 答案 100% 包含引用
- [ ] P50 < 5s, P95 < 15s (单查询)
- [ ] 10 并发 × 100 轮无崩溃
- [ ] cargo-audit 无高危漏洞

---

## 二、测试范围

### 2.1 功能测试范围

| 模块 | 测试内容 | 测试方法 | 优先级 |
|------|----------|----------|--------|
| 数据导入 | 1696 文档 → gmp_documents/chunks | 记录数验证 | P0 |
| 向量索引 | bge-m3 嵌入 → HNSW 索引 | 相似搜索验证 | P0 |
| 图谱构建 | 实体节点 + 关系边 | 节点/边计数 + Cypher 查询 | P0 |
| RuleChannel | agent-knowledge YAML 匹配 | 命中率 + 分数 | P0 |
| VectorChannel | bge-m3 + HNSW 检索 | recall@20 | P0 |
| FtsChannel | BM25 + FMM 分词 | recall@20 | P0 |
| GraphChannel | Cypher + BFS 邻居扩展 | 图谱召回数 | P0 |
| RRF 融合 | 四通道融合 → Top-20 | 融合结果排位一致性 | P0 |
| Reranker | bge-reranker-v2-m3 重排 | Top-1 相关性提升 | P0 |
| LLM 问答 | qwen2.5:14b RAG 生成 | 答案质量 + 引用完整性 | P0 |
| 意图分类 | 7 种意图 | 准确率 ≥ 80% | P0 |
| CLI | index/search/evaluate/shell | 各子命令可用 | P0 |

### 2.2 通道评估要求

| 通道 | 指标 | 要求 |
|------|------|------|
| RuleChannel | 命中率 | ≥ 64% (对齐 agent-knowledge 基线) |
| VectorChannel | recall@20 | ≥ 70% |
| FtsChannel | recall@20 | ≥ 50% |
| GraphChannel | 召回数 | ≥ 3 个关联文档 |

### 2.3 排除范围

- Web API 端点测试 (P2, v1.1.0)
- 多语言支持测试
- 分布式部署测试
- 长时间稳定性测试 (> 1 天)
- 渗透/模糊测试

---

## 三、测试环境

### 3.1 硬件环境

| 组件 | 配置 | 用途 |
|------|------|------|
| CPU | ≥ 16 核 | 嵌入生成 + BM25 索引 |
| RAM | ≥ 32 GB | 模型加载 + 向量索引 |
| 磁盘 | ≥ 20 GB SSD | 1696 文档 + HNSW + 图存储 |
| GPU | 可选 (RTX 5080) | Reranker 加速 (当前 CPU fallback) |

### 3.2 软件环境

| 软件 | 版本 | 用途 |
|------|------|------|
| Rust | ≥ 1.80 | 编译 + 测试 |
| Ollama | latest | bge-m3 / reranker / qwen2.5 推理 |
| bge-m3 | latest | 文档嵌入 (1024d) |
| bge-reranker-v2-m3 | latest | Cross-Encoder 重排 |
| qwen2.5:14b | 14B | LLM 答案生成 |
| SQLRustGo | v3.2.0 + Hermes PR | 统一数据底座 |
| macOS / Linux | — | 运行环境 |

### 3.3 测试数据

| 数据集 | 路径 | 规模 | 用途 |
|--------|------|------|------|
| gmp-md | `gmp-md/` | 1696 MD | 主文档集 |
| agent-knowledge | `agent-knowledge/` | 55+ YAML | 规则库 |
| 391 scenarios | `gmp_kg_build/test_results_final/` | 391 JSON | 测试集 |

---

## 四、功能测试矩阵

### 4.1 Alpha Gate 功能测试 (A1-A9)

| # | 测试项 | 输入 | 期望结果 | 优先级 |
|---|--------|------|----------|--------|
| A1 | 编译测试 | `cargo build --release` | 无错误 | P0 |
| A2 | 单元测试 | `cargo test` | ≥ 80% 通过 | P0 |
| A3 | Clippy | `cargo clippy -- -D warnings` | 零警告 | P0 |
| A4 | 格式化 | `cargo fmt -- --check` | 无错误 | P0 |
| A5 | 数据完整性 | `gmp-retrieve index --validate` | 1696/1696 | P0 |
| A6 | FTS 查询 | `gmp-retrieve search "SOP" --channel fts` | ≥ 5 结果 | P0 |
| A7 | 向量查询 | `gmp-retrieve search "偏差" --channel vector` | ≥ 5 结果 | P0 |
| A8 | 图谱查询 | `gmp-retrieve search "CAPA" --channel graph` | ≥ 3 结果 | P0 |
| A9 | RRF 融合 | `gmp-retrieve search "偏差分级" --all` | 多通道融合 | P0 |

### 4.2 Beta Gate 功能测试 (B1-B9)

| # | 测试项 | 输入 | 期望结果 | 优先级 |
|---|--------|------|----------|--------|
| B1-B4 | 同 A1-A4 | — | — | P0 |
| B5 | CLI 命令 | `gmp-retrieve --help` + 子命令 | 全部可用 | P0 |
| B6 | Reranker | `gmp-retrieve search "SOP起草" --rerank` | 重排后结果变化 | P0 |
| B7 | LLM 问答 | `gmp-retrieve shell` 交互 | 带引用答案 | P0 |
| B8 | 规则召回 | `gmp-retrieve search "偏差分级" --channel rule` | 匹配到规则 | P0 |
| B9 | 391 场景 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 65% | P0 |

### 4.3 RC Gate 功能测试 (R1-R10)

| # | 测试项 | 输入 | 期望结果 | 优先级 |
|---|--------|------|----------|--------|
| R1-R4 | 同 A1-A4 | — | — | P0 |
| R5 | 391 场景 | 全量 391 测试 | ≥ 80% | P0 |
| R6 | 性能基准 | `gmp-retrieve bench --concurrent 10 --rounds 100` | P50 < 5s, P95 < 15s | P0 |
| R7 | 稳定性 | `gmp-retrieve stress --iterations 50` | 无崩溃 | P0 |
| R8 | 安全 | `cargo audit` | 无高危 | P0 |
| R9 | 文档 | 用户手册 + 部署指南 | 存在可执行 | P0 |
| R10 | 引用溯源 | 抽样 20 个 LLM 答案 | 引用来源可追溯 | P0 |

---

## 五、391 场景测试详细计划

### 5.1 测试集结构

```
gmp_kg_build/test_results_final/
├── 391_scenarios.json         # 全量测试场景
│   [{ "id": "SC-001", "query": "...", "expected": "...", "category": "SOP" }, ...]
```

### 5.2 评估方法

```bash
# 批量评估
gmp-retrieve evaluate --test-set 391_scenarios.json --output results.json

# 输出格式
{
  "total": 391,
  "correct": N,
  "partial": N,
  "incorrect": N,
  "unanswerable": N,
  "pass_rate": "XX%",
  "by_category": {
    "SOP":     { "total": N, "correct": N, "pass_rate": "XX%" },
    "偏差":     { "total": N, "correct": N, "pass_rate": "XX%" },
    "验证":     { "total": N, "correct": N, "pass_rate": "XX%" },
    "CAPA":    { "total": N, "correct": N, "pass_rate": "XX%" },
    "法规":     { "total": N, "correct": N, "pass_rate": "XX%" }
  }
}
```

### 5.3 分阶段测试时间表

| 阶段 | 日期 | 内容 | 通过率目标 |
|------|------|------|-----------|
| Alpha 基线 | Week 2 D14 | Layer 2 四通道检索 | ≥ 65% |
| Beta 基线 | Week 3 D21 | Layer 2+3+4 全链路 | ≥ 78% |
| RC 第一轮 | Week 4 D22 | 全量回归 | 记录基线 |
| 修复迭代 | Week 4 D23-24 | Top-5 问题修复 | ≥ 78% |
| RC 第二轮 | Week 4 D25 | 回归验证 | ≥ 80% |
| GA 回归 | Week 4 D28 | 最终验证 | ≥ 80% (无退化) |

### 5.4 失败分析流程

```
step 1: 运行 evaluate，获取失败场景列表
step 2: 对每个失败场景分类:
        - 知识缺口: 检索返回为空 → 标注数据缺失
        - 语义偏差: 检索返回相关但不完全相同 → Reranker/prompt 优化
        - 图谱不足: 关系链路缺失 → 补充图 schema
        - LLM 幻觉: 答案与上下文矛盾 → prompt 约束 + 引用验证
step 3: 优先修复 Top-5 高频失败模式
step 4: 回归验证
```

---

## 六、性能测试计划

### 6.1 性能指标

| 指标 | 目标 | 测量方法 |
|------|------|----------|
| 单查询 P50 | < 5s | `time gmp-retrieve search "SOP起草" --all --rerank` |
| 单查询 P95 | < 15s | 同命令 × 100 次 |
| 单查询 P99 | < 30s | 同命令 × 100 次 |
| 10 并发 P50 | < 30s | `gmp-retrieve bench --concurrent 10 --rounds 100` |
| 索引构建 | < 4h | `time gmp-retrieve index --source gmp-md/` |

### 6.2 延迟分解

| 阶段 | 预估延迟 | 说明 |
|------|----------|------|
| 规则召回 | < 10ms | 内存 Hash 匹配 |
| 向量召回 | ~200-500ms | bge-m3 嵌入 + HNSW 搜索 |
| 全文召回 | ~50-100ms | BM25 评分 |
| 图谱召回 | ~100-200ms | Cypher + BFS |
| RRF 融合 | < 5ms | 纯计算 |
| Reranker | ~2-5s/chunk (CPU) | bge-reranker-v2-m3 Cross-Encoder |
| LLM 答案 | ~2-8s | qwen2.5:14b 推理 |
| **总计** | **~5-14s** | 单查询预估 |

---

## 七、稳定性测试

### 7.1 稳定性测试用例

| # | 测试项 | 方法 | 通过标准 |
|---|--------|------|----------|
| S1 | 连续查询 | `stress --iterations 50` | 0 崩溃 |
| S2 | 并发查询 | `bench --concurrent 10 --rounds 100` | 0 崩溃 |
| S3 | 大数据量 | 100 次不同查询 | 无内存泄漏 |
| S4 | 数据库重启 | 重启后索引恢复可用 | 查询正常 |
| S5 | Ollama 断连 | Ollama 停止 → 重启 → 恢复 | graceful degradation |

---

## 八、安全测试

| # | 测试项 | 方法 | 通过标准 |
|---|--------|------|----------|
| SEC1 | 依赖审计 | `cargo audit` | 无高危 |
| SEC2 | API Key | `grep API_KEY` 源码 | 无硬编码 |
| SEC3 | 文件权限 | `.db` 文件权限 600 | 仅所有者读写 |
| SEC4 | 输入验证 | 特殊字符查询 (SQL/HTML/unicode) | 不崩溃不注入 |

---

## 九、门禁检查映射

| 门禁检查项 | 本测试计划对应项 |
|-----------|-----------------|
| A1 (编译) | §4.1 A1 |
| A2 (单元测试) | §4.1 A2 |
| B6 (Reranker) | §4.2 B6 |
| B7 (LLM) | §4.2 B7 |
| R5 (391 场景) | §5 |
| R6 (性能) | §6 |
| R7 (稳定性) | §7 |
| R8 (安全) | §8 |

---

## 十、风险与 mitigation

| 风险 | 概率 | 缓解 |
|------|------|------|
| Ollama 模型加载失败 | 中 | 前置验证脚本 |
| 嵌入生成内存溢出 | 中 | 分批处理，每批 ≤ 100 |
| 391 场景答案评估主观性 | 高 | 明确判定规则，抽样人工复核 |
| 性能未达预期 | 中 | 分阶段测量，识别瓶颈 |
| 测试数据环境差异 | 低 | Docker 化测试环境 |

---

> **下一步**: 在 Alpha 阶段末执行 A1-A9 检查，记录门禁结果。