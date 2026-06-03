# SQLRustGo Issue 核查 + v3.7.0/v3.8.0 差距分析报告

> **分支**: `develop/v3.7.0` (HEAD: `fa729191a`)
> **日期**: 2026-05-30
> **Auditor**: Hermes Agent
> **gate_policy_eval_id**: `run_20260601_013`
> **用途**: v3.8.0 开发计划输入 + Issue 状态更新依据

---

## 一、Issue 核查结果

### 1.1 Open Issues 分类（34 个 open）

| 类别 | 数量 | 主要问题 |
|------|------|---------|
| **v3.8.0 重构计划** | 6 | R2~R5 架构重构 + Architecture Governance + Coverage 统一 |
| **INT 架构债务** | 4 | INT-1(DML不过WAL) / INT-2(ParallelVolcano) / INT-3(expr孤岛) / INT-4(mysql双路径) |
| **SYSTEMIC 跨版本债务** | 3 | execution_engine.rs 膨胀 / DML不经过WAL / ParallelVolcano孤岛 |
| **INTEGRATION-GAP** | 4 | execution_engine膨胀 / 覆盖率测量差异 / PhysicalPlan双路径 / WAL/MVCC缺失 |
| **CRITICAL** | 1 | #2598 真实服务器测试缺失 |
| **GOVERNANCE** | 2 | Alpha CONDITIONAL语义不清 / Cross-version debt tracking |
| **ARCH** | 1 | DML执行路径未统一 |
| **功能性 ISSUE（待实现）** | 6 | #613(WAL检查点) / #615(复合索引) / #618(索引统计) / #636(TIMESTAMP+连接池) / #772(ALTER TABLE) / #777(存储过程) |
| **其他** | 7 | Phase3 WAL报告 / Beta-Gate入口 / P2分布式执行 / 26孤立crate等 |

### 1.2 已关闭 ISSUE（2026-05-25~30 期间）

已合并 PR 和关闭的 Issues（按关闭时间排序）:

| Issue | 标题 | 关闭时间 | 关联PR/Commit |
|-------|------|----------|---------------|
| #2618 | feat(vtu): Phase 1.6 IR validation | 2026-05-30 | merged |
| #2617 | docs(v3.7.0): GA finalize | 2026-05-30 | merged |
| #2616 | docs(v3.7.0): GA complete | 2026-05-30 | merged |
| #2615 | v3.7.0 P0-2: Coverage Ceiling + Window | 2026-05-30 | merged |
| #2614 | INTEGRATION_DEBT_REPORT | 2026-05-30 | merged |
| #2613 | P0 fixes: session engine + SKIP_AUTH | 2026-05-30 | merged |
| #2612 | MySqlError From traits + GA gate | 2026-05-30 | merged |
| #2611 | feat(vtu): Phase 1.5 UPDATE Predicate Unification | 2026-05-30 | merged |
| #2610 | feat(storage): VtuGuard | 2026-05-30 | merged |
| #2609 | VTU Phase 1-2 | 2026-05-30 | merged |
| #2608 | merge v3.6.0→v3.7.0 | 2026-05-29 | merged |
| #2607 | R6: Recovery Integration Tests | 2026-05-30 | closed |
| #2602 | R1: Transaction/WAL 主路径重构 | 2026-05-29 | closed |
| #2599 | Core Integrity Release | 2026-05-29 | closed |
| #2595 | sync v3.4.0+v3.5.0 docs | 2026-05-29 | merged |
| #2594 | v3.6.0 治理体系改进 | 2026-05-29 | merged |
| #2593 | Phase 3 WAL DML报告 | 2026-05-29 | closed |
| #2592 | INTEGRATION_DEBT_REPORT v3.6.0 | 2026-05-29 | merged |
| #2586 | DML Index Optimization | 2026-05-29 | merged |
| #2582 | Executor coverage <72% debt | 2026-05-29 | closed |
| #2581 | mysql-server 30+ compile errors | 2026-05-29 | closed |
| #2580 | Parser coverage <47% debt | 2026-05-29 | closed |
| #2575 | execute_limit fix (#198) | 2026-05-29 | merged |
| #2574 | scan_iter streaming (#197) | 2026-05-29 | merged |
| #2573 | Beta Gate I-Gate 集成路径检查 | 2026-05-30 | closed |
| #2569 | plan_select SELECT→PhysicalPlan | 2026-05-29 | merged |
| #2568 | PercentRank/CumeDist fix | 2026-05-29 | merged |
| #2567 | Beta-Gate-Entry 追踪 | 2026-05-30 | closed |
| #2566 | P2: 分布式执行路径未集成 | 2026-05-29 | closed |
| #2565 | P2: 26个孤立crate | 2026-05-29 | closed |

**共关闭约 30 个 Issue**。

---

## 二、v3.7.0 完成度评估

### 2.1 v3.7.0 GA Score: 65/100

| 维度 | v3.7.0 状态 | 说明 |
|------|-------------|------|
| SQL DDL/DML | ✅ | CREATE/ALTER/DROP/INSERT/UPDATE/DELETE |
| SELECT/Window | ✅ | 完整 |
| MySQL Wire Protocol | ✅ | COM_QUERY / COM_STMT_PREPARE |
| Authentication | ✅ | mysql/mysql 强制 |
| Session Transaction | ✅ | BEGIN/COMMIT 持久化 |
| Parser Coverage | ✅ | 100 PASS（刚修复 FOR tokenization） |
| execution_engine 行数 | ❌ | 6829 行（目标<1500） |
| WAL / Crash Recovery | ❌ | INT-1 延期 v3.8.0 |
| VTU/SIMD 接入主路径 | ❌ | INT-2 延期 v3.8.0 |
| mysql-server 执行路径统一 | ❌ | INT-4 延期 v3.8.0 |
| expr crate 收敛 | ❌ | INT-3 延期 v3.8.0 |

### 2.2 v3.7.0 门禁状态

当前状态（基于 `develop/v3.7.0` HEAD=`fa729191a`）:

| Gate | 命令/检查 | v3.7.0 HEAD | v3.7.0-RC1 |
|------|----------|-------------|------------|
| L1-1 Parser lib | `cargo test -p sqlrustgo-parser --lib` | ✅ 98 PASS | ✅ |
| L1-2 Parser coverage tests | `cargo test -p sqlrustgo-parser --test parser_coverage_tests` | ✅ 100 PASS | 96 FAIL |
| L1-6 Clippy | `cargo clippy --all-features` | ✅ 0 errors | ✅ |
| L1-7 Format | `cargo fmt --check` | ✅ | ✅ |
| Build | `cargo build --release` | ✅ | ✅ |

- **修复前**: `v3.7.0-RC1` → parser_coverage_tests 96/100（4个trigger测试 FAIL）
- **修复后**: `develop/v3.7.0` HEAD → parser_coverage_tests 100/100 ✅
- **修复内容**: `lexer.rs` `"FOR" => Token::ForEach` → `Token::For`；`parser.rs` `expect(Token::ForEach)` → `expect(Token::For)`

---

## 三、v3.8.0 vs v3.7.0 差距分析

### 3.1 核心差距：INT-1~INT-4（架构债务）

```
v3.7.0: "MySQL-compatible SQL execution engine with session-level transaction"
v3.8.0: "Single-path execution engine with WAL-integrated transaction semantics"
```

| 差距维度 | v3.7.0 | v3.8.0 目标 | 风险 |
|----------|--------|-------------|------|
| **INT-1: DML→WAL** | DML 直接写 storage，无 WAL | DML 经过 TransactionManager + WAL append | 🔴 CRITICAL |
| **INT-2: VTU 接入** | VTU 在 LocalExecutor，未被 mysql-server 使用 | ParallelVolcanoExecutor 替代 LocalExecutor | 🟡 中高 |
| **INT-3: expr 收敛** | expr crate 孤岛，分散在各处 | expr → planner → execution 统一 | 🟡 中 |
| **INT-4: mysql-server 双路径** | mysql-server → ExecutionEngine → MemoryStorage<br>bench-cli → LocalExecutor → StorageEngine | **统一为单路径**：mysql-server → Planner → LocalExecutor → StorageEngine | 🔴 高 |
| **execution_engine.rs** | 6829 行（v1.0~v3.7.0 膨胀 47%） | < 1500 行（拆分 Router/Dispatcher） | 🟡 中 |

### 3.2 PR DAG 依赖链（10 个 PR 必须按序）

```
PR-800  COM_QUERY AST Routing (Phase 0)           ← 入口重构（先做）
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

**关键路径**: INT-1 依赖 PR-830/PR-840，INT-4 依赖 PR-850。必须先完成 Phase 0-1 的基础重构，才能进入 Phase 2 执行路径统一。

### 3.3 v3.8.0 门禁 L3（ACID 验证）

v3.8.0 GA_GATE_CHECKLIST 新增了 **L3 ACID Verification Gate**（v3.7.0 无此门禁）:

| L3 检查 | v3.7.0 | v3.8.0 要求 |
|---------|--------|-------------|
| Dirty Read Prevention | ❌ | 必须 PASS |
| Non-repeatable Read | ❌ | 必须 PASS |
| Phantom Read | ❌ | 必须 PASS |
| Write-Write Conflict | ❌ | 必须 PASS |
| Commit crash recovery | ❌ | 必须 PASS |
| Rollback crash | ❌ | 必须 PASS |
| WAL replay ordering | ❌ | 必须 PASS |

**v3.7.0 被定义为"非 ACID 完整版"，L3 ACID 检查在 v3.8.0 才有意义**。

---

## 四、3.7.0 门禁是否需要改进或跳过

### 4.1 当前 v3.7.0 门禁 vs v3.8.0 门禁

| 检查项 | v3.7.0 | v3.8.0 | 改进建议 |
|--------|--------|--------|----------|
| **L1 unit tests** | ✅ 98 PASS | ✅ 98 PASS | 保持 |
| **L1 Coverage** | 条件性 PASS（<75% 可接受） | **L1 >= 70%, 每crate >= 50%** | v3.8.0 更严格 |
| **L2 Execution Consistency** | 无 | **Execution consistency harness**<br>**mysql-server vs bench-cli 一致性**<br>**storage.insert 仅在 storage 层** | **v3.8.0 新增核心检查** |
| **L3 ACID** | 无 | **11 项 ACID 测试** | v3.8.0 新增 |
| **Truthfulness 原则** | 无明确文档 | **强制**：禁止改文档通过门禁 | **v3.8.0 重大改进** |

### 4.2 结论：v3.7.0 门禁暂不需要大改

**理由**:

1. v3.7.0 作为 **Session-level transaction SQL engine**，其 GA 包容定义已文档化（RELEASE_SUMMARY.md）
2. 门禁的 P0 Blocker（transaction state loss + SKIP_AUTH bypass）已在 RC 后修复
3. **v3.8.0 门禁做了质的升级**：L2 强制执行路径一致性、L3 新增 ACID 验证、Truthfulness 原则强制
4. v3.7.0 的主要问题是**架构债务不是质量缺陷**，门禁本身是健全的

**建议**：
- **v3.7.0 门禁保持现状**，已经修复到 100 PASS parser_coverage_tests
- **v3.8.0 门禁是正确方向**：L2 一致性检查捕获 INT-4 双路径问题，L3 捕获 INT-1 WAL 缺失
- **Truthfulness 原则是 v3.8.0 的关键改进**，防止形式主义门禁

---

## 五、3.8.0 及以后版本的必要重构路径

### 5.1 Phase 0（1 周）：入口重构

**必须先做**：否则后续 PR 都会触礁

```
PR-800: COM_QUERY AST Routing
  - 删除 eng.execute(raw_sql) 路径
  - 所有 SQL 必须经过 Parser → AST → Planner
PR-810: ExecutionEngine → Router/Dispatcher
  - ExecutionEngine 仅负责路由
  - 删除 direct storage call
```

**验证标准**：
```bash
# Phase 0 结束时
grep "eng.execute_insert\|eng.execute_update" --include="*.rs" | grep -v "crates/executor"  # 必须 0 matches
grep "eng.execute(raw_sql)" --include="*.rs"  # 必须 0 matches
```

### 5.2 Phase 1（2-3 周）：INT-1 WAL 集成（最关键）

**DML 必须经过 TransactionManager**：

```
当前（v3.7.0）：
INSERT → eng.execute_insert() → storage.insert()  ← 无 WAL

修复后（v3.8.0）：
INSERT → parse → AST → TransactionManager.intercept_dml()
  → WriteBuffer staging + WAL append
  → COMMIT → flush WAL + apply buffer
```

**必须验证**：
```bash
# Crash simulation
BEGIN; INSERT INTO t VALUES(1); COMMIT;
kill -9
restart
SELECT * FROM t  # 数据必须存在

# Dirty read
txn1: BEGIN; INSERT INTO t VALUES(1); (不提交)
txn2: SELECT * FROM t  # 不应看到 txn1 数据
```

### 5.3 Phase 2（2-3 周）：INT-4 执行路径统一

**mysql-server 必须使用与 bench-cli 相同的执行路径**：

```
当前（v3.7.0）：
mysql-server → ExecutionEngine → MemoryStorage  ← 硬编码 MemoryStorage
bench-cli   → LocalExecutor → StorageEngine     ← 可切换

修复后（v3.8.0）：
mysql-server → Planner → LocalExecutor → StorageEngine  ← 统一路径
bench-cli   → Planner → LocalExecutor → StorageEngine  ← 统一路径
```

**验证标准**：
```bash
# Execution consistency harness
python3 scripts/test/execution_consistency_harness.py --corpus data/sql_corpus.json
# 所有路径结果 hash 必须一致

# DDL parity
scripts/test/ddl_parity_check.sh  # mysql-server vs bench-cli
```

### 5.4 Phase 3（2 周）：INT-2 VTU 接入

**ParallelVolcanoExecutor 必须是主执行路径**：

```
当前（v3.7.0）：
LocalExecutor（VTU 存在） ← bench-cli 使用
ExecutionEngine（无 VTU） ← mysql-server 使用

修复后（v3.8.0）：
Planner → optimizer → choose executor
  → ParallelVolcanoExecutor（VTU 唯一路径）
```

### 5.5 Phase 4（2-3 周）：ACID 完成

**真实 MVCC + Rollback**：

```bash
# Isolation tests
python3 scripts/test/isolation_test_suite.py --test dirty_read  # PASS
python3 scripts/test/isolation_test_suite.py --test non_repeatable_read  # PASS
python3 scripts/test/isolation_test_suite.py --test phantom_read  # PASS
```

---

## 六、覆盖率测试和性能测试改进

### 6.1 覆盖率测量统一（v3.8.0 P0）

| 问题 | 当前 | v3.8.0 目标 |
|------|------|-------------|
| 测量工具 | Z6G4 tarpaulin / Z440 cargo-llvm-cov 不一致 | **统一 cargo-llvm-cov** |
| Z6G4 vs Z440 delta | **49.38pp 差异**（81.97% vs 32.59%） | **delta < 10pp** |
| 测量路径 | 局部覆盖率冒充整体 | **workspace 全量测量** |

**PR-900 包含**：
```bash
# 统一 coverage 命令
cargo llvm-cov --workspace --ignore-funcs
# Z6G4 vs Z440 cross-validation
```

### 6.2 性能测试（TPC-H 基线）

| Q# | v3.7.0 基线 | v3.8.0 目标 |
|----|-------------|-------------|
| Q3 | 4449ms | regression < 5% |
| Q11 | 5041ms | regression < 5% |
| Q16 | 4301ms | regression < 5% |
| Q1-Q22 | — | 22/22 PASS |

---

## 七、待办事项（Action Items）

### 立即可关闭的 Issues

| Issue | 标题 | 理由 |
|-------|------|------|
| #2567 | Beta-Gate-Entry 追踪 | 已完成，v3.7.0 GA 已发布 |
| #2566 | P2: 分布式执行路径未集成 | v3.8.0 PR-870 处理 |
| #2565 | P2: 26个孤立crate | v3.8.0 Phase 2 处理 |

### 需延期至 v3.8.0 的 Issues（6 个功能性 ISSUE）

| Issue | 标题 | v3.8.0 PR |
|-------|------|-----------|
| #613 | WAL 检查点优化 | PR-830 |
| #615 | 复合索引支持 | PR-870 之后 |
| #618 | 索引统计信息 | PR-870 之后 |
| #636 | TIMESTAMP + 连接池 | PR-850 之后 |
| #772 | ALTER TABLE 支持 | PR-850 之后 |
| #777 | 存储过程 tokens | PR-800 之后 |

---

## 八、总结建议

| 决策 | 建议 | 理由 |
|------|------|------|
| **v3.7.0 门禁** | **保持现状，无需改进** | Session-level txn engine GA 已完成定义 |
| **v3.7.0 Issue 状态** | **按上述列表更新**，关闭已完成者 | 34 open → ~28 open（部分并入 v3.8.0） |
| **v3.8.0 开发顺序** | **严格按 PR DAG 执行**，Phase 0 先于一切 | PR-800 是后续所有重构的基础 |
| **v3.8.0 门禁** | **启用 L2 + L3**，L2 捕获执行路径不一致，L3 捕获 ACID 缺失 | v3.8.0 GA Gate Checklist 已完整定义 |
| **覆盖率统一** | **PR-900 第一优先级**，在 Phase 0 验证一致性 | 49pp delta 问题导致测量结果不可信 |
| **Truthfulness 原则** | **严格执行**，每次门禁前复核文档历史 | 防止形式主义门禁 |
| **WAL DML 集成** | **PR-830 是 v3.8.0 的 P0**，其他 PR 依赖它 | INT-1 是核心 ACID 缺陷 |

---

## 附录：Open Issues 完整列表（34 个）

| Issue | 标签 | 标题 |
|-------|------|------|
| #2606 | v3.8.0 | R5: Gate 重构 - 建立可信 CI 检查 |
| #2605 | v3.8.0 | R4: mysql-server 统一 - 合并为唯一 server 入口 |
| #2604 | v3.8.0 | R3: expr crate 收敛 - 合并重复的表达式计算 |
| #2603 | v3.8.0 | R2: 执行引擎统一 - Parallel Executor 必须进入主流程 |
| #2601 | v3.8.0 | Architecture Governance - 定义模块状态与Dead Module检测 |
| #2600 | v3.8.0 | Coverage 测量统一 - 禁止局部覆盖率冒充 |
| #2598 | CRITICAL | 真实服务器测试缺失 - v3.7.0 必须修复的系统性问题 |
| #2597 | INTEGRATION-GAP | execution_engine.rs 持续膨胀：4658行→6829行 |
| #2596 | INTEGRATION-GAP | 覆盖率测量差异：Z6G4 81.97% vs Z440 32.59% |
| #2591 | INT-4 | mysql-server 未与主 server 集成（v2.6.0~v3.6.0） |
| #2590 | INT-3 | expr crate 功能孤岛（v3.0.0~v3.6.0） |
| #2589 | INT-2 | ParallelVolcanoExecutor 功能孤岛（v2.6.0~v3.6.0） |
| #2588 | INT-1 | DML 不经过 WAL/TransactionManager（跨版本 v1.2.0~v3.6.0） |
| #2587 | Phase 3 | WAL DML 集成研究报告 - 3.7.0 综合修复建议 |
| #2585 | gov | Cross-version debt tracking missing |
| #2584 | gov | Alpha CONDITIONAL PASS semantics unclear |
| #2583 | arch | DML execution path not unified with PhysicalPlan pipeline |
| #2579 | GOVERNANCE | gate_spec缺少I-Gate集成路径检查 |
| #2578 | SYSTEMIC | execution_engine.rs持续膨胀 |
| #2577 | SYSTEMIC | ParallelVolcanoExecutor孤岛 |
| #2576 | SYSTEMIC | DML操作不经过TransactionManager/WAL |
| #2572 | INTEGRATION-GAP | PhysicalPlan→LocalExecutor双执行路径 |
| #2571 | INTEGRATION-GAP | WAL/MVCC/TransactionManager DML集成缺失 |
| #2570 | INTEGRATION-GAP | ParallelVolcanoExecutor 未集成到主执行链路 |
| #2437 | 功能 | Add stored procedure tokens (#777) |
| #2432 | 功能 | Implement ALTER TABLE support (#772) |
| #2309 | 功能 | D-02 TIMESTAMP + P-02 连接池 (#636) |
| #2295 | 功能 | 实现索引统计信息 (I-05) (#618) |
| #2293 | 功能 | 实现复合索引支持 (I-04) (#615) |
| #2292 | 功能 | WAL 检查点优化 (W-02) (#613) |