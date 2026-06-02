# v3.8.0 Alpha Stage Review Report

> **评审日期**: 2026-06-03
> **评审依据**: DeepSeek Analyze Mode 评审意见 + 原始 Alpha 门禁执行记录
> **维护者**: Hermes C
> **状态**: DRAFT

---

## 1. 评审背景

v3.8.0 被定义为 **Architecture Unification Release**，核心目标是消灭双执行路径、统一 SQL → AST → Plan → Execution、接入 WAL 事务核心层。

Alpha 门禁于 2026-05-31 执行，commit `b61548eb`，结果为 **PASS (10/10)**。

本报告基于 DeepSeek 对 Alpha 阶段计划与门禁的独立评审，识别出以下关键问题。

---

## 2. 核心问题总结

### 2.1 架构冻结语义验收不足

| 问题 | 现状 | 期望 |
|------|------|------|
| 双路径残留 | `eng.execute(raw_sql)` 仍存在于测试代码及部分模块 | 双路径归零 |
| ExecutionEngine 行数 | 仍 >1500 行 | AD-001 要求 <1500 行 |
| PR-800 完整度 | 仅合并 DriftGate、TransactionContext 等基础设施 | COM_QUERY → Parser → Planner → LocalExecutor 全链路 |
| TransactionalFacade | 未实现 (DEFERRED) | 应至少存在 STUB |

**根因**: Alpha 门禁过度关注形式化指标（测试计数、覆盖率、文档存在性），而对架构冻结的核心语义验收不足。

### 2.2 测试层次执行缺失

| 测试层 | 规划目标 | Alpha 实际执行 | 缺失原因 |
|--------|----------|----------------|----------|
| Layer 1 - Unit | 核心 crate 单测 | ✅ 749 tests passed | — |
| Layer 2 - Execution Consistency | 三路径结果哈希一致 | ❌ 未执行 | 依赖 PR-850 (mysql-server 统一)，Alpha 阶段未完成 |
| Layer 3 - ACID Verification | 崩溃恢复、隔离级别 | ❌ 未执行 | 依赖 PR-830/840 完整 WAL 集成 |
| RECOVERY-007 | DELETE 恢复 | ❌ 仍 IGNORE | DELETE replay 未实现 |

### 2.3 PR DAG 与执行进度脱节

DEVELOPMENT_PLAN.md 定义了 10 个 PR 的串行依赖链，但 Alpha 通过时实际情况：

| PR | 规划状态 | 实际状态 |
|----|----------|----------|
| PR-800 (COM_QUERY AST Routing) | 入口重构 | ⚠️ 部分落地，TransactionalFacade DEFERRED |
| PR-810 ~ PR-890 | 未规划 | ❌ 均未合并 |
| PR-830A~E (WAL 链) | Alpha 目标 | ✅ 已合并 |

---

## 3. 门禁有效性评估

### 3.1 Alpha 门禁评分

| 维度 | 评分 (1-5) | 说明 |
|------|-------------|------|
| 功能设计合理性 | 4 | PR DAG 清晰，架构决策文档优秀 |
| 测试设计合理性 | 3.5 | 三层测试框架先进，但 Alpha 阶段未充分执行 |
| 验收全面性 | 2.5 | 过度依赖形式化指标，缺失架构冻结核心语义验收 |
| **综合** | **3.3** | 门禁形式完整但执行不足 |

### 3.2 关键缺失检查项

| 检查项 | 重要性 | 说明 |
|--------|--------|------|
| 双路径残留检查 | P0 | `grep -r "eng.execute" src/ --include="*.rs"` 应为 0 |
| 架构关键路径可达性 | P0 | COM_QUERY 必须能走到 LocalExecutor |
| AD-001 落地检查 | P1 | ExecutionEngine 行数 <1500 |
| 负面测试 | P1 | DriftGate 违规阻断验证 |
| Layer 2 执行一致性 | P2 | 三路径结果哈希验证 |

---

## 4. 改进建议

### 4.1 对功能设计的改进

1. **PR DAG 绑定门禁**: 每个 Gate 应运行 `scripts/gate/check_pr_dag.sh`，验证要求的 PR 是否已合并
2. **增加架构冻结检查脚本**: 自动验证 AD 决策是否落地
3. **明确"完成"定义**: 每个 PR 必须有测试用例和验收标准

### 4.2 对测试设计的改进

1. **补齐 Layer 2 测试依赖**: 至少验证已实现路径之间的一致性
2. **增加接口兼容性测试**: 使用 `cargo-semver-checks` 检查 breaking changes
3. **RECOVERY-007 提升为 P0**: DELETE 恢复是基础功能

### 4.3 对验收全面性的改进

1. **Alpha 门禁增加架构关键路径检查**: 例如 `grep -r "eng.execute"` 应为 0
2. **引入负面测试**: 至少一个 DriftGate 违规场景的集成测试
3. **覆盖率目标调整为"核心执行路径覆盖率"**: 而非全域平均值

---

## 5. 整改行动计划

| Issue | Action | Owner | Priority |
|-------|--------|-------|----------|
| Alpha 门禁过于宽松 | 增加 A7 架构冻结检查项 | Hermes C | P0 |
| 缺少架构冻结脚本 | 创建 `scripts/gate/check_architecture_freeze.sh` | Hermes C | P0 |
| PR DAG 未绑定门禁 | 在 Beta Gate Contract 中强制绑定 | Hermes C | P1 |
| RECOVERY-007 未完成 | 提升为 Beta 门禁必须项 | TBD | P1 |
| 缺少负面测试 | 在 ALPHA_GATE_CONTRACT.md 中记录为 TODO | Hermes C | P2 |

---

## 6. 结论

Alpha 门禁的"PASS"标志被过于宽松地赋予，导致团队误以为架构已冻结，而实际上大量核心功能（单路径、WAL 完整恢复、事务拦截）尚未完成。

**建议**: 在下一个版本中，重新设计 Alpha 门禁的"架构冻结"定义，要求必须通过至少一条端到端的 WAL 恢复路径测试和双路径残留为零的静态检查，方可称为 Alpha PASS。

---

## 7. 参考文档

- `DEVELOPMENT_PLAN.md` - PR DAG 定义
- `ALPHA_GATE_CONTRACT.md` - Alpha 门禁契约
- `ALPHA_GATE_REPORT.md` - Alpha 门禁执行报告
- `ARCHITECTURE_DECISIONS.md` - 架构决策记录
- `FEATURE_CHECKLIST.md` - 功能清单及状态
