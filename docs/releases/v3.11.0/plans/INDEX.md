# v3.11.0 Plans — 业务战略 vs 门禁控制 分类

> **状态**: 2026-07-13 DRAFT stage init
> **目的**: 列出 v3.11.0/plans/ 4 个文档的分类与快速导航

## 分类总览

| 文档 | 类别 | 必读 | 内容 |
| --- | --- | --- | --- |
| `V311_VERSION_PLAN.md` | **业务战略** | ✅ 必须 | 战略定位、22 任务大纲、阶段排期、风险 |
| `V311_DEVELOPMENT_PLAN.md` | **业务战略** | ✅ 必须 | 22 任务详细分解、阶段排期、验收标准 |
| `V311_DEBT_CLOSURE_PLAN.md` | **业务战略** | ✅ 必须 | 23 项债务清零 (基于 debt-registry.yaml) |
| `V311_DOCS_RESTRUCTURE_PLAN.md` | **业务战略** | ⚠️ ALPHA 必读 | v3.11.0 文档架构整理 |

## v3.11.0 vs v3.10.0 plans 对照

| v3.10.0 (已合并/删除) | v3.11.0 |
| --- | --- |
| `V310_VERSION_PLAN.md` (280 行) | `V311_VERSION_PLAN.md` (282 行) |
| `V310_DEVELOPMENT_PLAN.md` (200+ 行) | `V311_DEVELOPMENT_PLAN.md` (403 行) |
| `V310_ISSUES_PLAN.md` (300+ 行) | `V311_DEBT_CLOSURE_PLAN.md` (247 行) |
| `V310_10_COVERAGE_PLAN.md` (200+ 行) | (合并到 V311-14) |
| `V310_CLI_BINARY_PLAN.md` (200+ 行) | (合并到 V311-07/V311-19) |

**5 个 plans → 3 个 plans** (合并 + 删除)

## 必读顺序

1. 📖 `V311_VERSION_PLAN.md` (10 分钟) — 战略概览
2. 📖 `V311_DEBT_CLOSURE_PLAN.md` (15 分钟) — 23 项债务基线
3. 📖 `V311_DEVELOPMENT_PLAN.md` (30 分钟) — 22 任务详细

## 关联 SSOT

- `docs/governance/debt/debt-registry.yaml` — 债务 SSOT
- `docs/governance/STAGE_CONFIG.yaml` — 门禁 SSOT
- `docs/governance/STAGE.yaml` (per-version state)

---

*Created: 2026-07-13 DRAFT init*
