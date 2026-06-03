# v3.8.0 审核文档清单

> 整理时间：2026-06-04
> 用途：设计评审、测试验收审核
> 基于 commit: `3b337c97f` (docs: organize v3.8.0 release docs)

---

## 一、需要审核的设计文档（design/）

### PR-850 — mysql-server 与 LocalExecutor 统一
- **文件**: `design/PR-850_DESIGN.md`
- **关联测试**: `test-design/PR-850_TEST_DESIGN.md`
- **关联验收**: `test-acceptance/PR-850_TEST_ACCEPTANCE.md`（无此文件，实际无独立验收）
- **状态**: 设计阶段
- **审核要点**: 双路径统一范围、AST Routing 路由逻辑、向后兼容性

### PR-870 — VTU Merge，MergeExecutor 接入
- **文件**: `design/PR-870_DESIGN.md`
- **关联测试**: `test-design/PR-870_TEST_DESIGN.md`
- **关联验收**: 无
- **状态**: 设计阶段
- **审核要点**: MergeExecutor 与 LocalExecutor 集成点、语义等价性

### PR-830E — WAL Recovery Lifecycle（启动恢复）
- **文件**: `design/PR-830E_SPEC.md`
- **关联合约**: `design/PR-830E_CONTRACT.md`
- **关联实现计划**: `plans/PR-830E_IMPLEMENTATION_PLAN.md`
- **关联测试**: `test-design/RECOVERY_TEST_DESIGN.md`
- **状态**: WAL 链已合并（PR-830A~E），RECOVERY 测试 22/22 PASS
- **审核要点**: RECOVERY 测试 22/22 覆盖是否充分；RTI Chain 是否完整

### PR-830F — WAL Lifecycle Controller（Checkpoint + Truncation）
- **文件**: `design/PR-830F_SPEC.md`
- **关联合约**: `design/PR-830F_CONTRACT.md`
- **关联实现计划**: `plans/PR-830F_IMPLEMENTATION_PLAN.md`
- **状态**: WAL 链已合并，Checkpoint + Truncation 已实现
- **审核要点**: safe_truncate_lsn 安全性、WAL 磁盘空间回收机制

### PR-840 — 门禁重构（合约存在，但无设计文档）
- **文件**: `design/PR-840_CONTRACT.md`
- **关联验收**: `test-acceptance/PR-840_TEST_ACCEPTANCE.md`
- **审核要点**: 门禁规则是否覆盖所有风险点

### PR-800 — COM_QUERY AST Routing
- **文件**: `design/PR-800_SPEC.md`
- **关联测试**: `test-design/PR-800_TEST_PLAN.md`
- **关联验收**: `test-acceptance/PR-800_ACCEPTANCE.md`
- **审核要点**: 路由完整性、错误处理

### PR-800F — Transactional Facade
- **文件**: `design/PR-800F_TRANSACTIONAL_FACADE_SPEC.md`
- **关联测试**: `test-design/PR-800F_TRANSACTIONAL_FACADE_TEST_DESIGN.md`, `PR-800F_TRANSACTIONAL_FACADE_TEST_PLAN.md`
- **关联验收**: 无
- **审核要点**: 门面语义、并发安全

---

## 二、需要审核的测试文档（test-design/）

### RECOVERY_TEST_DESIGN.md
- **路径**: `test-design/RECOVERY_TEST_DESIGN.md`
- **覆盖范围**: WAL Recovery 场景
- **测试结果**: 22/22 PASS（RECOVERY-007 已 re-enabled）
- **状态**: ✅ PASS
- **审核要点**: 是否覆盖所有故障注入场景（crash、corruption、partial write）

### PR-850_TEST_DESIGN.md
- **路径**: `test-design/PR-850_TEST_DESIGN.md`
- **状态**: 待审核
- **审核要点**: 双路径覆盖度、边界条件

### PR-870_TEST_DESIGN.md
- **路径**: `test-design/PR-870_TEST_DESIGN.md`
- **状态**: 待审核
- **审核要点**: Merge 语句覆盖度（INSERT/UPDATE/DELETE）

### PR-880F_VTU_PIPELINE_TEST_DESIGN.md
- **路径**: `test-design/PR-880F_VTU_PIPELINE_TEST_DESIGN.md`
- **关联验收**: `test-acceptance/PR-880F_TEST_ACCEPTANCE.md`
- **状态**: 待审核
- **审核要点**: VTU Pipeline 完整性、数据一致性

### PR-800F_TRANSACTIONAL_FACADE_TEST_DESIGN.md
- **路径**: `test-design/PR-800F_TRANSACTIONAL_FACADE_TEST_DESIGN.md`
- **状态**: 待审核
- **审核要点**: 并发测试覆盖、死锁场景

### TEST_PLAN.md / TEST_PLAN_INTEGRATED.md
- **路径**: `test-design/TEST_PLAN.md`, `test-design/TEST_PLAN_INTEGRATED.md`
- **状态**: 综合测试计划
- **审核要点**: 与 GA 门禁的覆盖度差距

### TEST_REVIEW.md / TEST_REVIEW_INTEGRATED.md
- **路径**: `test-design/TEST_REVIEW.md`, `test-design/TEST_REVIEW_INTEGRATED.md`
- **状态**: 综合测试评审
- **审核要点**: 发现的问题和遗留风险

---

## 三、测试验收文档（test-acceptance/）

| 文件 | 对应 PR | 状态 |
|------|---------|------|
| `PR-800_ACCEPTANCE.md` | PR-800 | 待审核 |
| `PR-840_TEST_ACCEPTANCE.md` | PR-840 | 待审核 |
| `PR-880F_TEST_ACCEPTANCE.md` | PR-880F | 待审核 |
| `PR-900F_TEST_ACCEPTANCE.md` | PR-900F | 待审核 |
| `TEST_ACCEPTANCE_INTEGRATED.md` | 综合 | 待审核 |
| `TEST_ACCEPTANCE_SUMMARY.md` | 综合 | 待审核 |

---

## 四、门禁报告（直接证明）

| 阶段 | 文件 | 结果 |
|------|------|------|
| Alpha | `alpha/ALPHA_GATE_REPORT.md` | 10/10 PASS（A1-A5 + A6-1~5）|
| Beta | `beta/BETA_GATE_REPORT.md` | 11/11 PASS（B1-B4 + B-F1~B-F7）|
| Integration | `rc/COMPREHENSIVE_GATE_REPORT.md` | 4/4 sections PASS |
| Alpha+RC/GA | `rc/RC_GA_GATE_REPORT.md` | D1 10/10, D2 5/5, D3 5/5, D4 5/5, D5 9/10 ✅ PASS |
| RECOVERY | `test-design/RECOVERY_TEST_DESIGN.md` | 22/22 PASS（RECOVERY-001~008）|

---

## 五、债务 SPEC（实现完整性对账）

| ID | 文件 | 实现状态 |
|----|------|---------|
| F-16 | `specs/debt/F16_GAP_LOCKING_SPEC.md` | 需对照 INT5_PLUS_DEBT_INVENTORY |
| F-23 | `specs/debt/F23_CLUSTERED_INDEX_SPEC.md` | 同上 |
| F-24 | `specs/debt/F24_AHI_SPEC.md` | 同上 |
| F-25/F-26 | `specs/debt/F25_F26_STORAGE_BUFFERS_SPEC.md` | 同上 |
| F-27 | `specs/debt/F27_COMPRESSION_SPEC.md` | 同上 |
| F-29 | `specs/debt/F29_RLS_SPEC.md` | 同上 |
| F-31 | `specs/debt/F31_PERFORMANCE_SCHEMA_SPEC.md` | 同上 |
| F-32 | `specs/debt/F32_MYSQLADMIN_SPEC.md` | 同上 |
| F-35 | `specs/debt/F35_PASSWORD_ROTATION_SPEC.md` | 同上 |
| I-12 | `specs/debt/I12_PARALLEL_EXECUTOR_SPEC.md` | 同上 |
| T-15 | `specs/debt/T15_DEADLOCK_INJECTION_SPEC.md` | 同上 |
| T-17/18 | `specs/debt/T17_T18_FAULT_INJECTION_SPEC.md` | 同上 |

债务实现状态以 `debt/INT5_PLUS_DEBT_INVENTORY.md` 为准。

---

## 六、审核流程建议

### 设计文档审核
1. 对照 `alpha/ALPHA_GATE_CONTRACT.md` 的 A1-A6 门禁项
2. 检查设计文档是否覆盖 PR 的全部 scope
3. 检查 `design/PR-xxx_CONTRACT.md` 与 `design/PR-xxx_SPEC.md` 的一致性
4. 标记不一致项到 `debt/INT5_PLUS_DEBT_INVENTORY.md`

### 测试文档审核
1. 对照 `test-design/TEST_PLAN.md` 的覆盖矩阵
2. 检查 `RECOVERY_TEST_DESIGN.md` 22/22 是否覆盖所有故障场景
3. 检查 `PR-850_TEST_DESIGN.md` 和 `PR-870_TEST_DESIGN.md` 的边界条件
4. 确认 `test-acceptance/` 验收文件是否有对应 test-design/ 的签署

### 门禁结果审核
1. Alpha 10/10 PASS — 已有证据
2. Beta E2E 11/11 PASS — 已有证据
3. RECOVERY 22/22 PASS — 已有证据
4. RC/GA 门禁 — 需要 `rc/COMPREHENSIVE_GATE_REPORT.md` 和 `rc/RC_GA_GATE_REPORT.md` 确认
