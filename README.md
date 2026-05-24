# GMP-Platform — GMP 合规内审检索系统

> **基于 SQLRustGo 统一数据底座，五层检索架构的开源合规审核平台**

[![Version](https://img.shields.io/badge/version-1.3.0--beta-blue)](./docs/releases/v1.3.0/)
[![Stage](https://img.shields.io/badge/stage-Beta-orange)](#)
[![License](https://img.shields.io/badge/license-MIT-green)](./LICENSE)

---

## 项目定位

GMP-Platform 是一个面向制药行业 GMP 合规内审的智能检索系统，基于 [SQLRustGo](http://192.168.0.252:3000/openclaw/sqlrustgo) 统一数据底座，实现 **五层检索架构**，为 391 个 GMP 合规审核场景提供高精度检索问答能力。

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Layer 5: LLM 推理与问答    → Qwen2.5-14B 答案生成 + 引用溯源            │
│ Layer 4: Reranker 重排    → BGE-Reranker-v2-m3 Cross-Encoder 精排      │
│ Layer 3: 检索引擎层        → 规则/向量/全文/图谱 四通道 → RRF → Top-20  │
│ Layer 2: 知识图谱层        → GMP 实体节点 + 语义关系边 + Cypher 查询     │
│ Layer 1: 数据存储层        → SQLRustGo (RDBMS + Vector + FTS + Graph)   │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 0: 离线知识抽取管线  ★ 初始化时运行 ★                             │
│   术语库 + NER + RE + 多维分类 + 相关性预评分 → 图谱写入                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### 核心目标

- **391 场景通过率 ≥ 80%**（基线 v22 为 44.5%）
- **单查询 P50 < 5s, P95 < 15s**
- **单二进制部署，零外部数据库依赖**

---

## 架构亮点

| 特性 | 实现 |
|------|------|
| 统一数据底座 | SQLRustGo: RDBMS + 向量索引(HNSW) + 全文索引(BM25) + 图存储(DiskGraphStore+WAL) |
| 五层检索管线 | Layer 0(离线抽取) → Layer 1(存储) → Layer 2(图谱) → Layer 3(检索) → Layer 4(Reranker) → Layer 5(LLM) |
| 四通道召回 | 规则通道(2.0) + 向量通道(bge-m3) + 全文通道(BM25) + 图谱通道(Cypher) |
| RRF 融合 | Reciprocal Rank Fusion (k=60)，替换线性加权 |
| Reranker 重排 | bge-reranker-v2-m3 Cross-Encoder 精排 |
| LLM 问答 | Ollama + Qwen2.5-14B，支持 7 种意图分类 |
| 事件驱动 | gmp-event-bus (InMemoryBus + AuditHandler + MQTT Gateway) |
| GMP 合规 | 审计链 + 电子签名内置 |

---

## 文档导航

### 快速入口

| 你想... | 看这里 |
|---------|--------|
| 了解项目 | [项目总览](./README.md) |
| 开始开发 | [v1.3.0 开发计划](./docs/releases/v1.3.0/DEVELOPMENT_PLAN.md) |
| 部署使用 | [用户手册](./docs/releases/v1.3.0/USER_MANUAL.md) + [部署指南](./docs/releases/v1.3.0/DEPLOYMENT_GUIDE.md) |
| 查看进度 | [ISSUE 追踪](./docs/releases/v1.3.0/ISSUE_TRACKER.md) + [功能矩阵](./docs/releases/v1.3.0/FEATURE_MATRIX.md) |
| 安装初始化 | [安装指南](./docs/INSTALLATION_AND_INITIALIZATION.md) |
| 质量门禁 | [门禁规范](./docs/governance/GATE_SPEC.md) + [检查清单](./docs/releases/v1.3.0/RELEASE_GATE_CHECKLIST.md) |

### 完整文档索引

详见 [DOCS_INDEX.md](./DOCS_INDEX.md)

---

## 项目结构

```
GMP-Platform/
├── README.md                         # 项目总览 (本文件)
├── DOCS_INDEX.md                     # 文档索引与导航
├── AGENTS.md                         # AI Agent 开发指令
├── CLAUDE.md                         # Claude Code 配置
│
├── docs/
│   ├── design/                       # 设计文档
│   │   ├── gmp-sqlrustgo-four-layer-plan.md    # 历史四层架构设计
│   │   ├── five-layer-kg-pipeline-plan.md      # 五层架构 + Layer 0 管线
│   │   └── sqlrustgo-incremental-enhancement-plan.md # SQLRustGo 增量补全计划
│   │
│   ├── governance/                   # 治理文档
│   │   ├── GOVERNANCE_STANDARD.md    # 治理标准总纲 (SSOT)
│   │   ├── GATE_SPEC.md              # 门禁检查规范
│   │   ├── RELEASE_LIFECYCLE.md      # 版本生命周期
│   │   ├── VERSION_DOCS_SPEC.md      # 版本文档规范
│   │   └── CONTRIBUTING.md           # 贡献指南
│   │
│   ├── releases/
│   │   ├── v1.3.0/                  # v1.3.0 Beta 版本文档 (21 个文档)
│   │   │   ├── 00-release-summary.md     # 发布摘要
│   │   │   ├── DEVELOPMENT_PLAN.md       # 开发计划 (ISSUE 映射)
│   │   │   ├── TEST_PLAN.md              # 测试计划 (391 场景)
│   │   │   ├── RELEASE_GATE_CHECKLIST.md # 门禁检查清单
│   │   │   └── ...                       # 见 DOCS_INDEX.md
│   │   ├── v1.2.0/                  # v1.2.0 Beta 版本文档
│   │   └── v1.0.0/                  # v1.0.0 GA 版本文档
│   │
│   ├── INSTALLATION_AND_INITIALIZATION.md  # 安装与初始化指南 (五层架构)
│   │
│   └── user/                         # 用户文档
│       └── USER_MANUAL.md            # 用户手册
│
├── gmp-md/                           # GMP 知识文档集 (1696 篇)
├── gmp-md-masking/                   # 脱敏版文档
├── agent-knowledge/                  # 规则知识库 (YAML)
├── scripts/                          # 工具脚本
│   └── gate/                         # 门禁检查脚本
│
└── .claude/                          # Claude 配置
    └── skills/                       # AI 技能定义
```

---

## 版本路线

| 版本 | 目标 | 状态 |
|------|------|------|
| **v1.3.0** | 五层架构, Event Bus, Audit Middleware, MQTT, Binary BLOB | 🔄 Beta 开发中 |
| v1.2.0 | Agent 编排, JWT, E2E 测试 | ✅ Beta |
| v1.1.0 | BM25 修复, 集成测试 | ✅ GA |
| v1.0.0 | 四层检索, 391 ≥ 80% | ✅ GA |

---

## 治理体系

本项目遵循严格的开发治理流程，所有 AI Agent 和开发者必须遵循：

- **SSOT 原则**: 治理标准为唯一权威来源
- **门禁强制**: L0-Alpha → L0-Beta → L0-RC → L0-GA 四级门禁，不可跳过
- **Issue 闭环**: FAIL → Issue → PR → 验证 → 关闭
- **版本延续**: 未完成任务必须映射到下一版本

详见 [docs/governance/GOVERNANCE_STANDARD.md](./docs/governance/GOVERNANCE_STANDARD.md)

---

## 快速开始

```bash
# 克隆项目
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.3.0

# 构建
cargo build --release --workspace

# 查看开发计划
cat docs/releases/v1.3.0/DEVELOPMENT_PLAN.md

# 查看文档索引
cat DOCS_INDEX.md
```

---

## 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| [SQLRustGo](http://192.168.0.252:3000/openclaw/sqlrustgo) | latest | 统一数据底座 |
| [Ollama](https://ollama.com) | ≥ 0.1.0 | 本地 LLM 推理 |
| bge-m3 | latest | 嵌入模型 (1024d) |
| qwen2.5 | 14b | LLM 答案生成 |
| bge-reranker-v2-m3 | latest | 检索结果重排 |

---

## Gitea

- **仓库**: http://192.168.0.252:3000/openclaw/GMP-Platform
- **分支策略**: main / develop/vX.Y.Z / release/vX.Y.Z
- **ISSUE 追踪**: [ISSUE_TRACKER.md](./docs/releases/v1.3.0/ISSUE_TRACKER.md)
