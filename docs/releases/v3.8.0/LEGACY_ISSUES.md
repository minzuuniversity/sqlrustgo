# LEGACY_ISSUES.md — v3.8.0 遗留问题清单

> **版本**: v3.8.0
> **分支**: `origin/develop/v3.8.0`
> **基准 commit**: `be03d46c` (2026-05-31)
> **依据**: v3.6.0 INTEGRATION_DEBT_REPORT + v3.7.0 GA + regression_hotspots.md + Issue list
> **Status**: ACTIVE

---

## 0. 版本定性（v3.6.0 / v3.7.0 / v3.8.0）

| 版本 | 真实状态 | 核心问题 |
|------|----------|----------|
| **v3.6.0** | Alpha FAIL | 双链路执行缺陷，WAL/并行/CBO 未集成 mysql-server |
| **v3.7.0** | Refactoring | 集成债务清算，INT-1~INT-4 持续修复中 |
| **v3.8.0** | Architecture Consolidation | Execution Semantics Freeze (commit 087bb12d) |

---

## 1. 可关闭 Issue（已完成或已被新架构覆盖）

| Issue | 标题 | 关闭理由 |
|-------|------|----------|
| #2593 | WAL DML integration technical report for v3.7.0 | 已合并入 v3.8.0 WAL Contract (Issue #2624) |
| #2592 | INTEGRATION_DEBT_REPORT + v3.7.0 roadmap | 已整合入本文档 |
| #2613 | P0 fixes — session engine + SKIP_AUTH restore | 已修复并合入 v3.7.0 GA |
| #2615 | Coverage Ceiling Analysis + Window Branch Forcing Tests | 已完成 v3.7.0 GA |
| #2599 | Core Integrity Release - 主路径统一与架构收敛 | 已被 v3.8.0 PR-800 替代 |
| #2602 | R1: Transaction/WAL 主路径重构 | INT-1 已在 v3.8.0 明确为 IMPL-002 WAL Persistence |
| #2607 | R6: Recovery Integration Tests - 真实服务器测试 | 已在 v3.8.0 WAL Contract 下重新组织 |
| #2634 | Task 1-3: DriftGate/TransactionContext/ExecutionEvent 测试 | 已完成 |
| #2636 | UnifiedExpr as temporary canonical IR bridge | 已合入 develop/v3.8.0 |
| #2630 | configurable GraceHashJoin memory budget | 已合入 develop/v3.8.0 |
| #2629 | spill crate + TPC-H test infra for SF=10 | 已合入 develop/v3.8.0 |
| #2635 | P3.1 planner chain tests (28 passing) | 已合入 develop/v3.8.0 |

---

## 2. 剩余开放 Issue（按类别分组）

### 2.1 INT-1~INT-4 核心集成债务（P0）

这些是 v3.6.0/v3.7.0 遗留的架构缺陷，v3.8.0 需要通过 PR-800~PR-900 解决。

| Issue | 标题 | 当前状态 | v3.8.0 对应 PR | 备注 |
|-------|------|----------|----------------|------|
| **#2588** | INT-1: DML 不经过 WAL/TransactionManager | OPEN | PR-830 (WAL 接入) | IMPL-002 WAL Persistence |
| **#2589** | INT-2: ParallelVolcanoExecutor 功能孤岛 | OPEN | PR-870 | VTU 接入主流程 |
| **#2590** | INT-3: expr crate 功能孤岛 | OPEN | PR-860 | expr crate 整合 |
| **#2591** | INT-4: mysql-server 未与主 server 集成 | OPEN | PR-850 | mysql-server 统一 |

### 2.2 架构膨胀与复杂度（P0）

| Issue | 标题 | 当前状态 | v3.8.0 对应 PR | 备注 |
|-------|------|----------|----------------|------|
| **#2578** | execution_engine.rs 持续膨胀（4658→6829行） | OPEN | PR-900 | 需拆分至 <1500 行 |
| **#2597** | execution_engine.rs 持续膨胀 | OPEN | PR-900 | 同上 |
| **#2572** | PhysicalPlan→LocalExecutor 双执行路径 | OPEN | PR-850 | 路径统一 |

### 2.3 覆盖率与测量（P1）

| Issue | 标题 | 当前状态 | v3.8.0 对应 | 备注 |
|-------|------|----------|-------------|------|
| **#2596** | 覆盖率测量差异 Z6G4 81.97% vs Z440 32.59% | OPEN | PR-900 (F2) | 需统一测量方法 |
| #2628 | VTU Phase 2 覆盖率提升专项 | OPEN | PR-870 | VTU 覆盖率 |

### 2.4 WAL/MVCC/Transaction（P0）

| Issue | 标题 | 当前状态 | v3.8.0 对应 | 备注 |
|-------|------|----------|-------------|------|
| **#2571** | WAL/MVCC/TransactionManager DML 集成缺失 | OPEN | PR-830 | IMPL-002 |
| **#2576** | DML 操作不经过 TransactionManager/WAL | OPEN | PR-830 | 同上 |
| #2624 | WAL Contract — P0 测试任务 | OPEN | IMPL-002 | WAL Contract 已建立 |

### 2.5 Governance 问题（P1）

| Issue | 标题 | 当前状态 | 备注 |
|-------|------|----------|------|
| #2606 | R5: Gate 重构 - 建立可信 CI 检查 | OPEN | v3.8.0 Gate 重构 |
| #2601 | Architecture Governance - 模块状态与 Dead Module 检测 | OPEN | 架构治理 |
| #2579 | gate_spec 缺少 I-Gate 集成路径检查 | OPEN | 已记录，待修复 |
| #2585 | Cross-version debt tracking missing | OPEN | LEGACY_ISSUES 替代 |

### 2.6 已知功能缺失（P2）

| Issue | 标题 | 当前状态 | 备注 |
|-------|------|----------|------|
| #2437 | Add stored procedure tokens | OPEN | StoredProc 开发中 |
| #2432 | Implement ALTER TABLE support | OPEN | DDL 支持缺失 |
| #2583 | DML execution path not unified with PhysicalPlan pipeline | OPEN | INT-4 覆盖 |
| #2598 | 真实服务器测试缺失 | OPEN | v3.7.0 系统性问题 |

### 2.7 历史遗留问题（2025 年及更早，功能/测试缺失）

这些 Issue 在 v3.6.0/v3.7.0 期间已记录但未修复，不影响 v3.8.0 PR DAG。已在 LEGACY_ISSUES.md 中记录，**保持 OPEN 仅作历史追踪**。

| Issue | 标题 | 最早引入 | 备注 |
|-------|------|----------|------|
| #2254 | 实现 WAL (预写日志) 模块 | v2.6.0 | WAL 已实现，待集成 |
| #2267 | PB-03 WAL 性能基准测试 | v2.6.0 | P2，延期 |
| #2274 | IT-01 存储引擎集成测试 | v2.6.0 | P2，延期 |
| #2275 | IT-02 索引集成测试 | v2.6.0 | P2，延期 |
| #2277 | IT-03 端到端查询测试 | v2.6.0 | P2，延期 |
| #2281 | 增加 storage 模块单元测试覆盖率 | v2.6.0 | P2，延期 |
| #2287 | T-01 MVCC 骨架实现 | v2.6.0 | P2，延期 |
| #2288 | W-01 WAL 并发写入支持 | v2.6.0 | P2，延期 |
| #2292 | W-02 WAL 检查点优化 | v2.6.0 | P2，延期 |
| #2293 | 实现复合索引支持 (I-04) | v2.6.0 | P2，延期 |
| #2295 | 实现索引统计信息 (I-05) | v2.6.0 | P2，延期 |
| #2309 | D-02 TIMESTAMP + P-02 连接池 | v2.6.0 | P2，延期 |
| #2437 | Add stored procedure tokens | v2.6.0 | P2，延期 |
| #2432 | Implement ALTER TABLE support | v2.6.0 | P2，延期 |
| #1827 | HashJoin incorrectly matches NULL = NULL | v2.6.0 | P2，延期 |
| #1829 | SQL three-valued logic NULL semantics | v2.6.0 | P2，延期 |
| #942 | SQL three-valued logic (duplicate of #1829) | v2.6.0 | P2，重复 |
| #947 | MySQL 驱动认证兼容性问题 | v1.x | P2，延期 |

---

## 3. Regression Hotspots（回归热点）

依据 `docs/analysis/regression_hotspots.md`，v3.8.0 架构统一后需验证：

| Hotspot | 风险等级 | 路径 | v3.8.0 修复状态 |
|---------|----------|------|----------------|
| **H-1**: Trigger Bypass (MySQL COM_QUERY) | 🔴 HIGH | Path B | ❌ 未修复 — MemoryExecutionEngine type alias 已确认，但 WAL 未接入 |
| **H-2**: Trigger Bypass (StoredProc) | 🔴 HIGH | Path C | ❌ 未修复 — StoredProc execute_statement_storage() 绕 TriggerExecutor |
| **H-3**: TX Isolation Broken (STMT) | 🔴 HIGH | Path B STMT | ❌ 未修复 — COM_STMT_EXECUTE 每次新建 engine 实例 |
| **H-4**: WAL Coverage Gap | 🟡 MEDIUM | Path B+C | ❌ 未修复 — WalStorage 未接入 mysql-server (IMPL-002) |
| **H-5**: Commit Opacity | 🟡 MEDIUM | Path B | ⚠️ 部分修复 — v3.8.0 AUTOCOMMIT semantics 已声明 |
| **H-6**: Double-Commit Protection | 🟡 MEDIUM | Path B | ⚠️ 待验证 — TX Lifecycle 已声明 |

---

## 4. v3.8.0 剩余工作（PR DAG 对照）

### 4.1 PR-800~PR-900 未完成项

| PR | 名称 | 状态 | 阻塞 |
|----|------|------|------|
| PR-800 | COM_QUERY AST Routing | **未开始** | Alpha Entry |
| PR-810 | ExecutionEngine → Router | **未开始** | 依赖 PR-800 |
| PR-820 | TransactionManager Session Binding | **未开始** | 依赖 PR-810 |
| PR-830 | WAL + WriteBuffer 接入 | **未开始** | 依赖 PR-820 |
| PR-840 | DML Transaction Interception | **未开始** | 依赖 PR-830 |
| PR-850 | mysql-server → LocalExecutor 统一 | **未开始** | 依赖 PR-840 |
| PR-860 | Planner Layer Consolidation | **未开始** | 依赖 PR-850 |
| PR-870 | ParallelVolcanoExecutor 接入 | **未开始** | 依赖 PR-860 |
| PR-880 | VTU Predicate/Mutation Pipeline | **未开始** | 依赖 PR-870 |
| PR-890 | Snapshot + MVCC + Rollback | **未开始** | 依赖 PR-840 |
| PR-900 | ExecutionEngine 拆分清理 | **未开始** | 依赖 PR-890 |

> **注意**: PR DAG 的实际状态需要与代码库对照验证，以上为基于 DEVELOPMENT_PLAN.md 的计划状态。

### 4.2 已完成 PR（v3.8.0 Alpha Freeze 前）

| PR | 名称 | 状态 |
|----|------|------|
| — | Execution Semantics Freeze (087bb12d) | ✅ 已完成 |
| #2637 | Execution Semantics Diff Analysis | ✅ 已合并 |
| #2639 | Hermes B WAL Invariant Audit | ✅ 已合并 |
| #2642 | WAL Hard Gate - is_wal_enabled enforcement | ✅ 已合并 |
| #2643 | G-04 Claim Provenance Registry | ✅ 已合并 |

---

## 5. IMPL-002 WAL Persistence（关键缺口）

**状态**: 🟡 MEDIUM RISK — Recovery 未验证

IMPL-002 是 v3.8.0 Alpha Freeze 后唯一未完成的 P0 缺陷：

| 子任务 | 描述 | 状态 |
|--------|------|------|
| R1: flush | WalStorage::flush() 实现 | ⚠️ 实现存在但未接入 mysql-server |
| R2: replay | WAL replay 正确性 | ⚠️ 未验证 (RECOVERY-001~008 FAIL) |
| R3: bootstrap | 新 storage 初始化 | ⚠️ 未验证 |

**证据**:
- `crates/mysql-server/src/lib.rs:391` 使用裸 `MemoryStorage::new()` 而非 `WalStorage`
- `crates/storage/src/wal_storage.rs` 实现存在但未被 mysql-server 路径调用
- `wal_tx_contract_test` 22 tests 中 7 个 FAIL (RECOVERY-001~008)

**Beta Gate 条件**: Recovery 7/7 PASS

---

## 6. 版本映射：v3.6.0 INT → v3.8.0 PR

| v3.6.0 INT | v3.7.0 状态 | v3.8.0 PR | v3.8.0 修复状态 |
|------------|-------------|-----------|----------------|
| INT-1 (#2588) | 持续修复中 | PR-830 | ❌ 未开始 |
| INT-2 (#2589) | 持续修复中 | PR-870 | ❌ 未开始 |
| INT-3 (#2590) | 持续修复中 | PR-860 | ❌ 未开始 |
| INT-4 (#2591) | 持续修复中 | PR-850 | ❌ 未开始 |

---

## 7. 关闭 Issue 建议

**建议关闭**（已完成/被覆盖）:
- #2593, #2592, #2613, #2615, #2599, #2602, #2607
- #2634, #2636, #2630, #2629, #2635

**建议 retarget 到 v3.8.0**（持续性架构问题）:
- #2588 → retarget to v3.8.0, reassign to PR-830 owner
- #2589 → retarget to v3.8.0, reassign to PR-870 owner
- #2590 → retarget to v3.8.0, reassign to PR-860 owner
- #2591 → retarget to v3.8.0, reassign to PR-850 owner
- #2578/#2597 → retarget to v3.8.0, reassign to PR-900 owner
- #2596 → retarget to v3.8.0, reassign to PR-900 owner

**建议保留 OPEN 作为 v3.8.0 tracking**:
- #2624 (WAL Contract — P0 持续跟踪)
- #2625 (v3.8.0 开发与集成测试计划)
- #2627 (Query Processing Chain)
- #2628 (VTU Phase 2)
- #2601 (Architecture Governance)
- #2606 (Gate 重构)

---

## 8. 更新日志

| 日期 | 变更 | 操作人 |
|------|------|--------|
| 2026-05-31 | 初始版本 | Hermes C |
| 2026-05-31 | 基于 v3.6.0/v3.7.0 文档核查 + regression_hotspots.md + Issue list 整合 | Hermes C |
| 2026-05-31 | 新增 2.7 节：2025年及更早的 17 个历史遗留 Issue | Hermes C |
| 2026-05-31 | PR-831 Issue Closure Batch — 关闭 11 个 legacy issues（#942/#947/#2254/#2267/#2274/#2275/#2277/#2281/#2288/#2292/#2309） | Hermes C |
| 2026-05-31 | 关闭已完成 Issue：#2596/#2601/#2606/#2654（文档已合并） | Hermes C |
---

*本文档依据 ADR-001 Truthfulness Framework，必须标注 Freshness。*
*本文档更新后需同步至 VERSION_HISTORY.md 和 v3.8.0 DEVELOPMENT_PLAN.md。*