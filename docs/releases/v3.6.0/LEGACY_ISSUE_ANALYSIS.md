# SQLRustGo v3.6.0 严重遗留问题分析报告

**报告日期**: 2026-05-30
**分支**: develop/v3.6.0 (commit 7f7620850)
**作者**: hermes-agent

---

## 1. 问题是否应在 v3.6.0 彻底解决？

### 1.1 Alpha 延续问题（3项）

| 问题 | 来源版本 | 当前值 | 目标值 | 是否应在本版本解决 |
|------|----------|--------|--------|-------------------|
| Parser 覆盖率 | v3.5.0 | 47.16% | ≥75% (Beta), ≥85% (GA) | **否** — 结构性缺陷，需要 2+ 版本 |
| Executor 覆盖率 | v3.5.0 | 72.04% | ≥75% (Beta), ≥85% (GA) | **部分** — 差距小，可修复 |
| mysql-server tests2 | v3.5.0 | 30+ 编译错误 | 0 | **否** — 遗留债务，可延迟 |

**结论**: Parser 覆盖率 47% 是结构性缺陷（嵌套测试问题），在 v3.6.0 内无法达标 75%。Executor 覆盖率 72% 差距 3%，可修复但需时间投入。

### 1.2 Beta Gate 未完成项（8项）

| ID | Check | 当前状态 | 通过条件 |
|----|-------|----------|----------|
| B1 | Build (release) | ❌ PENDING | cargo build --release --workspace |
| B2 | Workspace test | ❌ PENDING | cargo test --workspace ≥90% PASS |
| B3 | Clippy zero | ❌ PENDING | cargo clippy --all-features |
| B4 | Format | ❌ PENDING | cargo fmt --all -- --check |
| B5 | Coverage | ❌ PENDING | L1 avg ≥85% (当前 81.97%) |
| B6 | TPC-H SF=1 | ❌ PENDING | 22/22 PASS |
| B7 | Security | ❌ PENDING | cargo audit |
| B8 | SQL compat | ❌ PENDING | SQL Corpus ≥85% |

**结论**: Beta Gate 0/8 通过。v3.6.0 当前处于 **Alpha CONDITIONAL PASS** 状态，距离 Beta 还有 8 项门禁，距离 GA 还有 12+ 项门禁。

---

## 2. 架构是否需要重新重构？

### 2.1 ExecutionEngine 单体架构分析

**数据**:
- `execution_engine.rs`: 2577 行（v3.6.0），曾是 6829 行（v3.5.0）
- 存在 `crates/executor/src/local_executor.rs`: 2152 行
- 存在 `crates/executor/src/parallel_executor.rs`: 1762 行

**问题**:
1. **双轨执行器** — `execution_engine.rs` 和 `local_executor.rs` 并存，前者直接调用 storage.insert/delete，后者基于 PhysicalPlan trait
2. **职责不清** — ExecutionEngine 同时负责：语句分发、查询执行、JOIN 算法、触发器、事务管理、CBO 代价估算、查询缓存
3. **违反单一职责** — 单个文件 2577 行，横跨 P0-P2 所有功能
4. **DML 直接调用 storage** — execute_insert 直接调用 `storage.insert()`，绕过了 PhysicalPlan 流水线

### 2.2 架构问题根源

```
当前架构（双轨）:
ExecutionEngine (2577行)
  └── 直接调用 storage.insert/delete/update
  └── execute_select → 复杂 join/聚合逻辑
       └── 调用 MergeExecutor (740行)

LocalExecutor (2152行, PhysicalPlan-based)
  └── ParallelExecutor (1762行)
       └── TaskScheduler
       └── VectorBatch
```

**问题**: DML (INSERT/UPDATE/DELETE) 走 ExecutionEngine 直接 storage 调用，SELECT 可走 LocalExecutor/ParallelExecutor 双轨。没有统一执行路径。

### 2.3 是否需要重构？

**结论: 不需要全面重构，但需要清理双轨，执行路径统一。**

理由:
1. PhysicalPlan trait 已存在，ParallelExecutor/LocalExecutor 基础设施已就绪
2. 问题不是架构设计错误，而是 **DML 执行没有接入 PhysicalPlan 流水线**
3. WAL 已生产集成，SIMD 已集成，核心基础设施无根本性缺陷
4. 建议: DML 迁移到 LocalExecutor/PhysicalPlan 路径，统一执行引擎

---

## 3. 治理体系是否可信？如何整改？

### 3.1 当前治理体系可信度评估

| 维度 | 评分 | 问题 |
|------|------|------|
| 文档完整性 | 6/10 | Alpha/Beta/GA 入口文档齐全，但存在文档与实现不一致 |
| 门禁执行 | 3/10 | Alpha CONDITIONAL PASS（未达标准即放行），Beta 未执行 |
| 覆盖率测量 | 5/10 | L1 8 crates 平均算法正确，但 Parser 结构性缺陷导致数值失真 |
| 版本计划 | 5/10 | 有 DEVELOCPMENT_PLAN，但时间线和里程碑不现实 |
| 问题追踪 | 2/10 | 延续问题未创建 Issue，跨版本债务无追踪机制 |

### 3.2 治理体系核心问题

**问题 1: Alpha 门禁形同虚设**
- Alpha 标准: 覆盖率 ≥75%
- 实际: 81.97%，但 Parser 47%、Executor 72% 均未达标
- 评估: "CONDITIONAL PASS" 语义模糊 — 什么叫 CONDITIONAL？条件是什么？

**问题 2: 文档与实现不一致**
- v3.6.0 Alpha Gate 报告声称 "cargo test --lib --workspace" PASS
- 但实际 BETA_GATE_CHECKLIST 显示 B1-B8 全部 PENDING
- 说明: Alpha 通过后没有真正触发 Beta 入口检查流程

**问题 3: 延续问题无 Issue 追踪**
- Parser 覆盖率 (47%) 从 v3.5.0 延续到 v3.6.0，无独立 Issue
- mysql-server 编译错误从 v2.5.0 延续至今，无 Issue
- 版本之间的债务没有闭环追踪

**问题 4: 门禁标准被随意调整**
- 用户记忆: "Alpha≥75% 实际 50%，Beta≥85% 实际 75%"
- Gate Contract v3.6.0 记录: Alpha A5 ≥75%, Beta B4 ≥85%, GA ≥85%
- **教训**: 曾凭记忆写错阈值，被严厉纠正

### 3.3 整改方案

```
治理体系整改:
1. 门禁执行流程
   - Alpha CONDITIONAL PASS 必须明确标注"条件是什么"
   - Beta 入口必须逐项验证，不能只检查文档存在
   - 建立 Gate 执行日志，每次 gate run 的实际输出必须存档

2. 问题追踪
   - 所有跨版本延续问题必须在当前版本创建 Issue
   - Issue 标题格式: [debt:<来源版本>] <问题描述>
   - Example: "[debt:v3.5.0] Parser coverage 47% structural deficiency"

3. 覆盖率测量
   - Parser 结构性缺陷（嵌套测试）需要独立 Issue + 专项修复计划
   - 不能通过提升其他 crate 覆盖率来弥补 Parser 缺陷

4. 版本计划现实性
   - v3.6.0 计划 Beta 2026-06-15，GA 2026-06-30
   - 实际上 Beta 0/8 通过，不可能在 16 天内完成
   - 建议修正时间线，或明确标注"风险"
```

---

## 4. 完整治理和版本设计-开发计划-测试-门禁检查体系的改进计划

### 4.1 架构清理计划

```
阶段 1: 执行路径统一（DML 接入 PhysicalPlan）
  - Issue: DML execution path unification
  - 将 INSERT/UPDATE/DELETE 从 execution_engine.rs 迁移到 LocalExecutor/PhysicalPlan
  - 预计: 2 周

阶段 2: 双轨合并
  - 评估 execution_engine.rs 是否可以降级为 thin wrapper
  - LocalExecutor 作为唯一执行入口
  - 预计: 2 周

阶段 3: 长期 — 架构解耦
  - ExecuteEngine 拆分为 StatementExecutor / QueryExecutor / DMLExecutor / TransactionCoordinator
  - 预计: v3.7.0 或 v4.0.0
```

### 4.2 Parser 覆盖率专项修复计划

```
根本原因: 嵌套测试导致子测试覆盖率无法计入
修复策略:
1. 创建专项 Issue + Issue 标签: [debt:structural]
2. 逐模块分析覆盖率缺口
3. 将嵌套测试重构为扁平化测试结构
4. 验证覆盖率从 47% 提升到 ≥75%

预计时间: 3-4 周（需要专项投入）
```

### 4.3 测试系统重建计划

```
阶段 1: mysql-server 编译错误修复
  - 当前: 30+ 编译错误
  - 策略: 基于 v3.5.0 修复版本重写测试
  - 预计: 2 周

阶段 2: Beta 测试轨
  - B2: cargo test --workspace ≥90% PASS
  - B6: TPC-H SF=1 22/22 PASS
  - B8: SQL Corpus ≥85%
  - 预计: 3 周

阶段 3: GA 测试轨
  - R2: TPC-H SF=1 22/22 PASS
  - R4: QPS regression ≤5%
  - 预计: 2 周
```

### 4.4 门禁体系改进计划

```
改进 1: 门禁执行日志
  - 每次 gate run 必须保存 stdout/stderr 日志
  - 文件命名: gate_<阶段>_<commit>_<timestamp>.log
  - 目录: docs/releases/v<版本>/logs/

改进 2: CONDITIONAL PASS 语义明确化
  - 创建 GATE_CONDITIONS.md 文件
  - 明确列出 CONDITIONAL 的具体条件
  - 每一项条件都必须有 Issue 追踪

改进 3: Beta 入口验证流程
  - Beta 入口检查必须逐项验证
  - 不能只检查文档存在
  - 必须运行实际命令并验证输出

改进 4: 跨版本债务追踪
  - 所有延续问题创建 Issue
  - Issue 包含: 来源版本、当前状态、目标、预计修复版本
  - 季度审查跨版本债务健康度
```

---

## 5. 整改时间线估计

### 5.1 现实时间线

| 阶段 | 目标时间 | 关键路径 | 风险 |
|------|----------|----------|------|
| v3.6.0 Beta | 2026-06-20 | Executor 覆盖率 72%→75%, B1-B4 通过 | 中 — 可达 |
| v3.6.0 GA | 2026-07-15 | TPC-H SF=1, SQL Corpus ≥85%, Coverage ≥85% | 高 — Parser 是瓶颈 |
| v3.7.0 Alpha | 2026-08-01 | Parser 覆盖率修复, DML 统一执行路径 | 高 — Parser 结构性缺陷 |
| v3.7.0 GA | 2026-09-30 | 架构清理完成, 所有门禁通过 | 中 |

### 5.2 资源需求

```
人力资源:
- 1 FTE: Parser 覆盖率专项修复 (3-4 周)
- 1 FTE: Beta/GA 测试轨 (4-6 周)
- 0.5 FTE: 架构清理 (并行，4 周)

基础设施:
- Z6G4: 主要编译/测试节点
- Mac Mini: CI 调度
- 预计编译次数: 20-30 次
```

### 5.3 当前版本时间线风险

| 原计划 | 现实评估 | 偏差 |
|--------|----------|------|
| Beta: 2026-06-15 | 2026-06-20 | +5 天 |
| GA: 2026-06-30 | 2026-07-15 | +15 天 |

**原因**: Parser 结构性缺陷和 mysql-server 编译错误需要专项投入。

---

## 6. 严重遗留问题汇总

### 6.1 Issue 清单

| Issue | 标题 | 级别 | 来源版本 | 当前状态 | 建议行动 |
|-------|------|------|----------|----------|----------|
| I#DEBT-v3.5.0-001 | Parser coverage structural deficiency (47%) | 🔴 CRITICAL | v3.5.0 | 未解决 | v3.7.0 专项修复 |
| I#DEBT-v2.5.0-001 | mysql-server 30+ compile errors | 🔴 CRITICAL | v2.5.0 | 未解决 | v3.6.0 Beta 前修复 |
| I#DEBT-v3.5.0-002 | Executor coverage below GA threshold (72%) | 🟡 MAJOR | v3.5.0 | 未解决 | v3.6.0 内修复 |
| I#ARCH-001 | DML execution path not unified | 🟡 MAJOR | v3.0.0 | 未解决 | v3.7.0 架构清理 |
| I#GOV-001 | Alpha CONDITIONAL PASS semantics unclear | 🟡 MAJOR | v3.6.0 | 未解决 | 治理改进 |
| I#GOV-002 | Cross-version debt tracking missing | 🟡 MAJOR | v3.5.0 | 未解决 | 治理改进 |

### 6.2 问题优先级矩阵

```
                    低影响        高影响
          ┌──────────┬──────────┐
          │          │ I#DEBT   │
低紧迫性  │          │ -v3.5.0  │
          │          │ -001    │
          ├──────────┼──────────┤
          │ I#GOV-   │ I#DEBT   │
高紧迫性  │ 002      │ -v2.5.0 │
          │          │ -001    │
          └──────────┴──────────┘
```

### 6.3 核心结论

1. **v3.6.0 无法在当前版本彻底解决所有问题** — Parser 结构性缺陷需要跨版本专项修复
2. **架构不需要全面重构** — PhysicalPlan 基础设施已就绪，需要统一执行路径（DML 接入流水线）
3. **治理体系存在严重可信度问题** — CONDITIONAL PASS 语义模糊，延续问题无追踪，门禁执行流于形式
4. **整改需要 3-4 个月** — v3.6.0 GA (07-15) → v3.7.0 GA (09-30)
5. **立即行动项**: 
   - 创建跨版本债务追踪 Issue（6 项）
   - 修复 mysql-server 编译错误（2 周）
   - 提升 Executor 覆盖率到 75%（1 周）
   - Parser 专项修复计划（3-4 周，v3.7.0）

---

## 附录

### A. v3.6.0 当前门禁状态

| 阶段 | 状态 | 通过项 | 总计 |
|------|------|--------|------|
| Alpha | CONDITIONAL PASS | A1-A5 (Coverage 81.97%) | 5/5 |
| Beta | PENDING | 0/8 | 8/8 |
| GA | PENDING | 0/12 | 12/12 |

### B. 参考文档

- Gate Contract: `docs/releases/v3.6.0/GATE_CONTRACT_v3.6.0.md`
- Alpha Report: `docs/releases/v3.6.0/ALPHA_GATE_REPORT_v3.6.0.md`
- Beta Checklist: `docs/releases/v3.6.0/BETA_GATE_CHECKLIST.md`
- Development Plan: `docs/releases/v3.6.0/DEVELOPMENT_PLAN.md`

---

**报告生成时间**: 2026-05-30
**下次审查**: 2026-06-05 (v3.6.0 Beta 进展检查)