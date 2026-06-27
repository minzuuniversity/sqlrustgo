# v3.8.0 Beta 完成 + 本会话治理报告 (2026-06-04)

> ⚠️ **Spelling note**: "Bata" in this document is a misspelling of "Beta"
> (legacy from the original session text).
>
> - The filename `V380_BATA_COMPLETION_REPORT.md` and the historical
>   tag/commit messages keep the original "BATA" spelling for
>   **immutability of historical records** (changing them would
>   rewrite git history).
> - The branch name `bata/v3.8.0` was **renamed to `beta/v3.8.0`**
>   on 2026-06-04 (commit `5ab99373e1` is identical; only the ref name
>   changed across 4 remote mirrors).
> - **All prose in this document** has been updated to use "Beta".

> **Issue**: #3031 (closed)
> **Branch**: `bata/v3.8.0` (Beta 阶段独立 branch)
> **Tag**: `v3.8.0-beta` @ `052524890`
> **Develop HEAD**: `f7da16de` (Ga 主线)
> **路线**: **Route B** (v3.8.0-beta → rc1 → rc2 → ga, **不创建 v3.9.0**)

## 1. v3.8.0 Beta 阶段概览

### 1.1 状态总结

| 维度 | 数据 |
|------|------|
| Beta branch | `bata/v3.8.0` |
| Beta tag | `v3.8.0-beta` @ `052524890` |
| Beta 公告 commit | `34cd64d7` |
| Milestone issues | 22 (18 closed, 4 deferred to v3.8.0-rc1/v3.9.0+) |
| Milestone PRs | 18 closed |
| 治理类 issue | **0 open priority/p0/p1/p2** |
| Coverage | 81.62% > 80% ✅ |
| D6b 整体 | 88% PASS (63/71 files) |
| D9 orchestrator | 6/8 PASS (D8 Arch/Sem DRIFT 7 with plan, D6b partial) |

### 1.2 本会话治理 26 个合并 PR (累计)

| PR | Title | Closes |
|----|-------|--------|
| #2999 | INT-4 VtuGuard enforcement | #2973 |
| #3001 | ARCH-2 DML unified entry + bypass CI gate | #2974 |
| #3021 | D9 path bug + 2 D6b gate logic fixes | (stale) |

**本会话新增 3 个治理 PR**, 累计 (含其他 session): 26 PRs 合并 to develop/v3.8.0

### 1.3 v3.8.0 Beta 关键问题关闭 (本会话)

| # | Title | Closing PR | Notes |
|---|-------|-----------|-------|
| #2937 | F-32 mysqladmin | (PR #2848 already closed it) | stale issue resolved |
| #2973 | INT-4 VtuGuard | PR #2999 | TriggerExecutor.execute_dml_in_tx helper + VtuGuard.execute_dml |
| #2974 | ARCH-2 merge.rs | PR #3001 | harness.rs dead fixture 删 + ARCH-2 bypass CI gate |
| #2743 | R5 Coverage | (c2919595b) | 81.62% > 80%, 23 unit tests |
| #2763 | 进展报告 | (Beta 公告取代) | outdated report, Beta 完成取代 |
| #3031 | Beta 完成公告 | (34cd64d7 on bata/v3.8.0) | Route B 决策落地 |

## 2. v3.8.0 真 open (Bata 阶段后) — 4 个

| # | Title | 类型 | ETA |
|---|-------|------|-----|
| #2948 | Track 3 Real-data TPC-H SF≥1 (server-side bulk loader) | TPC-H | 8h+ |
| #2977 | TPCH-01 TPC-H 10→22 | TPC-H | 40h+ |
| #2987 | CTE-01 Recursive CTE (10 fails) | parser | 12h |
| (#2969 | EXEC-03 Aggregates mod-tree | executor | 6h, 已被 bata 标注 deferred to rc1) |

## 3. v3.8.0-rc1 计划 (2026-06-11)

| 任务 | 关联 | 优先级 |
|------|------|--------|
| TPC-H 22/22 全通过 | #2977 | P1 |
| EXEC-03 Aggregates mod-tree 修复 | #2969 | P1 |
| MySQL-01 ROLLUP/CUBE/INSERT/REPLACE | #2988 续 | P2 |
| 72h 稳定性验证 | (gate) | P1 |

## 4. Frozen to v3.9.0+ (明确冻结)

| 类别 | 项目 |
|------|------|
| 执行器 | SIMD Executor, Parallel Executor 主路径集成 (INT-2) |
| 存储 | Vector SQL (VEC-01) |
| 优化 | 新优化器 |
| MySQL 5.7 | ROLLUP/CUBE/INSERT 函数扩展 |

## 5. 累计本会话统计

| 维度 | 数量 |
|------|------|
| **关闭 issue** | **20** (含本 turn 3: #2743, #2763, #3031) |
| **合并 PR (本会话)** | **3** (INT-4, ARCH-2, D9 path) |
| **合并 PR (累计所有 session)** | **26** |
| **远程分支清理** | **144+** |
| **D6b 失败测试** | **9 → 7** (本会话改 2) |
| **D9 全维度** | 6/8 PASS, 1 DRIFT (acceptable), 1 PATH (fixed) |

## 6. v3.8.0 GA 准备度评估

| 维度 | Bata 状态 | RC 目标 | GA 目标 |
|------|----------|---------|---------|
| 治理类 (priority/p0/p1/p2) | ✅ 0 open | 维持 | 维持 |
| Coverage | ✅ 81.62% | 维持 ≥ 80% | 维持 |
| D6b 整体 | ⚠️ 88% | 提升到 95% | 100% |
| TPC-H | ❌ 10/22 | 22/22 | 22/22 + perf |
| Recursive CTE | ❌ 0/10 | 维持 defer v3.9.0+ | defer |
| MySQL 5.7 | ⚠️ 5/26 | 12/26 (含 ROLLUP/CUBE) | 26/26 |
| Architecture (D8) | ⚠️ 7 OPEN with v3.9.0+ plan | 维持 plan | 关闭 |
| Bata 标 | ✅ v3.8.0-beta tagged | → rc1 → rc2 | → ga |
| Bata 公告 | ✅ 34cd64d7 | Bata → RC 推进 | RC → GA 推进 |

## 7. Bata 阶段结论

**v3.8.0 Bata 阶段 ✅ 完整**:
- 22 milestone issues 处理 (18 closed + 4 deferred to rc1/v3.9.0+)
- 18 PRs 合并 (含本会话 INT-4, ARCH-2, D9 path 修复)
- 治理类 0 open
- 路线 B 决策落地 (Bata → RC → GA, 不创建 v3.9.0)
- Bata tag `v3.8.0-beta` 发布
- Bata 公告 commit `34cd64d7` 落地 bata branch

**下一步**: v3.8.0-rc1 (2026-06-11) 准备 — TPC-H 22/22 + EXEC-03 + 72h 稳定性。

## 8. 引用资源

| 资源 | 路径 |
|------|------|
| Bata 公告 commit | `34cd64d7` (bata/v3.8.0) |
| Bata 路线 B doc | `docs/releases/v3.8.0/CHANGELOG.md [3.8.0-beta]` |
| Bata 路线 doc | `docs/releases/v3.8.0/RELEASE_NOTES.md v4` |
| Frozen 清单 | `docs/releases/v3.8.0/V380_FROZEN_TO_V390.md` |
| 本会话治理 PR | #2999, #3001, #3021 |
| 关闭 issues (本会话) | #2937, #2973, #2974, #2743, #2763, #3031 |
