# Coverage Recovery v3.4.0 实施计划

> 本文件是给实施 Agent 的任务清单。请在开始任务前阅读规范文档：
> `openspec/specs/coverage-recovery-v340.spec.md`

## 实施顺序

```
Phase 1 ──────→ Phase 2 ──────────→ Phase 3 ──────────→ Phase 4
修复UPDATE      DELETE/MERGE        Crash Recovery       Coverage验证
P0 Bug          错误路径             测试                 脚本改进
```

## Phase 1: 修复 UPDATE Bug（P0）

### Task 1.1: 分析 UpdateExec 实现方案

**文件**: `crates/planner/src/physical_plan.rs`

**任务**: 分析现有 UpdateExec 实现状态，确认修复方案

**验证**: `grep -n "UpdateExec" crates/planner/src/physical_plan.rs`

### Task 1.2: 实现 UpdateExec

**文件**: `crates/planner/src/physical_plan.rs`

**任务**: 实现完整的 UpdateExec 物理计划节点

### Task 1.3: 实现 execute_update

**文件**: `crates/executor/src/local_executor.rs`

**任务**: 在 local_executor.rs 中添加 execute_update() 函数

### Task 1.4: 添加 UPDATE 测试

**文件**: `crates/executor/tests/update_execution_test.rs`（新建）

**内容**: 6 个测试用例覆盖 UPDATE 路径

**验证**: `cargo test -p sqlrustgo-executor --test update_execution_test`

### Task 1.5: 验证 UPDATE QPS

**命令**:
```bash
cargo test --test qps_benchmark_test -- test_qps_update --nocapture
```

**验证**: QPS 不低于基准值 10,000

---

## Phase 2: DELETE/MERGE 错误路径

### Task 2.1: DELETE 错误路径测试

**文件**: `crates/executor/tests/delete_error_path_test.rs`（新建）

**内容**:
- test_delete_nonexistent_table
- test_delete_storage_error
- test_delete_complex_predicate

**验证**: `cargo test -p sqlrustgo-executor --test delete_error_path_test`

### Task 2.2: MERGE 错误路径测试

**文件**: `crates/executor/tests/merge_error_path_test.rs`（新建）

**内容**:
- test_merge_invalid_condition_type
- test_merge_update_failure
- test_merge_insert_failure

**验证**: `cargo test -p sqlrustgo-executor --test merge_error_path_test`

---

## Phase 3: Crash Recovery 测试

### Task 3.1: 运行 crash-sim

**命令**:
```bash
cargo test -p sqlrustgo-crash-sim --test crash_sim_test -- --nocapture
```

**验证**: 所有 crash injection 测试通过

### Task 3.2: WAL 覆盖率测试

**命令**:
```bash
cargo llvm-cov -p sqlrustgo-wal --html
```

**验证**: WAL 模块覆盖率达到 ≥90%

---

## Phase 4: Coverage 脚本改进

### Task 4.1: 改进 coverage 运行脚本

**文件**: `scripts/run_coverage.sh`（新建）

**功能**:
- 控制并行任务数（默认 4）
- 内存限制（2GB/job）
- CPU 负载监控（阈值 80%）
- 分 crate 独立运行
- 增量 coverage 支持
- 失败时自动降级

**验证**: `bash scripts/run_coverage.sh --help`

### Task 4.2: 验证整体覆盖率

**命令**:
```bash
bash scripts/run_coverage.sh --target 70
```

**验证**: TOTAL regions ≥70%

---

## 执行检查清单

- [ ] Task 1.1: UpdateExec 分析完成
- [ ] Task 1.2: UpdateExec 实现完成
- [ ] Task 1.3: execute_update 实现完成
- [ ] Task 1.4: UPDATE 测试通过
- [ ] Task 1.5: UPDATE QPS ≥10,000
- [ ] Task 2.1: DELETE 错误路径测试通过
- [ ] Task 2.2: MERGE 错误路径测试通过
- [ ] Task 3.1: crash-sim 测试通过
- [ ] Task 3.2: WAL 覆盖率 ≥90%
- [ ] Task 4.1: coverage 脚本完成
- [ ] Task 4.2: 整体覆盖率 ≥70%
