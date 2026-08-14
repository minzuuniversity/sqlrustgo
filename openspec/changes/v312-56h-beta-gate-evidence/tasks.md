# Tasks — V312-56H: Beta gate/docs/release evidence 集成

## Phase 1: 子项状态汇总 ✅

- [x] 1.1 V312-56A (#4250): Metadata Teaching - 32/36 tasks, gate PASS
- [x] 1.2 V312-56B (#4251): SQL Teaching Corpus - 创建了 teaching_sql_v3_12/ 目录
- [x] 1.3 V312-56C (#4252): Transaction/Crash Recovery - 调研完成
- [x] 1.4 V312-56D (#4253): Prepared Statement/Wire - 调研完成
- [x] 1.5 V312-56E (#4254): Optimizer/EXPLAIN - 创建了 5 个 teaching fixtures
- [x] 1.6 V312-56F (#4255): VIEW/CTE/MERGE - 调研完成
- [x] 1.7 V312-56G (#4256): Partition/FullText - 调研完成

## Phase 2: Beta Gate 集成 ✅

### 2.1 Beta Gate 脚本状态 ✅
- [x] `check_beta_v3.12.0.sh` 已存在
- [x] B1-B7 检查项已定义

### 2.2 V312-56 集成到 Beta Gate ✅
- [x] 2.2.1 V312-56 条目已添加到 ISSUES_PLAN.md
- [ ] 2.2.2 Beta gate 需后续集成 teaching_sql_v3_12/ manifest 检查

## Phase 3: 文档一致性 ✅

### 3.1 ISSUES_PLAN.md ✅
- [x] 3.1.1 V312-56A~56H 条目已添加
- [x] 3.1.2 与实际实现一致

### 3.2 Evidence Bundle ✅
- [x] 3.2.1 创建 `docs/releases/v3.12.0/evidence/v312-56/` 目录
- [x] 3.2.2 创建 `V312-56-VERIFICATION.md`:
  - [x] branch: `fix/v312-32-33-35-41-42-open-remediation`
  - [x] commit: `2a181cd7484649befe90f0ea7ba92d5119466838`
  - [x] teaching_sql_v3_12/ 结构说明
  - [x] 不支持范围说明

### 3.3 TEST_PLAN.md / PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md
- [ ] 3.3.1 确认 V312-56 测试覆盖正确 (后续)
- [ ] 3.3.2 确认与实际测试一致 (后续)

### 3.4 COMPREHENSIVE_ASSESSMENT_REPORT.md
- [ ] 3.4.1 确认 V312-56 涉及的功能状态正确 (后续)
- [ ] 3.4.2 确认与实际实现一致 (后续)

## Phase 4: 最终验证 (PENDING)

- [ ] 4.1 所有子项有 PR 合并或明确 DEFERRED/UNSUPPORTED 决策
- [ ] 4.2 Beta gate 验证 V312-56A~56D 状态
- [ ] 4.3 文档一致性检查 PASS

## Acceptance Criteria

- [x] V312-56 条目已添加到 ISSUES_PLAN.md
- [x] V312-56-VERIFICATION.md evidence bundle 已创建
- [ ] Beta gate 能验证 V312-56A~56D 的完成/降级状态 (后续)
- [ ] 所有文档状态一致 (后续)
