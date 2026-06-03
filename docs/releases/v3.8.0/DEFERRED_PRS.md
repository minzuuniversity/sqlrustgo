# DEFERRED PRs — v3.8.0 未实现功能状态说明

> **Created**: 2026-06-02
> **Auditor**: Hermes Agent
> **Status**: ACTIVE — 5 个 PR 标记 NOT_DONE，状态诚实记录

---

## 0. 重要说明（Truthfulness 承诺）

本文档**不**为以下 5 个未实现 PR 编造 SPEC/TEST_DESIGN/ACCEPTANCE 文档。
理由：这些 PR 的实现本身不存在，没有代码可测，硬造文档=作弊。

每个 PR 仅给：
1. **应该测什么**（目标契约）
2. **当前未实现**（根因）
3. **重启工作的入口**（接手者直接续上）

---

## 1. F-07: PR-810 ExecutionEngine → Router

### 1.1 目标契约

将 `ExecutionEngine` 解耦为 `Router` + 多个 `Handler`：
```rust
pub trait EngineHandler: Send + Sync {
    fn can_handle(&self, stmt: &Statement) -> bool;
    fn execute(&self, ctx: &mut ExecCtx, stmt: &Statement) -> Result<...>;
}

pub struct Router {
    handlers: Vec<Box<dyn EngineHandler>>,
}
```

### 1.2 当前状态

- **代码**: 不存在
- **根因**: PR-800 仅做 L0 入口重构（去掉 eng.execute(raw_sql)），Router 模式未引入
- **影响**: ExecutionEngine 仍是 1566 行 monolith，未做多 Handler 分发
- **关联 Issue**: #2591 (INT-4)

### 1.3 重启入口

```bash
git checkout develop/v3.8.0
git checkout -b feature/pr-810-router

# Step 1: 定义 EngineHandler trait
touch crates/executor/src/execution/router.rs
# ... 实现 ...

# Step 2: 添加测试
touch crates/executor/tests/router_test.rs

# Step 3: 跑测试
cargo test -p sqlrustgo-executor --test router_test
```

### 1.4 推荐测试矩阵（接手者参考）

- Router::new() 创建
- Router::register(handler) 添加
- Router::route(Statement::Select) 找到正确 handler
- Router::route(Statement::Insert) 找到正确 handler
- 多个 handler 优先级
- 无匹配 handler → 返回 ExecutionError

---

## 2. F-08: PR-820 TransactionManager Session Binding

### 2.1 目标契约

将 `TransactionManager` 与 session 绑定（每个 connection 一个 TM 实例）：
```rust
pub struct Session {
    pub id: SessionId,
    pub tx_manager: TransactionManager,
    pub snapshot: Snapshot,
}
```

### 2.2 当前状态

- **代码**: `TransactionManager` 已存在但全局共享
- **根因**: PR-820 未实施，会话/事务绑定在更高层（ExecutionEngine）手动管理
- **影响**: 多 session 并发事务隔离不严格
- **关联 Issue**: #2588 (INT-1), #2571 (WAL/MVCC)

### 2.3 重启入口

```bash
git checkout develop/v3.8.0
git checkout -b feature/pr-820-session-tm

# Step 1: 在 transaction crate 加 Session 抽象
touch crates/transaction/src/session.rs

# Step 2: 改写 TransactionManager 为 Session-scoped
# Step 3: 测试
touch crates/transaction/tests/session_test.rs
```

### 2.4 推荐测试矩阵

- Session::begin() 启动事务
- Session::commit() 提交
- Session::rollback() 回滚
- 多 Session 并发互不干扰
- Session 关闭自动 rollback 未提交事务

---

## 3. F-10: PR-850 mysql-server → LocalExecutor 统一

### 3.1 目标契约

`mysql-server` 端所有 SQL 走 `LocalExecutor` 而非独立代码路径：
```
mysql-server COM_QUERY → parser::parse → planner::plan → LocalExecutor::execute
```

### 3.2 当前状态

- **代码**: mysql-server 仍部分绕过 LocalExecutor
- **根因**: PR-850 是 PR-840/830 的下游，依赖 DML Transaction Interception 完成
- **影响**: DML 在 mysql-server 路径与 bench-cli 路径可能产生不同结果
- **关联 Issue**: #2591 (INT-4), #2572 (双执行路径)

### 3.3 重启入口

```bash
git checkout develop/v3.8.0
git checkout -b feature/pr-850-mysql-server-unify

# Step 1: 检查 mysql-server 当前是否仍用 eng.execute
grep -rn "eng.execute" crates/mysql-server/src/

# Step 2: 替换为 Parser → Planner → LocalExecutor 路径
# Step 3: 添加跨路径一致性测试
touch crates/mysql-server/tests/cross_path_consistency_test.rs
```

### 3.4 推荐测试矩阵

- SELECT 路径：mysql-server vs bench-cli vs direct 三路径结果一致
- INSERT 路径：三路径结果一致
- UPDATE 路径：三路径结果一致
- DELETE 路径：三路径结果一致
- 错误路径：三路径错误码一致

---

## 4. F-11: PR-860 Planner Layer Consolidation

### 4.1 目标契约

统一 Planner（消除 logical plan 与 physical plan 间的中间层）：
```
SQL AST → Planner::plan → PhysicalPlan (唯一)
```

### 4.2 当前状态

- **代码**: Planner 已存在（`crates/planner/src/planner.rs`），但中间层/优化器分散
- **根因**: PR-860 是 PR-850 的下游，依赖 mysql-server 统一后再合并
- **影响**: optimizer 与 planner 之间有冗余转换
- **关联 Issue**: #2590 (INT-3)

### 4.3 重启入口

```bash
git checkout develop/v3.8.0
git checkout -b feature/pr-860-planner-consolidate

# Step 1: 盘点现有 planner/optimizer 重复逻辑
diff crates/planner/src/planner.rs crates/optimizer/src/query_planner.rs

# Step 2: 合并到 unified planner
# Step 3: 测试
touch crates/planner/tests/consolidation_test.rs
```

### 4.4 推荐测试矩阵

- LogicalPlan → PhysicalPlan 唯一入口
- Optimizer pass 不破坏类型
- 同一 AST → 同一 PhysicalPlan
- 优化前后结果一致（语义保持）

---

## 5. F-12: PR-870 ParallelVolcanoExecutor 接入

### 5.1 目标契约

`ParallelVolcanoExecutor` 成为默认执行器：
```rust
pub struct ParallelVolcanoExecutor {
    workers: Vec<Worker>,
    exchange: ExchangeNode,
}
```

### 5.2 当前状态

- **代码**: `ParallelVolcanoExecutor` 存在但孤立（`crates/executor/src/executor.rs`）
- **根因**: PR-870 是 PR-860 的下游
- **影响**: 并行执行未接入主流程
- **关联 Issue**: #2589 (INT-2), #2628 (VTU Phase 2)

### 5.3 重启入口

```bash
git checkout develop/v3.8.0
git checkout -b feature/pr-870-parallel-volcano

# Step 1: 把 ParallelVolcanoExecutor 接入 ExecutionEngine
# Step 2: 单元测试
touch crates/executor/tests/parallel_volcano_integration_test.rs
```

### 5.4 推荐测试矩阵

- 4 worker 并行执行单 query
- 8 worker 并行 SF=1 TPC-H Q1
- 数据分区正确性
- Exchange 节点数据 shuffle 正确
- 并行 vs 串行结果一致

---

## 6. F-14: PR-890 Snapshot + MVCC + Rollback

### 6.1 目标契约

完整 MVCC 事务隔离：
```rust
pub struct Snapshot {
    pub snapshot_ts: u64,
    pub active_tx: BTreeSet<TxId>,
}

pub fn is_visible(snap: &Snapshot, row_version: &RowVersion) -> bool;
```

### 6.2 当前状态

- **代码**: MVCC 部分实现（`crates/transaction/src/mvcc.rs`）
- **根因**: PR-890 是 PR-820/840 的下游；SSI 检测器存在但未完整集成
- **影响**: ACID 完整性不足
- **关联 Issue**: #2588, #2571

### 6.3 重启入口

```bash
git checkout develop/v3.8.0
git checkout -b feature/pr-890-mvcc-rollback

# Step 1: 完善 Snapshot 可见性规则
# Step 2: 集成 SSI Detector
# Step 3: 测试
touch crates/transaction/tests/mvcc_isolation_test.rs
```

### 6.4 推荐测试矩阵

- T-ISO-01 Dirty Read Prevention
- T-ISO-02 Non-repeatable Read
- T-ISO-03 Phantom Read
- T-ISO-04 Write-Write Conflict
- T-ISO-05 Lost Update
- 跨 snapshot 可见性
- 长事务快照稳定性

---

## 7. 进度总览

| F | PR | 状态 | 影响门禁 | 接手成本 |
|---|---|------|----------|----------|
| F-07 | PR-810 | NOT_DONE | RC | 中（需重写 Executor） |
| F-08 | PR-820 | NOT_DONE | RC | 高（影响 session 层） |
| F-10 | PR-850 | NOT_DONE | GA | 中 |
| F-11 | PR-860 | NOT_DONE | GA | 中 |
| F-12 | PR-870 | NOT_DONE | GA | 高（并行化） |
| F-14 | PR-890 | NOT_DONE | GA | 高（ACID 完整） |

**RC Gate 阻塞项**: F-07, F-08, F-09
**GA Gate 阻塞项**: F-10, F-11, F-12, F-13(F-13 已 PARTIAL via PR-880F), F-14, F-15 (DONE)

---

**最后更新**: 2026-06-02
**更新者**: Hermes Agent
