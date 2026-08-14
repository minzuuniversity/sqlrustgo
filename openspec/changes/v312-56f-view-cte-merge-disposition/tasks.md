# Tasks — V312-56F: VIEW/CTE/MERGE disposition 与门禁

## Phase 1: 现状调研 ✅

### 1.1 已有测试
- [x] `cte_materialization_test.rs` - CTE materialization 测试存在

### 1.2 缺失项识别
- ❌ 递归 CTE 未找到实现
- ❌ MERGE 执行路径未确认
- ❌ VIEW 实现状态未确认
- ❌ 文档状态需要更新

## Phase 2: CTE 教学覆盖

### 2.1 普通 CTE

- [ ] 2.1.1 基础 WITH clause → teaching_sql_v3_12/cte/basic.sql
- [ ] 2.1.2 多 CTE 引用 → teaching_sql_v3_12/cte/multi_cte.sql
- [ ] 2.1.3 CTE 嵌套

### 2.2 递归 CTE

- [ ] 2.2.1 WITH RECURSIVE 语法支持状态评估
- [ ] 2.2.2 递归终止条件
- [ ] 2.2.3 环形递归检测

### 2.3 错误语义

- [ ] 2.3.1 递归深度限制测试

## Phase 3: VIEW disposition

- [ ] 3.1 评估当前 VIEW 实现 (definition-only vs expansion)
- [ ] 3.2 若 definition-only，标为 PARTIAL/DEFERRED
- [ ] 3.3 若进入教学，实现 view expansion 正反例

## Phase 4: MERGE disposition

### 4.1 现状评估

- [ ] 4.1.1 检查 MERGE 当前 execute() 路径
- [ ] 4.1.2 确认是否返回 unsupported

### 4.2 决策

- [ ] 4.2.1 选择 A: 接入主执行路径并测试
- [ ] 4.2.2 选择 B: 明确 UNSUPPORTED/DEFERRED

## Phase 5: 文档一致性

- [ ] 5.1 更新 README.md VIEW/CTE/MERGE 状态
- [ ] 5.2 更新 MYSQL_COMPAT_STATUS.md
- [ ] 5.3 更新 COMPREHENSIVE_ASSESSMENT_REPORT.md

## Phase 6: 验证

- [ ] 6.1 `cargo test --test parser_e2e_test -- --nocapture` PASS (CTE part)
- [ ] 6.2 `bash scripts/gate/check_docs_consistency.sh` PASS

## Acceptance Criteria

- [ ] CTE 教学 fixture 覆盖普通 CTE、递归 CTE 当前边界、错误语义
- [ ] VIEW 标为 PARTIAL/DEFERRED (若只保存 definition)
- [ ] MERGE 选择: 接入主执行路径并测试，或明确 UNSUPPORTED/DEFERRED
- [ ] README、MYSQL_COMPAT_STATUS、COMPREHENSIVE_ASSESSMENT_REPORT 状态一致
