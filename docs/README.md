# SQLRustGo 文档索引

> **最后更新**: 2026-06-17
> **当前版本**: v3.9.0 (RC7 ✅ + Sprint 8 ✅, GA 目标 2026-12-15)
> **最新稳定**: v3.8.0 (GA, 2026-06-08)

---

## 一、文档目录结构

```
docs/
├── releases/                    # 版本发布文档 (40+ 版本)
│   ├── v3.9.0/                 # v3.9.0 (当前开发版本, RC7)
│   ├── v3.8.0/                 # v3.8.0 (GA, 2026-06-08)
│   ├── v3.7.0/                 # v3.7.0
│   └── ...
│
├── governance/                   # 治理文档 (P1-P10 框架)
│
├── plans/                       # 计划文档 (119 项)
│
├── architecture/                # 架构文档
│
├── formal/                      # 形式化验证文档
│
├── openspec/                    # 开放规范 (SQL 标准, MySQL 协议)
│
├── benchmark/                   # 性能基准测试
│
├── proof/                       # 证明/证据文档
│
├── audit/                       # 审计文档
│
├── analysis/                    # 分析文档
│
├── discovery/                   # 探索文档
│
├── security/                    # 安全文档
│
├── standard/                    # 标准文档
│
├── tutorials/                   # 教程文档
│
├── issues/                      # Issue 文档
│
├── runbooks/                    # 运维手册
│
├── design/                     # 设计文档
│
├── AI增强软件工程/              # AI 协作规范
│
├── 教学计划/                    # 教师准备材料
│
└── 教学实践/                    # 学生实践材料
```

---

## 二、版本发布文档 (最新版本)

### v3.9.0 (当前开发版本: RC7 + Sprint 8)

| 文档 | 说明 |
|------|------|
| [文档索引](releases/v3.9.0/README.md) | v3.9.0 文档总入口 (v2.0) |
| [变更日志](releases/v3.9.0/CHANGELOG.md) | v3.9.0 变更记录 (v1.1, +Sprint 8) |
| [路线图](releases/v3.9.0/ROADMAP.md) | 6 Phase 路线图 (v2.0, +Sprint 8) |
| [综合索引](releases/v3.9.0/INDEX.md) | 综合索引 (v3.0.1, +Sprint 8) |
| [全面评估](releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md) | v3.9.0 综合评估 v2.0 (RC7 + Sprint 8) |
| [GA 门禁报告](releases/v3.9.0/GA_GATE_REPORT.md) | GA 质量门禁 (16/16 + 5 meta-gates) |
| [GA 门禁状态](releases/v3.9.0/GA_GATE_STATUS_REPORT.md) | GA 治理合规状态 |
| [Long-stability 分析](releases/v3.9.0/LONG_STABILITY_TESTS_ANALYSIS.md) | 26 long-running tests (Sprint 8) |
| [测试真实性报告](releases/v3.9.0/TEST_TRUTHFULNESS_REPORT.md) | V1-V8 漏洞状态 (Sprint 8 更新) |
| [发布说明](releases/v3.9.0/RELEASE_NOTES.md) | 发布说明 (含 Sprint 8 增量) |

**状态**: RC7 ✅ (2026-06-12) + Sprint 8 ✅ (2026-06-17, PR #3465) → GA 待 Z6G4 真实 24h+ soak
**门禁**: 16/16 G1-G16 + 5/5 P11-P15 + 1/1 P16 = **22/22** | TPC-H 22/22 (Q8 = 0.18ms) | Corpus 818/818

### v3.8.0 (当前稳定版本: GA)

| 文档 | 说明 |
|------|------|
| [文档索引](releases/v3.8.0/README.md) | v3.8.0 文档总入口 |
| [变更日志](releases/v3.8.0/CHANGELOG.md) | v3.8.0 变更记录 |
| [GA 门禁报告](releases/v3.8.0/GA_GATE_REPORT.md) | GA 质量门禁结果 |
| [版本说明](releases/v3.8.0/RELEASE_NOTES.md) | 发布说明 |

**状态**: GA (2026-06-08)
**里程碑**: 3430+ PR 合并 | TPC-H 22/22 | Corpus 100%

### v3.7.0 (历史版本)

| 文档 | 说明 |
|------|------|
| [文档索引](releases/v3.7.0/README.md) | v3.7.0 文档总入口 |
| [变更日志](releases/v3.7.0/CHANGELOG.md) | v3.7.0 变更记录 |

---

## 三、治理文档

> **治理框架**: P1-P10 10 原则 (v3.8.0+)

| 文档 | 说明 |
|------|------|
| [治理索引](governance/INDEX.md) | 治理文档总入口 |
| [Governance System](governance/GOVERNANCE_SYSTEM.md) | P1-P10 治理框架 |
| [10 原则](governance/ENGINEERING_EVOLUTION_STANDARD.md) | CMM 4+ 轻量版标准 |
| [Baseline Verification](governance/Baseline-Verification.md) | 基线验证流程 |
| [Proof Registry](governance/Proof-Registry.md) | 证明注册表 |
| [ADR 索引](governance/adr/) | 架构决策记录 (12 项) |
| [Gate 脚本](governance/GA_SCRIPTS_SKILLS_REGISTRY.md) | 78 个门禁脚本 |

**门禁脚本位置**: `scripts/gate/`

---

## 四、版本历史总览

| 版本 | 发布日期 | 状态 | 核心特性 |
|------|----------|------|----------|
| v3.9.0 | 2026-12-15 (目标) | RC7 + Sprint 8 | Production Readiness (Q8 0.18ms, 5/5 meta-gates, soak infra) |
| v3.8.0 | 2026-06-08 | GA | Architecture Unification |
| v3.7.0 | 2026-05-31 | GA | GMP Integration |
| v3.6.0 | 2026-05-29 | GA | Protocol Stack |
| v3.5.0 | 2026-05-28 | GA | AI Native GMP |
| v3.4.0 | 2026-05-24 | GA | TPC-H 22/22 |
| v3.3.0 | 2026-05-20 | GA | Corpus 818/818 |
| v3.2.0 | 2026-05-17 | GA | WAL + MVCC |

详细版本历史见: [VERSION_HISTORY.md](releases/VERSION_HISTORY.md)

---

## 五、核心开发文档

| 文档 | 说明 |
|------|------|
| [架构概览](../architecture) | 系统架构 |
| [执行路径](architecture/EXECUTION_PATH.md) | SQL 执行路径 |
| [模块生命周期](architecture/MODULE_LIFECYCLE.md) | 模块管理 |
| [分支策略](governance/BRANCH_GOVERNANCE.md) | Git 分支管理 |
| [开发流程](governance/DEVELOPMENT_PROCESS.md) | 开发流程 |

---

## 六、AI 增强软件工程

| 文档 | 说明 |
|------|------|
| [AI Agent 提示词](AI增强软件工程/AI_AGENT_PROMPTS.md) | 4 AI Agent 提示词 |
| [多 Agent 配置](AI增强软件工程/MULTI_AGENT_CONFIG.md) | 环境配置指南 |
| [多身份隔离](AI增强软件工程/MULTI_IDENTITY_DEVELOPMENT_MODEL.md) | 四账号权限体系 |

---

## 七、教学材料

### 教学计划 (教师材料)

| 文档 | 说明 |
|------|------|
| [教学进度计划](教学计划/AI增强软件工程-教学进度计划.md) | 教学进度安排 |
| [上机实验指导书](教学计划/上机实验指导书.md) | 实验指导 |
| [实验报告模版](教学计划/实验报告模版.md) | 报告模板 |

### PPT 讲义

| 讲次 | 主题 | 文档 |
|------|------|------|
| 第1讲 | 软件工程概述与项目导论 | [PPT](教学计划/PPT/第1讲-软件工程概述与项目导论.md) |
| 第2讲 | 结构化设计与UML基础 | [PPT](教学计划/PPT/第2讲-结构化设计与UML基础.md) |
| 第3讲 | 面向对象设计与类图 | [PPT](教学计划/PPT/第3讲-面向对象设计与类图.md) |
| 第4讲 | 顺序图状态图与架构设计 | [PPT](教学计划/PPT/第4讲-顺序图状态图与架构设计.md) |
| 第5讲 | 架构设计原理与SQLRustGo架构 | [PPT](教学计划/PPT/第5讲-架构设计原理与SQLRustGo架构.md) |
| 第6讲 | 功能模块划分与接口设计 | [PPT](教学计划/PPT/第6讲-功能模块划分与接口设计.md) |
| 第7讲 | AI辅助核心模块实现 | [PPT](教学计划/PPT/第7讲-AI辅助核心模块实现.md) |
| 第8讲 | 测试驱动开发与Alpha版本 | [PPT](教学计划/PPT/第8讲-测试驱动开发与Alpha版本.md) |
| 第9讲 | 软件治理与分支策略 | [PPT](教学计划/PPT/第9讲-软件治理与分支策略.md) |
| 第10讲 | PR工作流与项目成熟度评估 | [PPT](教学计划/PPT/第10讲-PR工作流与项目成熟度评估.md) |

### 教学实践 (学生材料)

| 文档 | 说明 |
|------|------|
| [教学实践索引](教学实践/README.md) | 学生实践材料索引 |
| [学生执行手册](教学实践/v1.1.0-beta/handbook-student.md) | 学生可复现步骤 |
| [助教执行手册](教学实践/v1.1.0-beta/handbook-ta.md) | PR 证据链示例 |

---

## 八、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 3.1 | 2026-06-17 | 更新 v3.9.0 RC7 + Sprint 8 状态 (Q8 0.18ms, 5/5 meta-gates, soak infra, PR #3465) |
| 3.0 | 2026-06-17 | 更新为 v3.9.0 RC7，整理目录结构 |
| 2.1 | 2026-04-22 | 更新为 v2.7.0 GA，添加 v2.7.0 文档入口 |
| 2.0 | 2026-04-17 | 更新为 v2.6.0，清理过时版本链接 |
| 1.1 | 2026-03-05 | 新增 v1.3.0 计划，重组教学材料目录 |
| 1.0 | 2026-03-04 | 初始版本，整合所有文档索引 |

---

## 九、相关链接

- **Gitea 仓库**: http://192.168.0.252:3000/openclaw/sqlrustgo
- **Gitea Wiki**: http://192.168.0.252:3000/openclaw/sqlrustgo/wiki
- **Milestone v3.9.0**: http://192.168.0.252:3000/openclaw/sqlrustgo/milestones/32

---

*本文档由 Hermes Agent 维护*
*最后更新: 2026-06-17*
