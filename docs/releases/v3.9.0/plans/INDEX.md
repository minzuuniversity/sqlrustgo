# v3.9.0 Plans — 业务战略 vs 门禁控制 分类

> **状态**: 2026-07-01 由 claude-macmini 在 Stage Control Framework (PR #3668) 之后建立
> **目的**: 区分 v3.9.0/plans/ 7 个文档中, 哪些是"业务战略" (仍需 per-version 写),
> 哪些是"门禁控制" (现在由 STAGE_CONFIG.yaml 统一管理, 不再需要 per-version 重写).
> **替代**: 之前的"per-version 门禁计划"被 STAGE_CONFIG.yaml (框架 SSOT) + STAGE.yaml (per-version state) 替代.

---

## 分类总览

| 文档 | 类别 | 是否仍需 per-version | 由谁覆盖 | 备注 |
|---|---|---|---|---|
| `V390_VERSION_PLAN.md` | **业务战略** | ✓ **仍需要** | (无替代) | 战略定位, 主题, 演进关系 |
| `V390_DEVELOPMENT_PLAN.md` | **业务战略** | ✓ **仍需要** | (无替代) | P0/P1/P2/P3 任务, 资源分配, 阶段排期 |
| `SPRINT4_MASTER_PLAN.md` | **业务战略** | ✓ **仍需要** | (无替代) | Sprint 4 排期, 人/天, GA 证据链 |
| `TPCH_ORACLE_PLAN.md` | **业务战略** | ✓ **仍需要** | (无替代) | 本版本特定的 oracle 方案 |
| `V390_TEST_PLAN.md` | **门禁控制** (G1-G16 detail) | ⚠️ **可大幅瘦身** | `STAGE_CONFIG.yaml#gate_definitions` (name + script 映射) | 详细场景 / 100+ scenarios 仍可保留作为工程参考 |
| `V390_TEST_PLAN_SUPPLEMENT_PERF.md` | **门禁控制** (G11-G15 detail) | ⚠️ **可大幅瘦身** | `STAGE_CONFIG.yaml#gate_definitions` G11-G15 | 同上 |
| `V390_TEST_PLAN_ROUND2_REVIEW.md` | **业务决策** (G13 rebalance) | ✓ **仍需要** | (无替代) | 本版本特定的 G13/G16 调整决策 |

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

## 推荐的下一步 (per docs/releases/v3.9.0/plans/)

### V390_TEST_PLAN.md

**当前**: 481 行, 含 G1-G16 详细测试设计, 5h/30h/32h/40h/28h/40h/48h/40h/40h/44h 工作量

**推荐**:
- 保留为 v3.9.0 工程参考 (因为已经写好了, 没必要删)
- 但顶部加 banner: "本文件的 G1-G16 列表已被 STAGE_CONFIG.yaml#gate_definitions 替代, 此处只保留 v3.9.0 特定的场景细节"
- 在 v3.10.0 时, 这个文档可以只写"v3.10.0 特有场景", 不再列 G1-G16 (从 STAGE_CONFIG 引用)

### V390_TEST_PLAN_SUPPLEMENT_PERF.md

**当前**: 545 行, G11-G15 详细性能测试设计

**推荐**:
- 保留 (G11-G15 详细性能数据有参考价值)
- 顶部加 banner: 同样指向 STAGE_CONFIG.yaml#gate_definitions

### V390_TEST_PLAN_ROUND2_REVIEW.md

**当前**: 360 行, G13/G16 rebalance 决策

**推荐**:
- 保留 (这是 v3.9.0 特定的业务决策, 不可替代)
- 但因为 G13 的"24h minimum at GA" 决策已体现在 `STAGE.yaml#promotion_to_GA_requires`,
  可以加 cross-reference

---

## 通用模板 (供 v3.10.0+ 借鉴)

未来版本的 plans/ 目录可遵循:

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
    ├── 2026-XX-XX-RC8-dry-run.json
    ├── 2026-XX-XX-RC8-dry-run.md
    └── ...
```

G1-G16 详细测试场景放在 `docs/governance/STAGE_CONFIG.yaml#gate_definitions` 和 per-version `STAGE.yaml#stage_history` 中, **不再 per-version 重写**.

---

## 跨参考 (Cross-references)

- 框架: `docs/governance/STAGE_CONFIG.yaml` (5 stage definitions + G1-G16 mapping)
- 状态: `docs/releases/v3.9.0/STAGE.yaml` (current_stage=RC, blocker_issues)
- 结果: `docs/releases/v3.9.0/gate-results/` (per-stage JSON + MD)
- 模板: `docs/governance/GATE_RESULTS_TEMPLATE.md` (per-stage report template)
- 状态快照: issue #3667
- PR: #3668 (Stage Control Framework), #3664 (execution_engine 拆分)
