# GMP-Platform 文档索引与导航

> **版本**: 1.0.0-alpha | **更新**: 2026-05-21 | **维护**: TRAE + DeepSeek-V4-Pro

本文档是 GMP-Platform 项目的完整文档索引，按功能域分类并提供快速导航。

---

## 文档分类概览

```
DOCS_INDEX.md                         ← 你在这里
│
├── [A] 设计文档 (docs/design/)         ← 架构设计与技术方案
├── [B] 治理文档 (docs/governance/)     ← 开发规范与流程约束
├── [C] 版本文档 (docs/releases/)       ← 各版本详细文档
│   └── v1.0.0/                       ← 当前开发版本
├── [D] 用户文档 (docs/user/)          ← 使用手册与部署指南
└── [E] 数据资产 (gmp-md/)             ← GMP 知识文档集
```

---

## [A] 设计文档 (`docs/design/`)

系统架构与详细技术设计。

| 文档 | 描述 | 受众 |
|------|------|------|
| [gmp-sqlrustgo-four-layer-plan.md](./docs/design/gmp-sqlrustgo-four-layer-plan.md) | **四层架构详细设计**: 数据模型、检索流程、LLM 集成、4 周开发计划 | 架构师, 开发者 |
| [sqlrustgo-incremental-enhancement-plan.md](./docs/design/sqlrustgo-incremental-enhancement-plan.md) | **SQLRustGo 增量补全计划**: BM25/RRF/Reranker/LLM Chat/规则优先级 (给 Hermes) | Hermes, 开发者 |
| [gmp-four-layer-dev-plan.md](./docs/design/gmp-four-layer-dev-plan.md) | **GMP 四层检索开发计划** (早期方案): 原方案对比、391 场景定义 | 架构师 |
| [gmp-retrieval-system-comparison-and-design.md](./docs/design/gmp-retrieval-system-comparison-and-design.md) | **检索方案对比分析**: 多技术选型对比与推荐 | 架构师 |

---

## [B] 治理文档 (`docs/governance/`)

AI Agent 和开发者必须遵循的核心规范。

### B1. 治理总纲

| 文档 | 角色 | 核心内容 |
|------|------|----------|
| [GOVERNANCE_STANDARD.md](./docs/governance/GOVERNANCE_STANDARD.md) | **最高层规范 (SSOT)** | 五大核心原则, AI 合规机制, 文档体系索引 |
| [GOVERNANCE_INDEX.md](./docs/governance/GOVERNANCE_INDEX.md) | **AI 任务导航** | AI Agent 开发→测试→门禁的文档决策树 |

### B2. 版本与门禁

| 文档 | 角色 | 核心内容 |
|------|------|----------|
| [GATE_SPEC.md](./docs/governance/GATE_SPEC.md) | **门禁规范 (SSOT)** | A/B/R/G 四级门禁定义, 检查项, 通过标准, 不通过处理 |
| [RELEASE_LIFECYCLE.md](./docs/governance/RELEASE_LIFECYCLE.md) | **版本生命周期** | 版本阶段流转, 分支/Tag 规范, 紧急修复流程 |

### B3. 文档与协作

| 文档 | 角色 | 核心内容 |
|------|------|----------|
| [VERSION_DOCS_SPEC.md](./docs/governance/VERSION_DOCS_SPEC.md) | **版本文档规范** | 命名规则, 元数据要求, 最小文档集 |
| [CONTRIBUTING.md](./docs/governance/CONTRIBUTING.md) | **贡献指南** | 分支策略, Commit 规范, PR 模板, AI Agent 约束 |

---

## [C] 版本文档 (`docs/releases/v1.0.0/`)

v1.0.0 版本的完整文档包 (12 个核心文档 + 1 个发布摘要)。

### C1. 核心计划文档 (必读)

| # | 文档 | 核心内容 | 用途 |
|---|------|----------|------|
| 1 | [00-release-summary.md](./docs/releases/v1.0.0/00-release-summary.md) | 发布摘要, 构建信息, 测试结果 | GA 发布时填写 |
| 2 | [DEVELOPMENT_PLAN.md](./docs/releases/v1.0.0/DEVELOPMENT_PLAN.md) | **13 个 ISSUE 映射**, P0/P1/P2 分解, 里程碑 | 开发总纲 |
| 3 | [TEST_PLAN.md](./docs/releases/v1.0.0/TEST_PLAN.md) | A/B/R 三级测试, 391 场景评估 | 测试执行 |
| 4 | [RELEASE_GATE_CHECKLIST.md](./docs/releases/v1.0.0/RELEASE_GATE_CHECKLIST.md) | A/B/R/G 四级门禁执行清单 | 门禁验收 |

### C2. 技术参考文档

| # | 文档 | 核心内容 |
|---|------|----------|
| 5 | [ARCHITECTURE_DECISIONS.md](./docs/releases/v1.0.0/ARCHITECTURE_DECISIONS.md) | 6 项 ADR: SQLRustGo 底座, Ollama 选型, RRF, CLI first |
| 6 | [FEATURE_MATRIX.md](./docs/releases/v1.0.0/FEATURE_MATRIX.md) | 30+ 功能点状态矩阵 (四层 + CLI + 文档) |
| 7 | [INTEGRATION_STATUS.md](./docs/releases/v1.0.0/INTEGRATION_STATUS.md) | 模块依赖链, 外部依赖状态, 数据集就绪 |
| 8 | [PERFORMANCE_TARGETS.md](./docs/releases/v1.0.0/PERFORMANCE_TARGETS.md) | P50/P95/P99 延迟分解, 391 分阶段目标, 资源预算 |

### C3. 发布与追踪文档

| # | 文档 | 核心内容 |
|---|------|----------|
| 9 | [RELEASE_NOTES.md](./docs/releases/v1.0.0/RELEASE_NOTES.md) | 5 大核心特性, 性能基准, 已知限制 |
| 10 | [CHANGELOG.md](./docs/releases/v1.0.0/CHANGELOG.md) | Keep a Changelog 格式变更记录 |
| 11 | [ISSUE_TRACKER.md](./docs/releases/v1.0.0/ISSUE_TRACKER.md) | **Gitea ISSUE 创建清单**, API 命令, 模板 |
| 12 | [DEPLOYMENT_GUIDE.md](./docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md) | 软硬件要求, 安装, Ollama 部署, 故障排查 |

---

## [D] 用户文档 (`docs/user/`)

| 文档 | 描述 | 受众 |
|------|------|------|
| [USER_MANUAL.md](./docs/user/USER_MANUAL.md) | 快速开始, 索引构建, 检索查询, LLM 问答, 391 评估, 配置参考 | 用户 |

---

## [E] 数据资产

| 目录 | 内容 | 规模 |
|------|------|------|
| `gmp-md/` | GMP 知识文档集 (Markdown) | 1696 篇 |
| `gmp-md-masking/` | 脱敏版文档 | 1696 篇 |
| `agent-knowledge/` | 结构化规则知识库 (YAML) | 55+ 条规则 |

---

## [F] 工具脚本 (`scripts/`)

| 脚本 | 用途 |
|------|------|
| `scripts/gate/check_alpha_v100.sh` | Alpha Gate A1-A9 自动化检查 |
| `scripts/compare_retrieval_systems.py` | 多检索方案对比分析 |
| `scripts/mask_gmp_md.py` | GMP 文档脱敏处理 |
| `scripts/build_gmp_md.py` | GMP 文档构建工具 |

---

## 按角色导航

### 如果你是新开发者

1. [README.md](./README.md) — 了解项目
2. [GOVERNANCE_STANDARD.md](./docs/governance/GOVERNANCE_STANDARD.md) — 了解规范
3. [DEVELOPMENT_PLAN.md](./docs/releases/v1.0.0/DEVELOPMENT_PLAN.md) — 了解任务
4. [CONTRIBUTING.md](./docs/governance/CONTRIBUTING.md) — 开始贡献

### 如果你是 AI Agent (Claude/Codex)

1. [AGENTS.md](./AGENTS.md) — AI 开发指令 (GitNexus)
2. [GOVERNANCE_INDEX.md](./docs/governance/GOVERNANCE_INDEX.md) — AI 任务决策树
3. [GATE_SPEC.md](./docs/governance/GATE_SPEC.md) — 门禁约束
4. [VERSION_DOCS_SPEC.md](./docs/governance/VERSION_DOCS_SPEC.md) — 文档格式要求

### 如果你要执行门禁检查

1. [GATE_SPEC.md](./docs/governance/GATE_SPEC.md) — 门禁规范
2. [RELEASE_GATE_CHECKLIST.md](./docs/releases/v1.0.0/RELEASE_GATE_CHECKLIST.md) — 执行清单
3. `scripts/gate/check_alpha_v100.sh` — 自动化脚本

### 如果你想部署使用

1. [DEPLOYMENT_GUIDE.md](./docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md) — 部署指南
2. [USER_MANUAL.md](./docs/user/USER_MANUAL.md) — 用户手册

---

## 文档更新记录

| 日期 | 变更 | 版本 |
|------|------|------|
| 2026-05-21 | 初始化全部 v1.0.0 文档 (20 个文件) | 1.0.0-alpha |
| 2026-05-21 | 创建文档索引 | 1.0.0-alpha |