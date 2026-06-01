# SQLRustGo Issue 核查 + v3.7.0/v3.8.0 差距分析报告

> **版本**: v3.7.0 GA Final / v3.8.0 开发计划
> **分支**: `origin/develop/v3.7.0` (HEAD: `f8cf815f`) / `origin/develop/v3.8.0` (待建)
> **日期**: 2026-05-30
> **Auditor**: Hermes Agent

---

## 一、Gitea 最新状态

### 1.1 当前仓库状态

| 字段 | 值 |
|------|-----|
| 远程 | origin (http), gitea, gitcode, gitee |
| v3.7.0 HEAD | `f8cf815f` (develop/v3.7.0, 10 commits ahead of `faa5b715`) |
| v3.8.0 分支 | `develop/v3.8.0-claim-registry` (cherry-pick from `849ef685`) |
| claims | `develop/v3.8.0-claim-registry` (G-04 Claim Provenance Registry) |

### 1.2 门禁脚本混乱现状

**问题**：门禁脚本散落在多个位置，缺乏统一命名和调用规范：

| 路径 | 脚本 | 用途 |
|------|------|------|
| `gate/hermes_gate.sh` | hermes_gate.sh | L1 基础检查（clippy/fmt/python syntax/shell syntax） |
| `scripts/gate/gate.sh` | gate.sh | **废弃**，v2.7.0 遗留 |
| `scripts/gate/check_*.sh` | 14 个脚本 | 按功能分类（coverage/perf/security/evidence_binding 等） |
| `scripts/gate/run_hermes_gate.sh` | run_hermes_gate.sh | 调用 verification_report.json + audit_report.json |
| `scripts/gate/audit_*.sh` | 4 个审计脚本 | development/docs/process/testing 审计 |

**Gitea CI Workflow 门禁**（`.gitea/workflows/ci.yml`）：
```yaml
jobs:
  lint-build:    # clippy + fmt + build
  test:          # cargo test
  postcheck:     # gate/hermes_gate.sh + verify/verification_engine.py + audit/self_audit.py
```

**混乱点**：
1. `gate/hermes_gate.sh`（根目录）和 `scripts/gate/gate.sh`（scripts/）并存，容易混淆
2. `scripts/gate/run_hermes_gate.sh` 和 `gate/hermes_gate.sh` 功能重叠
3. CI workflow 调用 `gate/hermes_gate.sh`，但 `scripts/gate/gate.sh` 未被 CI 使用
4. `check_evidence_binding.sh` 等证据绑定脚本未在 CI workflow 中调用
5. `scripts/gate/check_perf_baseline.sh` 简化为 stub（只输出提示，无实际验证）
6. CI workflow 只在 `develop/v2.8.0` 相关分支运行，不在 `develop/v3.7.0` 运行

---

## 二、Issue 核查结果（42 个 open）

### 2.1 Open Issues 分类

| 类别 | 数量 | 主要问题 |
|------|------|---------|
| **v3.8.0 重构计划** | 5 | R2~R5 架构重构 + Architecture Governance |
| **INT 架构债务** | 4 | INT-1(DML不过WAL) / INT-2(ParallelVolcano) / INT-3(expr孤岛) / INT-4(mysql双路径) |
| **SYSTEMIC 跨版本债务** | 4 | execution_engine膨胀 / ParallelVolcano孤岛 / DML不经WAL / 真实服务器测试缺失 |
| **INTEGRATION-GAP** | 4 | execution_engine膨胀 / 覆盖率差异 / 双执行路径 / WAL集成缺失 |
| **功能性 ISSUE** | 8 | WAL检查点(#613) / 复合索引(#615) / 索引统计(#618) / TIMESTAMP(#636) / ALTER TABLE(#772) / 存储过程(#777) / 等等 |
| **GOVERNANCE** | 4 | 跨版本债务追踪缺失 / Alpha CONDITIONAL语义不清 / I-Gate缺失 / DML执行路径未统一 |
| **测试类 ISSUE** | 4 | storage覆盖率 / IT-03端到端 / IT-02索引 / IT-01引擎 |
| **WAL/MVCC** | 4 | WAL模块(#566) / MVCC骨架(#607) / 并发写入(#608) / 性能基准(#582) |
| **SQL 语义 BUG** | 4 | NULL三值逻辑(#1829/1827) / 认证兼容性(#947) / NULL语义(#942) |

### 2.2 已关闭 ISSUE（2026-05-25~30 期间）

| Issue | 标题 | 关联PR/Commit |
|-------|------|---------------|
| #2618 | feat(vtu): Phase 1.6 IR validation | ✅ merged |
| #2617 | docs(v3.7.0): GA finalize | ✅ merged |
| #2616 | docs(v3.7.0): GA complete | ✅ merged |
| #2615 | v3.7.0 P0-2: Coverage Ceiling + Window | ✅ merged |
| #2614 | INTEGRATION_DEBT_REPORT | ✅ merged |
| #2613 | P0 fixes: session engine + SKIP_AUTH | ✅ merged |
| #2612 | MySqlError From traits + GA gate | ✅ merged |
| #2611 | feat(vtu): Phase 1.5 UPDATE Predicate Unification | ✅ merged |
| #2610 | feat(storage): VtuGuard | ✅ merged |
| #2609 | VTU Phase 1-2 | ✅ merged |
| #2608 | merge v3.6.0→v3.7.0 | ✅ merged |
| #2607 | R6: Recovery Integration Tests | ✅ closed |
| #2602 | R1: Transaction/WAL 主路径重构 | ✅ closed |
| #2599 | Core Integrity Release | ✅ closed |
| #2593 | Phase 3 WAL DML报告 | ✅ closed |
| #2592 | INTEGRATION_DEBT_REPORT v3.6.0 | ✅ merged |
| #2586 | DML Index Optimization | ✅ merged |
| #2582 | Executor coverage <72% debt | ✅ closed |
| #2581 | mysql-server 30+ compile errors | ✅ closed |
| #2580 | Parser coverage <47% debt | ✅ closed |
| #2575 | execute_limit fix (#198) | ✅ merged |
| #2574 | scan_iter streaming (#197) | ✅ merged |
| #2573 | Beta Gate I-Gate 集成路径检查 | ✅ closed |
| #2569 | plan_select SELECT→PhysicalPlan | ✅ merged |
| #2568 | PercentRank/CumeDist fix | ✅ merged |
| #2567 | Beta-Gate-Entry 追踪 | ✅ closed |

**约 27 个 Issue 已关闭或合并**。

---

## 三、v3.7.0 vs v3.8.0 差距分析

### 3.1 核心架构债务

```
v3.7.0: MySQL-compatible SQL execution engine with session-level transaction
v3.8.0: Single-path execution engine with WAL-integrated transaction semantics
```

| 债务 | 持续版本 | v3.7.0 现状 | v3.8.0 目标 | 风险 |
|------|---------|-----------|------------|------|
| **INT-1: DML→WAL** | v1.2.0~v3.7.0 (7版本) | DML 直接写 storage，无 WAL | DML 经过 TransactionManager + WAL append | 🔴 CRITICAL |
| **INT-4: mysql-server 双路径** | v2.6.0~v3.7.0 (5版本) | mysql-server→ExecutionEngine→MemoryStorage<br>bench-cli→LocalExecutor→StorageEngine | **单路径**：mysql-server→Planner→LocalExecutor→StorageEngine | 🔴 HIGH |
| **INT-2: VTU 未接入** | v2.6.0~v3.7.0 (5版本) | VTU 在 LocalExecutor，未被 mysql-server 调用 | ParallelVolcanoExecutor 替代 LocalExecutor | 🟡 MEDIUM-HIGH |
| **INT-3: expr 收敛** | v3.0.0~v3.7.0 (3版本) | expr crate 孤岛，分散各处 | expr→planner→execution 统一 | 🟡 MEDIUM |
| **execution_engine.rs** | v1.0~v3.7.0 | **6829 行**（+46.9% 膨胀） | < 1500 行（Router/Dispatcher 拆分） | 🟡 MEDIUM |

### 3.2 PR DAG（10 个 PR 必须按序）

```
PR-800  COM_QUERY AST Routing (Phase 0)           ← 入口重构（先做）
   ↓
PR-810  ExecutionEngine → Router                 ← 解耦层（依赖800）
   ↓
PR-820  TransactionManager Session Binding         ← 事务会话绑定（依赖810）
   ↓
PR-830  WAL + WriteBuffer 接入                    ← INT-1 核心（依赖820）
   ↓
PR-840  DML Transaction Interception              ← DML 截获（依赖830）
   ↓
PR-850  mysql-server → LocalExecutor 统一        ← INT-4（依赖840）
   ↓
PR-860  Planner Layer Consolidation               ← Planner 收敛（依赖850）
   ↓
PR-870  ParallelVolcanoExecutor 接入             ← INT-2（依赖860）
   ↓
PR-880  VTU Predicate/Mutation Pipeline          ← VTU 统一（依赖870）
   ↓
PR-890  Snapshot + MVCC + Rollback 完成          ← ACID 完成层（依赖840）
   ↓
PR-900  ExecutionEngine 拆分清理                 ← 收尾重构（依赖890）
```

**关键约束**：Phase 0~1 必须先完成，否则 PR-830~PR-900 无法整合。

---

## 四、门禁体系问题分析

### 4.1 当前门禁体系混乱点

#### 4.1.1 脚本位置混乱

| 实际 | 期望 |
|------|------|
| `gate/hermes_gate.sh` + `scripts/gate/gate.sh` 并存 | 统一为 `gate/gate.sh` |
| `scripts/gate/run_hermes_gate.sh` 调用旧验证引擎 | 调用 CI 同一验证引擎 |
| CI workflow 调用根目录 `gate/`，但文档在 `scripts/gate/` | 统一入口 |

#### 4.1.2 CI Workflow 范围过窄

当前 CI 仅在 `develop/v2.8.0` / `beta/v2.8.0` / `ci/*` 分支运行，**不在 `develop/v3.7.0` 运行**。

#### 4.1.3 覆盖率测量差异未解决

| 问题 | Z6G4 | Z440 |
|------|------|------|
| 覆盖率 | 81.97% | 32.59% |
| 工具 | tarpaulin | cargo-llvm-cov |
| delta | **49.38pp** | — |

#### 4.1.4 证据绑定未集成到 CI

`scripts/gate/check_evidence_binding.sh` 存在但未在 CI workflow 中调用，无法防止文档伪造。

#### 4.1.5 L2/L3 门禁缺失

- **L2 Execution Consistency**：无 mysql-server vs bench-cli 一致性检查
- **L3 ACID Verification**：无 transaction isolation 测试
- v3.7.0 仅有 L1 基础检查（test/clippy/fmt）

### 4.2 门禁脚本功能映射

| 门禁层级 | 检查项 | 当前脚本 | 状态 |
|----------|--------|----------|------|
| **L1** | build | cargo build | ✅ CI 已集成 |
| **L1** | test | cargo test | ✅ CI 已集成 |
| **L1** | clippy | cargo clippy | ✅ CI 已集成 |
| **L1** | fmt | cargo fmt | ✅ CI 已集成 |
| **L1** | coverage ≥ 50% | scripts/gate/check_coverage.sh | ⚠️ 手动运行，未集成CI |
| **L2** | execution consistency | 无 | ❌ 缺失 |
| **L2** | perf baseline | scripts/gate/check_perf_baseline.sh | ⚠️ stub，只有提示 |
| **L3** | ACID isolation | 无 | ❌ 缺失 |
| **L3** | crash recovery | 无 | ❌ 缺失 |
| **Governance** | evidence binding | scripts/gate/check_evidence_binding.sh | ⚠️ 存在但未集成 |
| **Governance** | truthfulness | 无 | ❌ 缺失 |

### 4.3 v3.7.0 门禁是否需要改进

**结论：v3.7.0 门禁结构无需大改，但需要以下小修小补**

| 建议 | 优先级 | 说明 |
|------|--------|------|
| 将 CI workflow 扩展到 `develop/v3.7.0` 分支 | P0 | 当前仅在 v2.8.0 相关分支运行 |
| 统一门禁脚本入口为 `gate/gate.sh` | P1 | 合并 `gate/hermes_gate.sh` 和 `scripts/gate/gate.sh` |
| 覆盖率检查集成到 CI（≥50% 基线） | P1 | 当前 `check_coverage.sh` 未在 CI 中调用 |
| `check_evidence_binding.sh` 集成到 CI | P2 | 防止文档伪造 |
| 建立执行路径一致性检查 | P2 | L2 Execution Consistency |

**不建议在 v3.7.0 添加 L3 ACID 检查**（v3.7.0 定义为 Session-level transaction engine，ACID 延期至 v3.8.0）。

---

## 五、3.8.0 及以后版本的必要重构路径

### 5.1 Phase 0（1 周）：入口重构 — PR-800/810

```bash
# 验证标准
grep "eng.execute_insert\|eng.execute_update" --include="*.rs" | grep -v "crates/executor"
# → 必须 0 matches
grep "eng.execute(raw_sql)" --include="*.rs"
# → 必须 0 matches
```

### 5.2 Phase 1（2-3 周）：INT-1 WAL 集成 — PR-830/840

```bash
# Crash simulation
BEGIN; INSERT INTO t VALUES(1); COMMIT;
kill -9; restart
SELECT * FROM t  # 数据必须存在

# Dirty read
txn1: BEGIN; INSERT INTO t VALUES(1); (不提交)
txn2: SELECT * FROM t  # 不应看到 txn1 数据
```

### 5.3 Phase 2（2-3 周）：INT-4 执行路径统一 — PR-850

```bash
# Execution consistency harness
python3 scripts/test/execution_consistency_harness.py --corpus data/sql_corpus.json
# 所有路径结果 hash 必须一致
```

### 5.4 Phase 3（2 周）：INT-2 VTU 接入 — PR-870/880

### 5.5 Phase 4（2-3 周）：ACID 完成 — PR-890

### 5.6 Phase 5（1-2 周）：Hardening — PR-900

---

## 六、覆盖率测试和性能测试改进

### 6.1 覆盖率测量统一（PR-900，P0）

| 问题 | 当前 | v3.8.0 目标 |
|------|------|-------------|
| 测量工具 | tarpaulin vs cargo-llvm-cov | **统一 cargo-llvm-cov** |
| Z6G4 vs Z440 delta | **49.38pp** | **delta < 10pp** |
| 局部覆盖率冒充 | 存在 | **workspace 全量测量** |

### 6.2 性能测试（TPC-H SF=1 基线）

| Q# | 基线（实测 2026-05-29） | v3.8.0 目标 |
|----|------------------------|-------------|
| Q3 | 4449ms | regression < 5% |
| Q11 | 5041ms | regression < 5% |
| Q16 | 4301ms | regression < 5% |
| Q1-Q22 | — | 22/22 PASS |

---

## 七、待完成 Open Issues 整理

### 7.1 建议立即关闭的 Issues

| Issue | 标题 | 理由 |
|-------|------|------|
| #2567 | Beta-Gate-Entry 追踪 | v3.7.0 GA 已完成 |
| #2566 | P2: 分布式执行路径未集成 | 延期 v3.8.0 PR-870 |
| #2565 | P2: 26个孤立crate | v3.8.0 Phase 2 处理 |
| #2593 | Phase 3 WAL DML 报告 | 已合并 |
| #2607 | R6: Recovery Integration Tests | 已关闭 |

### 7.2 v3.8.0 PR 映射

| PR | 内容 | 解决 Issue |
|----|------|-----------|
| PR-800 | COM_QUERY AST Routing | #2583, #2579 |
| PR-810 | ExecutionEngine → Router | #2578 |
| PR-820 | TransactionManager Session Binding | #2571 |
| PR-830 | WAL + WriteBuffer 接入 | #2588, #2576, #613, #566 |
| PR-840 | DML Transaction Interception | #2588, #2576, #2571 |
| PR-850 | mysql-server → LocalExecutor 统一 | #2591, #2572, #2583, #636, #772 |
| PR-860 | Planner Layer Consolidation | #2590, #2583 |
| PR-870 | ParallelVolcanoExecutor 接入 | #2589, #2577, #2570, #615, #618 |
| PR-880 | VTU Predicate/Mutation Pipeline | #2589, #2577, #2570 |
| PR-890 | Snapshot + MVCC + Rollback | #2588, #607, #608, #582 |
| PR-900 | ExecutionEngine 拆分清理 | #2597, #2596, #2578 |

---

## 八、总结建议

| 决策 | 建议 | 理由 |
|------|------|------|
| **v3.7.0 门禁** | **小修小补，无需大改** | Session-level engine GA 已完成定义 |
| **CI Workflow 扩展** | **扩展到 develop/v3.7.0** | 当前仅在 v2.8.0 相关分支运行 |
| **门禁脚本统一** | **统一入口为 gate/gate.sh** | 消除 gate/hermes_gate.sh 和 scripts/gate/gate.sh 并存混乱 |
| **覆盖率检查** | **集成到 CI（≥50% 基线）** | 当前 scripts/gate/check_coverage.sh 未在 CI 中调用 |
| **evidence binding** | **集成到 CI postcheck** | 防止文档伪造 |
| **L2/L3 门禁** | **v3.8.0 再加** | v3.7.0 不需要 ACID，L2 执行一致性在 v3.8.0 PR-850 后才有意义 |
| **v3.8.0 PR DAG** | **按序执行，不能并行** | Phase 0/1 是所有后续工作的基础 |
| **覆盖率统一** | **PR-900 第一优先级** | 49pp delta 导致测量结果不可信 |