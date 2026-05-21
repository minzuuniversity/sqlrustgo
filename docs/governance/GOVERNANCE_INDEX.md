# GMP 内审检索系统 Governance 文档体系索引

> **版本**: 1.0
> **创建日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **用途**: AI 和开发者必须遵循的文档导航指南

---

## 一、文档体系分类

### 1.1 核心 SSOT 文档（必须遵循）

| 文档 | 用途 | 强制级别 |
|------|------|----------|
| **GOVERNANCE_STANDARD.md** | 治理标准总纲 | **强制** |
| **GATE_SPEC.md** | 全版本、全阶段门禁定义 | **强制** |
| **DEVELOPMENT_PLAN_TEMPLATE.md** | 版本开发计划模版 | **强制** |
| **TEST_PLAN_TEMPLATE.md** | 测试计划模版 | **强制** |
| **GATE_CHECKLIST_TEMPLATE.md** | 门禁检查清单模版 | **强制** |

### 1.2 治理流程文档

| 文档 | 用途 | 强制级别 |
|------|------|----------|
| RELEASE_LIFECYCLE.md | 四级门禁模型、阶段转换规则 | 强制 |
| VERSION_DOCS_SPEC.md | 版本文档规范、命名规则 | 强制 |
| CONTRIBUTING.md | 贡献指南 | 强制 |

### 1.3 版本文档 (docs/releases/vX.Y.Z/)

| 文档 | 用途 | 阶段要求 |
|------|------|----------|
| DEVELOPMENT_PLAN.md | 开发计划 | Alpha+ |
| TEST_PLAN.md | 测试计划 | Alpha+ |
| RELEASE_GATE_CHECKLIST.md | 门禁检查清单 | Beta+ |
| RELEASE_NOTES.md | 发布说明 | RC+ |
| CHANGELOG.md | 变更日志 | RC+ |
| FEATURE_MATRIX.md | 功能矩阵 | Beta+ |
| PERFORMANCE_TARGETS.md | 性能目标 | Beta+ |
| INTEGRATION_STATUS.md | 功能集成状态 | Beta+ |
| ARCHITECTURE_DECISIONS.md | ADR | RC+ |
| USER_MANUAL.md | 用户手册 | RC+ |
| DEPLOYMENT_GUIDE.md | 部署指南 | RC+ |

---

## 二、AI 执行任务时的文档导航

### 2.1 开始新版本开发

```
任务: 开始 v{X}.{Y}.{Z} 开发
↓
1. 阅读 GOVERNANCE_STANDARD.md 了解治理原则
2. 阅读 DEVELOPMENT_PLAN_TEMPLATE.md 了解开发计划格式
3. 创建 docs/releases/v{X}.{Y}.{Z}/DEVELOPMENT_PLAN.md
4. 创建 Gitea ISSUE 追踪开发任务
```

### 2.2 创建测试计划

```
任务: 创建测试计划
↓
1. 阅读 TEST_PLAN_TEMPLATE.md 了解测试计划格式
2. 确保测试覆盖 GATE_SPEC.md 定义的所有门禁检查项
3. 创建 docs/releases/v{X}.{Y}.{Z}/TEST_PLAN.md
```

### 2.3 执行门禁检查

```
任务: 执行门禁检查
↓
1. 阅读 GATE_SPEC.md 确认检查项和标准
2. 创建 RELEASE_GATE_CHECKLIST.md
3. 执行所有检查项，记录证据
4. 失败项 → 创建 ISSUE → PR 修复
5. 通过 → 进入下一阶段
```