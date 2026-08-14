# Proposal — V312-56H: Beta gate/docs/release evidence 集成

## Why

Issue #4258 (待创建): V312-56A~56G 需要一个总控来集成 Beta gate、文档一致性和 evidence 收集。

## What Changes

### 1. Beta Gate 集成

验证 V312-56A~56D 的完成/降级状态:
- V312-56A: Metadata/SHOW/information_schema - 教学实验可验证
- V312-56B: SQL teaching corpus - manifest 完整
- V312-56C: Transaction/crash recovery - 实验可复跑
- V312-56D: Prepared statement/wire - packet trace 可验证

### 2. 文档一致性

确保:
- `docs/releases/v3.12.0/ISSUES_PLAN.md` 与实际状态一致
- `docs/releases/v3.12.0/TEST_PLAN.md` 与实际状态一致
- `docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md` 与实际状态一致
- `docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` 与实际状态一致

### 3. Evidence 收集

生成 `docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md`:
- branch
- commit
- PR
- merge commit
- 命令
- exit code
- 输出摘要
- evidence hash
- 不支持范围

## Capabilities

### New Capabilities

- **V312-56 总控 gate** - 集成验证
- **evidence bundle** - 可追溯的验证记录

## Non-goals

- 不替代各个子项的独立验证
- 不修改已关闭的 issue

## Acceptance Criteria

- [ ] V312-56A~56H 全部有 PR 合并到 `develop/v3.12.0`，或有明确的 DEFERRED/UNSUPPORTED 决策
- [ ] Beta gate 能验证 V312-56A~56D 的完成/降级状态
- [ ] 生成 `V312-56-VERIFICATION.md` evidence bundle
- [ ] 所有文档状态一致

## Issue Reference

Issue #4258 (V312-56H) - 待创建
