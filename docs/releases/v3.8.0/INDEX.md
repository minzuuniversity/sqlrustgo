# v3.8.0 文档索引

> 文档已按生命周期（alpha/beta/rc/ga）和功能模块（design/test-design/specs/debt）重组。
> 上次整理：2026-06-05

## 门禁报告

### Alpha 阶段
- [基线报告](alpha/ALPHA_BASELINE_REPORT.md)（首次运行，2 blockers）
- [门禁契约](alpha/ALPHA_GATE_CONTRACT.md)（A1-A6 判定规则）
- [最终报告](alpha/ALPHA_GATE_REPORT.md)（10/10 PASS）
- [阶段评审](alpha/ALPHA_STAGE_REVIEW.md)

### Beta 阶段
- [门禁契约](beta/BETA_GATE_CONTRACT.md)
- [Beta 报告](beta/BETA_GATE_REPORT.md)
- [Beta E2E PR-DAG 映射](beta/E2E_PR_DAG_MAPPING.md)
- [SGL Beta 报告](beta/SGL_BETA_GATE_REPORT.md)

### RC / GA 阶段
- [RC 门禁契约](rc/RC_GATE_CONTRACT.md)
- [综合门禁报告](rc/COMPREHENSIVE_GATE_REPORT.md)
- [RC+GA 报告](rc/RC_GA_GATE_REPORT.md)
- [集成门禁计划](rc/INTEGRATION_GATE_PLAN.md)
- [集成门禁报告](rc/INTEGRATION_GATE_REPORT.md)

### GA 清单
- [GA 放行清单](ga/GA_GATE_CHECKLIST.md)
- [GA 门禁报告](ga/GA_GATE_REPORT.md) (2026-06-05, ✅ PASS)

## 功能设计

### PR-800 系列
- [PR-800 SPEC](design/PR-800_SPEC.md)
- [PR-800F 事务门面 SPEC](design/PR-800F_TRANSACTIONAL_FACADE_SPEC.md)
- [PR-800 验收](test-acceptance/PR-800_ACCEPTANCE.md)

### PR-830E / PR-830F（WAL 重构）
- [PR-830E SPEC](design/PR-830E_SPEC.md)
- [PR-830E 合约](design/PR-830E_CONTRACT.md)
- [PR-830E 实现计划](plans/PR-830E_IMPLEMENTATION_PLAN.md)
- [PR-830F SPEC](design/PR-830F_SPEC.md)
- [PR-830F 合约](design/PR-830F_CONTRACT.md)
- [PR-830F 实现计划](plans/PR-830F_IMPLEMENTATION_PLAN.md)

### PR-850 / PR-870
- [PR-850 设计](design/PR-850_DESIGN.md)
- [PR-870 设计](design/PR-870_DESIGN.md)

### PR-840
- [PR-840 合约](design/PR-840_CONTRACT.md)
- [PR-840 测试验收](test-acceptance/PR-840_TEST_ACCEPTANCE.md)

## 测试设计
- [测试计划](test-design/TEST_PLAN.md)
- [测试计划（集成版）](test-design/TEST_PLAN_INTEGRATED.md)
- [测试评审](test-design/TEST_REVIEW.md)
- [测试评审（集成版）](test-design/TEST_REVIEW_INTEGRATED.md)
- [PR-800 测试计划](test-design/PR-800_TEST_PLAN.md)
- [PR-800F 测试设计](test-design/PR-800F_TRANSACTIONAL_FACADE_TEST_DESIGN.md)
- [PR-800F 测试计划](test-design/PR-800F_TRANSACTIONAL_FACADE_TEST_PLAN.md)
- [PR-850 测试设计](test-design/PR-850_TEST_DESIGN.md)
- [PR-870 测试设计](test-design/PR-870_TEST_DESIGN.md)
- [RECOVERY 测试设计](test-design/RECOVERY_TEST_DESIGN.md)
- [VTU Pipeline 测试设计](test-design/PR-880F_VTU_PIPELINE_TEST_DESIGN.md)

## 测试验收
- [验收汇总](test-acceptance/TEST_ACCEPTANCE_SUMMARY.md)
- [验收汇总（集成版）](test-acceptance/TEST_ACCEPTANCE_INTEGRATED.md)
- [PR-880F 测试验收](test-acceptance/PR-880F_TEST_ACCEPTANCE.md)
- [PR-900F 测试验收](test-acceptance/PR-900F_TEST_ACCEPTANCE.md)

## 技术 SPEC

### 债务修复 SPEC（specs/debt/）
| ID | 文件 | 状态 |
|----|------|------|
| F-16 | [Gap Locking](specs/debt/F16_GAP_LOCKING_SPEC.md) | |
| F-23 | [Clustered Index](specs/debt/F23_CLUSTERED_INDEX_SPEC.md) | |
| F-24 | [Adaptive Hash Index](specs/debt/F24_AHI_SPEC.md) | |
| F-25/F-26 | [Storage Buffers](specs/debt/F25_F26_STORAGE_BUFFERS_SPEC.md) | |
| F-27 | [Compression](specs/debt/F27_COMPRESSION_SPEC.md) | |
| F-29 | [Row Level Security](specs/debt/F29_RLS_SPEC.md) | |
| F-31 | [Performance Schema](specs/debt/F31_PERFORMANCE_SCHEMA_SPEC.md) | |
| F-32 | [mysqladmin](specs/debt/F32_MYSQLADMIN_SPEC.md) | |
| F-35 | [Password Rotation](specs/debt/F35_PASSWORD_ROTATION_SPEC.md) | |
| I-12 | [Parallel Executor](specs/debt/I12_PARALLEL_EXECUTOR_SPEC.md) | |
| T-15 | [Deadlock Injection](specs/debt/T15_DEADLOCK_INJECTION_SPEC.md) | |
| T-17/18 | [Fault Injection](specs/debt/T17_T18_FAULT_INJECTION_SPEC.md) | |

### 门禁工具 SPEC（specs/gate/）
- [SPEC-001 B4 Format](specs/gate/SPEC-001-b4-format-truthfulness.md)
- [SPEC-002 PR-830F Lifecycle](specs/gate/SPEC-002-pr830f-lifecycle.md)
- [SPEC-003 WAL Replay](specs/gate/SPEC-003-update-wal-replay.md)
- [SPEC-004 G-01 Validation Chain](specs/gate/SPEC-004-g01-validation-chain.md)
- [SPEC-005 Ignore Test Reasons](specs/gate/SPEC-005-ignore-test-reasons.md)
- [SPEC-006 Weak Assertions](specs/gate/SPEC-006-weak-assertions.md)
- [SPEC-007 Test Cleanup](specs/gate/SPEC-007-test-cleanup.md)
- [SPEC-008 Cross-Version Debt](specs/gate/SPEC-008-cross-version-debt.md)
- [SPEC-008 Pre-Existing Clippy](specs/gate/SPEC-008-pre-existing-clippy.md)
- [SPEC-009 Docs Consistency](specs/gate/SPEC-009-docs-consistency.md)
- [SPEC-010 Bash Compat](specs/gate/SPEC-010-bash-compat.md)
- [SPEC-011 MySQL Server Dual Path](specs/gate/SPEC-011-mysql-server-dual-path.md)
- [SPEC-012 Execution Engine Split](specs/gate/SPEC-012-execution-engine-split.md)
- [SPEC-013 v3.8.1 TX WAL Repair](specs/gate/SPEC-013-v380-1-tx-wal-repair.md)
- [SPEC-014 Arch Invariants](specs/gate/SPEC-014-arch-invariants.md)
- [SPEC-015 Evidence Binding](specs/gate/SPEC-015-evidence-binding.md)
- [SPEC-015 v3.6.0 P2 Deferred](specs/gate/SPEC-015-v360-p2-deferred.md)
- [SPEC-017 Post-Merge Evidence](specs/gate/SPEC-017-post-merge-evidence-cleanup.md)
- [SPEC-018 Post-Merge Evidence v2](specs/gate/SPEC-018-post-merge-evidence-cleanup-v2.md)
- [SPEC-019 Docs Template Automation](specs/gate/SPEC-019-docs-template-automation.md)
- [SPEC-020 env-blocker Integration](specs/gate/SPEC-020-env-blocker-integration.md)
- [SPEC-021 Gitea CI Integration](specs/gate/SPEC-021-gitea-ci-integration.md)
- [SPEC-022 Beta Stage Update](specs/gate/SPEC-022-beta-stage-update.md)
- [SPEC-023 Beta E2E Closure](specs/gate/SPEC-023-beta-e2e-closure.md)
- [SPEC-024 Beta Test Supplement](specs/gate/SPEC-024-beta-test-supplement.md)
- [SPEC-025 Deferred PR Status](specs/gate/SPEC-025-deferred-pr-status-update.md)
- [SPEC-026 RC Stage Launch](specs/gate/SPEC-026-rc-stage-launch.md)
- [P01 Test Inventory](specs/gate/P01_TEST_INVENTORY_SPEC.md)
- [P02 Cargo Test Paths](specs/gate/P02_CARGO_TOML_TEST_PATHS_SPEC.md)
- [P11 INT Debt](specs/gate/P11_INT_DEBT_SPEC.md)
- [P12 Arch Sem Debt](specs/gate/P12_ARCH_SEM_DEBT_SPEC.md)
- [P13 Test Plan](specs/gate/P13_TEST_PLAN_SPEC.md)
- [P24 R-Gate YAML](specs/gate/P24_R_GATE_YAML_SPEC.md)
- [P15 PR Template](specs/gate/P15_PR_TEMPLATE_SPEC.md)

## 债务追踪

- [债务总表](debt/INT5_PLUS_DEBT_INVENTORY.md)（68 项，主表，唯一事实来源）
- 债务修复 SPEC 详见 [specs/debt/](specs/debt/)

## 计划文档

- [版本计划](plans/VERSION_PLAN.md)
- [开发计划](plans/DEVELOPMENT_PLAN.md)
- [路线图](plans/ROADMAP.md)
- [后 GA 计划](plans/POST_GA_PLAN.md)
- [延期 PR 说明](plans/DEFERRED_PRS.md)
- [测试质量整治计划](plans/TEST_QUALITY_REMEDIATION_PLAN.md)
- [RECOVERY 测试迁移计划](plans/RECOVERY_TEST_MIGRATION_PLAN.md)

## 架构文档

- [架构总览](design/ARCHITECTURE.md)
- [架构决策](design/ARCHITECTURE_DECISIONS.md)
- [功能 DAG](design/COMPREHENSIVE_FEATURE_DAG.md)
- [功能追踪](design/COMPREHENSIVE_FEATURE_TRACKING.md)
- [功能检查清单](design/FEATURE_CHECKLIST.md)

## 治理

- [门禁有效性矩阵](governance/GATE_EFFECTIVENESS_MATRIX.md)
- [门禁执行提示词](governance/GATE_EXECUTION_PROMPT.md)
- [Agent 执行提示词](governance/AGENT_EXECUTION_PROMPT.md)
- [Graph vs Legacy 门禁对账](governance/GRAPH_VS_LEGACY_GATE_RECONCILIATION.md)
- [多版本治理 DAG](governance/MULTI_VERSION_GOVERNANCE_DAG.md)
- [多版本治理报告](governance/MULTI_VERSION_GOVERNANCE_REPORT.md)
- [R5 门禁改革](governance/R5-GATE-REFORM.md)
- [治理工程方法论](governance/GOVERNANCE_HARNESS_ENGINEERING.md)（.md / .docx / .pptx）
- [SPEC 模板](governance/SPEC-TEMPLATE.md)

## 历史审计

- [v3.0.0 测试覆盖率矩阵](historical/V300_DOC_TEST_COVERAGE_MATRIX.md)
- [v3.6.0 测试覆盖率矩阵](historical/V360_DOC_TEST_COVERAGE_MATRIX.md)
- [历史功能覆盖矩阵](historical/HISTORICAL_FEATURE_COVERAGE_MATRIX.md)
- [遗留问题清单](historical/LEGACY_ISSUES.md)
- [遗留修复验证报告](historical/LEGACY_FIXES_VERIFICATION_REPORT.md)
- [Issue 审计与差距分析](historical/ISSUE_AUDIT_AND_GAP_ANALYSIS.md)
- [覆盖率 Delta 分析](historical/COVERAGE-DELTA-ANALYSIS.md)
- [WAL Recovery RTI Chain](historical/INT1_WAL_RECOVERY_RTI_CHAIN.md)
- [INT2/3/4 RTI Chain](historical/INT234_RTI_CHAIN.md)
- [SGL-005 StorageBypass 审计](historical/SGL-005_STORAGEBYPASS_AUDIT.md)
- [F06 门面设计笔记](historical/F06_FACADE_DESIGN_NOTES.md)
- [F09 双写 Bug](historical/F09_DUAL_WRITE_BUG.md)
- [测试系统审计](historical/TEST_SYSTEM_AUDIT.md)

## 草案 / 过时内容（drafts/）

> 以下文档未纳入 v3.8.0 正式门禁，仅作历史参考。

- [ALPHA_DESIGN_TEST_GATE.md](drafts/ALPHA_DESIGN_TEST_GATE.md)（A7 设计门禁提案，未正式纳入）
- [ALPHA_CONDITIONAL_PASS.md](drafts/ALPHA_CONDITIONAL_PASS.md)（CONDITIONAL PASS 语义讨论，已过时）
- [ALPHA_GATE_PROMPT.md](drafts/ALPHA_GATE_PROMPT.md)（早期手工执行手册，已被 CI 替代）

## 完全归档（archived/）

> 已过期或已被完整替换的文档。

- [ARCH-900 死模块报告](archived/ARCH-900-DEAD-MODULE-REPORT.md)
- [架构语义债务整治计划](archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md)
- [INT 债务整治计划](archived/INT_DEBT_REMEDIATION_PLAN.md)
- [项目分析报告完整版](archived/项目分析报告_完整版.md)

---

## 目录结构一览

```
v3.8.0/
├── INDEX.md                          ← 本文件
├── README.md
├── REVIEW_CHECKLIST.md               审核文档清单
├── alpha/          ( 4)              Alpha 门禁
├── beta/           ( 4)              Beta 门禁
├── rc/             ( 5)              RC+GA 门禁
├── ga/             ( 1)              GA 放行清单
├── design/         (14)              功能设计
├── test-design/    (11)              测试方案
├── test-acceptance/( 6)              测试验收
├── specs/debt/     (12)              债务修复 SPEC
├── specs/gate/     (34)              门禁工具 SPEC
├── debt/           ( 1)              债务主表
├── plans/          ( 9)              计划文档
├── governance/     (11)              治理分析
├── historical/     (13)              历史审计
├── drafts/         ( 3)              草案
└── archived/       ( 5)              完全归档
```

## 维护规则

1. **SSOT**：债务状态以 `debt/INT5_PLUS_DEBT_INVENTORY.md` 为唯一事实来源，各 SPEC 只记录实现细节
2. **新增文档必须入子目录**，根目录除 INDEX.md、README.md、REVIEW_CHECKLIST.md 外不允许新增 .md
3. **每个文档头部标记 Status**：`ACTIVE` / `SUPERSEDED` / `COMPLETED` / `ARCHIVED`
4. **每次 PR 合并同步债务库存**：债务项状态变更必须同步更新 `debt/INT5_PLUS_DEBT_INVENTORY.md`
5. **定期清理 drafts**：每个版本 GA 后将 drafts/ 中未采用的提案移入 archived/ 或删除
