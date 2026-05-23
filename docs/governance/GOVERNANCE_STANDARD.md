# GMP 内审检索系统 治理标准总纲

> **版本**: 2.0
> **创建日期**: 2026-05-23
> **维护人**: hermes-agent
> **用途**: GMP 内审检索系统项目的核心治理规范，所有 AI Agent 和开发者必须遵循的最高层治理文档
> **SSOT**: 本文档是 GMP-Platform 治理体系的最高层规范文档，定义治理原则、版本生命周期和文档体系索引。所有其他治理文档必须与本文档保持一致。

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
│ │   执行机制     │  │   管理             │  │    索引             │   │
│ │              │  │                   │  │                     │   │
│ │ 引用:        │  │ 引用:             │  │ 引用:               │   │
│ │ CONTRIBUTING │  │ RELEASE_LIFECYCLE │  │ GOVERNANCE_INDEX    │   │
│ │ AI_COMPLIANCE│  │ VERSION_DOCS_SPEC │  │ VERSION_DOCS_SPEC  │   │
│ └──────────────┘  └──────────────────┘  └─────────────────────┘   │
│        │                     │                     │               │
│        └─────────────────────┼─────────────────────┘                │
│                              ▼                                       │
│ ┌────────────────────────────────────────────────────────────────┐  │
│ │                    分阶段门禁 + Issue 追踪                        │  │
│ ├────────────────────────────────────────────────────────────────┤  │
│ │  门禁规范 SSOT: gate_spec_v120.md (v1.2.0+)                     │  │
│ │  门禁检查清单: RELEASE_GATE_CHECKLIST.md (per version)            │  │
│ │  文档完整性: VERSION_DOCS_SPEC.md (21 docs mandatory)            │  │
│ └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 治理目标

| 目标 | 说明 |
|------|------|
| **质量保证** | 所有发布版本必须通过定义的门禁检查 |
| **可追溯性** | 从需求到代码到发布全链路可追溯 |
| **一致性** | 跨版本、跨阶段开发活动遵循统一规范 |
| **文档完整性** | 每个版本必须有 21 个文档，否则禁止门禁通过 |
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

### 2.1 七大核心原则

```
┌─────────────────────────────────────────────────────────────────────┐
│                        七大核心原则                                   │
├─────────────────────────────────────────────────────────────────────┤
│  1. SSOT 原则       │ 单一权威来源，规范不得重复定义                  │
│  2. 门禁强制原则     │ 所有发布必须通过对应阶段门禁                    │
│  3. Issue 闭环原则   │ FAIL → Issue → PR → 验证 → 关闭，无例外      │
│  4. 版本延续原则     │ 未完成任务必须映射到下版本，不得丢失            │
│  5. 证据驱动原则     │ 所有检查必须记录证据（命令 + 输出 + 阈值）      │
│  6. OpenSpec 原则   │ 所有开发任务必须先 SPEC 分析，再执行            │
│  7. 文档完整原则     │ 21 docs 未齐，禁止进入下一阶段门禁              │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 SSOT 代表

| 文档类型 | SSOT 文档 |
|----------|-----------|
| 门禁规范 | `docs/governance/gate_spec_v120.md` |
| 版本生命周期 | `docs/governance/RELEASE_LIFECYCLE.md` |
| 文档完整性规范 | `docs/governance/VERSION_DOCS_SPEC.md` |
| 文档导航 | `docs/governance/GOVERNANCE_INDEX.md` |
| 治理标准总纲 | `docs/governance/GOVERNANCE_STANDARD.md` |

### 2.3 禁止的模式

```
❌ 规范冲突: 在版本文档中定义与 gate_spec_v120.md 不一致的检查项
❌ 门禁绕过: Gate FAIL 但跳过 Issue 创建直接合并
❌ Issue 丢失: 门禁失败但未创建 Issue，任务未延续到下版本
❌ 证据缺失: 检查通过但未记录命令、输出、阈值等证据
❌ 版本混乱: Tag 与版本号不一致
❌ 文档缺失: 21 docs 未齐就进入 Beta/Rc/GA 门禁
❌ 无 SPEC 开发: 未编写 SPEC 文档就直接写代码
❌ AI 越权: AI Agent 执行需要 Human Architect 审批的操作
```

### 2.4 正确的模式

```
✅ 规范遵循: 检查项引用 gate_spec_v120.md，不自行定义
✅ 门禁闭环: Gate FAIL → Issue #N → PR 修复 → 验证 PASS → 关闭 Issue
✅ 任务延续: 当前版本无法修复 → 在 v{NEXT} DEVELOPMENT_PLAN.md 建立映射
✅ 证据完整: {command, exit_code, output_summary, threshold, pass}
✅ 版本规范: v{MAJOR}.{MINOR}.{PATCH}
✅ 文档完整: Beta 前 21 docs 必须齐全（见 VERSION_DOCS_SPEC.md）
✅ 权限分明: AI 执行需审批操作前先创建 Issue，等待 Human Architect 决策
✅ 先 SPEC 后代码: 所有开发任务先编写 SPEC 再实现（见 3.3）
```

---

## 三、文档完整性强制条款

### 3.1 21 docs 强制要求（版本 2.0 新增）

每个版本在 **任意阶段门禁通过前**，必须完成 21 个文档：

| 类别 | 数量 | 包含 |
|------|------|------|
| 核心文档 | 9 | 00-release-summary, RELEASE_NOTES, CHANGELOG, DEVELOPMENT_PLAN, TEST_PLAN, RELEASE_GATE_CHECKLIST, FEATURE_MATRIX, PERFORMANCE_REPORT, USER_MANUAL |
| 架构与设计 | 5 | ARCHITECTURE_DECISIONS, ARCHITECTURE_DEVIATION_REPORT, COMPREHENSIVE_STATUS_REPORT, DEPLOYMENT_GUIDE, INCREMENTAL_UPDATE_GUIDE |
| 测试与质量 | 4 | TEST_DESIGN, EVALUATION_REPORT, COVERAGE_REPORT, INTEGRATION_STATUS |
| 项目管理 | 3 | ISSUE_TRACKER, PROGRESS_REPORT, FEATURE_TRACKING_MATRIX |

> **强制检查**: `docs/completeness_check.sh vX.Y.Z` 必须在 Beta 门禁前通过，输出 `21/21`。

### 3.2 文档完整性检查流程

```
Beta 门禁前 → 执行 docs/completeness_check.sh vX.Y.Z → 输出 21/21 → 才能执行 Beta 门禁
RC 门禁前 → 再次检查完整性 → 如有新文档需求 → 补充到 VERSION_DOCS_SPEC.md
GA 门禁前 → 最终检查 → 确保所有文档已更新到最终状态
```

---

## 四、AI 合规执行机制

### 4.1 AI 可执行任务清单

| 任务类型 | 触发关键词 | 必须使用的模版 |
|----------|------------|----------------|
| 版本开发计划 | "开发计划"、"版本计划" | `DEVELOPMENT_PLAN.md` |
| 测试计划 | "测试计划"、"测试策略" | `TEST_PLAN.md` |
| 门禁检查 | "门禁检查"、"Gate" | `RELEASE_GATE_CHECKLIST.md` |
| Issue 追踪 | "创建 Issue"、"关闭 Issue" | `RELEASE_GATE_CHECKLIST.md` |
| 文档补全 | "补全文档"、"文档不完整" | `VERSION_DOCS_SPEC.md` |
| 功能开发 | 任何涉及代码/crate/功能的任务 | **SPEC 文档** + `brainstorming` skill |

### 4.2 AI 禁止执行的操作

```
❌ 禁止: 未使用模版创建 governance 文档
❌ 禁止: 在非 SSOT 文档中定义门禁检查项
❌ 禁止: 在没有 PR 证据的情况下关闭 Issue
❌ 禁止: 跳过门禁检查直接合并代码
❌ 禁止: 执行需要 Human Architect 审批的操作（合并 PR、发布版本等）
❌ 禁止: 未编写 SPEC 就开始实现（任何涉及代码的功能开发）
❌ 禁止: 21 docs 未齐就声称版本已完成或进入门禁
❌ 禁止: 生成文档内容时编造性能数据或测试结果
```

### 4.3 SPEC 开发流程

所有涉及代码的功能开发任务必须遵循：
1. **OpenSpec 分析**（使用 brainstorming skill）
2. **编写 SPEC**（docs/plans/YYYY-MM-DD-<feature>-spec.md）
3. **执行和验证**

---

## 五、版本生命周期

> **详见**: [RELEASE_LIFECYCLE.md](./RELEASE_LIFECYCLE.md)

### 5.1 四级门禁模型

```
L0-Alpha → L0-Beta → L0-RC → L0-GA
```

### 5.2 版本号规范 (Semantic Versioning)

```
v{MAJOR}.{MINOR}.{PATCH}

MAJOR: 不兼容的 API 修改
MINOR: 向下兼容的功能新增
PATCH: 向下兼容的问题修正
```

---

## 六、治理文档体系索引

### 6.1 治理文档 (docs/governance/)

| 文档 | 用途 | 强制级别 |
|------|------|----------|
| `GOVERNANCE_STANDARD.md` | 治理标准总纲 (SSOT) | 强制 |
| `gate_spec_v120.md` | v1.2.0+ 门禁规范 | 强制 |
| `RELEASE_LIFECYCLE.md` | 版本生命周期 | 强制 |
| `VERSION_DOCS_SPEC.md` | 版本文档规范（21 docs） | 强制 |
| `GOVERNANCE_INDEX.md` | 文档导航 | 强制 |
| `CONTRIBUTING.md` | 贡献指南 | 强制 |
| `GATE_EXEMPTIONS.md` | 门禁豁免记录 | 强制（如有豁免） |

### 6.2 版本文档 (docs/releases/vX.Y.Z/)

**Beta 前必须达到 21 个**，详见 `VERSION_DOCS_SPEC.md`。

### 6.3 版本化门禁规范

| 版本 | 门禁文档 | 状态 |
|------|----------|------|
| v1.0.0 | `gate_spec.md` | 已归档 |
| v1.1.0 | `gate_spec.md` | 已归档 |
| v1.2.0 | `gate_spec_v120.md` | 生效 |

---

## 七、治理体系自检清单

每次发布前，AI Agent 必须执行以下自检：

```
□ GOVERNANCE_STANDARD.md 是最新版本（版本 2.0）
□ gate_spec_v120.md 存在且内容完整
□ VERSION_DOCS_SPEC.md 定义了 21 docs 要求
□ 当前版本有 21 个文档（docs/completeness_check.sh vX.Y.Z 输出 21/21）
□ 所有门禁脚本已修复 REPO_ROOT 路径问题
□ 所有门禁脚本无 grep -v 导致的 set -e 管道错误
□ 所有性能数据包含来源和测试命令
□ 无文档之间内容矛盾
□ Issue 追踪已更新到最新状态
□ 分支保护已正确配置
```

---

## 八、版本更新历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 2.0 | 2026-05-23 | 增加第七大原则"文档完整原则"；明确 21 docs 强制要求；增加自检清单；增加文档完整性强制检查流程 |
| 1.0 | 2026-05-21 | 初始版本 |
