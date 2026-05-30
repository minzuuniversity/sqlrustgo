# SQLRustGo 治理与版本状态综合报告

> **版本**: v3.8.0
> **分支**: develop/v3.7.0
> **报告日期**: 2026-05-30
> **状态**: Phase A 启动前审计
> **Auditor**: Hermes Agent

---

## 一、当前仓库状态

| 分支 | 最新提交 | 状态 |
|------|----------|------|
| `main` | fc7f603f | v3.7.0 GA released |
| `origin/develop/v3.7.0` | 3e044c6f | **Phase A 启动点** |
| `origin/develop/v3.8.0` | 745f24f1 | 17 commits ahead of main |

---

## 二、ISSUE 审计 — Open Issues 全览

**总数: 27 个 Open Issues**

### 2.1 按分类

| 类别 | Issues | 核心问题 |
|------|--------|----------|
| **R系列 (v3.8.0 Roadmap)** | R1~R5 (#2603~#2606, #2601) | 执行引擎统一、Gate重构、mysql-server统一、expr收敛、Architecture Governance |
| **INTEGRATION-GAP** | 9个: #2570,#2571,#2572,#2588,#2589,#2590,#2591,#2596,#2597 | WAL/TransactionManager未集成、ParallelVolcanoExecutor孤岛、expr crate孤岛、execution_engine.rs膨胀(+46.9%)、覆盖率测量差异(Z6G4 81.97% vs Z440 32.59%) |
| **GOVERNANCE** | 3个: #2579,#2584,#2585 | gate_spec缺I-Gate集成路径检查(导致6版本集成缺失)、Alpha CONDITIONAL PASS语义不清、跨版本债务追踪缺失 |
| **SYSTEMIC** | 3个: #2576,#2577,#2578 | DML不经过WAL、ParallelVolcanoExecutor孤岛、execution_engine.rs五版本未重构 |
| **ARCH** | 2个: #2583,#2601 | DML执行路径未统一、Architecture Governance缺失 |
| **CRITICAL** | 1个: #2598 | 真实服务器测试缺失(v1.2.0~v3.7.0全版本) |
| **Feature/Other** | 8个: #2288,#2292,#2293,#2295,#2309,#2432,#2437,#2587 | WAL并发写入、检查点优化、复合索引、索引统计、ALTER TABLE、存储过程、时间戳+连接池、VTU Phase3 |

### 2.2 按版本归属（retargeted状态）

| 目标版本 | Issues | 说明 |
|----------|--------|------|
| **v3.8.0** | R1~R5 + #2570~2597大部分 | 全部集成工作延期至v3.8.0 |
| **v3.7.0（已冻结）** | 0 | 全部关闭 |
| **未指派** | 8个历史ISSUE | #2288,#2292,#2293,#2295,#2309,#2432,#2437,#2587 |

### 2.3 已关闭（v3.7.0相关，#2570~2621范围）

**29 个已关闭**，关键完成项：
- #2599 Core Integrity Release — 主路径统一
- #2602 R1 Transaction/WAL主路径重构 — DML经过事务
- #2607 R6 Recovery Integration Tests — 真实服务器测试
- #2609/2610/2611 VTU Phase 1-2 — VtuGuard + DML路由
- #2612 mysql-server From traits修复
- #2613 P0 fixes — session engine + SKIP_AUTH恢复
- #2615 Coverage Ceiling Analysis + Window Branch Forcing
- #2616/2617/2619 GA docs + freeze流程
- #2621 GA freeze → main

---

## 三、v3.7.0 vs v3.8.0 差距分析

### 3.1 v3.7.0 完成度矩阵

| 维度 | v3.7.0目标 | 实际 | 状态 |
|------|------------|------|------|
| 主路径统一 | DML经过事务 | ✅ P0-1 done | GA |
| Auth恢复 | SKIP_AUTH关闭 | ✅ P0-2 done | GA |
| VTU | Phase 1-2 | ✅ 2611完成 | GA |
| Governance | G-01~G-07 | ✅ GATE_SPEC done | GA |
| Documentation | GA freeze docs | ✅ 2616/17 done | GA |
| **集成债务** | INT-1~INT-4 | ❌ 全延期v3.8.0 | Deferred |
| **真实服务器测试** | R6 | ⚠️ 部分完成(#2607) | Partial |
| **execution_engine.rs** | 重构 | ❌ 4658→6829行 | Deferred |
| **ParallelExecutor** | 主流程集成 | ❌ 孤岛 | Deferred |

### 3.2 v3.8.0 待完成工作

**按 R1~R5 + INT-1~INT-4 分解：**

| Issue | 标题 | 优先级 | 估计规模 |
|-------|------|--------|----------|
| #2603 | R2: Parallel Executor主流程 | P0 | 大 |
| #2598 | 真实服务器测试 | P0 | 大 |
| #2597 | execution_engine.rs重构 | P1 | 很大 |
| #2576 | DML经过WAL | P1 | 中 |
| #2570 | ParallelVolcanoExecutor集成 | P1 | 中 |
| #2571 | WAL/MVCC/TransactionManager DML | P1 | 中 |
| #2572 | PhysicalPlan双执行路径 | P2 | 中 |
| #2577 | ParallelVolcanoExecutor孤岛 | P2 | 中 |
| #2588 | INT-1: DML不经过WAL | P1 | 中 |
| #2589 | INT-2: ParallelVolcanoExecutor | P1 | 中 |
| #2590 | INT-3: expr crate | P2 | 中 |
| #2591 | INT-4: mysql-server未集成 | P2 | 中 |
| #2596 | 覆盖率测量差异 | P1 | 小 |
| #2604 | R3: expr crate收敛 | P2 | 中 |
| #2605 | R4: mysql-server统一 | P2 | 中 |
| #2606 | R5: Gate重构 | P1 | 中 |
| #2579 | I-Gate集成路径检查缺失 | P1 | 小 |
| #2584 | Alpha CONDITIONAL PASS语义 | P3 | 小 |
| #2585 | 跨版本债务追踪 | P3 | 小 |
| #2601 | Architecture Governance | P2 | 中 |
| #2583 | DML执行路径未统一 | P2 | 中 |
| #2288 | WAL并发写入 W-01 | P2 | 小 |
| #2292 | WAL检查点优化 W-02 | P2 | 小 |
| #2293 | 复合索引 I-04 | P3 | 小 |
| #2295 | 索引统计 I-05 | P3 | 小 |
| #2432 | ALTER TABLE | P3 | 中 |
| #2437 | 存储过程 | P3 | 大 |
| #2309 | TIMESTAMP+连接池 | P3 | 小 |
| #2587 | WAL DML Phase3报告 | 文档 | 小 |

---

## 四、Phase A 治理验证 — 当前状态评估

### 4.1 Governance Framework 资产盘点（v3.7.0）

| 资产 | 文件 | 状态 |
|------|------|------|
| **Rule Spec** | GATE_SPEC.md (G-01~G-07) | ✅ 已建立 |
| **Templates** | TEMPLATE_GATE_REPORT, CLAIM, DECISION, OBS | ✅ 5个 |
| **Patterns** | PATTERN_ARCHITECTURE_DEBT, COVERAGE_DISPUTE, EVIDENCE_CHAIN | ✅ 3个 |
| **Playbooks** | PLAYBOOK_LEGACY_RETIREMENT | ✅ Phase A~E |
| **Replay** | REPLAY_v3.7.0_GA.md | ✅ 完成 |
| **Lessons** | LESSON_v3.7.0_GA.md | ✅ 3 lessons |
| **ADR** | ADR-001~ADR-005 | ✅ done |
| **Phase A CI** | ci-v370.yml (L1-Fmt + L2-Build + L3-Coverage + L7-Evidence) | ✅ 已推送 |
| **Neo4j Graph** | 9184 nodes | ✅ 已写入 |

### 4.2 当前 Phase A 的核心缺陷

**当前 ci-v370.yml 只验证了"正常路径"（A类），缺少以下实验级能力：**

| 缺陷 | 说明 | 影响 |
|------|------|------|
| **无 Fault Injection** | 没有故意制造G-03/G-06/G-02失败的测试 | 无法验证治理规则本身 |
| **无 Run-level Trace ID** | 每次CI没有GOV-RUN-ID | 无法追踪单次决策链 |
| **无 Gate Decision Snapshot** | CI只输出PASS/FAIL | 无法分析哪条规则触发/阻止 |
| **无 Evidence Hash Chain** | 没有sha256锚定证据 | 无法防篡改 |
| **无 Replay Ingest** | CI结束后没有Neo4j写入 | Knowledge Graph无法追踪实时CI |
| **无 3维实验矩阵** | 30次CI全是baseline | 无法统计FP/FN/Precision/Recall |

**Phase A 正确目标不是"30次重复CI PASS"，而是"验证治理规则是否工作"**

---

## 五、重构与功能集成 — v3.8.0 及以后

### 5.1 必须重构（按依赖关系排序）

```
Layer 0: StorageBackend重构（并行搜索）
    ↓
Layer 1: execution_engine.rs拆分（4658→1500行级别模块）
    ↓
Layer 2: DML主路径→WAL/TransactionManager
    ↓
Layer 3: ParallelVolcanoExecutor→主执行链路
    ↓
Layer 4: expr crate集成→主执行链路
    ↓
Layer 5: mysql-server统一入口
    ↓
Layer 6: PhysicalPlan→LocalExecutor统一
```

### 5.2 性能测试改进

| 当前问题 | 改进方案 |
|----------|----------|
| Z6G4 vs Z440 coverage delta 49pp | 统一coverage测量，禁止局部覆盖率 |
| 无真实服务器压测 | bench-cli + TPC-H 真实服务器测试 |
| execution_engine.rs无perf profile | 添加 flamegraph + cachegrind |
| ParallelExecutor性能未测 | 添加 parallel_scan benchmark |
| WAL write性能未测 | 添加并发写入压测 |

### 5.3 覆盖率测试改进

| 目标 | 策略 |
|------|------|
| 覆盖率真实可信 | tarpaulin workspace全量，排除test-fixture |
| 跨平台一致 | Z6G4 + Z440 双平台CI |
| 阈值锁定 | GATE_SPEC.md中FROZEN，不许降低 |
| 结构化覆盖 | per-crate coverage report + trend chart |

---

## 六、v3.7.0 门禁评估

### 6.1 v3.7.0 门禁现状

| 门禁 | v3.7.0实际结果 | 问题 |
|------|----------------|------|
| Alpha (L1+L2) | ✅ PASS (552 tests) | — |
| Beta (Integration) | ✅ PASS (28/28 E2E) | — |
| RC (TPC-H) | ⚠️ PARTIAL (CI env issue, local PASS) | CI环境问题 |
| Document (D1-D8) | ⚠️ 4 historical errors | 非阻塞性文档债务 |
| Evidence Graph | ✅ PASS (G-01/02/03 enforced) | — |

**结论：v3.7.0 门禁基本有效，不需要跳过。改进点：RC阶段CI环境问题需修复。**

### 6.2 v3.8.0 门禁改进建议

| 改进项 | 优先级 | 说明 |
|--------|--------|------|
| G-08: Real Server Test | P0 | 真实服务器E2E必须通过 |
| I-Gate: 集成路径检查 | P1 | 防止INT-1~INT-4再次发生 |
| G-09: Coverage Measurement Unification | P1 | Z6G4/Z440一致 |
| G-10: Evidence Hash Chain | P2 | 防篡改 |

---

## 七、Phase A Governance Validation 验证矩阵

### 7.1 验证Level

| Level | 内容 | 测试用例 |
|-------|------|----------|
| **L1: Gate Validation** | 验证G-01~G-07 | A1(无证据→BLOCK), A2(过期→FAIL), A3(PASS无输出→FAIL) |
| **L2: Replay Validation** | 验证Replay体系 | B1(ISSUE-2613复现), B2(ISSUE-2582复现) |
| **L3: Pattern Detection** | 验证Pattern Library | C1(coverage dispute), C2(architecture debt) |
| **L4: Knowledge Graph** | 验证图谱价值 | D1(Issue Trace Query), D2(Rule Impact Query) |
| **L5: Parallel Run** | 3维实验矩阵 | 10 baseline + 10 fault injection + 10 edge cases |

### 7.2 成功标准（量化）

| 指标 | 目标 | 当前 |
|------|------|------|
| CI次数 | ≥30 | 0 |
| False Positive | <5% | N/A |
| False Negative | 0 | N/A |
| Replay Success | 100% | ✅ |
| Knowledge Graph Trace | 100% | ✅ |
| G-01~G-07覆盖 | 100% | ✅ |
| Pattern Detection | ≥90% | ⚠️ 3/3 |
| Fault Injection Runs | ≥10 | ❌ |

---

## 八、3.8.0 及以后版本规划

### 8.1 v3.8.0 Phase 计划

| Phase | 目标 | 关键任务 |
|-------|------|----------|
| **Phase 0** | Governance Validation | Phase A Shadow CI（故障注入+replay+graph ingest） |
| **Phase 1** | Execution Engine重构 | execution_engine.rs拆分(4658→1500级) + ParallelExecutor集成 |
| **Phase 2** | DML主路径 | DML→WAL/TransactionManager + MVCC |
| **Phase 3** | Integration | expr crate + mysql-server统一 + PhysicalPlan |
| **Phase 4** | Performance | TPC-H + bench-cli + flamegraph |
| **GA** | — | 全部INT解决 + 覆盖率≥70% + 真实服务器测试PASS |

### 8.2 推荐的下一步行动

**立即执行：**

1. **升级 ci-v370.yml** — 增加实验级能力（GOV-RUN-ID + Gate Snapshot + Evidence Hash Chain + Neo4j Ingest）
2. **创建 FAULT_INJECTION_SUITE** — G-01~G-07 故障注入测试
3. **创建 PLAYBOOK_PHASE_A_GOVERNANCE_VALIDATION.md** — CI分类策略 + Metrics脚本 + Fail fast规则
4. **将R5(Gate重构 #2606)提前到Phase A** — Governance Gate本身是第一个被验证对象

**后续执行（分PR）：**

| PR | 内容 |
|----|------|
| PR-A1 | Governance Shadow Mode Runner（ci-v370.yml升级版） |
| PR-A2 | Synthetic Failure Test Suite（G-01~G-07故障注入） |
| PR-A3 | Replay Automation |
| PR-A4 | Governance Metrics Dashboard |

---

## 九、关键文件引用

- GATE_SPEC.md v3.8.0: `docs/governance/GATE_SPEC.md`
- Phase A CI: `.github/workflows/ci-v370.yml`
- Legacy Retirement Playbook: `docs/governance/playbooks/PLAYBOOK_LEGACY_RETIREMENT.md`
- Replay: `docs/governance/replay/REPLAY_v3.7.0_GA.md`
- Lessons: `docs/governance/lessons/LESSON_v3.7.0_GA.md`
- Integration Debt Report: `docs/governance/DEBT_TRACKING.md`
- Patterns: `docs/governance/patterns/`
- Templates: `docs/governance/templates/`
- ADR: `docs/governance/adr/`