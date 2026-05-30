# SQLRustGo v3.8.0 版本计划

> **版本**: v3.8.0
> **类型**: Architecture Unification Release
> **分支**: `origin/develop/v3.8.0` (to be created)
> **创建日期**: 2026-05-30
> **状态**: Planned
> **Auditor**: Hermes Agent

---

## 1. 版本目标

### 1.1 核心目标

v3.8.0 是**架构收敛版本**，不是 feature 版本：

1. **消灭双执行路径** — mysql-server 与 LocalExecutor 统一
2. **接入 WAL 核心** — DML 经过 TransactionManager + WAL
3. **统一 SQL → AST → Plan → Execution** — 单一路由
4. **为 MVCC 完整化奠基** — Snapshot/Rollback 架构就绪

### 1.2 与 v3.7.0 的关系

```
v3.7.0: "MySQL-compatible SQL execution engine with session-level transaction"
    ↓ (v3.8.0 架构收敛)
v3.8.0: "Single-path execution engine with WAL-integrated transaction semantics"
    ↓ (v3.9.0 ACID 完整化)
v3.9.0: "ACID database engine with full MVCC"
```

---

## 2. 阶段计划

| 阶段 | 目标时间 | PRs | 核心交付 |
|------|----------|-----|----------|
| Alpha | Week 1-3 | PR-800~840 | 执行入口 + 事务核心 |
| Beta | Week 4-6 | PR-850~880 | 执行路径统一 + VTU 接入 |
| RC | Week 7-9 | PR-890~900 | MVCC + 收尾重构 |
| GA | Week 10 | All | 全量验证 + 发布 |

---

## 3. PR 里程碑映射

| Milestone | PR | 描述 | Gate |
|-----------|---|------|------|
| v3.8.0-Alpha | PR-800 | COM_QUERY AST Routing | A1-A3 |
| v3.8.0-Alpha | PR-810 | ExecutionEngine → Router | A4 |
| v3.8.0-Alpha | PR-820 | TransactionManager Session Binding | B1 |
| v3.8.0-Alpha | PR-830 | WAL + WriteBuffer 接入 | B2-B4 |
| v3.8.0-Alpha | PR-840 | DML Transaction Interception | B5 |
| v3.8.0-Beta | PR-850 | mysql-server → LocalExecutor 统一 | C1-C5 |
| v3.8.0-Beta | PR-860 | Planner Layer Consolidation | C2 |
| v3.8.0-Beta | PR-870 | ParallelVolcanoExecutor 接入 | D1-D3 |
| v3.8.0-Beta | PR-880 | VTU Predicate/Mutation Pipeline | D4 |
| v3.8.0-RC | PR-890 | Snapshot + MVCC + Rollback | E1-E4 |
| v3.8.0-RC | PR-900 | ExecutionEngine 拆分清理 | F1-F5 |

---

## 4. 依赖关系（关键）

```
PR-800 → PR-810 → PR-820 → PR-830 → PR-840
                              ↓
                      PR-850 → PR-860
                              ↓
                      PR-870 → PR-880
                              ↓
                      PR-890
                              ↓
                      PR-900
```

**不允许并行合并的 PR 对**：
- PR-840 必须在 PR-850 之前
- PR-850 必须在 PR-870 之前
- PR-890 必须在 PR-900 之前

---

## 5. 版本门禁时间线

| 日期 | 门禁 | 目标 |
|------|------|------|
| Week 1 | Alpha Gate A1-A3 | AST routing + 无 raw SQL execution |
| Week 3 | Beta Entry B1-B5 | WAL + DML interception + transaction core |
| Week 6 | Beta Gate C1-D4 | Execution path unified + VTU接入 |
| Week 9 | RC Gate E1-F5 | MVCC + execution_engine 拆分 |
| Week 10 | GA Gate | Full suite PASS |

---

## 6. Issue 映射

| Issue | 标题 | 映射到 PR | 状态 |
|-------|------|-----------|------|
| #2588 | INT-1: DML → WAL | PR-830, PR-840 | Planned |
| #2589 | INT-2: ParallelVE | PR-870 | Planned |
| #2590 | INT-3: expr crate | PR-860 | Planned |
| #2591 | INT-4: mysql-server | PR-850 | Planned |
| #2597 | execution_engine.rs 膨胀 | PR-900 | Planned |
| #2596 | 覆盖率测量差异 | PR-900 | Planned |
| #2600 | Coverage 测量统一 | PR-900 | Planned |
| #2603 | R2: 执行引擎统一 | PR-850 | Planned |
| #2604 | R3: expr crate | PR-860 | Planned |
| #2605 | R4: mysql-server | PR-850 | Planned |
| #2606 | R5: Gate 重构 | 新门禁脚本 | Planned |

---

## 7. 成功标准

v3.8.0 GA 发布时，系统必须满足：

| 标准 | 验证方法 |
|------|----------|
| 单执行路径 | `grep storage.insert` 仅在 storage layer |
| WAL 集成 | `SELECT * FROM WAL log` 包含 DML ops |
| ACID 语义 | T-ISO-01~05 ALL PASS |
| Crash recovery | T-CRA-01~05 ALL PASS |
| execution_engine.rs | < 1500 lines |
| 覆盖率一致性 | Z6G4 vs Z440 delta < 10pp |
| TPC-H | 22/22 PASS |

---

## 8. 版本间映射（v3.7.0 → v3.8.0）

| v3.7.0 排除项 | v3.8.0 处理 |
|--------------|-----------|
| WAL / crash recovery | INT-1: PR-830/840 |
| ParallelVolcanoExecutor 未接入 | INT-2: PR-870 |
| expr crate 孤岛 | INT-3: PR-860 |
| mysql-server 双路径 | INT-4: PR-850 |
| execution_engine.rs 膨胀 | PR-900 |
| 覆盖率差异 | PR-900 |
| SHOW TABLES | v3.7.x (P1) |
| 空密码 auth | v3.7.x (P1) |