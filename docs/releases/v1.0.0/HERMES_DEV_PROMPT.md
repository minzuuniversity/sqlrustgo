# GMP-Platform v1.0.0 开发任务 — 给 Hermes

## 项目上下文

你将在 GMP-Platform 仓库中进行开发，该项目是一个 **GMP 合规内审检索系统**，基于 SQLRustGo 统一数据底座，实现四层检索架构。

- **Gitea 仓库**: http://192.168.0.252:3000/openclaw/GMP-Platform
- **开发分支**: `develop/v1.0.0`（已受保护，禁止直接 push）
- **SQLRustGo 代码库**: http://192.168.0.252:3000/openclaw/sqlrustgo（已完成 BM25/RRF/Reranker 增量补全）
- **Ollama 环境**: 192.168.0.252:11434

```
四层架构:
  Layer 4: LLM 问答层    → Qwen2.5-14B (Ollama) 答案生成 + 引用溯源
  Layer 3: Reranker 重排 → BGE-Reranker-v2-m3 Cross-Encoder 精排
  Layer 2: 检索引擎层     → 规则/向量(HNSW)/全文(BM25)/图谱(Cypher) 四通道 + RRF 融合
  Layer 1: 数据存储层     → SQLRustGo (RDBMS + 向量库 + 图数据库 统一底座)
```

### 关键文档（必读）
- [开发计划](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/releases/v1.0.0/DEVELOPMENT_PLAN.md)
- [四层架构设计](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/design/gmp-sqlrustgo-four-layer-plan.md)
- [治理标准](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/governance/GOVERNANCE_STANDARD.md)
- [门禁规范](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/governance/GATE_SPEC.md)
- [测试计划](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/releases/v1.0.0/TEST_PLAN.md)
- [集成状态](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/releases/v1.0.0/INTEGRATION_STATUS.md)
- [ISSUE 追踪](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/releases/v1.0.0/ISSUE_TRACKER.md)

### 知识库
- `gmp-md/` 目录包含 1696 个 GMP 合规文档（FDA/EMA/中国 GMP 法规、检查指南等）
- `gmp_kg_build/` 包含 agent-knowledge 规则 YAML（55 条 DevSecOps 合规规则）

---

## 开发任务清单

按优先级和依赖顺序排列。**每个 ISSUE 独立开发，完成后提交 PR**。

### Week 1: 数据底座 + 索引基础设施

#### [#8] [M1] 项目脚手架 + Cargo workspace + CLI 框架
- **Milestone**: v1.0.0-A | **Priority**: P0 | **Labels**: priority/p0, type/feature
- **目标**: 建立项目骨架，CLI 入口
- **技术栈**: Rust >= 1.80, clap, serde, tokio, anyhow, ollama-rs
- **验收**: `cargo build --release` 通过，`gmp-retrieve --help` 输出帮助

#### [#9] [M2] 数据导入 + 索引基础设施 (FTS/向量/图谱)
- **Milestone**: v1.0.0-B | **Priority**: P0 | **Labels**: priority/p0, type/feature
- **目标**: 导入 gmp-md/ 文档到 SQLRustGo，建立三种索引
- **子任务**:
  - 2a. 文档导入 → `gmp_documents` + `gmp_document_chunks` 表（~8700 chunks）
  - 2b. FTS 索引 + BM25 评分（GMP 词典 FMM 分词）
  - 2c. 向量索引 → HNSW (1024d)，通过 Ollama bge-m3 生成嵌入
  - 2d. 图谱构建 → 实体节点 + 关系边（目标 >=700 节点, >=1500 边）
- **验收**: `gmp-retrieve index --validate` 显示 1696/1696，各通道查询返回结果

---

### Week 2: 检索引擎 + 融合重排

#### [#10] [M3] 四通道检索引擎 (规则/向量/全文/图谱)
- **Milestone**: v1.0.0-B | **Priority**: P0 | **Labels**: priority/p0, type/feature
- **目标**: 四通道并行检索，各通道独立召回 Top-K
- **子任务**:
  - 3a. RuleChannel — 匹配 agent-knowledge YAML 规则，RRF weight=2.0, Top-10
  - 3b. VectorChannel — bge-m3 + HNSW 相似度，Top-30, recall@20 >= 70%
  - 3c. FtsChannel — BM25 评分，Top-30, recall@20 >= 50%
  - 3d. GraphChannel — Cypher 查询 + 2-hop BFS 邻居扩展，Top-10

#### [#11] [M4] RRF 融合 + Reranker 重排引擎
- **Milestone**: v1.0.0-B | **Priority**: P0 | **Labels**: priority/p0, type/feature
- **目标**: 多通道结果 RRF 融合（k=60），BGE-Reranker-v2-m3 精排
- **规则通道 weight=2.0**，其他通道 weight=1.0
- **Reranker 回退**: 不可用时直接输出 RRF 结果

#### [#12] [M4] LLM 问答集成 + 意图分类器
- **Milestone**: v1.0.0-RC | **Priority**: P0 | **Labels**: priority/p0, type/feature
- **目标**: RAG prompt 模板（4 context slots），引用溯源，7 种意图分类
- **LLM**: Ollama + qwen2.5:14b，温度 0.3, max_tokens 1024, 3 次重试 + 30s 超时
- **验收**: `gmp-retrieve shell` 交互问答可用，答案含 [来源: xxx.md] 引用

#### [#13] [M3] CLI 全命令 (index/search/evaluate/shell/bench)
- **Milestone**: v1.0.0-RC | **Priority**: P0 | **Labels**: priority/p0, type/feature
- **子命令**: `index`, `search`, `evaluate`, `shell`, `bench`
- **验收**: 所有子命令 `--help` 完整，evaluate 输出 JSON，bench 输出 P50/P95/P99

---

### Week 3-4: 测试 + 调优 + 发布

#### [#14] [M5] 391 场景测试 + 通过率报告
- **Milestone**: v1.0.0-RC | **Priority**: P0 | **Labels**: priority/p0, type/test
- **目标**: 系统化测试 391 个 GMP 合规场景
- **阶段目标**:
  - Alpha: Layer 2 only, >= 65%
  - Beta: Layer 2+3+4, >= 78%
  - RC: 全量回归, >= 80%
- **失败分类**: 知识缺口 ~15%, 语义偏差 ~10%, 图谱不足 ~5%, LLM 幻觉 ~5%

#### [#15] [M5] 用户手册 + 部署指南 + 发布文档
- **Milestone**: v1.0.0-GA | **Priority**: P0 | **Labels**: priority/p0, type/docs
- **目标**: 完成所有用户和管理文档
- **清单**: USER_MANUAL.md, DEPLOYMENT_GUIDE.md, RELEASE_NOTES.md, CHANGELOG.md

#### [#16] [P1] 索引增量更新支持
- **Milestone**: v1.0.0-GA | **Priority**: P1 | **Labels**: priority/p1, type/feature
- `gmp-retrieve index --source {doc} --append`，不影响已有数据

#### [#17] [P1] 性能压测 + 基准报告
- **Milestone**: v1.0.0-GA | **Priority**: P1 | **Labels**: priority/p1, type/test
- P50 < 5s, P95 < 15s, P99 < 30s, 50 轮 0 crash

---

### Future (v1.1.0)

#### [#18] [P2] 中国 GMP 条文补充到知识库
- **Milestone**: v1.1.0 | **Priority**: P2
- 补充中国 GMP 2010 核心条文，减少"知识缺口"场景失败 >= 5%

#### [#19] [P2] 设备手册数据集成
- **Milestone**: v1.1.0 | **Priority**: P2

#### [#20] [P2] Web API 模式 (RESTful)
- **Milestone**: v1.1.0 | **Priority**: P2
- POST /api/v1/search, POST /api/v1/evaluate, GET /api/v1/health

---

## 开发工作流

### 分支策略

```
develop/v1.0.0  ← 受保护，禁止直接 push
  ↑ PR merge
feature/#{N}-{short-desc}  ← 每个 ISSUE 一个分支
```

### 每个 ISSUE 的开发流程

```bash
# 1. 从 develop/v1.0.0 创建功能分支
git fetch origin
git checkout -b feature/8-cargo-workspace origin/develop/v1.0.0

# 2. 开发 + 本地验证
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check

# 3. 提交（commit message 格式: "feat(#8): {简短描述}" 或 "fix(#8): {修复描述}"）
git add -A
git commit -m "feat(#8): 初始化 Cargo workspace + CLI 框架

- 创建 Cargo workspace 结构
- 实现 clap CLI 入口 (gmp-retrieve)
- 添加 SQLRustGo crates 依赖声明
- 配置 ollama-rs, serde, tokio, anyhow

Closes #8"

# 4. 推送到远程
git push origin feature/8-cargo-workspace

# 5. 通过 Gitea API 创建 PR
# 或访问 http://192.168.0.252:3000/openclaw/GMP-Platform/pulls
# base: develop/v1.0.0, head: feature/8-cargo-workspace
```

### Gitea API Token（如需自动化 PR）

```
Token: 07dc6c9f753a10ffbdb779bfc7d87668143e619c
API: http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform
```

### 创建 PR API 示例

```bash
curl -H "Authorization: token 07dc6c9f753a10ffbdb779bfc7d87668143e619c" \
     -H "Content-Type: application/json" \
     -X POST "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/pulls" \
     -d '{"title":"feat(#8): Cargo workspace + CLI","head":"feature/8-cargo-workspace","base":"develop/v1.0.0","body":"Closes #8\n\n## 变更\n- ...\n\n## 验证\n- cargo build --release ✓\n- cargo test ✓"}'
```

---

## 门禁规范（每个 PR 必须通过）

参考 [GATE_SPEC.md](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/governance/GATE_SPEC.md)

| 阶段 | 门禁 | 检查项 |
|------|------|--------|
| **Alpha** | A-Gate | A1 `cargo build --release` / A3 `cargo clippy -- -D warnings` / A4 `cargo fmt --check` |
| **Beta** | B-Gate | B1 `cargo test` 100% 通过 / B6 Reranker 可用 |
| **RC** | R-Gate | R1 391 场景回归无退化 / R2 性能达标 |
| **GA** | G-Gate | G1 CHANGELOG / G2 文档齐全 |

---

## Commit 规范

```
feat(#{N}): {描述}      # 新功能
fix(#{N}): {描述}       # Bug 修复
docs(#{N}): {描述}      # 文档
test(#{N}): {描述}      # 测试
refactor(#{N}): {描述}  # 重构
```

每个 commit message 必须包含对应的 Gitea ISSUE 编号。

---

## 依赖关系

```
#8 (脚手架)
  └→ #9 (数据导入+索引)
       └→ #10 (四通道)
            └→ #11 (RRF+Reranker)
                 └→ #12 (LLM 问答)
                      └→ #13 (CLI 全命令)
                           └→ #14 (391 测试)
                                └→ #15 (文档)
#16 (增量更新) — 独立
#17 (性能压测) — 独立
#18, #19, #20 — v1.1.0, 最后处理
```

**关键依赖**: 
- #9 依赖 #8（需要 workspace 结构）
- #10 依赖 #9（需要索引数据）
- #11 依赖 #10（需要通道结果）
- #12 依赖 #11（需要精排后的检索结果）
- #14 依赖 #13（需要完整 CLI 才能运行评估）

---

## 开始行动

请按照以下顺序开始开发：

1. **首先阅读** [DEVELOPMENT_PLAN.md](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/releases/v1.0.0/DEVELOPMENT_PLAN.md) 和 [四层架构设计](http://192.168.0.252:3000/openclaw/GMP-Platform/src/branch/develop/v1.0.0/docs/design/gmp-sqlrustgo-four-layer-plan.md)
2. **从 #8 开始**，逐个完成每个 ISSUE
3. **每个 ISSUE 完成后**，创建 PR 指向 `develop/v1.0.0`
4. **PR 描述中**注明 `Closes #{N}` 以自动关联 ISSUE
5. **验证门禁**，确保 build/test/lint 全部通过
6. **遇到阻塞时**，在对应 ISSUE 评论区记录问题并 @openclaw

目标：4 周内完成 #8~#17，391 场景通过率 >= 80%。