# GMP 内审检索系统 v1.0.0 开发计划

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **阶段状态**: Alpha
> **Gitea Repo**: http://192.168.0.252:3000/openclaw/GMP-Platform
> **Milestone**: v1.0.0 GMP Compliance Retrieval System

> **架构设计引用**: [docs/design/gmp-sqlrustgo-four-layer-plan.md](../../design/gmp-sqlrustgo-four-layer-plan.md)

---

## 一、概述与目标

### 1.1 版本目标

基于 SQLRustGo 统一数据底座，构建 GMP 合规内审检索系统 v1.0.0，实现四层架构（数据存储 → 多路检索 → Reranker 重排 → LLM 问答），目标 391 场景通过率 ≥ 80%。

### 1.2 战略定位

```
v1.0.0 = GMP 合规内审检索的首个正式版本
  - 零外部数据库依赖 (SQLRustGo 内置 RDBMS + Vector + Graph + FTS)
  - 单二进制交付 (< 50MB)
  - 四通道检索融合 (规则/向量/全文/图谱)
  - LLM 辅助答案生成 (带引用溯源)
```

### 1.3 关键交付物

| 交付物 | 类型 | 关联 Issue |
|--------|------|-----------|
| `gmp-retrieve` CLI 二进制 | Feature | #1 |
| 数据导入 + 索引基础设施 | Feature | #2 |
| 四通道检索引擎 (规则/向量/全文/图谱) | Feature | #3 |
| RRF 融合 + Reranker 重排 | Feature | #4 |
| LLM 问答 + 意图分类 | Feature | #5 |
| CLI 全命令 (index/search/evaluate/shell) | Feature | #6 |
| 391 场景测试 + 通过率报告 | Test | #7 |
| 用户手册 + 部署指南 | Docs | #8 |

### 1.4 与基线的对比

| 方面 | v22 基线 | v1.0.0 目标 | 提升 |
|------|----------|-----------|------|
| 检索方式 | FTS5 BM25 + 简单语义 | 四通道 + RRF + Reranker | 质的飞跃 |
| 391 通过率 | 44.5% | ≥ 80% | +35.5% |
| 答案质量 | 片段返回 | RAG 生成 + 引用溯源 | 新增 |
| 规则匹配 | agent-knowledge 单向 | 规则通道 (weight=2.0) 优先 | 增强 |
| 图谱能力 | 无 | Cypher + BFS 邻居扩展 | 新增 |
| 部署 | 多组件 | 单二进制 | 简化 |

### 1.5 成功的定义

- [ ] `gmp-retrieve index --source gmp-md/` 完成 1696 文档导入
- [ ] `gmp-retrieve search "偏差分级标准" --all` 返回多通道融合结果
- [ ] `gmp-retrieve evaluate --test-set 391_scenarios.json` 通过率 ≥ 80%
- [ ] `gmp-retrieve shell` 交互式问答可用
- [ ] P50 延迟 < 5s, P95 延迟 < 15s

---

## 二、功能列表

### 2.1 P0 必须完成 (Milestone: Week 1-3)

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| 项目脚手架 + Cargo workspace | #1 | `cargo build` 通过，`gmp-retrieve --help` 输出 | TODO |
| 数据导入: gmp-md → SQLRustGo tables | #2 | 1696 文档导入到 `gmp_documents` + `gmp_document_chunks` | TODO |
| 向量索引: bge-m3 嵌入 + HNSW | #2 | ~8700 chunks 向量入库，`idx_embedding_hnsw` 可用 | TODO |
| 图谱构建: agent-knowledge → 实体+关系 | #2 | ≥ 700 节点 + ≥ 1500 边 | TODO |
| 规则召回通道 (RuleChannel) | #3 | 匹配 ≥ 55 规则，weight=2.0 | TODO |
| 向量召回通道 (VectorChannel) | #3 | recall@20 ≥ 70% | TODO |
| 全文召回通道 (FtsChannel + BM25) | #3 | recall@20 ≥ 50% | TODO |
| 图谱召回通道 (GraphChannel) | #3 | Cypher + BFS 邻居扩展可用 | TODO |
| RRF 融合引擎 | #4 | 四通道 RRF(k=60)，输出 Top-20 | TODO |
| Reranker 集成 (bge-reranker-v2-m3) | #4 | Cross-Encoder 重排可用，支持 CPU 模式 | TODO |
| LLM 问答集成 (qwen2.5:14b) | #5 | RAG 生成带引用溯源答案 | TODO |
| 意图分类器 | #5 | 7 种意图分类准确率 ≥ 80% | TODO |
| CLI: index 子命令 | #6 | 文档导入 + 向量化 + 图谱构建 | TODO |
| CLI: search 子命令 | #6 | 单通道/全通道检索 + `--explain` 调试 | TODO |
| CLI: evaluate 子命令 | #6 | 391 场景批量评估 + JSON 输出 | TODO |
| CLI: shell 子命令 | #6 | 交互式问答 | TODO |

### 2.2 P1 强烈建议 (Milestone: Week 4)

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| 失败模式分类分析 | #7-1 | 知识缺口/语义偏差/图谱不足/LLM幻觉 分类 | TODO |
| 性能压测 | #7-2 | 10 并发 × 100 轮，P50/P95/P99 | TODO |
| 跨基线对比报告 | #7-3 | v22 vs Layer2 vs Layer2+3 vs Full | TODO |
| 索引增量更新 | #9 | 支持部分文档重新导入 | TODO |

### 2.3 P2 期望完成 (Milestone: Post v1.0.0)

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| 中国 GMP 条文补充 | #10 | 补充中国 GMP 2010 版知识库 | TODO |
| 设备手册集成 | #11 | 设备操作细节数据导入 | TODO |
| Web API 模式 | #12 | RESTful API 替代 CLI | TODO |

---

## 三、技术任务

### 3.1 架构依赖

| 依赖 | 版本要求 | 用途 |
|------|----------|------|
| SQLRustGo (增量补全版) | v3.2.0+ + Hermes PR | 统一数据底座 (RDBMS + Vector + Graph + BM25) |
| Ollama + bge-m3 | latest | 文档嵌入生成 (1024d) |
| Ollama + bge-reranker-v2-m3 | latest | Cross-Encoder 重排 |
| Ollama + qwen2.5:14b | 14B | LLM 答案生成 |
| Rust | ≥ 1.80 | 编译环境 |

### 3.2 Cargo Workspace 依赖

| Crate | 来源 | 说明 |
|-------|------|------|
| `sqlrustgo-storage` | SQLRustGo | FTS BM25 + 文档存储 |
| `sqlrustgo-vector` | SQLRustGo | HNSW/IVF 向量索引 |
| `sqlrustgo-graph` | SQLRustGo | DiskGraphStore + Cypher |
| `sqlrustgo-qmd-bridge` | SQLRustGo | RRF 融合 |
| `sqlrustgo-gmp` | SQLRustGo | GMP 合规特性 (审计链) |
| `ollama-rs` | crates.io | Ollama API 客户端 |
| `clap` | crates.io | CLI 框架 |
| `serde` / `serde_json` | crates.io | 序列化 |
| `tokio` | crates.io | 异步运行时 |
| `anyhow` / `thiserror` | crates.io | 错误处理 |

### 3.3 工具链

| 工具 | 版本 | 用途 |
|------|------|------|
| rustc | ≥ 1.80 | 编译 |
| cargo | ≥ 1.80 | 构建/测试 |
| clippy | latest | 代码质量 |
| rustfmt | latest | 格式化 |
| cargo-audit | latest | 安全扫描 |

---

## 四、测试策略

> **详见**: [TEST_PLAN.md](./TEST_PLAN.md)

### 4.1 测试层级

| 层级 | 覆盖内容 | 工具 |
|------|----------|------|
| 单元测试 | 每个 channel/reanker/llm 模块 | `cargo test` |
| 通道评估 | recall@K, 命中率 | 自定义 Python 脚本 |
| 集成测试 | 391 场景全链路 | `gmp-retrieve evaluate` |
| 性能测试 | 并发压测 | `gmp-retrieve bench` |

### 4.2 391 场景测试计划

| 阶段 | 时间 | 通过率目标 | 备注 |
|------|------|-----------|------|
| Alpha 基线 | Week 2 末 | ≥ 65% | Layer 2 only |
| Beta 基线 | Week 3 末 | ≥ 78% | Layer 2+3+4 |
| RC 验证 | Week 4 D22-25 | ≥ 80% | 全链路 + 修复 |
| GA 回归 | Week 4 D28 | ≥ 80% (无退化) | 最终验证 |

### 4.3 失败模式分类

| 类别 | 预期比例 | 缓解策略 |
|------|----------|----------|
| 知识缺口 (缺中国 GMP) | ~15% | 标注，P2 补充 |
| 语义偏差 | ~10% | Reranker + prompt 优化 |
| 图谱不足 | ~5% | 增加 Cypher 规则 |
| LLM 幻觉 | ~5% | 引用溯源 + 人工抽查 |
| 数据不完整 | ~5% | 标注，P2 补充 |

---

## 五、门禁计划

> **详见**: [GATE_SPEC.md](../../governance/GATE_SPEC.md)

### 5.1 各阶段门禁入口条件

| 阶段 | 日期 | 门禁 | 关键条件 |
|------|------|------|----------|
| Alpha | Week 2 D14 | A-Gate | 四通道 + RRF 可用, 1696 文档入库 |
| Beta | Week 3 D21 | B-Gate | Reranker + LLM 集成, 391 ≥ 65% |
| RC | Week 4 D25 | R-Gate | 391 ≥ 80%, 性能达标, 文档齐全 |
| GA | Week 4 D28 | G-Gate | 回归验证, 无退化, 正式发布 |

### 5.2 门禁检查清单

| 阶段 | 检查清单文件 |
|------|-------------|
| Alpha | [RELEASE_GATE_CHECKLIST.md](./RELEASE_GATE_CHECKLIST.md) § Alpha |
| Beta | [RELEASE_GATE_CHECKLIST.md](./RELEASE_GATE_CHECKLIST.md) § Beta |
| RC | [RELEASE_GATE_CHECKLIST.md](./RELEASE_GATE_CHECKLIST.md) § RC |
| GA | [RELEASE_GATE_CHECKLIST.md](./RELEASE_GATE_CHECKLIST.md) § GA |

---

## 六、版本延续任务

> v1.0.0 为首个正式版本，无前版延续任务。

但以下 v22 基线的已知局限将作为 v1.0.0 改进重点：

| v22 局限 | v1.0.0 改进 |
|----------|-----------|
| FTS5 BM25 无序返回 | BM25 评分排序 |
| 单通道检索 | 四通道 + RRF 融合 |
| 无语义泛化 | bge-m3 向量语义匹配 |
| 无图谱推理 | Cypher + BFS 实体关系扩展 |
| 无重排 | bge-reranker-v2-m3 Cross-Encoder |
| 片段返回 | LLM RAG 生成 + 引用 |
| 多组件部署 | 单二进制 SQLRustGo 底座 |

---

## 七、风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| SQLRustGo 增量补全 PR 合并延迟 | 低 (已完成) | 中 | PR 已合并，无阻塞 |
| bge-m3 嵌入生成慢 (CPU only) | 高 | 中 | 一次性批处理，1696 文档 2-4h |
| Ollama API 不稳定 | 中 | 中 | 重试 3 次 + 30s 超时 |
| 图谱实体抽取不足 | 中 | 中 | agent-knowledge 55+ 规则可自动抽取 |
| 391 未达 80% | 中 | 中 | 失败分类 + 快速修复迭代 |
| GPU 兼容问题 (RTX 5080) | 高 | 低 | 全部 CPU 可运行 |
| 4 周时间不足 | 低 | 中 | Week 1-2 产物可独立交付 |

---

## 八、里程碑

| 里程碑 | 日期 | 目标 | 门禁 |
|--------|------|------|------|
| M1: 项目启动 | 2026-05-21 | 文档齐全, ISSUE 创建, 环境就绪 | — |
| M2: Layer 1 完成 | Week 1 D7 | 数据底座就绪, 索引可用 | — |
| M3: Layer 2 完成 | Week 2 D14 | 四通道 + RRF, 391 ≥ 65% | A-Gate |
| M4: Layer 3+4 完成 | Week 3 D21 | Reranker + LLM, 391 ≥ 78% | B-Gate |
| M5: 调优完成 | Week 4 D25 | 391 ≥ 80%, 性能达标 | R-Gate |
| M6: 正式发布 | Week 4 D28 | v1.0.0 GA | G-Gate |

---

## 九、附录

### 9.1 技术设计文档

| 文档 | 路径 |
|------|------|
| 四层架构设计 | [docs/design/gmp-sqlrustgo-four-layer-plan.md](../../design/gmp-sqlrustgo-four-layer-plan.md) |
| SQLRustGo 增量补全 | [docs/design/sqlrustgo-incremental-enhancement-plan.md](../../design/sqlrustgo-incremental-enhancement-plan.md) |
| 现有方案对比与改进 | [docs/design/gmp-retrieval-system-comparison-and-design.md](../../design/gmp-retrieval-system-comparison-and-design.md) |

### 9.2 治理文档

| 文档 | 路径 |
|------|------|
| 治理标准总纲 | [docs/governance/GOVERNANCE_STANDARD.md](../../governance/GOVERNANCE_STANDARD.md) |
| 门禁规范 | [docs/governance/GATE_SPEC.md](../../governance/GATE_SPEC.md) |
| 版本生命周期 | [docs/governance/RELEASE_LIFECYCLE.md](../../governance/RELEASE_LIFECYCLE.md) |
| 贡献指南 | [docs/governance/CONTRIBUTING.md](../../governance/CONTRIBUTING.md) |

### 9.3 Gitea ISSUE 映射

| Issue # | 标题 | Milestone | Assignee | Priority |
|---------|------|-----------|----------|----------|
| #1 | [M1] 项目脚手架 + Cargo workspace + CLI 框架 | v1.0.0-A | TRAE + DeepSeek-V4-Pro | P0 |
| #2 | [M2] 数据导入 + 索引基础设施 (FTS/向量/图谱) | v1.0.0-B | TRAE + DeepSeek-V4-Pro | P0 |
| #3 | [M3] 四通道检索引擎 (规则/向量/全文/图谱) | v1.0.0-B | TRAE + DeepSeek-V4-Pro | P0 |
| #4 | [M4] RRF 融合 + Reranker 重排引擎 | v1.0.0-B | TRAE + DeepSeek-V4-Pro | P0 |
| #5 | [M4] LLM 问答集成 + 意图分类器 | v1.0.0-RC | TRAE + DeepSeek-V4-Pro | P0 |
| #6 | [M3] CLI 全命令 (index/search/evaluate/shell) | v1.0.0-RC | TRAE + DeepSeek-V4-Pro | P0 |
| #7 | [M5] 391 场景测试 + 通过率报告 | v1.0.0-RC | TRAE + DeepSeek-V4-Pro | P0 |
| #8 | [M5] 用户手册 + 部署指南 | v1.0.0-RC | TRAE + DeepSeek-V4-Pro | P0 |
| #9 | [P1] 索引增量更新 | v1.0.0-GA | — | P1 |
| #10 | [P2] 中国 GMP 条文补充 | v1.1.0 | — | P2 |
| #11 | [P2] 设备手册集成 | v1.1.0 | — | P2 |
| #12 | [P2] Web API 模式 | v1.1.0 | — | P2 |

---

> **下一步**: 确认 Gitea ISSUE 创建完毕，开始 Week 1 开发。