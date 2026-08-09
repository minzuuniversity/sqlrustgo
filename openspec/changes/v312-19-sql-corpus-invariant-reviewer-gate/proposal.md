## Why

V312-19 SQL Corpus、Architecture Invariant 与 Reviewer Sign-off Gate

v3.11.0 release checklist 中仍为 TBD 的 SQL corpus、架构 invariant 和双 reviewer 签核需要变成 v3.12 RC/GA 阻断 gate。

## What Changes

- 更新 `scripts/test_sql_corpus.sh` 支持 all targets (fast/medium/full)
- 更新 `scripts/gate/check_arch_invariants.sh` 支持 R2.1-R2.8 invariant 输出
- 创建 `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md` — reviewer sign-off template
- 创建 `docs/releases/v3.12.0/sql-corpus-all-target-report.md` — all-target corpus report
- 创建 `docs/releases/v3.12.0/arch-invariant-report.md` — R2.1-R2.8 invariant 输出

## Capabilities

### New Capabilities

- `sql-corpus-all-target`: test_sql_corpus.sh fast + medium + full 全量通过
- `arch-invariant-gate`: check_arch_invariants.sh R2.1-R2.8 输出完整
- `reviewer-sign-off`: 2 名独立 reviewer 签核含 timestamp + evidence hash

## Issue Reference

- GitHub Issue: #3906
- Milestone: v3.12.0
- Priority: P0
