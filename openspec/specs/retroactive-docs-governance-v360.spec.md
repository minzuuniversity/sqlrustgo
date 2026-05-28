# Retroactive Documentation Governance Spec

> **Spec ID**: retroactive-docs-governance-v360
> **Version**: v1.0
> **Created**: 2026-05-29
> **Author**: hermes-z440
> **Status**: draft

---

## 一、背景与目标

### 1.1 问题描述

v3.6.0 文档改进（PR #1480-#1486）的所有 governance 文档集中在 `develop/v3.6.0` 分支，历史版本（v3.0.0-v3.5.0）缺少以下文档：

| 文档类型 | v3.6.0 | v3.5.0 | v3.4.0 | v3.3.0 | v3.2.0 | v3.1.0 | v3.0.0 |
|----------|--------|--------|--------|--------|--------|--------|--------|
| docs/README.md | ✅ | ? | ? | ? | ? | ? | ? |
| docs/DOC_PUBLISHING_GUIDE.md | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| scripts/gate/README.md | ✅ | ❌ | ❌ | ? | ? | ? | ? |
| scripts/tpch/TPC-H-TEST-GUIDE.md | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| openspec/specs/ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| .cargo/config.toml | ✅ | ? | ? | ? | ? | ? | ? |
| rustfmt.toml | ✅ | ? | ? | ? | ? | ? | ? |

### 1.2 目标

为每个历史版本补充必要的 governance 文档，确保版本合规性。

### 1.3 原则

1. **线性合并**: 每个版本 PR 独立，合并后形成线性历史
2. **逆序执行**: v3.5.0 → v3.4.0 → v3.3.0 → v3.2.0 → v3.1.0 → v3.0.0
3. **版本适配**: 文档内容根据版本实际情况调整
4. **可审计性**: 每个版本的文档状态可追溯

---

## 二、文档类型定义

### 2.1 必需文档（Must-Have）

| 文档 | 说明 | 适用版本 |
|------|------|----------|
| `docs/README.md` | 文档入口索引 | v3.0.0+ |
| `docs/DOC_PUBLISHING_GUIDE.md` | 文档发布指南 | v3.5.0+ |
| `scripts/gate/README.md` | 门禁脚本索引 | v3.1.0+ |
| `scripts/tpch/TPC-H-TEST-GUIDE.md` | TPC-H 测试指南 | v3.2.0+ |

### 2.2 可选文档（Should-Have）

| 文档 | 说明 | 适用版本 |
|------|------|----------|
| `.cargo/config.toml` | rustdocflags | v3.2.0+ |
| `rustfmt.toml` | 格式化配置 | v3.1.0+ |
| `openspec/specs/*.spec.md` | OpenSpec 规范 | v3.4.0+ |
| `openspec/changes/*-plan.md` | OpenSpec 计划 | v3.4.0+ |

---

## 三、执行计划

### 3.1 v3.5.0 (最近历史版本)

**基础状态**: v3.5.0 是 v3.6.0 的前一个版本，结构最接近。

| 任务 | 源文档 | 目标分支 |
|------|--------|----------|
| 创建 `docs/DOC_PUBLISHING_GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.5.0 |
| 创建 `scripts/gate/README.md` | 从 v3.6.0 复制并调整 | develop/v3.5.0 |
| 创建 `scripts/tpch/TPC-H-TEST-GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.5.0 |
| 创建 `openspec/specs/retrospective-v350.spec.md` | 新建 | develop/v3.5.0 |
| 创建 `openspec/changes/retrospective-v350-plan.md` | 新建 | develop/v3.5.0 |

### 3.2 v3.4.0

**基础状态**: v3.4.0 是 GA 版本。

| 任务 | 源文档 | 目标分支 |
|------|--------|----------|
| 创建 `docs/DOC_PUBLISHING_GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.4.0 |
| 创建 `scripts/gate/README.md` | 从 v3.6.0 复制并调整 | develop/v3.4.0 |
| 创建 `scripts/tpch/TPC-H-TEST-GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.4.0 |
| 创建 `openspec/specs/retrospective-v340.spec.md` | 新建 | develop/v3.4.0 |
| 创建 `openspec/changes/retrospective-v340-plan.md` | 新建 | develop/v3.4.0 |

### 3.3 v3.3.0

**基础状态**: 中间版本。

| 任务 | 源文档 | 目标分支 |
|------|--------|----------|
| 创建 `docs/DOC_PUBLISHING_GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.3.0 |
| 创建 `scripts/gate/README.md` | 从 v3.6.0 复制并调整 | develop/v3.3.0 |
| 创建 `scripts/tpch/TPC-H-TEST-GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.3.0 |
| 创建 `openspec/specs/retrospective-v330.spec.md` | 新建 | develop/v3.3.0 |

### 3.4 v3.2.0

**基础状态**: 包含 Trust Gate。

| 任务 | 源文档 | 目标分支 |
|------|--------|----------|
| 创建 `docs/DOC_PUBLISHING_GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.2.0 |
| 创建 `scripts/gate/README.md` | 从 v3.6.0 复制并调整 | develop/v3.2.0 |
| 创建 `scripts/tpch/TPC-H-TEST-GUIDE.md` | 从 v3.6.0 复制并调整 | develop/v3.2.0 |

### 3.5 v3.1.0

**基础状态**: 包含 AGENTS.md。

| 任务 | 源文档 | 目标分支 |
|------|--------|----------|
| 创建 `docs/README.md` | 检查并补充 | develop/v3.1.0 |
| 创建 `scripts/gate/README.md` | 从 v3.6.0 复制并调整 | develop/v3.1.0 |

### 3.6 v3.0.0

**基础状态**: 最早版本。

| 任务 | 源文档 | 目标分支 |
|------|--------|----------|
| 创建 `docs/README.md` | 检查并补充 | develop/v3.0.0 |
| 创建 `scripts/gate/README.md` | 从 v3.6.0 复制并调整 | develop/v3.0.0 |

---

## 四、执行约束

### 4.1 合并策略

- 每个版本创建独立 PR
- 使用 `force_merge=true` 强制合并（受保护分支）
- 合并顺序: v3.5.0 → v3.4.0 → v3.3.0 → v3.2.0 → v3.1.0 → v3.0.0

### 4.2 分支命名

```
feat/retro-docs-v350 → develop/v3.5.0
feat/retro-docs-v340 → develop/v3.4.0
feat/retro-docs-v330 → develop/v3.3.0
feat/retro-docs-v320 → develop/v3.2.0
feat/retro-docs-v310 → develop/v3.1.0
feat/retro-docs-v300 → develop/v3.0.0
```

### 4.3 提交格式

```
docs: retrospective governance docs for v3.X.0

Added:
- docs/DOC_PUBLISHING_GUIDE.md
- scripts/gate/README.md
- scripts/tpch/TPC-H-TEST-GUIDE.md
- openspec/specs/retrospective-vX30.spec.md
- openspec/changes/retrospective-vX30-plan.md
```

---

## 五、验收标准

| 版本 | PR | 状态 | 文件数量 |
|------|-----|------|----------|
| v3.5.0 | #???? | Merged | 5 |
| v3.4.0 | #???? | Merged | 5 |
| v3.3.0 | #???? | Merged | 4 |
| v3.2.0 | #???? | Merged | 3 |
| v3.1.0 | #???? | Merged | 2 |
| v3.0.0 | #???? | Merged | 2 |

---

## 六、风险与缓解

| 风险 | 缓解 |
|------|------|
| 分支保护无法合并 | 使用 `force_merge=true` |
| 文件冲突 | 逆序执行确保最新版本先合并 |
| 历史版本结构差异 | 文档内容根据版本调整 |