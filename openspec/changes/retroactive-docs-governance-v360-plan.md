# Retroactive Documentation Governance Plan

> **Plan ID**: retroactive-docs-governance-v360-plan
> **Spec**: [retroactive-docs-governance-v360.spec.md](../specs/retroactive-docs-governance-v360.spec.md)
> **Version**: v1.0
> **Created**: 2026-05-29
> **Author**: hermes-z440
> **Status**: draft

---

## 一、执行概述

### 1.1 目标

为 v3.0.0-v3.5.0 历史版本补充 governance 文档，确保每个版本文档健全。

### 1.2 执行顺序

```
v3.5.0 → v3.4.0 → v3.3.0 → v3.2.0 → v3.1.0 → v3.0.0
```

### 1.3 时间估算

| 版本 | 文件数 | 预估工时 |
|------|--------|----------|
| v3.5.0 | 5 | 15min |
| v3.4.0 | 5 | 15min |
| v3.3.0 | 4 | 10min |
| v3.2.0 | 3 | 10min |
| v3.1.0 | 2 | 5min |
| v3.0.0 | 2 | 5min |
| **合计** | **21** | **~60min** |

---

## 二、详细执行计划

### Phase 1: v3.5.0 文档补充

**PR 目标**: `feat/retro-docs-v350` → `develop/v3.5.0`

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 创建 worktree `.wt-v350-docs` | 基于 origin/develop/v3.5.0 |
| 2 | 复制 `docs/DOC_PUBLISHING_GUIDE.md` | 从 v3.6.0，调整版本号 |
| 3 | 复制 `scripts/gate/README.md` | 从 v3.6.0，调整版本索引 |
| 4 | 复制 `scripts/tpch/TPC-H-TEST-GUIDE.md` | 从 v3.6.0，调整版本标识 |
| 5 | 创建 OpenSpec 规范 | `openspec/specs/retrospective-v350.spec.md` |
| 6 | 创建 OpenSpec 计划 | `openspec/changes/retrospective-v350-plan.md` |
| 7 | 提交并创建 PR | |
| 8 | 合并 PR (force_merge=true) | |

**文件清单**:
- `docs/DOC_PUBLISHING_GUIDE.md` (v3.5.0 版本标识)
- `scripts/gate/README.md` (v3.5.0 索引)
- `scripts/tpch/TPC-H-TEST-GUIDE.md` (v3.5.0 版本标识)
- `openspec/specs/retrospective-v350.spec.md`
- `openspec/changes/retrospective-v350-plan.md`

### Phase 2: v3.4.0 文档补充

**PR 目标**: `feat/retro-docs-v340` → `develop/v3.4.0`

同 Phase 1，调整版本号为 v3.4.0。

### Phase 3: v3.3.0 文档补充

**PR 目标**: `feat/retro-docs-v330` → `develop/v3.3.0`

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 创建 worktree `.wt-v330-docs` | 基于 origin/develop/v3.3.0 |
| 2 | 复制 `docs/DOC_PUBLISHING_GUIDE.md` | 从 v3.6.0，调整版本号 |
| 3 | 复制 `scripts/gate/README.md` | 从 v3.6.0，调整版本索引 |
| 4 | 复制 `scripts/tpch/TPC-H-TEST-GUIDE.md` | 从 v3.6.0，调整版本标识 |
| 5 | 创建 OpenSpec 规范 | `openspec/specs/retrospective-v330.spec.md` |
| 6 | 提交并创建 PR | |
| 7 | 合并 PR (force_merge=true) | |

### Phase 4: v3.2.0 文档补充

**PR 目标**: `feat/retro-docs-v320` → `develop/v3.2.0`

同 Phase 3，调整版本号为 v3.2.0（无 openspec）。

### Phase 5: v3.1.0 文档补充

**PR 目标**: `feat/retro-docs-v310` → `develop/v3.1.0`

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 创建 worktree `.wt-v310-docs` | 基于 origin/develop/v3.1.0 |
| 2 | 复制 `scripts/gate/README.md` | 从 v3.6.0，调整版本索引 |
| 3 | 提交并创建 PR | |
| 4 | 合并 PR (force_merge=true) | |

### Phase 6: v3.0.0 文档补充

**PR 目标**: `feat/retro-docs-v300` → `develop/v3.0.0`

同 Phase 5，调整版本号为 v3.0.0。

---

## 三、文档版本适配规则

### 3.1 版本标识替换

| 源文档 | 目标版本 | 替换内容 |
|--------|----------|----------|
| 所有文档 | v3.X.0 | 版本号: `v3.6.0` → `v3.X.0` |
| 所有文档 | v3.X.0 | 日期: `2026-05-29` → 实际日期 |

### 3.2 门禁脚本索引调整

| 目标版本 | 包含的门禁脚本 |
|----------|----------------|
| v3.5.0 | v3.0.0 - v3.5.0 |
| v3.4.0 | v3.0.0 - v3.4.0 |
| v3.3.0 | v3.0.0 - v3.3.0 |
| v3.2.0 | v3.0.0 - v3.2.0 |
| v3.1.0 | v3.0.0 - v3.1.0 |
| v3.0.0 | v3.0.0 |

### 3.3 TPC-H 测试指南调整

| 目标版本 | TPC-H 支持 |
|----------|-----------|
| v3.5.0 | 22/22 查询 (100%) |
| v3.4.0 | 22/22 查询 (100%) |
| v3.3.0 | 根据实际支持情况 |
| v3.2.0 | 根据实际支持情况 |

---

## 四、验收清单

| 版本 | PR | 文件 | 验证 |
|------|-----|------|------|
| v3.5.0 | #???? | 5 | `ls docs/DOC_PUBLISHING_GUIDE.md` |
| v3.4.0 | #???? | 5 | `ls docs/DOC_PUBLISHING_GUIDE.md` |
| v3.3.0 | #???? | 4 | `ls docs/DOC_PUBLISHING_GUIDE.md` |
| v3.2.0 | #???? | 3 | `ls docs/DOC_PUBLISHING_GUIDE.md` |
| v3.1.0 | #???? | 2 | `ls scripts/gate/README.md` |
| v3.0.0 | #???? | 2 | `ls scripts/gate/README.md` |

---

## 五、风险控制

1. **分支保护**: 所有目标分支受保护，使用 `force_merge=true`
2. **工作区隔离**: 每个版本使用独立 worktree
3. **逆序执行**: 确保先合并较新版本，减少冲突风险
4. **线性历史**: 每个 PR 独立合并，不产生交叉合并