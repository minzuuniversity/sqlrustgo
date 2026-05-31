# Issue #2628: VTU Phase 2 覆盖率提升专项

## 目标

提升 VTU (Validated Transaction Unit) Phase 2 相关代码的测试覆盖率，重点关注 merge.rs、trigger.rs、mutation_compiler、predicate_compiler 等模块。

## 背景

v3.7.0 GA 报告中指出的关键缺口：
- `stored_proc`: 50% 覆盖率
- `window_executor`: 54% 覆盖率
- 4 个 dead stub 模块：mutation_compiler, predicate_compiler, update_compiler, local_executor_dml (全部 0%)

v3.8.0 Alpha Gate 目标：parser 85%, executor 85%

---

## 阶段 V1: Dead Stub 激活

**目标:** 让 0% 覆盖率的模块不再为空

### V1.1 mutation_compiler 激活

- [ ] `test_mutation_compiler_insert` — INSERT 编译
- [ ] `test_mutation_compiler_update` — UPDATE 编译
- [ ] `test_mutation_compiler_delete` — DELETE 编译
- [ ] `test_canonicalize_expr` — 表达式规范化
- [ ] `test_row_mutation_hash` — 行变更哈希

### V1.2 predicate_compiler 激活

- [ ] `test_predicate_compile_eq` — 等值谓词
- [ ] `test_predicate_compile_range` — 范围谓词
- [ ] `test_predicate_compile_boolean` — 布尔组合

### V1.3 update_compiler 激活

- [ ] `test_update_compiler_basic` — 基础更新
- [ ] `test_update_compiler_with_filter` — 带过滤更新

### V1.4 local_executor_dml 激活

- [ ] `test_local_dml_execute` — DML 执行
- [ ] `test_local_dml_transaction` — 事务内 DML

---

## 阶段 V2: 低覆盖率模块提升

**目标:** 将 stored_proc 和 window_executor 提升到 70%+

### V2.1 stored_proc 覆盖率提升

- [ ] `test_stored_proc_creation` — 存储过程创建
- [ ] `test_stored_proc_execution` — 存储过程执行
- [ ] `test_stored_proc_parameters` — 参数传递
- [ ] `test_stored_proc_error_handling` — 错误处理

### V2.2 window_executor 覆盖率提升

- [ ] `test_window_functions` — 窗口函数 (ROW_NUMBER, RANK, DENSE_RANK)
- [ ] `test_window_frame` — 窗口帧
- [ ] `test_window_aggregation` — 窗口聚合

---

## 阶段 V3: Merge Executor 强化

**目标:** VTU Phase 2 的核心执行器需要完整覆盖

### V3.1 merge.rs 测试

- [ ] `test_merge_executor_dml_routing` — DML 路由（VTU Phase 2 核心）
- [ ] `test_merge_executor_vtu_guard` — VtuGuard panic 触发
- [ ] `test_merge_executor_txn_boundary` — 事务边界
- [ ] `test_merge_executor_wal_sequence` — WAL 序列验证

### V3.2 trigger 集成测试

- [ ] `test_trigger_execution` — 触发器执行
- [ ] `test_trigger_nested` — 嵌套触发器
- [ ] `test_trigger_mutating_tables` — 变异表保护

---

## 测试文件位置

```
crates/executor/tests/
  ├── update_vtu_test.rs        (已存在，可扩展)
  ├── stored_proc_test.rs       (新建)
  ├── window_executor_test.rs   (新建)
  └── merge_vtu_test.rs        (新建)
```

---

## 验收标准

1. mutation_compiler, predicate_compiler, update_compiler, local_executor_dml: 0% → 60%+
2. stored_proc: 50% → 70%+
3. window_executor: 54% → 70%+
4. merge.rs: 有 DML 路由专项测试

---

## 责任人

- Hermes A: 主导 merge.rs 和 VTU 测试
- 可分配给 Hermes B 或 Hermes C

---

## 与 Issue #2625 的关系

| 依赖 | 说明 |
|------|------|
| ⚠️ 依赖 | 测试需要 import Executor 模块（#2625 修复后） |
| ✅ 可选 | 纯单元测试可以在修复前设计 |

---

## 并行价值

- v3.8.0 Alpha Gate 覆盖率要求必须达成
- 为 PR-800 Facade 集成提供回归保护
- 提前暴露 VTU Phase 2 设计问题