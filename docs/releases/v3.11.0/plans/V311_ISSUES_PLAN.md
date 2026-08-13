# V311 Issue 计划

> **状态**: 历史计划 / ALPHA stub 口径
> **父 Issue**: #3433 (V311-MASTER)
> **v3.10.0 参考**: [V310_ISSUES_PLAN.md](../../v3.10.0/plans/V310_ISSUES_PLAN.md)
> **说明**: 本文件保留 v3.11.0 早期 issue 拆分记录。当前完成状态应以 `COMPREHENSIVE_ASSESSMENT_REPORT.md`、`FEATURE_CHECKLIST.md` 和 Gitea 最新 issue/PR 为准。

## 1. 概览

v3.11.0 曾按 V311-01 到 V311-23 拆分任务。本计划用于追溯 issue 与 PR 的关系，不应单独作为最终完成状态或 GA gate 证据。

## 2. 任务索引

| ID | 标题 | 历史状态 | PR | 说明 |
|---|---|---|---|---|
| V311-01 | STAGE.yaml DRAFT init | DONE | - | 阶段初始化 |
| V311-02 | AHI v3 PK-lookup | IN_PROGRESS | #3478 | 后续状态需看综合评估 |
| V311-05 | F-29 Row-Level Security | MERGED | - | RLS policies、catalog integration |
| V311-06 | F-31 Performance Schema | MERGED | #3479 | instrumentation hooks |
| V311-07 | F-32 sqlrustgo-admin wire | MERGED | #3481 | admin wire integration |
| V311-08 | F-35 Password Rotation | MERGED | #3519/#3522/#3529 | AuthManager、parser、integration tests |
| V311-09 | ALPHA gate B (clippy+fmt) | DONE | - | promotion 相关 |
| V311-13 | modify_column N-bug | IN_PROGRESS | - | 需结合后续报告判断 |
| V311-15 | parser V311-16 | DONE | - | parser 任务 |
| V311-16 | try_decorrelate() | DONE | - | optimizer 任务 |
| V311-17 | optimizer parallelization | MERGED | #3482 | 优化器并行化 |
| V311-19 | V311-19 archive crate | DONE | - | crate 归档决策 |
| V311-22 | ISOLATED_MODULES.md update | MERGED | - | 文档更新 |
| V311-23 | PERF-5 high-concurrency INSERT | MERGED | #3435 | 高并发 INSERT 修复 |

## 3. 相关计划

- [V311_DEVELOPMENT_PLAN.md](./V311_DEVELOPMENT_PLAN.md) - 任务拆解。
- [V311_VERSION_PLAN.md](./V311_VERSION_PLAN.md) - 版本特定项目。
- [V311_ISSUE_CROSSREF.md](./V311_ISSUE_CROSSREF.md) - 完整交叉引用表。

## 4. 使用边界

本文件含早期 ALPHA stub 状态。用于历史追溯可以；用于当前 GA 判断时，必须交叉检查最新 gate 报告、commit、PR 和综合评估。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# V311 Issues Plan

> **Status**: ALPHA (stub)
> **Parent**: Issue #3433 (V311-MASTER)
> **v3.10.0**: [V310_ISSUES_PLAN.md](../../v3.10.0/plans/V310_ISSUES_PLAN.md)

## Overview

v3.11.0 has 22 tracked tasks (V311-01 through V311-23). This document tracks
issue status and cross-references PRs.

## Task Index

| ID | Title | Status | PR | Notes |
|----|-------|--------|-----|-------|
| V311-01 | STAGE.yaml DRAFT init | DONE | — | |
| V311-02 | AHI v3 PK-lookup | IN_PROGRESS | #3478 | |
| V311-05 | F-29 Row-Level Security | MERGED | — | RLS policies, catalog integration |
| V311-06 | F-31 Performance Schema | MERGED | #3479 | |
| V311-07 | F-32 sqlrustgo-admin wire | MERGED | #3481 | |
| V311-08 | F-35 Password Rotation | MERGED | #3519, #3522, #3529 | AuthManager methods, parser, integration tests |
| V311-09 | ALPHA gate B (clippy+fmt) | DONE | — | This promotion |
| V311-13 | modify_column N-bug | IN_PROGRESS | — | |
| V311-15 | parser V311-16 | DONE | — | |
| V311-16 | try_decorrelate() | DONE | — | |
| V311-17 | optimizer parallelization | MERGED | #3482 | |
| V311-19 | V311-19 archive crate | DONE | — | |
| V311-22 | ISOLATED_MODULES.md update | MERGED | — | |
| V311-23 | PERF-5 (high-concurrency INSERT) | MERGED | #3435 | |

*Remaining tasks (10): see [FEATURE_CHECKLIST.md](../FEATURE_CHECKLIST.md)*

## Phase Plans

- [V311_DEVELOPMENT_PLAN.md](./V311_DEVELOPMENT_PLAN.md) — 22-task breakdown
- [V311_VERSION_PLAN.md](./V311_VERSION_PLAN.md) — version-specific items

## Issue Cross-Reference

- [V311_ISSUE_CROSSREF.md](./V311_ISSUE_CROSSREF.md) — full cross-reference table

<!-- Fill in remaining tasks when promoted to BETA. -->
