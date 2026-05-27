# GMP-Platform 项目概述

> **状态**: v1.0.0 开发中
> **仓库**: `openclaw/GMP-Platform`
> **分支**: `develop/v1.0.0`
> **设计文档**: [GMP-Platform Fullstack 设计](../wiki/GMP-Platform-Fullstack-Design)

---

## 一、项目定位

**GMP-Platform** 是基于 SQLRustGo 的 GMP（良好作业规范）合规内审全栈平台，面向制药/食品/医疗器械企业的生产质量管理系统。

**核心价值**: 将 GMP 合规要求固化为可执行的工作流，通过四层 AI 隔离架构确保数据完整性（ALCOA 原则）和审计链不可篡改性。

---

## 二、与 SQLRustGo 的关系

```
GMP-Platform (上层应用)
    └── REST API 调用
            └── SQLRustGo (底层引擎)
                ├── RDBMS 存储 (偏差/CAPA/称量记录)
                ├── Vector HNSW (文档语义检索)
                ├── FTS BM25 (全文检索)
                ├── Graph (实体关系)
                └── Audit Chain (不可篡改审计链)
```

**依赖接口**:
- `sqlrustgo-gmp-api` crate (已集成到 SQLRustGo workspace)
- REST API (Axum, port 8080)
- gmp-cli (命令行检索工具)

---

## 三、四层 AI 隔离架构

```
┌─────────────────────────────────────────────┐
│  微信小程序 (移动端)                           │
└────────────────┬────────────────────────────┘
                 │ HTTPS + JWT
                 ▼
┌─────────────────────────────────────────────┐
│  L2 Cloud Zone: HermesAgent 编排层             │
│  (OCR/ASR 多模态理解, Prompt Firewall)        │
└────────────────┬────────────────────────────┘
                 │ DSL (仅抽象指令)
                 ▼
┌─────────────────────────────────────────────┐
│  L3 Validation Zone: 本地验证器                │
│  (DSL 验证, 权限检查, 审计验证)                │
└────────────────┬────────────────────────────┘
                 │ 验证通过的指令
                 ▼
┌─────────────────────────────────────────────┐
│  L0 Trust Zone: SQLRustGo 可信执行层           │
│  (规则引擎, 审计链, ALCOA 证据组装)            │
└─────────────────────────────────────────────┘
```

---

## 四、核心功能模块

| 模块 | 功能 | 状态 |
|------|------|------|
| 偏差管理 | 偏差上报→调查→关闭全流程 | 设计完成 |
| 称量记录 | 扫码→称量→规则判定→审计链固证 | 设计完成 |
| CAPA | 偏差关联→纠正/预防措施→有效性验证 | 设计完成 |
| 审计追踪 | ALCOA 证据链, SHA-256 哈希链 | 设计完成 |
| GMP 知识检索 | 四通道 RAG (BM25+向量+Graph+RRF) | ✅ Beta B9 PASSED (96.1%) |
| 用户/权限 | 角色: admin/auditor/operator/reviewer | 待开发 |

---

## 五、技术栈

| 层次 | 技术 |
|------|------|
| 底层引擎 | SQLRustGo v3.4.0 (RDBMS + Vector + FTS + Graph) |
| 本地 LLM | Ollama (qwen2.5:14b, bge-m3, bge-reranker-v2-m3) |
| 后端框架 | Axum (Rust) |
| 前端框架 | React 18 + TypeScript + shadcn/ui |
| 移动端 | 微信小程序 |
| 认证 | JWT |
| 实时通信 | SSE (Server-Sent Events) |

---

## 六、已确认设计决策

1. **移动端**: 微信小程序
2. **多租户**: 否 (单工厂部署)
3. **硬件集成**: 否 (手动输入)
4. **LLM**: 本地 Ollama (数据不出厂，合规要求)

---

## 七、当前进度

- [x] 设计阶段: GMP-Platform Fullstack Design (PR #63)
- [ ] Phase 0: 脚手架 + JWT + 数据库迁移
- [ ] Phase 1: 核心 CRUD + 审计链
- [ ] Phase 2: 检索 API 集成
- [ ] Phase 3: React 管理后台
- [ ] Phase 4: 微信小程序
- [ ] Phase 5: SSE 实时 + 报表

---

*最后更新: 2026-05-22*
