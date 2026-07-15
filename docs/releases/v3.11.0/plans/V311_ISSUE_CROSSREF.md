# v3.11.0 任务 ↔ Gitea 250 Issue 交叉引用表

> **创建日期**: 2026-07-15
> **创建人**: openclaw (通过 AI 协助整理)
> **目的**: 解决 v3.11.0 开发计划 (V311-XX) 与 Gitea 250 当前 open issue 编号之间的引用错位
> **状态**: DRAFT — 等待 hermes 审核后并入 V311_DEVELOPMENT_PLAN.md
> **范围**: 修正 22 任务到当前 Gitea 250 (commit 14979a5f1) 上 **open + v3.11.0 label** 的 issue 编号

---

## 1. 背景：为什么需要这份附录

v3.11.0 plan 文档（`V311_DEVELOPMENT_PLAN.md`）多处引用了 `(#NNNN)` 形式的 issue 编号。
这些编号在两个时空中含义不同：

1. **历史 issue 编号**（v3.9.0 / v3.10.0 GA 期间）—— 已 CLOSED，例如：
   - `#3423` (历史) = TPC-H SF=1 baseline scaffolding (PR #3827 merged, commit `285014147`)
   - `#3648` = TPC-H 混合负载 SOAK 跨平台验证 — 状态 SUPERSEDED (`debt-registry.yaml#ga_p0`)
   - `#3265` = 72h SOAK — 状态 CLOSED
   - `#3266` = 168h SOAK — 状态 CLOSED

2. **当前 Gitea 250 open issue 编号**（2026-07-15 现状，commit `14979a5f1`）——
   Gitea issue 编号已与历史 PR/issue 解耦，可被新 issue 复用。
   例如当前 open 的 **#3423 = "Fix examples compilation (q21_trace + q17_sf01)"**，
   与历史 `#3423` (TPC-H SF=1) **不是同一对象**。

**因此**：v3.11.0 启动时如果按 plan 文档中的 `(#3423)` 跳到 Gitea issue tracker，
会**误指到 examples 修复 issue**，而非 TPC-H SF=1 baseline 跟踪 issue。

本附录提供 **v3.11.0 任务 ↔ 当前 Gitea 250 open issue 编号** 的精确映射。

---

## 2. 完整映射表

### 2.1 任务直接对应（v3.11.0 plan ↔ open Gitea issue）

| V311 任务 | plan 中引用 | 当前 Gitea 250 对应 issue | 状态 | 备注 |
| --- | --- | --- | --- | --- |
| **V311-13** (SEM-3 ALTER TABLE RENAME/MODIFY) | (无 issue 引用) | **#3428** (SEM-3: ALTER TABLE RENAME/MODIFY support) | open, sql/v3.11.0 | ✅ 直接对应 |
| **V311-14** (SEM-4 覆盖率 ≥85%) | (无 issue 引用) | **#3420** (SEM-4: Coverage ≥ 80%) | open, coverage/v3.11.0 | ✅ 直接对应，#3420 标题阈值 80%、plan 写 85% — 需 hermes 决策 |
| **V311-14 子任务** | (无) | **#3421** (Re-enable 22 disabled integration tests) | open, test/v3.11.0 | 子任务，被 #3420 覆盖 |
| **V311-15** (Q4 Hash Semi Join) | "Issue #3792" | (无对应 open issue) | — | 需创建 |
| **V311-19** (Extension Crate 决策) | (无 issue 引用) | **#3427** (#3136: Remove MOCK storage backend) | open, storage/v3.11.0 | 是 V311-19 子项之一 |
| **V311-20** (TPC-H SF=1.0 baseline) | **(#3423)** | **#3431** (R8: TPC-H SF=1 full performance baseline) | open, perf/v3.11.0 | ❌ plan 引用错位（#3423 现在是 examples fix） |
| **V311-20 子任务** | (无) | **#3430** (TPC-H Q22 cell-level mismatch) | open, sql/v3.11.0 | 子任务 |
| **V311-21** (168h SOAK v3.11.0) | **(#3648)** | (无对应 open issue) | — | ❌ #3648 已 SUPERSEDED；V311-21 是新 SOAK 任务，需在启动时创建 issue 跟踪 |
| **V311-23** (PERF-5 High-concurrency INSERT) | #3434 (plan body 内) | **#3434** | open (无 label) | ✅ plan 与 Gitea 一致 |
| **V311-14 关联** | (无) | **#3418** (chore(coverage): full workspace llvm-cov baseline 16.30%) | open | SEM-4 基础工作 |

### 2.2 任务对应不上当前 open issue（需在 ALPHA 启动时创建）

| V311 任务 | 计划标题 | 建议 Gitea 标题 | 优先级 |
| --- | --- | --- | --- |
| **V311-01** | F-23 Clustered Index 主路径集成 | `V311-01: F-23 Clustered Index main-path integration` | P0 |
| **V311-02** | F-24 Adaptive Hash Index 主路径集成 | `V311-02: F-24 Adaptive Hash Index main-path integration` | P0 |
| **V311-09** | F-36 列级权限实现 | `V311-09: F-36 column-level privilege (column_privilege_test 12/12)` | P0 |
| **V311-15** | Q4 Hash Semi Join 算子 | `V311-15: Hash Semi Join operator (Issue #3792: Q4 96% bottleneck)` | P0 |

### 2.3 F-XX 编号在 Gitea 上的冲突

> ⚠️ **已知冲突** — Gitea 250 上两个 open issue 的 F-XX 编号与 `debt-registry.yaml` SSOT 不一致

| Gitea issue | 标题中 F-XX 引用 | debt-registry 中同名 F-XX | 影响 |
| --- | --- | --- | --- |
| **#3425** | "F-30: restore_filespace_resync" | F-30 = CREATE SEQUENCE (V311-10) | 编号错位 — Gitea issue body 是文件空间 resync，与 F-30 SEQUENCE 无关 |
| **#3426** | "F-36: restore_filespace_cleanup" | F-36 = 列级权限 (V311-09) | 编号错位 — Gitea issue body 是文件空间 cleanup，与 F-36 列权限无关 |

**修正建议**（需 hermes 决策）：
- 方案 A：把 #3425 / #3426 标题里的 F-30 / F-36 移除（改为 "restore_filespace_resync/cleanup"），不依赖错位编号
- 方案 B：把 #3425 / #3426 标题里的 F-30 / F-36 改用 `restore_filespace` 编号段（debt-registry 增设 F-37/F-38 临时项）
- 方案 C：保留 #3425 / #3426 原样，在 debt-registry 注释中加 "⚠️ 与 #3425 / #3426 同名异义"

### 2.4 Gitea open issue 暂无 V311 任务对应（可能属于治理/补丁类）

| Gitea issue | 标题（缩写） | 建议归属 |
| --- | --- | --- |
| **#3422** | Fix compiler warnings | 治理 E 维度 / clippy pass (无 V311-XX) |
| **#3429** | wire protocol CREATE/DROP/TRUNCATE 连接丢失 | 与 #3434 PERF-5 关联，V311-23 子任务 |
| **#3432** | v3.11.0 Git sync: cross-remote consistency | 流程任务，无 V311-XX |
| **#3433** | v3.11.0 Release Preparation | 启动期总 issue（含 STAGE.yaml / gate scripts） |
| **#3414** | [GA-BLOCKER] R2 gate TIMEOUT | v3.10.0 遗留（无 v3.11.0 label） |
| **#3416** | sync: final merge .250↔.252 | v3.10.0 同步（无 label） |

---

## 3. plan 文档待订正位置

`V311_DEVELOPMENT_PLAN.md` 内以下行需按本附录修正：

| 行 | 当前内容 | 建议改为 | 理由 |
| --- | --- | --- | --- |
| 35 | `V311-20 \| TPC-H SF=1.0 baseline (#3423) \| 80h \| P0 \| BETA-RC \|` | `V311-20 \| TPC-H SF=1.0 baseline (#3431) \| 80h \| P0 \| BETA-RC \|` | #3423 当前是 examples fix，不是 SF=1 |
| 36 | `V311-21 \| 168h SOAK v3.11.0 (#3648) \| ...` | `V311-21 \| 168h SOAK v3.11.0 (新 issue, ALPHA 启动时创建) \| ...` | #3648 已 SUPERSEDED |
| 344 | `### V311-20: TPC-H SF=1.0 baseline (#3423)` | `### V311-20: TPC-H SF=1.0 baseline (#3431)` | 同上 |
| 356 | `### V311-21: 168h SOAK v3.11.0 (#3648, Hermes 协作)` | `### V311-21: 168h SOAK v3.11.0 (Hermes 协作, ALPHA 启动时创建 issue)` | 同上 |
| 360 | `**验收**: Issue #3266 (168h SOAK) v3.11.0 PASS` | `**验收**: v3.11.0 168h SOAK PASS (启动时创建跟踪 issue)` | #3266 当前是 Float promotion Q1 SUM 修复 |

`V311_DEBT_CLOSURE_PLAN.md` 中如无对应 issue 引用，则保持现状（任务 ↔ debt ID 映射已准确）。

---

## 4. 待 hermes 决策的开放问题

1. **V311-14 覆盖率阈值**：plan 写 85%，#3420 写 80%。**以哪个为准？**
2. **V311-15 / V311-23 是否需要在 ALPHA 启动时由 AI 创建 Gitea issue？** 还是 hermes 亲自创建？
3. **V311-21 (168h SOAK) 是否复用现有 #3648 关闭记录 / 另建新 issue？**
4. **Gitea #3425 / #3426 标题的 F-30 / F-36 错位**采用方案 A / B / C 哪个？

---

## 5. 与 V311_DEVELOPMENT_PLAN 的关系

- **不替代** `V311_DEVELOPMENT_PLAN.md` —— 该文档是任务详细分解
- **补充** —— 本附录提供每个任务与 Gitea issue tracker 的精确跳转锚点
- **建议并入方式**：在 `V311_DEVELOPMENT_PLAN.md` 顶部加一行 "Issue 编号对照：本目录 `V311_ISSUE_CROSSREF.md`"，并在每个任务小节末尾追加 "**Gitea 跟踪**: #NNNN"

---

*Created: 2026-07-15 by openclaw + AI assistance*
*Status: DRAFT, pending hermes review*
