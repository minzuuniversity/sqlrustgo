# GMP-Platform 文档索引与导航

> **版本**: 1.3.0-beta | **更新**: 2026-05-24 | **维护**: Hermes Agent

本文档是 GMP-Platform 项目的完整文档索引，按功能域分类并提供快速导航。

---

## 文档分类概览

```
DOCS_INDEX.md                         ← 你在这里
│
├── [A] 设计文档 (docs/design/)         ← 架构设计与技术方案
├── [B] 治理文档 (docs/governance/)     ← 开发规范与流程约束
├── [C] 版本文档 (docs/releases/)       ← 各版本详细文档
│   └── v1.3.0/                       ← 当前开发版本 ★
├── [D] 用户文档 (docs/user/)          ← 使用手册与部署指南
└── [E] 数据资产 (gmp-md/)             ← GMP 知识文档集
```

---

## [A] 设计文档 (`docs/design/`)

系统架构与详细技术设计。

| 文档 | 描述 | 受众 |
|------|------|------|
| [five-layer-kg-pipeline-plan.md](./docs/design/five-layer-kg-pipeline-plan.md) | **五层架构 + Layer 0 管线设计**: 完整五层架构、Layer 0 六步离线抽取管线、Issue #124 规划 | 架构师, 开发者 |
| [gmp-sqlrustgo-four-layer-plan.md](./docs/design/gmp-sqlrustgo-four-layer-plan.md) | **四层架构详细设计**: 数据模型、检索流程、LLM 集成（历史版本） | 架构师 |
| [gmp-retrieval-system-comparison-and-design.md](./docs/design/gmp-retrieval-system-comparison-and-design.md) | **检索方案对比分析**: 多技术选型对比与推荐 | 架构师 |
| [sqlrustgo-incremental-enhancement-plan.md](./docs/design/sqlrustgo-incremental-enhancement-plan.md) | **SQLRustGo 增量补全计划**: BM25/RRF/Reranker/LLM Chat/规则优先级 (给 Hermes) | Hermes, 开发者 |

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
| [GATE_SPEC.md](./docs/governance/GATE_SPEC.md) | **门禁规范 (SSOT)** | L0-Alpha/Beta/RC/GA 四级门禁定义, 检查项, 通过标准 |
| [RELEASE_LIFECYCLE.md](./docs/governance/RELEASE_LIFECYCLE.md) | **版本生命周期** | 版本阶段流转, 分支/Tag 规范, 紧急修复流程 |

### B3. 文档与协作

| 文档 | 角色 | 核心内容 |
|------|------|----------|
| [VERSION_DOCS_SPEC.md](./docs/governance/VERSION_DOCS_SPEC.md) | **版本文档规范** | 命名规则, 元数据要求, 最小文档集 (21-docs) |
| [CONTRIBUTING.md](./docs/governance/CONTRIBUTING.md) | **贡献指南** | 分支策略, Commit 规范, PR 模板, AI Agent 约束 |

---

## [C] 版本文档 (`docs/releases/`)

### C0. 版本概览

| 版本 | 分支 | 状态 | 文档数 |
|------|------|------|--------|
| **v1.3.0** | `develop/v1.3.0` | 🔄 Beta 开发中 | 21-docs |
| v1.2.0 | `develop/v1.2.0` | ✅ Beta | 13-docs |
| v1.1.0 | `develop/v1.1.0` | ✅ GA | 13-docs |
| v1.0.0 | `develop/v1.0.0` | ✅ GA | 13-docs |

---

## [C1] v1.3.0 Beta 版本文档 (`docs/releases/v1.3.0/`)

v1.3.0 是当前活跃开发版本，聚焦五层架构完善、Event Bus、Audit Middleware、MQTT 集成。

> **架构**: 五层 (L0-L5) — 详见 [FEATURE_MATRIX.md](./docs/releases/v1.3.0/FEATURE_MATRIX.md)

### 核心计划文档 (必读)

| # | 文档 | 核心内容 |
|---|------|----------|
| 1 | [00-release-summary.md](./docs/releases/v1.3.0/00-release-summary.md) | 发布摘要, 构建信息, Beta 里程碑 |
| 2 | [DEVELOPMENT_PLAN.md](./docs/releases/v1.3.0/DEVELOPMENT_PLAN.md) | **P0/P1/P2 功能分解**, 里程碑, Issue 映射 |
| 3 | [TEST_PLAN.md](./docs/releases/v1.3.0/TEST_PLAN.md) | 单元/集成/MQTT/性能测试, 391 场景 |
| 4 | [RELEASE_GATE_CHECKLIST.md](./docs/releases/v1.3.0/RELEASE_GATE_CHECKLIST.md) | L0-Alpha/Beta/RC/GA 四级门禁执行清单 |

### 技术参考文档

| # | 文档 | 核心内容 |
|---|------|----------|
| 5 | [FEATURE_MATRIX.md](./docs/releases/v1.3.0/FEATURE_MATRIX.md) | **五层架构 (L0-L5)** + 平台能力 + P0 进度 |
| 6 | [ARCHITECTURE_DECISIONS.md](./docs/releases/v1.3.0/ARCHITECTURE_DECISIONS.md) | ADR: Event Bus, Binary BLOB, WAL |
| 7 | [ARCHITECTURE_DEVIATION_REPORT.md](./docs/releases/v1.3.0/ARCHITECTURE_DEVIATION_REPORT.md) | v1.3.0 架构偏离追踪 |
| 8 | [INTEGRATION_STATUS.md](./docs/releases/v1.3.0/INTEGRATION_STATUS.md) | Workspace 组件矩阵, Event Bus 事件流 |
| 9 | [COMPREHENSIVE_STATUS_REPORT.md](./docs/releases/v1.3.0/COMPREHENSIVE_STATUS_REPORT.md) | 组件/Issues/风险 综合状态 |

### 质量与测试

| # | 文档 | 核心内容 |
|---|------|----------|
| 10 | [TEST_DESIGN.md](./docs/releases/v1.3.0/TEST_DESIGN.md) | 单元/集成/MQTT/性能测试分层设计 |
| 11 | [EVALUATION_REPORT.md](./docs/releases/v1.3.0/EVALUATION_REPORT.md) | 391 场景基线 + v1.3.0 目标 |
| 12 | [COVERAGE_REPORT.md](./docs/releases/v1.3.0/COVERAGE_REPORT.md) | Crate 覆盖率目标 + llvm-cov 命令 |
| 13 | [PERFORMANCE_REPORT.md](./docs/releases/v1.3.0/PERFORMANCE_REPORT.md) | v1.2.0 基线 + v1.3.0 性能目标 |

### 发布与追踪

| # | 文档 | 核心内容 |
|---|------|----------|
| 14 | [RELEASE_NOTES.md](./docs/releases/v1.3.0/RELEASE_NOTES.md) | Event Bus / Audit Middleware / MQTT / Binary BLOB 变更 |
| 15 | [CHANGELOG.md](./docs/releases/v1.3.0/CHANGELOG.md) | Keep a Changelog 格式变更记录 |
| 16 | [ISSUE_TRACKER.md](./docs/releases/v1.3.0/ISSUE_TRACKER.md) | #121~#132 Issue 状态 |
| 17 | [PROGRESS_REPORT.md](./docs/releases/v1.3.0/PROGRESS_REPORT.md) | 里程碑完成, 周期统计 |
| 18 | [FEATURE_TRACKING_MATRIX.md](./docs/releases/v1.3.0/FEATURE_TRACKING_MATRIX.md) | P0/P1/P2 功能追踪 |

### 部署与用户

| # | 文档 | 核心内容 |
|---|------|----------|
| 19 | [DEPLOYMENT_GUIDE.md](./docs/releases/v1.3.0/DEPLOYMENT_GUIDE.md) | 二进制构建, Ollama, MQTT 部署 |
| 20 | [INCREMENTAL_UPDATE_GUIDE.md](./docs/releases/v1.3.0/INCREMENTAL_UPDATE_GUIDE.md) | v1.2.0 → v1.3.0 升级指南 |
| 21 | [USER_MANUAL.md](./docs/releases/v1.3.0/USER_MANUAL.md) | CLI / MQTT / FAQ |

### 安装与初始化

| 文档 | 描述 |
|------|------|
| [docs/INSTALLATION_AND_INITIALIZATION.md](../INSTALLATION_AND_INITIALIZATION.md) | **五层架构安装指南**: 系统要求, 源码构建, Layer 0 管线, 全量重建 |

---

## [C2] v1.2.0 Beta 版本文档 (`docs/releases/v1.2.0/`)

| 文档 | 核心内容 |
|------|----------|
| [00-release-summary.md](./docs/releases/v1.2.0/00-release-summary.md) | Agent 编排, JWT, E2E 测试 |
| [CHANGELOG.md](./docs/releases/v1.2.0/CHANGELOG.md) | v1.2.0 变更记录 |
| [PROGRESS_REPORT.md](./docs/releases/v1.2.0/PROGRESS_REPORT.md) | 四层架构状态, 组件进度 |

---

## [C3] v1.0.0 GA 版本文档 (`docs/releases/v1.0.0/`)

首个正式版本，四层架构，391 场景通过率 96.08%。

### 核心文档

| # | 文档 | 核心内容 |
|---|------|----------|
| 1 | [00-release-summary.md](./docs/releases/v1.0.0/00-release-summary.md) | 发布摘要, GA 里程碑 |
| 2 | [DEVELOPMENT_PLAN.md](./docs/releases/v1.0.0/DEVELOPMENT_PLAN.md) | 13 个 ISSUE 映射, P0/P1/P2 分解 |
| 3 | [TEST_PLAN.md](./docs/releases/v1.0.0/TEST_PLAN.md) | A/B/R 三级测试, 391 场景评估 |
| 4 | [RELEASE_GATE_CHECKLIST.md](./docs/releases/v1.0.0/RELEASE_GATE_CHECKLIST.md) | A/B/R/G 四级门禁执行清单 |
| 5 | [FEATURE_MATRIX.md](./docs/releases/v1.0.0/FEATURE_MATRIX.md) | 四层架构 + CLI + 文档功能矩阵 |
| 6 | [ARCHITECTURE_DECISIONS.md](./docs/releases/v1.0.0/ARCHITECTURE_DECISIONS.md) | 6 项 ADR |
| 7 | [RELEASE_NOTES.md](./docs/releases/v1.0.0/RELEASE_NOTES.md) | 5 大核心特性, 性能基准 |
| 8 | [CHANGELOG.md](./docs/releases/v1.0.0/CHANGELOG.md) | Keep a Changelog 格式 |
| 9 | [ISSUE_TRACKER.md](./docs/releases/v1.0.0/ISSUE_TRACKER.md) | Gitea ISSUE 创建清单 |
| 10 | [DEPLOYMENT_GUIDE.md](./docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md) | 部署指南 |
| 11 | [USER_MANUAL.md](./docs/user/USER_MANUAL.md) | 用户手册 |

---

## [D] 用户文档 (`docs/user/`)

| 文档 | 描述 |
|------|------|
| [USER_MANUAL.md](./docs/user/USER_MANUAL.md) | 快速开始, 索引构建, 检索查询, LLM 问答, 391 评估 |

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
3. [DEVELOPMENT_PLAN.md](./docs/releases/v1.3.0/DEVELOPMENT_PLAN.md) — 了解任务
4. [CONTRIBUTING.md](./docs/governance/CONTRIBUTING.md) — 开始贡献

### 如果你是 AI Agent (Claude/Codex)

1. [AGENTS.md](./AGENTS.md) — AI 开发指令 (GitNexus)
2. [GOVERNANCE_INDEX.md](./docs/governance/GOVERNANCE_INDEX.md) — AI 任务决策树
3. [GATE_SPEC.md](./docs/governance/GATE_SPEC.md) — 门禁约束
4. [VERSION_DOCS_SPEC.md](./docs/governance/VERSION_DOCS_SPEC.md) — 文档格式要求 (21-docs)

### 如果你要执行门禁检查

1. [GATE_SPEC.md](./docs/governance/GATE_SPEC.md) — 门禁规范
2. [RELEASE_GATE_CHECKLIST.md](./docs/releases/v1.3.0/RELEASE_GATE_CHECKLIST.md) — 执行清单
3. `scripts/gate/check_alpha_v100.sh` — 自动化脚本

### 如果你要部署使用

1. [DEPLOYMENT_GUIDE.md](./docs/releases/v1.3.0/DEPLOYMENT_GUIDE.md) — 部署指南
2. [USER_MANUAL.md](./docs/releases/v1.3.0/USER_MANUAL.md) — 用户手册
3. [INSTALLATION_AND_INITIALIZATION.md](./docs/INSTALLATION_AND_INITIALIZATION.md) — 安装与初始化

---

## 文档更新记录

| 日期 | 变更 | 版本 |
|------|------|------|
| 2026-05-24 | 更新为 v1.3.0-beta，添加五层架构概览，v1.3.0 完整 21-docs 索引 | 1.3.0-beta |
| 2026-05-24 | 修正 FEATURE_MATRIX.md 层编号错误 (4层→5层) | 1.3.0-beta |
| 2026-05-21 | 初始化全部 v1.0.0 文档 (20 个文件) | 1.0.0-alpha |
