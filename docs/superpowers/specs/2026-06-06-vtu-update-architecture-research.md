# VTU UPDATE 架构现状调研 — 与 #3236 整合分析

> **调研日期**: 2026-06-06
> **目标 issue**: #3236 (Refactor UPDATE 路径: storage.update() 唯一入口)
> **触发原因**: PR-841/842 (PR #3239) 合并后发现 work 已被 VTU 团队大幅超越
> **本文作用**: 重新评估 #3236 范围，提出与现有 VTU 系统的整合路径

## 执行摘要

**结论先行**: #3236 原始 6-phase 计划（14-20h）已**大幅被超越**。VTU（Visionary Transaction Unit）系统已在多个 ARCH/INT issue 中分阶段实施：
- VtuGuard 包装器（INT-4, #2973）已 closed
- UpdatePlan IR + AstAdapter + UpdateCompiler 已实施
- VTU main path（ARCH-3, #3129）已 closed
- `local_executor.rs:1371` 已使用 VTU-compliant 路径
- `execution_engine.rs:527` 仍走 DELETE+INSERT 旧路径，但 ARCH-3 已开始迁移

**实际剩余 #3236 工作量**: 6-10h（vs 原始 14-20h）

## 1. VTU 系统完整技术栈

### 1.1 架构图

```
┌─────────────────────────────────────────────────────────────┐
│  Parser AST (UpdateStatement)                                │
│  crates/parser/src/parser.rs:420                             │
└──────────┬──────────────────────────────────────────────────┘
           │ AstAdapter::to_update_plan()
           │ crates/executor/src/ast_adapter.rs:14
           ▼
┌─────────────────────────────────────────────────────────────┐
│  VTU IR (crates/storage/src/vtu_ir.rs)                       │
│  ├── UpdatePlan (predicate + mutation + hashes)              │
│  ├── AssignmentIR / ExprIR / MutationIR / PredicateIR       │
│  └── PlanTrace (用于调试与审计)                              │
└──────────┬──────────────────────────────────────────────────┘
           │ UpdateCompiler::compile()
           │ crates/executor/src/update_compiler.rs:47
           ▼
┌─────────────────────────────────────────────────────────────┐
│  VtuGuard::execute_dml(closure)  ← 统一 DML 入口            │
│  crates/storage/src/vtu_guard.rs                             │
│  - INT-4: 强制所有 DML 走 TransactionManager                 │
│  - Direct insert/update/delete 调用会 panic                  │
└──────────┬──────────────────────────────────────────────────┘
           ▼
┌─────────────────────────────────────────────────────────────┐
│  StorageEngine::update()  (5 个实现)                         │
│  ├── engine.rs:471 (trait 定义)                              │
│  ├── wal_storage.rs:417 (WAL 集成)                           │
│  ├── file_storage.rs:1460 (持久化)                           │
│  ├── columnar/storage.rs:705 (列存)                          │
│  └── vtu_guard.rs:96 (包装器)                                │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 关键组件状态

| 组件 | 路径 | 行数 | 状态 |
|------|------|------|------|
| VtuGuard 包装器 | `crates/storage/src/vtu_guard.rs` | ~250 | ✅ 实施 |
| UpdatePlan IR | `crates/storage/src/vtu_ir.rs` | ~600 | ✅ 实施 |
| AstAdapter | `crates/executor/src/ast_adapter.rs` | ~80 | ✅ 实施 |
| UpdateCompiler | `crates/executor/src/update_compiler.rs` | ~100 | ✅ 实施 |
| vtu_ir_test (unit) | `crates/storage/tests/vtu_ir_test.rs` | 780+ | ✅ 测试覆盖 |
| vtu_ir_pipeline_test | `crates/storage/tests/vtu_ir_pipeline_test.rs` | ~300 | ✅ E2E 测试 |

## 2. 两条 execute_update 路径对比

### 2.1 local_executor.rs:1371 — VTU 路径（推荐）

```rust
/// Execute update using VTU-compliant path
fn execute_update(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult> {
    use sqlrustgo_planner::UpdateExec;
    use crate::predicate_compiler::PredicateCompiler;
    use crate::mutation_compiler::MutationCompiler;
    // ...
    let update_exec = plan.as_any().downcast_ref::<UpdateExec>();
    // 编译 predicate + mutation，调用 storage.update()
}
```

**特点**:
- 走完整 VTU 流程
- 使用 UpdatePlan IR
- 通过 VtuGuard::execute_dml 入口
- 不直接做 DELETE+INSERT

### 2.2 execution_engine.rs:527 — 旧路径（PR-842 过渡）

```rust
pub fn execute_update(&mut self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
    // IMPL-001 & IMPL-004: TX lifecycle enforcement
    // INT-1: Begin an implicit transaction
    // PR-842 Option A: compute updates from SET clauses
    // ...
    if update.where_clause.is_none() {
        // 简化路径：先 storage.update() 记录 WAL
    } else {
        // 走 DELETE+INSERT（70+ 行）
    }
}
```

**问题**:
- 仍走 DELETE+INSERT 序列
- PR-842 Option A 是过渡补丁（双重调用 storage.update() + DELETE+INSERT）
- ARCH-3 (5ed7a1d75) 已给 INSERT/UPDATE/DELETE 加 `set_current_tx_id()`，但 VtuGuard 完整迁移未完

### 2.3 路径使用统计

| 路径 | 调用方 | 状态 |
|------|--------|------|
| `local_executor.rs:1371` | `crates/server` (via plan), `local_executor.rs:234, 269` | 主路径 |
| `execution_engine.rs:527` | `src/execution_engine.rs:234` (Statement::Update), `src/engine_select.rs:27`, `src/engine_builder.rs` | 旧路径，仍在用 |

**结论**: 旧路径 `execution_engine.rs` 仍是主 bin (sqlrustgo-mysql-server) 的默认入口，必须迁移。

## 3. #3236 原始 Phase 重评估

| Phase | 原始描述 | 估时 | 当前状态 | 实际剩余 |
|-------|---------|------|---------|---------|
| 1 | 规划重构路径 | 2-3h | ✅ 已由 VTU 团队完成（多 commit） | 0h |
| 2-3 | feature flag + 双路径对比 | 4-6h | 🟡 部分：ARCH-3 已加 set_current_tx_id() | 2-3h（一致性测试） |
| 4 | 迁移测试 | 6-8h | 🟡 vtu_ir_test 已覆盖；execution_engine 测试需迁移 | 2-3h |
| 5-6 | 文档 + 清理 | 2-3h | ❌ 未开始 | 2-3h |
| **总计** | | **14-20h** | | **6-10h** |

## 4. 相关 Issue 与依赖

### 4.1 已 Closed 的前置 issue

| Issue | 标题 | 关闭原因 |
|-------|------|---------|
| #2741 | UPDATE 路径债务 | 由 PR-2755/2758/2761 解决 |
| #2870 | storage tx tracking | F-09 系列修复合并 |
| #3223 | storage-layer tx tracking | SPEC-002/003 合并 |
| #2973 | INT-4 VtuGuard 未强制 | INT-4 实施（131f466bf） |
| #2974 | ARCH-2 merge.rs 统一 | 实施完成 |
| #3129 | ARCH-3 VTU main path | 5ed7a1d75 实施 |
| #3215/3216/3217 | TPC-H Q22/Q8/Q9 | PR-3206/3208 修复 |

### 4.2 仍 Open 的关联 issue

| Issue | 标题 | 状态 |
|-------|------|------|
| **#3236** | storage.update() 唯一入口重构 | open（本文档化） |
| #3108 | INT-2/INT-3 集成债务（5+ 版本） | open，跨多版本 |
| #3146 | INT-3 expr 完整合并 + INT-2 主路径 | open（#3108 follow-up） |

## 5. #3236 重新定义的 Phase 计划

### Phase B1: 双路径一致性测试（2-3h）
- 设计 UPDATE 行为的对比测试集（INSERT/UPDATE/DELETE/WHERE/索引）
- 同时跑 local_executor 和 execution_engine，断言结果一致
- 输出：测试报告（哪些一致，哪些不一致）

### Phase B2: execution_engine.rs 迁移（4-6h）
- 把 `execute_update` DELETE+INSERT 替换为 VtuGuard::execute_dml
- 利用 AstAdapter + UpdateCompiler 走 VTU 路径
- 保留 TX lifecycle（IMPL-001/004）和 INT-1（set_current_tx_id）
- 逐个迁移测试到新路径

### Phase B3: 文档更新（1-2h）
- 重写 `docs/contracts/UPDATE_REPLAY_CONTRACT.md` 为 VTU 视角
- 在 #3236 描述里更新 Phase 状态
- 添加 PR-841/842 已被合并的备注

### Phase B4: 路径收敛（后续）
- 关闭 execution_engine.rs 旧路径
- 仅保留 local_executor（统一入口）
- 标记 src/execution_engine.rs 为 deprecated

## 6. 推荐执行顺序

1. **立即**: B3 文档化（1-2h）—— 让 #3236 描述不再误导
2. **本周**: B1 双路径一致性测试（2-3h）—— 暴露实际问题
3. **下周**: B2 迁移（4-6h）—— 关闭路径分裂
4. **后续**: B4 + #3108 集成债务（多版本）

## 7. 风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 行为差异（DELETE+INSERT 触发 secondary index 两次） | 性能 + 正确性 | B1 一致性测试 + B2 迁移 |
| UNIQUE 约束 / 外键级联行为 | 正确性 | B1 测试覆盖 |
| 外部代码依赖 `storage.delete() + storage.insert()` | 兼容性 | 保留 VtuGuard 双调用（迁移期间） |
| 性能退化 | 性能 | 保留原 `perf_baselines/` 对比 |
| 跨多版本集成债务 (#3108) | 长期 | 本 issue 不阻塞 INT-2/3 工作 |

## 8. 关联文档

- `docs/contracts/UPDATE_REPLAY_CONTRACT.md`（需重写）
- `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`（P0 阻塞分析）
- `docs/superpowers/specs/2026-05-18-stored-proc-expression-functions-design.md`（同期 spec 格式参考）

## 9. 关联 Commits 时间线

```
c919ca024  feat(storage): add VtuGuard — VTU violation detection wrapper
9017162e4  sync: merge VTU Phase 1-2.3 + Gate docs
a412b3754  cherry-pick(storage): add RowFilter/RowMutation and update_if signature
131f466bf  fix(v3.8.0): INT-4 VtuGuard enforcement on trigger DML
ac1454ed2  fix(v3.8.0): INT-4 explicit TX path (Closes #2973)
5ed7a1d75  fix(arch): #3129 ARCH-3 VTU main path (MemoryStorage TX + autocommit)
feafa1ec0  PR-841: add structured UPDATE serialization (本会话 filter-branch)
90531c9ea  PR-842: storage.update() before DELETE+INSERT (本会话 filter-branch)
e0982b5ec  test: ignore test_partial_update_value_recovery (本会话)
```

---

**调研人**: claude-macmini
**调研范围**: VTU 实施进度 + #3236 范围重评估
**产出物**: 本文档 + #3236 描述更新
**下一步**: 双路径一致性测试 (Phase B1)
