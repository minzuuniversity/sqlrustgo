<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# V390 Document Correction Status (Final)

> **Last update**: 2026-06-24
> **Status**: ✅ All planned corrections applied (8/8 in v3.9.0-rc7 round)
> **Scope**: docs/releases/v3.9.0/ + 3 root docs (CHANGELOG / ROADMAP / README)

## Summary

Two rounds of V390 doc corrections were executed by Hermes Agent
in 2026-06:

1. **2026-06-01 — V390_DOC_CORRECTION_PLAN** (RC2 readiness round)
   - 8 issues identified in `ROADMAP.md`, `V390_VERSION_PLAN.md`,
     `CHANGELOG.md`, `alpha/` directory
   - Work report: `V390_DOC_CORRECTION_WORK_REPORT.md`
   - **Status: 8/8 applied**

2. **2026-06-13 — V390_GA_DOC_CORRECTION_PLAN** (pre-GA round, RC7 era)
   - 8 issues identified in `CHANGELOG.md`, root `CHANGELOG.md`,
     root `ROADMAP.md`, root `README.md`, `GA_GATE_REPORT.md`
   - Work report: `V390_GA_DOC_CORRECTION_WORK_REPORT.md`
   - **Status: 8/8 applied**

## Source analyses

The corrections were driven by:

- `V390_DOCUMENT_INCONSISTENCY_ANALYSIS.md` (2026-06-01) — 17
  issues across 5 categories (timeline, phase status, gate status,
  task completion, doc references)

- `DOC_CHECK_CORRECTION_RULES.md` v1.0.0 — the 5-step standard
  process used to find and fix inconsistencies

## Verification (2026-06-24)

After both correction rounds:

- ✅ Root `CHANGELOG.md` has `[Unreleased] - 2026-06-05` entry
  pointing to v3.9.0 detailed CHANGELOG
- ✅ Root `ROADMAP.md` has v3.9.0 RC7 + v3.10 plan
- ✅ Root `RELEASE_NOTES.md` (rewritten 2026-06-24) is the
  multi-version index page linking to each version's detailed
  release notes
- ✅ `docs/releases/v3.9.0/CHANGELOG.md` (4073 bytes, current)
  has v3.9.0-rc7 status
- ✅ `docs/releases/v3.9.0/README.md` references all v3.9.0 key docs
- ✅ `docs/releases/v3.9.0/RELEASE_NOTES.md` (22981 bytes, current)
  has 22 TPC-H 22/22 + Sprint 8 + Sprint 9/10 + GA target 2026-12-15
- ✅ `docs/releases/v3.9.0/GA_GATE_REPORT.md` has G1-G16 + G17 +
  V-漏洞 V1-V9 status

## Files in this directory

- `V390_COMPREHENSIVE_ASSESSMENT.md` (v3.0, 2026-06-18, 64KB) — current state
- `V390_EVIDENCE_INDEX.md` (2026-06-24) — single-page G1-G16 reference
- `V390_DOC_CORRECTION_PLAN.md` (2026-06-01) — round 1 plan
- `V390_DOC_CORRECTION_WORK_REPORT.md` (2026-06-01) — round 1 report
- `V390_GA_DOC_CORRECTION_PLAN.md` (2026-06-13) — round 2 plan
- `V390_GA_DOC_CORRECTION_WORK_REPORT.md` (2026-06-13) — round 2 report
- `V390_DOCUMENT_INCONSISTENCY_ANALYSIS.md` (2026-06-01) — original analysis
- `V390_DOC_CORRECTION_STATUS.md` (2026-06-24) — THIS FILE

## Maintenance

This directory's doc set is **closed** as of 2026-06-24. Any new
inconsistency found should be raised as a follow-up issue and
processed via the standard `DOC_CHECK_CORRECTION_RULES.md` workflow
(do not edit any doc directly without filing a plan + report).

Last verified against `develop/v3.9.0` HEAD on 2026-06-24.
