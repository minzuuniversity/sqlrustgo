# GMP 内审检索系统 治理标准总纲

> **版本**: 1.0
> **创建日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **用途**: GMP 内审检索系统项目的核心治理规范，所有 AI Agent 和开发者必须遵循的最高层治理文档

> **SSOT 声明**: 本文档是 GMP 内审检索系统治理体系的最高层规范文档，定义治理原则、版本生命周期和文档体系索引。所有其他治理文档必须与本文档保持一致。

---

## 一、治理体系概述

### 1.1 治理体系架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                    GOVERNANCE_STANDARD.md (本文档)                   │
│                         治理标准总纲 · SSOT 最高层                   │
├─────────────────────────────────────────────────────────────────────┤
│                              │                                       │
│        ┌─────────────────────┼─────────────────────┐                │
│        ▼                     ▼                     ▼                │
│ ┌──────────────┐  ┌──────────────────┐  ┌─────────────────────┐   │
│ │   AI合规     │  │   版本生命周期     │  │    文档体系         │   │
│ │ 执行机制     │  │   管理             │  │    索引             │   │
│ │              │  │                   │  │                     │   │
│ │ 引用:        │  │ 引用:             │  │ 引用:               │   │
│ │ CONTRIBUTING │  │ RELEASE_LIFECYCLE │  │ GOVERNANCE_INDEX    │   │
│ └──────────────┘  └──────────────────┘  └─────────────────────┘   │
│        │                     │                     │               │
│        └─────────────────────┼─────────────────────┘                │
│                              ▼                                       │
│ ┌────────────────────────────────────────────────────────────────┐  │
│ │                    分阶段门禁 + Issue 追踪                        │  │
│ ├────────────────────────────────────────────────────────────────┤  │
│ │  门禁规范 SSOT: GATE_SPEC.md                                     │  │
│ │  门禁检查清单: RELEASE_GATE_CHECKLIST.md (per version)            │  │
│ │  测试计划模版: TEST_PLAN_TEMPLATE.md                              │  │
│ │  开发计划模版: DEVELOPMENT_PLAN_TEMPLATE.md                       │  │
│ └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 治理目标

| 目标 | 说明 |
|------|------|
| **质量保证** | 所有发布版本必须通过定义的门禁检查 |
| **可追溯性** | 从需求到代码到发布全链路可追溯 |
| **一致性** | 跨版本、跨阶段开发活动遵循统一规范 |
| **合规性** | AI Agent 行为必须符合治理规范 |
| **持续改进** | 治理体系通过反馈机制持续优化 |

### 1.3 适用范围

- **项目**: GMP 内审检索系统 (gmp-retrieve)
- **版本**: v1.0.0 及以上
- **阶段**: Alpha / Beta / RC / GA 全生命周期
- **角色**: Human Architect、AI Developer、CI System
- **文档**: 所有 governance 和 releases 相关文档

---

## 二、核心治理原则

### 2.1 五大核心原则

```
┌─────────────────────────────────────────────────────────────────────┐
│                        五大核心原则                                   │
├─────────────────────────────────────────────────────────────────────┤
│  1. SSOT 原则     │ 单一权威来源，规范不得重复定义                    │
│  2. 门禁强制原则   │ 所有发布必须通过对应阶段门禁                      │
│  3. Issue 闭环原则 │ FAIL → Issue → PR → 验证 → 关闭，无例外          │
│  4. 版本延续原则   │ 未完成任务必须映射到下版本，不得丢失               │
│  5. 证据驱动原则   │ 所有检查必须记录证据                             │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 SSOT 代表

| 文档类型 | SSOT 文档 | 
|----------|-----------|
| 门禁规范 | `docs/governance/GATE_SPEC.md` |
| 版本生命周期 | `docs/governance/RELEASE_LIFECYCLE.md` |
| 文档导航 | `docs/governance/GOVERNANCE_INDEX.md` |
| 开发计划 | `docs/governance/DEVELOPMENT_PLAN_TEMPLATE.md` |
| 测试计划 | `docs/governance/TEST_PLAN_TEMPLATE.md` |

### 2.3 禁止的模式

```text
❌ 规范冲突: 在版本文档中定义与 GATE_SPEC.md 不一致的检查项
❌ 门禁绕过: Gate FAIL 但跳过 Issue 创建直接合并
❌ Issue 丢失: 门禁失败但未创建 Issue，任务未延续到下版本
❌ 证据缺失: 检查通过但未记录命令、输出、阈值等证据
❌ 版本混乱: Tag 与版本号不一致
❌ AI 越权: AI Agent 执行需要 Human Architect 审批的操作
```

### 2.4 正确的模式

```text
✅ 规范遵循: 检查项引用 GATE_SPEC.md，不自行定义
✅ 门禁闭环: Gate FAIL → Issue #N → PR 修复 → 验证 PASS → 关闭 Issue
✅ 任务延续: 当前版本无法修复 → 在 v{NEXT} DEVELOPMENT_PLAN.md 建立映射
✅ 证据完整: {command, exit_code, output_summary, threshold, pass}
✅ 版本规范: v{MAJOR}.{MINOR}.{PATCH}
✅ 权限分明: AI 执行需审批操作前先创建 Issue，等待 Human Architect 决策
```

---

## 三、AI 合规执行机制

### 3.1 AI 可执行任务清单

| 任务类型 | 触发关键词 | 必须使用的模版 |
|----------|------------|----------------|
| 版本开发计划 | "开发计划"、"版本计划" | `DEVELOPMENT_PLAN_TEMPLATE.md` |
| 测试计划 | "测试计划"、"测试策略" | `TEST_PLAN_TEMPLATE.md` |
| 门禁检查 | "门禁检查"、"Gate" | `GATE_CHECKLIST_TEMPLATE.md` |
| Issue 追踪 | "创建 Issue"、"关闭 Issue" | `RELEASE_GATE_CHECKLIST.md` |

### 3.2 AI 禁止执行的操作

```text
❌ 禁止: 未使用模版创建 governance 文档
❌ 禁止: 在非 SSOT 文档中定义门禁检查项
❌ 禁止: 在没有 PR 证据的情况下关闭 Issue
❌ 禁止: 跳过门禁检查直接合并代码
❌ 禁止: 执行需要 Human Architect 审批的操作（合并 PR、发布版本等）
```

---

## 四、版本生命周期

> **详见**: [RELEASE_LIFECYCLE.md](./RELEASE_LIFECYCLE.md)

四级门禁模型:
```
A-Gate → B-Gate → R-Gate → G-Gate
 (α入口)  (β入口)  (RC入口)  (GA入口)
```

### 版本号规范 (Semantic Versioning)

```
v{MAJOR}.{MINOR}.{PATCH}

MAJOR: 不兼容的 API 修改
MINOR: 向下兼容的功能新增
PATCH: 向下兼容的问题修正
```

---

## 五、文档体系索引

> **详见**: [GOVERNANCE_INDEX.md](./GOVERNANCE_INDEX.md)

### 5.1 治理文档 (docs/governance/)

| 文档 | 用途 | 强制级别 |
|------|------|----------|
| `GOVERNANCE_STANDARD.md` | 治理标准总纲 (SSOT) | 强制 |
| `GATE_SPEC.md` | 门禁检查规范 | 强制 |
| `DEVELOPMENT_PLAN_TEMPLATE.md` | 开发计划模版 | 强制 |
| `TEST_PLAN_TEMPLATE.md` | 测试计划模版 | 强制 |
| `GATE_CHECKLIST_TEMPLATE.md` | 门禁检查清单模版 | 强制 |
| `VERSION_DOCS_SPEC.md` | 版本文档规范 | 强制 |
| `GOVERNANCE_INDEX.md` | 文档导航 | 强制 |
| `RELEASE_LIFECYCLE.md` | 版本生命周期 | 强制 |
| `CONTRIBUTING.md` | 贡献指南 | 强制 |

### 5.2 版本文档 (docs/releases/vX.Y.Z/)

| 文档 | 用途 | 阶段要求 |
|------|------|----------|
| `DEVELOPMENT_PLAN.md` | 开发计划 | Alpha+ |
| `TEST_PLAN.md` | 测试计划 | Alpha+ |
| `RELEASE_GATE_CHECKLIST.md` | 门禁检查清单 | Beta+ |
| `RELEASE_NOTES.md` | 发布说明 | RC+ |
| `CHANGELOG.md` | 变更日志 | RC+ |
| `FEATURE_MATRIX.md` | 功能矩阵 | Beta+ |
| `PERFORMANCE_TARGETS.md` | 性能目标 | Beta+ |
| `INTEGRATION_STATUS.md` | 功能集成状态 | Beta+ |
| `ARCHITECTURE_DECISIONS.md` | ADR | RC+ |
| `USER_MANUAL.md` | 用户手册 | RC+ |
| `DEPLOYMENT_GUIDE.md` | 部署指南 | RC+ |
| `00-release-summary.md` | 发布摘要 | GA |