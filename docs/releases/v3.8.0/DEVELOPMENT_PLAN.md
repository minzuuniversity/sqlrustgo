# SQLRustGo v3.8.0 Architecture Unification — Development Plan

> **Version**: v3.8.0
> **Type**: Architecture Unification Release
> **Target**: Execute path consolidation + ACID foundation
> **Branch**: `origin/develop/v3.8.0` (to be created)
> **Created**: 2026-05-30
> **Auditor**: Hermes Agent

---

## 1. 版本定性

### 1.1 本质定义

v3.8.0 **不是 feature release**，而是：

> **Execution Architecture Consolidation Release**

核心目标：
- 消灭双执行路径
- 统一 SQL → AST → Plan → Execution
- 接入事务核心层（WAL）
- 为 MVCC 完整化奠定架构基础

---

### 1.2 交付定义

v3.8.0 完成时，系统应从：

```
v3.7.0: "MySQL-compatible SQL execution engine with session-level transaction"
```

进化为：

```
v3.8.0: "Single-path execution engine with WAL-integrated transaction semantics"
```

---

## 2. PR DAG（依赖执行图）

按依赖顺序排列，10 个 PR 必须依次合并：

```
PR-800  COM_QUERY AST Routing (L0)           ← 入口重构（先做）
   ↓
PR-810  ExecutionEngine → Router            ← 解耦层（依赖800）
   ↓
PR-820  TransactionManager Session Binding   ← 事务会话绑定（依赖810）
   ↓
PR-830  WAL + WriteBuffer 接入               ← INT-1核心（依赖820）
   ↓
PR-840  DML Transaction Interception        ← DML截获（依赖830）
   ↓
PR-850  mysql-server → LocalExecutor 统一    ← INT-4（依赖840）
   ↓
PR-860  Planner Layer Consolidation         ← Planner收敛（依赖850）
   ↓
PR-870  ParallelVolcanoExecutor 接入         ← INT-2（依赖860）
   ↓
PR-880  VTU Predicate/Mutation Pipeline     ← VTU统一（依赖870）
   ↓
PR-890  Snapshot + MVCC + Rollback 完成     ← ACID完成层（依赖840）
   ↓
PR-900  ExecutionEngine 拆分清理             ← 收尾重构（依赖890）
```

---

## 3. 阶段目标（Alpha / Beta / RC / GA）

### Phase 0 — Architecture Freeze（1 周）

**目标**: 锁定执行路径 + 路由模型 + 事务入口点

**PR**: PR-800 + PR-810

**Deliverables**:
- [ ] COM_QUERY AST routing 替换 raw SQL execution
- [ ] ExecutionEngine 变为 dispatcher/router only
- [ ] 删除 `eng.execute(raw_sql)` 路径
- [ ] Session → engine + txn_manager 绑定结构

**Gate**:
```
A1: cargo test --all-features  0 failures
A2: cargo clippy --all-features  0 errors
A3: 无 raw SQL execution path 存在
```

---

### Phase 1 — Transaction Core（2-3 周）

**目标**: INT-1 主体 — WAL 集成 + WriteBuffer + TransactionManager hook

**PR**: PR-820 + PR-830 + PR-840

**Deliverables**:
- [ ] WAL append on DML write
- [ ] WriteBuffer staging
- [ ] DML 经过 TransactionManager 拦截
- [ ] COMMIT → WAL flush + buffer apply
- [ ] BEGIN/COMMIT/ROLLBACK 语义正确

**Gate**:
```
B1: BEGIN/INSERT/COMMIT 数据持久化验证
B2: Crash simulation: kill -9 + restart WAL replay
B3: 并发事务隔离基本验证（basic snapshot isolation）
```

**关键测试**:
```bash
# Crash recovery test
BEGIN; INSERT; COMMIT; → kill -9 → restart → SELECT 数据存在
# Dirty read prevention
txn1: BEGIN; INSERT (uncommitted)
txn2: SELECT → 不应看到 txn1 未提交数据
```

---

### Phase 2 — Execution Path Unification（2-3 周）

**目标**: INT-4 + INT-2 — mysql-server 与 LocalExecutor 统一

**PR**: PR-850 + PR-860

**Deliverables**:
- [ ] mysql-server → Planner → LocalExecutor → Storage（单路径）
- [ ] 删除 ExecutionEngine direct storage call
- [ ] PhysicalPlan → LocalExecutor → StorageEngine 统一
- [ ] `grep storage.insert` 仅存在于 Storage 层

**Gate**:
```
C1: mysql-server vs bench-cli SQL 结果一致
C2: 28 E2E test files PASS
C3: TPC-H SF=1 Q1-Q22 回归 < 5%
```

**关键测试**:
```bash
# Execution consistency harness
SQL corpus → 3 execution paths → diff engine
所有路径结果 hash 一致
```

---

### Phase 3 — VTU Integration（2 周）

**目标**: INT-2 — ParallelVolcanoExecutor 接入主流程

**PR**: PR-870 + PR-880

**Deliverables**:
- [ ] PhysicalPlan → optimizer → choose executor
- [ ] ParallelVolcanoExecutor 替代 LocalExecutor 简单执行
- [ ] PredicateCompiler / MutationCompiler 迁移到 Planner layer
- [ ] SIMD batch execution enable
- [ ] VTU path NOT fallback path（唯一路径）

**Gate**:
```
D1: VTU vectorized vs non-vectorized 结果一致
D2: regression QPS test < 5% degradation
D3: execution_engine.rs 行数开始下降（<6500）
```

---

### Phase 4 — ACID Completion（2-3 周）

**目标**: 真实 MVCC + Rollback + Snapshot

**PR**: PR-890

**Deliverables**:
- [ ] MVCC visibility rules（write-write conflict）
- [ ] rollback buffer（非 stub 实现）
- [ ] snapshot isolation 读
- [ ] dirty read / non-repeatable read prevention

**Gate**:
```
E1: ROLLBACK 实际隔离未提交数据
E2: 并发快照读一致性
E3: isolation level tests (READ COMMITTED basic)
```

**关键测试**:
```bash
# Isolation test
txn1: BEGIN; UPDATE SET x=2; (未提交)
txn2: SELECT x → 仍为旧值（不是 2）
txn1: ROLLBACK
txn2: SELECT x → 仍为旧值（未受 txn1 影响）
```

---

### Phase 5 — Hardening（GA prep，1-2 周）

**目标**: 收尾 + 压测 + failure injection

**PR**: PR-900

**Deliverables**:
- [ ] ExecutionEngine.rs 拆分（<1500 行）
- [ ] 删除 legacy fallback path
- [ ] 覆盖率对齐（Z6G4 vs Z440 < 10pp）
- [ ] Stress test: 100 并发连接，24h 稳定
- [ ] Failure injection: 网络断连、半写入恢复

**Gate**:
```
F1: execution_engine.rs < 1500 lines
F2: coverage Z6G4 vs Z440 delta < 10pp
F3: stress test 24h 0 panic
F4: failure injection 所有场景恢复
```

---

## 4. 测试体系（三层验证）

> 不是 checklist，是验证系统。每一层对应特定风险。

---

### Layer 1 — Unit Correctness（已有基础）

| 模块 | 测试目标 | 门禁 |
|------|----------|------|
| parser | AST 生成正确性 | 100% AST node coverage |
| transaction manager | 状态机正确性 | BEGIN/COMMIT/ROLLBACK 状态转换 |
| WAL | append/replay 正确性 | crash recovery test |
| planner | AST → PhysicalPlan 映射 | SQL ↔ Plan 双向验证 |

---

### Layer 2 — Execution Consistency（新增）

**核心**: CLI vs SERVER vs LOCAL EXECUTOR 三路径一致

```
SQL corpus
  ↓
execution_paths = [mysql-server, bench-cli, LocalExecutor direct]
  ↓
result_hash_diff engine
  ↓
assert: all paths same result
```

**测试矩阵**:

| Path | Status v3.7 | Target v3.8 |
|------|-------------|-------------|
| mysql-server → ExecutionEngine → MemoryStorage | ⚠️ broken | ✅ unified |
| bench-cli → LocalExecutor → Storage | ✅ | ✅ |
| Integration → Planner → LocalExecutor | ❌ not exist | ✅ |

**门禁**:
```
execution_consistency_harness:
  all SQL from corpus → result_hash consistent across all paths
```

---

### Layer 3 — ACID Verification（新增）

#### 3.1 Transaction Isolation Suite（新增）

| 测试 | 目标 | 验证方法 |
|------|------|----------|
| Dirty Read Prevention | 未提交数据不可见 | txn1 write → txn2 read → must not see |
| Non-repeatable Read | 同一事务内多次读取一致 | txn1 read → txn1 write → txn1 read → must same |
| Phantom Read | 范围查询一致性 | txn1 range query → txn2 insert → txn1 range query → must not include new |
| Write Conflict | 并发写入冲突检测 | txn1 write row X → txn2 write row X → one must wait or fail |

#### 3.2 Crash Simulation Suite（新增）

```bash
# 每个测试：写入 → crash → 重启 → 验证 WAL replay
test_crash_during_commit:  commit 过程中 kill -9
test_crash_during_rollback: rollback 过程中 kill -9
test_partial_write:        写入中断 → 重启后数据一致
test_wal_replay_ordering:  乱序写入 → replay 后数据正确
```

#### 3.3 Execution Divergence Test（新增）

```bash
same SQL → all execution paths must match result hash
```

**门禁**: 任何路径结果不一致 → PR 阻断

---

## 5. 关键风险与对策

### Risk 1: ExecutionEngine 膨胀锁死

**状态**: 6829 行，5 版本未重构

**对策**:
- PR-800 阶段立即开始 AST routing（不等待其他 PR）
- PR-900 最终拆分严格执行（<1500 行验收）

**Rollback plan**: 若 PR-900 失败，保留 PR-800~PR-890 作为 "unified but not shrunk" 版本

---

### Risk 2: 双路径残留

**状态**: `mysql-server → ExecutionEngine → MemoryStorage` 与 `bench-cli → LocalExecutor` 并存

**对策**:
- PR-850 强制统一路径（唯一路径：mysql-server → Planner → LocalExecutor → Storage）
- `grep storage.insert` 仅允许存在于 Storage layer

**验证**: 
```bash
grep -r "storage\.insert\|storage\.update\|storage\.delete" --include="*.rs" \
  | grep -v "crates/storage/" \
  | grep -v "crates/executor/" \
  → must be empty
```

---

### Risk 3: VTU 未完全接入

**状态**: ParallelVolcanoExecutor 存在但未使用

**对策**:
- PR-870 强制接入（LocalExecutor is NOT fallback）
- 设置 gate：DML query 性能无 VTU 提升 = PR 阻断

**验证**:
```bash
# VTU enabled vs disabled performance delta must be < 5% regression
# (VTU should be faster or equal, never slower)
```

---

## 6. 版本里程碑

| Milestone | 目标 | PRs | Gate |
|-----------|------|-----|------|
| v3.8.0-Alpha | L0 + L1 完成 | PR-800, 810, 820, 830, 840 | A1-A3, B1-B3 |
| v3.8.0-Beta | L2 + L3 完成 | PR-850, 860, 870, 880 | C1-C3, D1-D3 |
| v3.8.0-RC | L4 + hardening | PR-890, 900 | E1-E3, F1-F4 |
| v3.8.0-GA | 全量验证 | All PRs | Full test suite + stress |

---

## 7. 与 v3.7.0 的版本映射

| v3.8.0 Issue | v3.7.0 Issue | 说明 |
|--------------|--------------|------|
| INT-1 (#2588) | INT-1 | DML → WAL 集成 |
| INT-2 (#2589) | INT-2 | ParallelVolcanoExecutor 接入 |
| INT-3 (#2590) | INT-3 | expr crate 收敛 |
| INT-4 (#2591) | INT-4 | mysql-server 统一 |
| #2597 | execution_engine.rs 膨胀 | PR-900 拆分 |
| #2596 | 覆盖率差异 | PR-900 统一测量 |

---

## 8. 一句话总结

> **v3.7.0 = "能跑的 SQL engine"**  
> **v3.8.0 = "真正的数据库内核成型版本"**  
> **核心：3 个架构收敛点（入口统一、WAL 集成、单路径执行）+ 7 个渐进修复层**