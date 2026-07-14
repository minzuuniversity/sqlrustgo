# v3.11.0 文档架构整理计划

> **创建日期**: 2026-07-13
> **创建人**: openclaw
> **目标**: 统一 v3.11.0 文档架构, 删除 v3.10.0 多余 plans 文件
> **关联**: [`V311_DEVELOPMENT_PLAN.md`](V311_DEVELOPMENT_PLAN.md)

---

## 1. v3.10.0 文档现状 (问题诊断)

### 1.1 现状

v3.10.0 文档存在 6 个 plans 文件, 但 `VERSION_PLAN.md` 与 `plans/V310_VERSION_PLAN.md` 内容重叠:

```
docs/releases/v3.10.0/
├── VERSION_PLAN.md                    ← 简短入口
├── plans/
│   ├── INDEX.md                       ← 分类说明
│   ├── V310_VERSION_PLAN.md           ← 详细战略 (内容与 VERSION_PLAN.md 重叠)
│   ├── V310_DEVELOPMENT_PLAN.md       ← 26 任务
│   ├── V310_ISSUES_PLAN.md            ← 12 子 ISSUE
│   ├── V310_10_COVERAGE_PLAN.md       ← 覆盖率详细
│   └── V310_CLI_BINARY_PLAN.md        ← CLI 详细
```

### 1.2 问题

1. **VERSION_PLAN.md 与 plans/V310_VERSION_PLAN.md 内容重叠** - 同一战略写两遍
2. **plans/V310_*.md 5 个文件** - 多文件导致信息分散, 阅读路径长
3. **V310_ISSUES_PLAN.md 内容已迁移到 debt-registry.yaml** - 重复维护两处
4. **V310_10_COVERAGE_PLAN.md 是 V310_DEVELOPMENT_PLAN 的子集** - 重复

### 1.3 v3.11.0 目标架构

```
docs/releases/v3.11.0/
├── VERSION_PLAN.md                       ← 简洁入口 (战略定位 + 快速链接)
├── plans/
│   ├── INDEX.md                          ← 4 个文件导航
│   ├── V311_VERSION_PLAN.md              ← 详细战略 (扩展版)
│   ├── V311_DEVELOPMENT_PLAN.md          ← 22 任务详细分解 (含原 5 个 plans 内容)
│   └── V311_DEBT_CLOSURE_PLAN.md         ← 23 项债务清零计划
└── (V311_DOCS_RESTRUCTURE_PLAN.md 已合并到本文)
```

**5 个 plans → 3 个 plans** (合并 + 删除)

---

## 2. v3.10.0 → v3.11.0 文档迁移表

### 2.1 删除 (5 项, 内容已合并到 v3.11.0 plans)

| v3.10.0 文件 | 大小 | 合并到 v3.11.0 | 备注 |
| --- | --- | --- | --- |
| `plans/V310_VERSION_PLAN.md` | ~280 行 | `VERSION_PLAN.md` + `plans/V311_VERSION_PLAN.md` | 内容重叠 |
| `plans/V310_DEVELOPMENT_PLAN.md` | 估算 200+ 行 | `plans/V311_DEVELOPMENT_PLAN.md` | 26 → 22 任务重写 |
| `plans/V310_ISSUES_PLAN.md` | 估算 300+ 行 | `plans/V311_DEBT_CLOSURE_PLAN.md` | 12 子 ISSUE → 23 项债务清零 |
| `plans/V310_10_COVERAGE_PLAN.md` | 估算 200+ 行 | `plans/V311_DEVELOPMENT_PLAN.md` § V311-14 | SEM-4 覆盖率 |
| `plans/V310_CLI_BINARY_PLAN.md` | 估算 200+ 行 | `plans/V311_DEVELOPMENT_PLAN.md` § V311-07/V311-19 | F-32 Admin |

### 2.2 新建 (4 项, v3.11.0 全新)

| v3.11.0 文件 | 大小 | 内容来源 |
| --- | --- | --- |
| `VERSION_PLAN.md` | 70 行 | 重写 (v3.11 战略) |
| `plans/V311_VERSION_PLAN.md` | 282 行 | 全新 (基于 v3.10.0 评估) |
| `plans/V311_DEVELOPMENT_PLAN.md` | 403 行 | 全新 (22 任务) |
| `plans/V311_DEBT_CLOSURE_PLAN.md` | 247 行 | 全新 (23 债务清零) |
| `plans/V311_DOCS_RESTRUCTURE_PLAN.md` | (本文件) | 本文件本身就是整改记录 |

**v3.11.0 文档量**: 4 个文件 ~1100 行 (vs v3.10.0 5+1 个 ~1400 行)

---

## 3. 文档整改步骤 (V311-22)

### 3.1 Step 1: 删除 v3.10.0 冗余 plans

```bash
cd /home/ai/sqlrustgo/docs/releases/v3.10.0/plans/
git rm V310_VERSION_PLAN.md
git rm V310_DEVELOPMENT_PLAN.md
git rm V310_ISSUES_PLAN.md
git rm V310_10_COVERAGE_PLAN.md
git rm V310_CLI_BINARY_PLAN.md
```

### 3.2 Step 2: 更新 INDEX.md

`docs/releases/v3.10.0/plans/INDEX.md` 删除 (因 plans 文件全删), 仅保留 `docs/releases/v3.11.0/plans/INDEX.md`。

### 3.3 Step 3: 更新 VERSION_PLAN.md (v3.10.0)

`docs/releases/v3.10.0/VERSION_PLAN.md` 添加一行说明:
```markdown
> 注意: v3.10.0 详细 plans 已整合到 v3.11.0/plans/, 请参见 v3.11.0/plans/V311_VERSION_PLAN.md § "v3.11.0 vs v3.10.0 关系" 获取上版本基线。
```

### 3.4 Step 4: 重写 ISOLATED_MODULES.md (root)

v3.7.0 时的 `ISOLATED_MODULES.md` 已过时 (v3.10.0 0 主路径孤岛), v3.11.0 重写反映 F-XX 全部集成的状态。

### 3.5 Step 5: 更新 MAINLINE_COMPONENTS.md (root)

v3.7.0 时的 `MAINLINE_COMPONENTS.md` 添加 v3.11.0 新组件 (Hash Semi Join, Adaptive Hash Index, Clustered Index, etc.)

---

## 4. 文档架构原则 (v3.11.0 起强制)

### 4.1 单一职责

| 文档类型 | 职责 | 数量限制 |
| --- | --- | --- |
| `VERSION_PLAN.md` | 版本入口 (战略定位 + 快速链接) | 1 |
| `plans/V<XXX>_VERSION_PLAN.md` | 详细战略 | 1 |
| `plans/V<XXX>_DEVELOPMENT_PLAN.md` | 任务分解 | 1 |
| `plans/V<XXX>_DEBT_CLOSURE_PLAN.md` | 债务清零 | 1 (新增) |

**禁止**: 同一版本出现 `*_VERSION_PLAN.md` 与 `plans/V<XXX>_VERSION_PLAN.md` 同时存在。

### 4.2 合并原则

- **子领域详细 plan** 应合并到 `DEVELOPMENT_PLAN.md` 中作为子章节, 而非独立文件
- **Issue 详细追踪** 应通过 Gitea Issue + `debt-registry.yaml`, 不写 per-version plans 文件
- **gate 控制** 由 `STAGE_CONFIG.yaml` + `STAGE.yaml` 统一管理, 不写 per-version 测试计划

### 4.3 跨版本一致性

| 文档 | v3.10.0 | v3.11.0 | 必达 |
| --- | --- | --- | --- |
| `VERSION_PLAN.md` | ✅ | ✅ | 简洁 (≤ 100 行) |
| 详细 plans | 5 个文件 | 3 个文件 | 不超过 4 |
| `plans/INDEX.md` | 存在 | 重写 | 列出所有 plans |

---

## 5. 验收清单

### 5.1 必达

- [ ] `docs/releases/v3.11.0/VERSION_PLAN.md` 创建
- [ ] `docs/releases/v3.11.0/plans/` 仅含 4 个文件
- [ ] `docs/releases/v3.10.0/plans/V310_*.md` 5 个文件删除
- [ ] `docs/releases/v3.10.0/plans/INDEX.md` 删除
- [ ] `ISOLATED_MODULES.md` (root) 重写为 v3.11.0 状态
- [ ] `MAINLINE_COMPONENTS.md` (root) 更新

### 5.2 应达

- [ ] v3.11.0 GA 时新版本 review 检查文档架构一致性
- [ ] v3.11.0 文档 review workflow 中加入 "plans 文件 ≤ 4" 检查
- [ ] 文档 CONTRIBUTING.md 添加 "per-version plans 文件数量限制"

---

## 6. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
| --- | --- | --- | --- |
| **删除 v3.10.0 plans 破坏外部引用** | 低 | 低 | grep 验证无外部 md 链接; git history 保留 |
| **新版本 plans 漏写关键内容** | 中 | 中 | ALPHA 阶段 review + BETA 阶段 review 双轮 |
| **ISOLATED_MODULES.md 重写遗漏模块** | 低 | 低 | 用 grep 验证所有模块提及 |

---

## 7. 关联资源

- `docs/governance/DOCUMENT_COMPLETENESS_CHECK.md` — 文档完整性
- `docs/governance/DOCUMENT_REVIEW_WORKFLOW.md` — 文档 review 工作流
- `docs/governance/DIRECTORY_POLICY.md` — 目录策略

---

*Created: 2026-07-13 (DRAFT stage init)*
*Author: openclaw*
