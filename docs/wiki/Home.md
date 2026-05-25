# LLM-Wiki

> **面向人类和 LLM 的结构化项目文档系统**
> 三层知识体系：QMD（原始）→ GBrain（结构化）→ LLM-Wiki（概念索引）


## 🔗 快速导航

### 基础设施

| 页面 | 说明 |
|------|------|
| [Gitea Infrastructure](../wiki/Gitea-Infrastructure) | Gitea 部署、容器、网络、账户体系 |
| [CI CD Platform](../wiki/CI-CD-Platform) | CI/CD 流水线概览（三阶段演进） |
| [CI CD Platform Deployment](../wiki/CI-CD-Platform-Deployment) | Nomad 双节点部署细节、事故记录 |
| [Nomad Scheduler](../wiki/Nomad-Scheduler) | Nomad v1.9.6 CI runner 调度系统 |
| [Gitea Nomad Health Checks](../wiki/Gitea-Nomad-Health-Checks) | 健康检查清单 + 故障诊断流程 |
| [Git Remote Configuration](../wiki/Git-Remote-Configuration) | SSH 密钥、PAT 令牌、远程配置、多 Agent 协作 |

### AI Agent 协作

| 页面 | 说明 |
|------|------|
| [Hermes Agent](../wiki/Hermes-Agent) | Hermes Agent 配置与使用（Mac Mini 主控） |
| [Multi Agent Orchestration 2026-04](../wiki/Multi-Agent-Orchestration-2026-04) | 三平台多 Agent 协同开发方案 |
| [AI Agent Collaboration Architecture](../wiki/AI-Agent-Collaboration-Architecture) | AI Agent 协作架构（三平台、CI/CD、知识管理） |
| [AI Factory Brainstorming](../wiki/Brainstorming-AI-Factory-Control-Plane) | AI Factory 头脑风暴 - 根因分析 |
| [AI Factory Implementation Plan](../wiki/AI-Factory-Implementation-Plan) | AI Factory 落地执行计划 |
| [agent-development-workflow](../wiki/agent-development-workflow) | Agent 开发工作流 |

### 知识管理

| 页面 | 说明 |
|------|------|
| [GBrain Knowledge](../wiki/GBrain-Knowledge) | 结构化知识库系统（`~/wiki/gbrain/`） |
| [QMD Search](../wiki/QMD-Search) | 本地 BM25 + 向量搜索 + LLM 重排序 |
| [Proof Registry](../wiki/Proof-Registry) | 形式化证明注册表 |

### SQLRustGo 模块

| 页面 | 说明 |
|------|------|
| [Architecture](../wiki/Architecture) | v3.2.0 模块架构（~250K LOC） |
| [executor](../wiki/executor) | 执行器模块 |
| [optimizer](../wiki/optimizer) | 优化器模块 (CBO) |
| [planner](../wiki/planner) | Planner 模块 |
| [storage](../wiki/storage) | 存储引擎模块 |
| [transaction](../wiki/transaction) | 事务管理模块 (MVCC/WAL) |
| [parser](../wiki/Parser) | Parser 模块 |
| [server](../wiki/server) | MySQL 协议服务器 |
| [Modules](../wiki/Modules) | 模块总览 |
| [Patterns](../wiki/Patterns) | 设计模式 |

### 双项目协同

| 页面 | 说明 |
|------|------|
| [GMP-Platform 概述](../wiki/GMP-Platform-Overview) | GMP-Platform 项目概述、技术架构、与 SQLRustGo 的依赖关系 |
| [GMP-Platform Fullstack 设计](../wiki/GMP-Platform-Fullstack-Design) | 前后端架构设计草稿（微信小程序 + React + Axum） |

### SQLRustGo 版本

| 页面 | 说明 |
|------|------|
| [SQLRustGo Overview](../wiki/SQLRustGo-Overview) | 项目概述、版本路线图、治理系统 |
| [v3.4.0 Overview](../wiki/v3.4.0-Overview) | **v3.4.0 GA** (2026-05-24, GMP-Platform v1.0.0) |
| [v3.3.0 Overview](../wiki/v3.3.0-Overview) | v3.3.0 GA (2026-05-20, 已归档) |
| [v3.2.0 Overview](../wiki/v3.2.0-Overview) | v3.2.0 GA (2026-05-17, 已归档) |
| [v3.1.0 Overview](../wiki/v3.1.0-Overview) | v3.1.0 Alpha (2026-05-13, 已归档) |
| [v3.0.0 Overview](../wiki/v3.0.0-Overview) | v3.0.0 GA (2026-05-11, 已归档) |
| [v2.9.0 Overview](../wiki/v2.9.0-Overview) | v2.9.0 GA (2026-05-05, 已归档) |
| [v2.8.0 Overview](../wiki/v2.8.0-Overview) | v2.8.0 GA (2026-05-02, 已归档) |
| [v2.7.0 Overview](../wiki/v2.7.0-Overview) | v2.7.0 GA (2026-04-22, 已归档) |
| [v2.4.0 Overview](../wiki/v2.4.0-Overview) | v2.4.0 GA (2026-04-09, 已归档) |
| [v2.1.0 Overview](../wiki/v2.1.0-Overview) | v2.1.0 GA (2026-04-04, 已归档) |
| [v2.0.0 Overview](../wiki/v2.0.0-Overview) | v2.0.0 GA (2026-03-29, 已归档) |
| [v1.9.0 Overview](../wiki/v1.9.0-Overview) | v1.9.0 GA (2026-03-26, 已归档) |
| [v1.7.0 Overview](../wiki/v1.7.0-Overview) | v1.7.0 GA (2026-03-23, 已归档) |
| [v1.6.0 Overview](../wiki/v1.6.0-Overview) | v1.6.0 GA (2026-03-20, 已归档) |
| [v1.5.0 Overview](../wiki/v1.5.0-Overview) | v1.5.0 GA (2026-03-18, 已归档) |
| [v1.4.0 Overview](../wiki/v1.4.0-Overview) | v1.4.0 GA (2026-03-18, 已归档) |
| [v1.3.0 Overview](../wiki/v1.3.0-Overview) | v1.3.0 GA (2026-03-15, 已归档) |
| [v1.2.0 Overview](../wiki/v1.2.0-Overview) | v1.2.0 GA (2026-03-13, 已归档) |
| [v1.1.0 Overview](../wiki/v1.1.0-Overview) | v1.1.0 GA (2026-03-04, 已归档) |
| [v1.0.0 Overview](../wiki/v1.0.0-Overview) | v1.0.0 GA (2026-02-21, 初始版本) |
| [SQLRustGo-MySQL-Compatibility](../wiki/SQLRustGo-MySQL-Compatibility) | MySQL 5.7/8.0 功能差距矩阵 |
| [OO-Document-Status](../wiki/OO-Document-Status) | OO 文档状态追踪 |
| [Weak-Points-Analysis](../wiki/Weak-Points-Analysis) | 弱项分析与改进路线图 |

### SQLRustGo 功能模块

| 页面 | 说明 |
|------|------|
| [Security Module](../wiki/Security-Module) | 安全模块: 审计/防火墙/TLS |
| [Storage Engine](../wiki/Storage-Engine) | 存储架构: B+Tree/WAL/MVCC |
| [Distributed Module](../wiki/Distributed-Module) | 分布式: 分区/复制/故障转移 |
| [mysql-server](../wiki/mysql-server) | MySQL 协议兼容 |

### GMP-Platform

| 页面 | 说明 |
|------|------|
| [GMP-Platform v1.0.0 Beta B9](gbrain/projects/GMP-Platform-v1.0.0-Beta-B9) | GMP 文档检索平台 B9 测试状态 (96.1%) |
| [GMP-Platform OO 总体设计](https://github.com/yinglichina163/GMP-Platform.wiki/OO-Overall-Design) | 四层检索架构、模块设计 |
| [GMP-Platform OO 执行路径](https://github.com/yinglichina163/GMP-Platform.wiki/OO-Execution-Path) | 执行路径分析 |
| [GMP-Platform OO 详细设计](https://github.com/yinglichina163/GMP-Platform.wiki/OO-gmp-storage-Design) | gmp-storage FTS5 分词实现 |

### 差距分析

| 页面 | 说明 |
|------|------|
| [MySQL Gap Analysis](../wiki/MySQL-Gap-Analysis) | MySQL 兼容性差距分析 |
| [sqlrustgo-distributed-dev](../wiki/sqlrustgo-distributed-dev) | 分布式开发差距分析 |

### 治理与流程

| 页面 | 说明 |
|------|------|
| [Governance System](../wiki/Governance-System) | R1-R7 规则、AV1-AV9 攻击面、STRICT PROOF MODE |
| [gate_spec](../wiki/gate_spec) | 门禁规范 |
| [release_process](../wiki/release_process) | 发布流程 |
| [ISSUE_LIFECYCLE](../wiki/ISSUE_LIFECYCLE) | Issue 生命周期 |
| [Baseline Verification](../wiki/Baseline-Verification) | 基线验证（R5 规则）— 226 PASS / 0 FAIL |

### 历史状态报告

| 页面 | 说明 |
|------|------|
| [SQLRustGo Gitea Status 2026-04](../wiki/SQLRustGo-Gitea-Status-2026-04) | 2026-04 项目综合状态 |
| [SQLRustGo Gitea Comprehensive Status 2026-05](../wiki/SQLRustGo-Gitea-Comprehensive-Status-2026-05) | 2026-05 综合状态 |
| [Hermes-SQLRustGo-Gitea-System-Status-2026-05](../wiki/Hermes-SQLRustGo-Gitea-System-Status-2026-05) | 2026-05 系统状态一览 |
| [v2.8.0 Overview](../wiki/v2.8.0-Overview) | v2.8.0 版本总览 |
| [v2.8.0 Progress](../wiki/v2.8.0-Progress) | v2.8.0 开发进度 |
| [v2.8.0 Assessment](../wiki/v2.8.0-Assessment) | v2.8.0 评估 |
| [v2.8.0 Beta Schedule](../wiki/v2.8.0-Beta-Schedule) | v2.8.0 Beta 日程 |
| [SQLRustGo v2.9.0 Development Governance](../wiki/SQLRustGo-v2.9.0-Development-Governance) | v2.9.0 开发治理 |

### 改进路线图

| 页面 | 说明 |
|------|------|
| [Refactoring Roadmap](../wiki/Refactoring-Roadmap) | 重构路线图 |
| [GitNexus Analysis Plan](../wiki/GitNexus-Analysis-Plan) | GitNexus 分析计划 |

### Agent 与 CI/CD

| 页面 | 说明 |
|------|------|
| [hermes-gate](../wiki/hermes-gate) | Hermes Gate 规范 |
| [Harness Self Check Enhancement 2026-05](../wiki/Harness-Self-Check-Enhancement-2026-05) | Harness 自检增强 |

### 运维

| 页面 | 说明 |
|------|------|
| [Mac Mini Disk Maintenance](../wiki/Mac-Mini-Disk-Maintenance) | Mac Mini 磁盘维护指南 |
| [Gitea Cloud Sync](../wiki/Gitea-Cloud-Sync) | GitCode/Gitee/GitHub 同步配置 |

### 旧版/归档

| 页面 | 说明 |
|------|------|
| [distributed](../wiki/distributed) | 分布式（旧版） |


*最后更新: 2026-05-22*
