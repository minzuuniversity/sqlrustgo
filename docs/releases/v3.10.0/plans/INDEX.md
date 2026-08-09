# v3.10.0 Plans — 业务战略 vs 门禁控制 分类

> **状态**: 2026-07-01 由 claude-macmini 在 Stage Control Framework (PR #3668) + G1-G16 framework (PR #3669) 之后建立
> **目的**: 区分 v3.10.0/plans/ 2 个文档中, 哪些是"业务战略" (仍需 per-version 写), 哪些是"门禁控制" (现在由 STAGE_CONFIG.yaml 统一管理, 不再需要 per-version 重写).
> **替代**: 之前的"per-version 门禁计划"被 STAGE_CONFIG.yaml (框架 SSOT) + STAGE.yaml (per-version state) 替代.

---

## 分类总览

| 文档 | 类别 | 是否仍需 per-version | 由谁覆盖 | 备注 |
|---|---|---|---|---|
| `V310_VERSION_PLAN.md` (待创建) | **业务战略** | ✓ **仍需要** | (无替代) | 战略定位, 主题, 演进关系 |
| `V310_DEVELOPMENT_PLAN.md` | **业务战略** | ✓ **仍需要** | (无替代) | P0/P1/P2 任务, 资源分配, 阶段排期 |
| `V310_CLI_BINARY_PLAN.md` | **业务战略** | ✓ **仍需要** | (无替代) | C-6 sqlrustgo-cli + C-7 ARCH-2 详细实现计划 |
| `V310_TEST_PLAN.md` (待创建) | **门禁控制** | ⚠️ **可大幅瘦身** | `STAGE_CONFIG.yaml#gate_definitions` (name + script 映射) | G1-G16 详细场景 / 100+ scenarios 仍可保留作为工程参考 |
| `V310_TEST_PLAN_SUPPLEMENT_PERF.md` (待创建) | **门禁控制** | ⚠️ **可大幅瘦身** | `STAGE_CONFIG.yaml#gate_definitions` G11-G15 | 同上 |
| `V310_TEST_PLAN_ROUND2_REVIEW.md` (待创建, 可选) | **业务决策** | ✓ **仍需要** (如果 v3.10.0 有 G13 rebalance 决策) | (无替代) | v3.9.0 是因为 Z6G4 限制 |

---

## 决策规则 (Rule of thumb)

**业务战略 (Business Strategy)**:
- 回答 "为什么" / "做什么" / "何时做" / "投入多少"
- 包含: 主题, 战略定位, 任务清单, 资源分配, 阶段排期, GA 决策
- 不可被 SSOT 替代 (每版本需要自己的产品/工程决策)
- 仍需 per-version 写

**门禁控制 (Gate Control)**:
- 回答 "怎么验" / "通过什么" / "失败怎么办"
- 包含: G1-G16 详细测试设计, 100+ scenarios 清单, 性能基准
- 已被 STAGE_CONFIG.yaml (框架) + STAGE.yaml (per-version state) 覆盖:
  - 哪个 stage 跑哪些 gate → STAGE_CONFIG.yaml
  - 当前 stage → STAGE.yaml
  - G1-G16 name + script 映射 → STAGE_CONFIG.yaml#gate_definitions
- 详细场景 / 100+ scenarios 仍可保留作为工程参考, 但不再需要"重写门禁计划"
- 大幅瘦身 (移除重复的 gate 列表和 required_files), 只保留"本版本特定的 oracle 细节"

---

## v3.10.0 已知内容 (从 v3.10.0/ 目录审计)

| 文档 | 状态 | 备注 |
| --- | --- | --- |
| `README.md` | ✓ 已存在 | 版本定位 + 核心目标 + 历史债务整合 |
| `plans/V310_DEVELOPMENT_PLAN.md` | ✓ 已存在 (2026-06-25 创建) | 26 任务 / 500h 预算 / 4 阶段 / G1-G8 门禁 |
| `plans/V310_CLI_BINARY_PLAN.md` | ✓ 已存在 (2026-06-27 创建) | C-6 sqlrustgo-cli + C-7 ARCH-2 详细计划 |
| `VERSION_PLAN.md` | ⚠ **缺失** | 需补充战略定位 |
| `TEST_PLAN.md` | ⚠ **缺失** | 可选 (Stage Control Framework 已覆盖) |
| `TEST_PLAN_SUPPLEMENT_PERF.md` | ⚠ **缺失** | 可选 (Stage Control Framework 已覆盖) |
| `CHANGELOG.md` | ⚠ **缺失** | 待 stage 转换时创建 |
| `RELEASE_NOTES.md` | ⚠ **缺失** | 待 GA 阶段创建 |
| `GA_GATE_REPORT.md` | ⚠ **缺失** | 待 GA 阶段创建 |
| `STAGE.yaml` | ✓ **刚创建 (2026-07-01)** | current_stage: DRAFT, branch: develop/v3.10.0 (待创建) |
| `gate-results/` | ✓ 目录已创建 (待实际跑 gate 时填) | 当前 DRAFT 阶段, 无实际 gate 运行结果 |

---

## 推荐的下一步 (供 v3.10.0 借鉴)

未来版本的 plans/ 目录可遵循 (v3.9.0 也是这个模式):

```
docs/releases/vX.Y.Z/
├── INDEX.md                          # 文档分类 (本文件)
├── plans/
│   ├── V<X.Y.Z>_VERSION_PLAN.md     # 业务战略 - 战略定位
│   ├── V<X.Y.Z>_DEVELOPMENT_PLAN.md # 业务战略 - 任务清单
│   ├── SPRINT<N>_MASTER_PLAN.md      # 业务战略 - Sprint 排期
│   ├── <SPECIFIC>_ORACLE_PLAN.md     # 业务战略 - 本版本特定 oracle
│   └── (no G1-G16 detail, that's in STAGE_CONFIG.yaml)
├── STAGE.yaml                         # per-version state (SSOT)
├── CHANGELOG.md
├── RELEASE_NOTES.md
└── gate-results/                     # 每阶段运行结果 (auto-generated)
    ├── 2026-XX-XX-<STAGE>-dry-run.json
    ├── 2026-XX-XX-<STAGE>-dry-run.md
    └── ...
```

G1-G16 详细测试场景放在 `docs/governance/STAGE_CONFIG.yaml#gate_definitions` 和 per-version `STAGE.yaml#stage_history` 中, **不再 per-version 重写**.

---

## 跨参考 (Cross-references)

- 框架: `docs/governance/STAGE_CONFIG.yaml` (5 stage definitions + G1-G16 mapping)
- 状态: `docs/releases/v3.10.0/STAGE.yaml` (current_stage=DRAFT, blocker_issues=[])
- 结果: `docs/releases/v3.10.0/gate-results/` (每阶段 JSON + MD, DRAFT 阶段暂无)
- 模板: `docs/governance/GATE_RESULTS_TEMPLATE.md` (per-stage report template)
- 状态快照: issue #3667 (closed)
- PR: #3668 (Stage Control Framework), #3669 (G1-G16 framework), #3670 (CHANGELOG 待合并)
- v3.9.0 模板: `docs/releases/v3.9.0/plans/INDEX.md` (PR #3669 引入)
