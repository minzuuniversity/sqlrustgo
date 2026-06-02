# v3.8.0 历史遗留问题改进核实报告

> **版本**: v3.8.0
> **分支**: `origin/develop/v3.8.0` (commit `456ae294b`)
> **报告日期**: 2026-06-01
> **报告版本**: v2（修订版）
> **分析基准**: LEGACY_ISSUES.md + 实际代码 + 测试结果 + PR 合并记录

---

## 一、执行摘要

| 维度 | 状态 | 详情 |
|------|------|------|
| **代码集成** | ✅ 正确 | 所有 PR 均正确合并到 `develop/v3.8.0` |
| **单元测试** | ✅ 317/317 + 283/283 PASS | executor + storage crate |
| **Clippy** | ✅ 0 warnings | `cargo clippy --all-features -D warnings` |
| **格式** | ⚠️ 已修复 | `cargo fmt --all -- --check` → EXIT:1 (auto-fix 后 EXIT:0) |
| **WAL Recovery** | ✅ 22/22 PASS | RECOVERY-007 re-enabled + PASS |
| **MERGE 语法** | ❌ 未实现 | Parser 不支持 MERGE SQL 语法 |
| **mysql-server WAL** | ❌ 未接入 | FileStorage 绕过 WAL |

**总体结论**: 核心架构集成正确，主要缺陷已识别并记录。PR-870 VTU Merge 和 mysql-server WAL 接入为已知缺口，需在后续版本修复。

---

## 二、PR 完成状态逐项核实

### 2.1 R2: WAL Facade — DML 统一路由

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2690 | R2-A: DML 路由到 UnifiedFacade | `1975700a` | `local_executor.rs` (+21/-3) | ❌ 无独立测试 | ✅ 正确 — `execute_dml` 闭包通过 `facade.execute_dml()` 调用 WalStorage |
| #2692 | R2-B: 移除 WAL bypass fallback (fail-fast) | `9b844498` | `local_executor.rs` (+12/-10) | ❌ 无独立测试 | ✅ 正确 — `None` 时返回明确 `SqlError::ExecutionError` |

**集成验证**:
```rust
// local_executor.rs 约 line 1309
let deleted = match self.unified_facade {
    Some(ref facade) => facade.execute_dml(|storage| storage.delete(table_name, &[]))?,
    None => return Err(SqlError::ExecutionError(
        "DELETE without WAL facade — remove direct storage access".to_string()
    )),
};
```
✅ 代码路径清晰，错误消息明确。

**已知缺口**:
- WAL Facade 只在 `local_executor.rs` 内部使用，未暴露给 mysql-server
- `mysql-server` 使用 `FileStorage` 直接操作，绕过 UnifiedFacade

---

### 2.2 R3: Expr Convergence — MergeStatement 使用 planner::Expr

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2694 | R3-A: 添加 `contains_subquery` 方法 | `6fb51a36` | `planner/src/lib.rs` (+6) | ❌ 无独立测试 | ✅ 正确 — 返回 `false`（子查询支持 Phase 2） |
| #2695 | R3-B: MergeStatement 使用 `planner::Expr` | `9ec27732` | `merge.rs` (+113/-196), `statement.rs` | ✅ 单元测试 (196→113行) | ✅ 正确 — `Expression` → `Expr` + `Operator` 枚举 |

**集成验证**:
```rust
// planner/src/statement.rs
pub struct MergeStatement {
    pub on_condition: Expr,           // ✅ 从 Expression 迁移
    pub merge_clauses: Vec<MergeClause>,
}
pub struct MergeClause {
    pub update_values: Vec<Expr>,    // ✅ 从 Expression 迁移
    pub insert_values: Vec<Expr>,    // ✅ 从 Expression 迁移
}
```
✅ 类型迁移干净，`expression_to_value` 移除（不再需要），`Expr::Literal(Value)` 提供类型安全。

**测试覆盖** (`merge.rs` 内联测试):
- `test_compare_values_integer` ✅
- `test_compare_values_text` ✅
- `test_compare_values_null` ✅
- `test_compare_values_float_mixed` ✅
- `test_eval_binary_op` / `test_eval_binary_op_comparisons` ✅
- `test_eval_binary_op_or` ✅
- `test_op_compare_edge_cases` ✅
- `test_find_column_index` ✅

**已知缺口**:
- `eval_merge_expr` 不处理 `Expr::UnaryExpr`（NOT 条件）
- `Expr::Like` 在 `eval_binary_op` 中返回 `Value::Null`，未测试

---

### 2.3 R4: mysql-server 统一引擎 — STMT EXECUTE 复用共享 Engine

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2696 | R4: STMT EXECUTE 复用 shared engine | `83dd2ee23` | `mysql-server/src/lib.rs` (+1/-1) | ❌ 无独立测试 | ✅ 正确 — 1 行修复 |

**集成验证**:
```rust
// mysql-server/src/lib.rs 约 line 1291
// 修复前:
let mut eng = ExecutionEngine::new(storage.clone());

// 修复后:
let mut eng = engine.write().unwrap();  // ✅ 复用共享 engine
```
✅ `do_command_loop` 接收 `engine: Arc<RwLock<ExecutionEngine<FileStorage>>>` 参数，COM_QUERY 和 COM_STMT_EXECUTE 共用同一实例。

**已知缺口** (HIGH RISK):
- `FileStorage` 无 WAL 支持 — 所有 mysql-server DML 绕过 WAL
- `replace_placeholders` 在 line 1287 使用 `Vec::new()` — 参数绑定未实现
- 无 mysql-server crate 级别的集成测试

---

### 2.4 PR-830E: WAL Recovery Lifecycle — One-Shot Guard

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2688 | 文档: PR-830E SPEC | `86a67b46` | 3 个文档文件 (+660/-121) | N/A | ✅ 文档完整 |
| #2691 | 实现: StatefulRecoveryEngine + RecoveryState | `35e76479` | `recovery_engine.rs` (+82), `engine_builder.rs` (+16) | ✅ 2 个新测试 | ✅ 正确 |

**集成验证**:
```rust
// src/engine_builder.rs
pub fn with_wal_recovery(data_dir: PathBuf) -> SqlResult<ExecutionEngine<WalStorage<...>>> {
    let mut engine = Self::with_wal_file(data_dir)?;
    recover_wal(&mut engine)?;  // ✅ auto recovery on construction
    Ok(engine)
}
```
✅ `StatefulRecoveryEngine` 三状态机 (`Unrecovered` → `Recovered`/`Failed`) 实现 one-shot guard。

**测试覆盖**:
- `test_recovery_state_default` ✅ — 验证初始状态为 `Unrecovered`
- `test_stateful_engine_blocks_double_recovery` ✅ — 验证二次恢复返回空报告

**恢复流程**:
1. 读取 WAL 所有条目
2. `count_status()` 按 `tx_id` 分组
3. `filter_committed_entries()` 保留已提交事务的 DML 条目
4. `apply_entry()` 重放:
   - **Insert**: ✅ 完全支持
   - **Update**: ❌ **跳过** — "Full semantic parsing requires a separate improvement pass"
   - **Delete**: ✅ 支持

**已知缺口**:
- Update 重放未实现（这是文档记录的已知限制）

---

### 2.5 PR-830F: WAL Lifecycle Controller — Checkpoint + Truncation

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2693 | 文档: PR-830F SPEC | `6b9eb16b` | 2 个文档文件 (+637) | N/A | ✅ 文档完整 |
| #2697 | 实现: WalTruncationGate + CheckpointManager | `46a51bbc` | 8 个文件 (+265) | ✅ 2 个新测试 | ✅ 架构正确 |

**集成验证**:
```rust
// crates/storage/src/wal/mod.rs
pub trait WalTruncationGate: Send + Sync {
    fn safe_truncate_lsn(&self) -> Option<u64>;  // None = no checkpoint = no truncation
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.safe_truncate_lsn().is_some_and(|cp_lsn| wal_lsn <= cp_lsn)
    }
}
```
✅ CheckpointManager 实现 `WalTruncationGate`，`ExecutionEngine` 持有 `Option<Arc<RwLock<CheckpointManager>>>`。

**测试覆盖**:
- `test_truncation_gate_blocks_before_checkpoint` ✅ — checkpoint 前不允许截断
- `test_truncation_gate_allows_after_checkpoint` ✅ — checkpoint 后允许截断

**已知缺口 — CRITICAL: PR-830F 生命周期未闭合**:

| 组件 | 状态 | 验证 |
|------|------|------|
| `advance_checkpoint()` 定义 | ✅ 已有 (line 219) | `grep` 确认 |
| `try_truncate_wal()` 定义 | ✅ 已有 (line 237) | `grep` 确认 |
| `commit_transaction()` 调用 `advance_checkpoint` | ❌ **从未调用** | `grep` 仅返回定义行 |
| `commit_transaction()` 调用 `try_truncate_wal` | ❌ **从未调用** | `grep` 仅返回定义行 |

```rust
// execution_engine.rs line 1153-1173 — commit_transaction()
fn commit_transaction(&mut self) -> SqlResult<ExecutorResult> {
    // ...
    if let Ok(mut storage) = self.storage.write() {
        let _ = storage.commit_transaction();  // ✅ 调用了
    }
    self.transaction_manager.commit(tx_id)?;      // ✅ 调用了
    self.current_tx_id = None;
    // ❌ advance_checkpoint() 从未调用
    // ❌ try_truncate_wal() 从未调用
    Ok(ExecutorResult::empty())
}
```

**影响**: WAL 文件会无限增长，PR-830F 的 checkpoint-based truncation 机制从未激活。

**v3 修正 (2026-06-02, SPEC-002)**:
经深度代码审查 + 单元测试，发现实际缺陷更深层:
- `ExecutionEngine::advance_checkpoint`/`try_truncate_wal` 是 **dead code** (0 调用方)
- 更严重: `WalStorage` 写入 entry 时 lsn 全部为 0 字面量 (9 处)
- `current_lsn()` 基于 entry.lsn 计算 → 永远返回 0
- `commit_transaction` 中 `if commit_lsn > 0` 条件永不触发
- **PR-830F truncation 机制实际完全没工作**，不仅是 ExecutionEngine 端

修复详见 `docs/releases/v3.8.0/SPEC-002-pr830f-lifecycle.md`:
1. WalStorage 添加 `next_lsn: u64` 字段 + `append_wal_entry` helper
2. 9 处 `self.wal.append` → `self.append_wal_entry` (自动分配递增 lsn)
3. ExecutionEngine 删除 dead code
4. 新增 `test_pr830f_lifecycle_commit_advances_checkpoint_and_truncates` 验证

**状态**: ✅ **FIXED** (SPEC-002)

---

### 2.6 PR-850A/B: Stateless WAL + Tx Context Route A

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2680 | PR-850A/B: Stateless WAL + Tx Context | `ee440cc2` | `wal_storage.rs` (+83/-64) | ✅ 断言更新 | ✅ 正确 |
| #2682 | fix: 移除所有 `tx_id=0` 占位符 | `35d74c17` | `wal_storage.rs` (+11/-6) | ❌ 无新测试 | ✅ 正确 |

**集成验证**:
✅ 所有 WAL 操作 (`log_begin`, `log_commit`, `log_mutation`) 现使用 `self.inner.current_tx_id()` 获取真实 `tx_id`。

**测试覆盖**:
- `test_wal_storage_multiple_transactions` ✅
- `test_wal_storage_rollback` ✅
- `test_wal_storage_is_wal_enabled` ✅

**已知缺口**:
- 无独立测试验证 `tx_id` 在 WAL 条目中的正确性（需要日志检查）

---

### 2.7 PR-870: VTU Merge — MergeExecutor 接入

| PR | 描述 | 提交 | 文件变更 | 测试 | 集成正确性 |
|----|------|------|----------|------|------------|
| #2683 | feat: 注册 merge 模块 | `31aacf72` | `lib.rs` (+1), `merge.rs` (+84/-103), `statement.rs` | ✅ 移除过期测试 | ⚠️ 部分正确 |
| #2689 | feat: MERGE 检测接入 execute_dml | `5b5d63923` | `local_executor.rs` (+15) | ❌ 无新测试 | ❌ 未完成 |

**集成验证**:
```rust
// local_executor.rs 约 line 1501
if sql_upper.starts_with("MERGE") {
    return Err(SqlError::ExecutionError(
        "MERGE via ExecutionEngine: wired but needs parser support. ".to_string()
    ));
}
```
❌ `execute_merge()` **从未被调用** — 只是返回错误消息。

**已知缺口** (CRITICAL — 阻止 MERGE 执行):
1. **Parser 不支持 MERGE 语法** — `crates/parser/src/` 中零 MERGE 匹配
2. **LocalExecutor 没有 `Arc<Mutex<dyn ExecutionEngine>>`** — `MergeExecutor::new()` 需要此参数但 LocalExecutor 不提供
3. **`WHEN MATCHED THEN DELETE` 未实现** — 只支持 UPDATE/INSERT
4. **无端到端测试** — `merge_vtu_test.rs` 只测试 `MergeStatement` 构造，不测试实际执行

---

## 三、测试覆盖矩阵

| 类别 | 单元测试 | 集成测试 | E2E 测试 | 覆盖率 |
|------|----------|----------|----------|---------|
| **Executor** | 317 PASS ✅ | 大量 | ❌ 无 mysql-server E2E | 高 |
| **Storage** | 283 PASS ✅ | 16 (`wal_integration_test.rs`) | 8 (`crash_recovery_test.rs`) | 高 |
| **WAL Contract** | — | 23 (`wal_tx_contract_test.rs`) | — | 21/22 ✅ |
| **MergeExecutor** | 单元 ✅ | ❌ 无 execute_merge | ❌ 无 | 低 |
| **mysql-server** | ❌ 无 | ❌ 无 | ❌ 无 | 缺失 |

---

## 四、已知缺口与遗留问题

### 4.1 Critical 缺口（影响功能正确性）

| ID | 缺口 | 影响 | 优先级 |
|----|------|------|--------|
| **G1** | mysql-server 使用 `FileStorage` 无 WAL | DML 绕过 WAL，事务不持久化 | P0 |
| **G2** | PR-870: Parser 不支持 MERGE 语法 | MERGE 语句无法解析 | P0 |
| **G3** | PR-870: `execute_merge()` 未被调用 | MERGE 路径死代码 | P0 |
| **G4** | PR-870: LocalExecutor 缺少 `Arc<Mutex<dyn ExecutionEngine>>` | 无法实例化 MergeExecutor | P1 |
| **G5** | Update 重放在 Recovery 中跳过 | crash recovery 后 UPDATE 数据不一致 | P1 |

### 4.2 Critical 缺口 — Truthfulness 违规

| ID | 缺口 | 影响 | 优先级 |
|----|------|------|--------|
| **B4** | **Format Truthfulness 违规** | `cargo fmt --all -- --check` → EXIT:1，文档声称 EXIT:0 | 🔴 P0 |

**B4 详细**:
```
$ cargo fmt --all -- --check
EXIT: 1  (违规: merge.rs x4, engine_builder.rs x1, execution_engine.rs x2)

$ cargo fmt --all && cargo fmt --all -- --check
EXIT: 0  (掩盖了 HEAD 本身的格式问题)
```

根因: BETA_GATE_CONTRACT.md 声称 B4 "exit 0"，但实际用的是 `cargo fmt --all` (auto-fix) 然后检查，掩盖了 HEAD 的格式问题。

### 4.3 Medium 缺口（影响完整性）

| ID | 缺口 | 影响 | 优先级 |
|----|------|------|--------|
| **M1** | `replace_placeholders` 使用空 `Vec` | Prepared Statement 参数绑定无效 | P1 |
| **M2** | CheckpointManager 未在主循环中调用 | WAL 截断生命周期未闭合 | P2 |
| **M3** | `eval_merge_expr` 不处理 `UnaryExpr` | NOT 条件在 MERGE 中不支持 | P2 |
| **M4** | mysql-server 无 crate 级别测试 | R4 修复无自动化验证 | P2 |

### 4.4 历史架构违规 (AV-001~AV-007 — 来自 v3.7.0 ARCHITECTURE_VIOLATIONS.md)

这些违规在 LEGACY_ISSUES.md 中追踪，仍存在于 v3.8.0 代码库中：

| ID | 文件 | 位置 | 问题 |
|----|------|------|------|
| **AV-001** | `trigger.rs` | 429, 507, 509, 531 | `storage.insert/delete` 直接调用 |
| **AV-002** | `harness.rs` | 274, 315, 375 | `storage.insert` 直接调用 |
| **AV-003** | `merge.rs` | 88, 107 | `storage.update/insert` 直接调用 |
| **AV-004** | `parallel_vector_executor.rs` | 689, 706, 724, 743 | `storage.insert` 直接调用 |
| **AV-005** | `parallel_executor.rs` | 1140, 1145, 1253, 1258, 1578, 1583, 1733, 1738 | `memory_storage.insert` 直接调用 |
| **AV-006** | `local_executor.rs` | 1054, 1328, 1385, 1469 | `storage.delete/update` 直接调用 |
| **AV-007** | `vector_executor.rs` | 193, 220, 242, 293 | `storage.insert` 直接调用 |

验证命令:
```bash
grep -n "storage.insert\|storage.delete\|storage.update" crates/executor/src/trigger.rs
grep -n "storage.insert" crates/executor/src/harness.rs
```

### 4.5 回归热点状态 (LEGACY_ISSUES.md Section 3)

| Hotspot | 风险 | v3.8.0 状态 |
|---------|------|-------------|
| H-1: Trigger Bypass (COM_QUERY) | 🔴 HIGH | ❌ 未修复 — FileStorage bypass |
| H-2: Trigger Bypass (StoredProc) | 🔴 HIGH | ❌ 未修复 |
| H-3: TX Isolation Broken (STMT) | 🔴 HIGH | ✅ **已修复** — #2696 共享 engine |
| H-4: WAL Coverage Gap | 🟡 MEDIUM | ❌ 未修复 — FileStorage bypass |
| H-5: Commit Opacity | 🟡 MEDIUM | ⚠️ 部分修复 — AUTOCOMMIT semantics 声明 |
| H-6: Double-Commit Protection | 🟡 MEDIUM | ⚠️ 待验证 — TX Lifecycle 声明 |

---

## 五、下一步整改计划

### 5.1 立即修复（P0）

| 任务 | 描述 | 阻塞 | 负责 |
|------|------|------|-------|
| **T1** | mysql-server 接入 WalStorage | PR-840 WriteBuffer 先完成 | — |
| **T2** | Parser 添加 MERGE 语法 | 解析器工作 | — |
| **T3** | LocalExecutor 添加 `Arc<Mutex<dyn ExecutionEngine>>` | PR-870 | — |
| **T4** | 调用 `execute_merge()` 替代错误返回 | T2+T3 | — |

### 5.2 短期修复（P1）

| 任务 | 描述 | 阻塞 | 负责 |
|------|------|------|-------|
| **T5** | 实现 Update 重放 | PR-830E | — |
| **T6** | `replace_placeholders` 参数绑定 | R4 | — |
| **T7** | 添加 mysql-server 集成测试 | T1 | — |

### 5.3 中期改进（P2）

| 任务 | 描述 | 负责 |
|------|------|-------|
| **T8** | CheckpointManager 集成到主循环 | — |
| **T9** | `eval_merge_expr` 支持 `UnaryExpr` | — |
| **T10** | MERGE 支持 `WHEN MATCHED THEN DELETE` | — |

---

## 六、结论

### 6.1 核实方法缺陷分析

**v1 报告犯的四个错误**:

| 错误 | 严重性 | 说明 |
|------|--------|------|
| **错误1**: 运行 `cargo fmt --all` 而非 `cargo fmt --all -- --check` | 🔴 Truthfulness | auto-fix 掩盖了 HEAD 本身的格式违规 |
| **错误2**: 没有验证 `advance_checkpoint`/`try_truncate_wal` 是否被调用 | 🔴 功能缺失 | 只检查了函数存在，没检查调用点 |
| **错误3**: 没有检查 AV-001~AV-007 是否仍存在 | 🟡 历史债务 | 应该跨模块 grep storage.insert/delete/update |
| **错误4**: 混淆 "测试通过" 和 "集成正确" | 🟡 方法论 | PR-870 测试全过但 execute_merge 从未被调用 |

### 6.2 核实结论总结

**✅ 正确集成的 PR**:
- R2 (WAL Facade): DML 路由到 UnifiedFacade，fail-fast 正确
- R3 (Expr Convergence): 类型迁移干净，317 测试通过
- R4 (mysql-server 引擎复用): 1 行修复解决事务上下文丢失
- PR-830E (WAL Recovery): One-shot guard 正确实现，21/22 测试通过
- PR-850A/B (Stateless WAL): `tx_id=0` 占位符全部清理
- PR-870 (VTU Merge): 模块注册完成，但执行路径未连接

**❌ 未完成集成 / 存在缺陷的 PR**:
- PR-830F (WAL Lifecycle): 架构正确但生命周期未闭合（`advance_checkpoint`/`try_truncate_wal` 从未被调用）
- PR-870 (VTU Merge): Parser 缺失 + `execute_merge()` 从未被调用

**⚠️ Truthfulness 违规（需立即修复）**:
- B4 Format: `cargo fmt --all -- --check` → EXIT:1，文档声称 EXIT:0

### 6.3 测试覆盖总结

| 指标 | 值 | 状态 |
|------|---|------|
| Executor 单元测试 | 317/317 PASS | ✅ |
| Storage 单元测试 | 283/283 PASS | ✅ |
| WAL Contract 测试 | 22/22 PASS ✅ | RECOVERY-007 re-enabled + PASS |
| Clippy | 0 warnings | ✅ |
| Format (auto-fix 后) | 0 diffs | ✅ |
| Format (直接检查) | 8 违规文件 | ❌ |
| E2E (mysql-server) | 0 tests | ❌ |

### 6.4 立即整改行动

| 优先级 | 任务 | 负责 |
|--------|------|-------|
| 🔴 P0 | 修复 B4 Format Truthfulness: 提交 `cargo fmt --all` 结果，更新 BETA_GATE_CONTRACT.md | — |
| 🔴 P0 | 修复 PR-830F: 在 `commit_transaction` 中调用 `advance_checkpoint`/`try_truncate_wal` | — |
| 🔴 P0 | 修复 PR-870: Parser 添加 MERGE 语法 | — |
| 🟡 P1 | 修复 AV-001~AV-007: DML 绕过 WAL 的架构违规 | — |
| 🟡 P1 | mysql-server 接入 WalStorage | — |

---

*本报告基于 `develop/v3.8.0` commit `456ae294b` 生成*
*验证命令: `cargo test -p sqlrustgo-executor --lib` + `cargo test -p sqlrustgo-storage --lib` + `cargo clippy --all-features -- -D warnings` + `cargo fmt --all -- --check`*
*v2 更新: 添加了 B4 Format Truthfulness 违规、PR-830F 生命周期未闭合、AV-001~AV-007 架构违规的核实结果*
