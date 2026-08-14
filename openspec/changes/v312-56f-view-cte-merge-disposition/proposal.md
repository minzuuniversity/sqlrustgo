# Proposal — V312-56F: VIEW/CTE/MERGE disposition 与门禁

## Why

Issue #4256: VIEW、CTE、MERGE 在 v3.12、4.0.0、教学场景中的真实边界未明确。MERGE 当前主 `execute()` 路径返回 unsupported，不得宣传。

## What Changes

### 1. CTE 教学 Fixture

- 普通 CTE 教学覆盖
- 递归 CTE 当前边界
- 错误语义正反例

### 2. VIEW disposition

若只保存 definition:
- 标为 PARTIAL/DEFERRED

若进入教学场景:
- 实现 view expansion
- 正反例 fixture

### 3. MERGE disposition

明确选择:
- 接入主执行路径并测试
- 或明确 `UNSUPPORTED/DEFERRED`

### 4. 文档一致性

README、MYSQL_COMPAT_STATUS、COMPREHENSIVE_ASSESSMENT_REPORT 状态一致。

## Capabilities

### New Capabilities

- **CTE 教学 fixtures** - 覆盖普通/递归 CTE
- **VIEW expansion** (若进入教学)
- **MERGE 明确状态** - SUPPORTED 或 UNSUPPORTED

### Modified Capabilities

- 现有 CTE 实现 → 确认教学边界
- 现有 VIEW 实现 → 确认 definition-only 或 expansion
- 现有 MERGE 实现 → 明确 unsupported

## Non-goals

- 不实现复杂的递归 CTE 边界情况
- 不实现 updatable VIEW

## Acceptance Criteria

- [ ] CTE 教学 fixture 覆盖普通 CTE、递归 CTE 当前边界、错误语义
- [ ] VIEW 若只保存 definition，标为 PARTIAL/DEFERRED
- [ ] MERGE 必须选择: 接入主执行路径并测试，或明确 UNSUPPORTED/DEFERRED
- [ ] README、MYSQL_COMPAT_STATUS、COMPREHENSIVE_ASSESSMENT_REPORT 状态一致
- [ ] 运行 `cargo test --test parser_e2e_test view -- --nocapture` PASS
- [ ] 运行 `cargo test --test merge_e2e_test -- --nocapture` PASS
- [ ] 运行 `bash scripts/gate/check_docs_consistency.sh` PASS

## Issue Reference

Issue #4256 (V312-56F)
