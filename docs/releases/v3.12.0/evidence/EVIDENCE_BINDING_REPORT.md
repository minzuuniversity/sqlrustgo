# vv3.12.0 证据绑定检查报告

> **检查日期**: 2026-08-11
> **版本**: v3.12.0
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | 144 |
| 警告 | 1 |
| 失败（违规） | 0 |
| 未验证声明 | 0 |

**总结**: ✅ 无违规发现

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

✅ 无 Type A/B 违规

---

### 警告项（需要人工复核）

- ⚠️ v312-11-round14-status.md: Type C 警告: evidence_hash 与当前文件 SHA256 不匹配，且无可验证 commit/log 上下文: v312-11-round14-status.md (声明: 91d4b971af98a494..., 实际: da1a04376c5b7758...)

---

### 通过项

- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：GMP_COMPLIANCE_MATRIX.md
- ✅ 文档有 provenance 元数据：GMP_COMPLIANCE_MATRIX.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：sqllogictest-oracle-gate-report.md
- ✅ 文档有 provenance 元数据：sqllogictest-oracle-gate-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312_DAG_ANALYSIS.md
- ✅ 文档有 provenance 元数据：V312_DAG_ANALYSIS.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-11-round14-status.md
- ✅ 文档有 provenance 元数据：v312-11-round14-status.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：ARCHITECTURE.md
- ✅ 文档有 provenance 元数据：ARCHITECTURE.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：DEVELOPMENT_PLAN.md
- ✅ 文档有 provenance 元数据：DEVELOPMENT_PLAN.md
- ✅ 无状态声明（无需证据检查）：evidence/sql_corpus/ALL_TARGETS_REPORT.md
- ✅ 文档有 provenance 元数据：evidence/sql_corpus/ALL_TARGETS_REPORT.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/issue-3969-3970-3971/README.md
- ✅ 文档有 provenance 元数据：evidence/issue-3969-3970-3971/README.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/mysql_compat/V312-21-VERIFICATION.md
- ✅ 文档有 provenance 元数据：evidence/mysql_compat/V312-21-VERIFICATION.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/mysql_compat/DEFERRED_FOLLOWUPS.md
- ✅ 文档有 provenance 元数据：evidence/mysql_compat/DEFERRED_FOLLOWUPS.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/mysql_compat/SURFACE_DISPOSITION.md
- ✅ 文档有 provenance 元数据：evidence/mysql_compat/SURFACE_DISPOSITION.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/V312-11-DDL-GIS-JSON-verification.md
- ✅ 文档有 provenance 元数据：evidence/V312-11-DDL-GIS-JSON-verification.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/crash_recovery/V312-14-CRASH-RECOVERY.md
- ✅ 文档有 provenance 元数据：evidence/crash_recovery/V312-14-CRASH-RECOVERY.md
- ✅ 无状态声明（无需证据检查）：evidence/wire_load_data/V312-13-REPORT.md
- ✅ 文档有 provenance 元数据：evidence/wire_load_data/V312-13-REPORT.md
- ✅ 历史快照文件已豁免 PASS/FAIL 逐行检查: evidence/EVIDENCE_BINDING_REPORT.md (env:historical-snapshot 标记)
- ✅ 历史快照文件已豁免 provenance 元数据检查: evidence/EVIDENCE_BINDING_REPORT.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/tpch/V312-12-TPCH-CORRECTNESS.md
- ✅ 文档有 provenance 元数据：evidence/tpch/V312-12-TPCH-CORRECTNESS.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/sqllogictest/smoke-report.md
- ✅ evidence_hash 真实验证通过（log 文件 SHA256）: evidence/sqllogictest/smoke-report.md (docs/releases/v3.12.0/logs/sqllogictest_03537812a_20260811_004524.log)
- ✅ 文档有 provenance 元数据：evidence/sqllogictest/smoke-report.md
- ✅ 历史快照文件已豁免 PASS/FAIL 逐行检查: evidence/sqllogictest/EVIDENCE_BINDING_REPORT.md (env:historical-snapshot 标记)
- ✅ 历史快照文件已豁免 provenance 元数据检查: evidence/sqllogictest/EVIDENCE_BINDING_REPORT.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/sqllogictest/V312-11-VERIFICATION.md
- ✅ 文档有 provenance 元数据：evidence/sqllogictest/V312-11-VERIFICATION.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/sqllogictest/V312-SLT-GATE-ISSUE.md
- ✅ evidence_hash 真实验证通过（log 文件 SHA256）: evidence/sqllogictest/V312-SLT-GATE-ISSUE.md (docs/releases/v3.12.0/logs/sqllogictest_004056a62_20260809_145319.log)
- ✅ 文档有 provenance 元数据：evidence/sqllogictest/V312-SLT-GATE-ISSUE.md
- ✅ 无状态声明（无需证据检查）：evidence/arch_invariants/R2_INVARIANTS_REPORT.md
- ✅ 文档有 provenance 元数据：evidence/arch_invariants/R2_INVARIANTS_REPORT.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/REVIEWER_SIGNOFF_V312-19_SLICE3.md
- ✅ 文档有 provenance 元数据：evidence/REVIEWER_SIGNOFF_V312-19_SLICE3.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：evidence/V312-32-anti-fab-doc-remediation-ISSUE.md
- ✅ 文档有 provenance 元数据：evidence/V312-32-anti-fab-doc-remediation-ISSUE.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-09-backup-restore-report.md
- ✅ 文档有 provenance 元数据：v312-09-backup-restore-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：CHANGELOG.md
- ✅ 文档有 provenance 元数据：CHANGELOG.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：window-gis-json-feature-delivery-report.md
- ✅ 文档有 provenance 元数据：window-gis-json-feature-delivery-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：test-infrastructure-activation-report.md
- ✅ 文档有 provenance 元数据：test-infrastructure-activation-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-27_anti_fab_fix_report.md
- ✅ 文档有 provenance 元数据：V312-27_anti_fab_fix_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-06-graph-projection-report.md
- ✅ 文档有 provenance 元数据：v312-06-graph-projection-report.md
- ✅ 无状态声明（无需证据检查）：disabled-test-registry.md
- ✅ 文档有 provenance 元数据：disabled-test-registry.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-22-round12-status.md
- ✅ 文档有 provenance 元数据：v312-22-round12-status.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-26_warn_only_fix_report.md
- ✅ 文档有 provenance 元数据：V312-26_warn_only_fix_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：sql-corpus-invariant-reviewer-gate-report.md
- ✅ 文档有 provenance 元数据：sql-corpus-invariant-reviewer-gate-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：VERSION_PLAN.md
- ✅ 文档有 provenance 元数据：VERSION_PLAN.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-18-baseline-status.md
- ✅ 文档有 provenance 元数据：v312-18-baseline-status.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：arch-invariant-report.md
- ✅ 文档有 provenance 元数据：arch-invariant-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312_verification_report.md
- ✅ 文档有 provenance 元数据：v312_verification_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-30_reconciliation_report.md
- ✅ 文档有 provenance 元数据：V312-30_reconciliation_report.md
- ✅ 无状态声明（无需证据检查）：README.md
- ✅ 文档有 provenance 元数据：README.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：TEST_PLAN.md
- ✅ 文档有 provenance 元数据：TEST_PLAN.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：execution-architecture-debt-report.md
- ✅ 文档有 provenance 元数据：execution-architecture-debt-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-30_stage_transition_report.md
- ✅ 文档有 provenance 元数据：V312-30_stage_transition_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：REVIEWER_SIGN_OFF.md
- ✅ 文档有 provenance 元数据：REVIEWER_SIGN_OFF.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312_issue_comments.md
- ✅ evidence_hash 真实验证通过（git commit object）: v312_issue_comments.md (bbb3dacb5078636c7c526004d2f05de0c6d13dda)
- ✅ 文档有 provenance 元数据：v312_issue_comments.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：DRAFT_ASSESSMENT_AND_ALPHA_GATE.md
- ✅ 文档有 provenance 元数据：DRAFT_ASSESSMENT_AND_ALPHA_GATE.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-02-gmp-schema-report.md
- ✅ 文档有 provenance 元数据：v312-02-gmp-schema-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：crash-recovery-upgrade-verification-report.md
- ✅ 文档有 provenance 元数据：crash-recovery-upgrade-verification-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-03-gmp-ingestion-report.md
- ✅ 文档有 provenance 元数据：v312-03-gmp-ingestion-report.md
- ✅ 无状态声明（无需证据检查）：sequence-executor-gap-assessment.md
- ✅ 文档有 provenance 元数据：sequence-executor-gap-assessment.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：ISSUES_PLAN.md
- ✅ 文档有 provenance 元数据：ISSUES_PLAN.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：sqllogictest-baseline/smoke-report.md
- ✅ evidence_hash 真实验证通过（log 文件 SHA256）: sqllogictest-baseline/smoke-report.md (docs/releases/v3.12.0/logs/sqllogictest_004056a62_20260809_145319.log)
- ✅ 文档有 provenance 元数据：sqllogictest-baseline/smoke-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：sqllogictest-baseline/V312-11-VERIFICATION.md
- ✅ 文档有 provenance 元数据：sqllogictest-baseline/V312-11-VERIFICATION.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：RELEASE_NOTES.md
- ✅ 文档有 provenance 元数据：RELEASE_NOTES.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-25_e2e_retire_report.md
- ✅ 文档有 provenance 元数据：V312-25_e2e_retire_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-30_signoff_report.md
- ✅ 文档有 provenance 元数据：V312-30_signoff_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-24_test_infra_activation_report.md
- ✅ 文档有 provenance 元数据：V312-24_test_infra_activation_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：storage-index-wal-backlog-report.md
- ✅ 文档有 provenance 元数据：storage-index-wal-backlog-report.md
- ✅ 无状态声明（无需证据检查）：compliance-audit-access-control-report.md
- ✅ 文档有 provenance 元数据：compliance-audit-access-control-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-08-compliance-audit-report.md
- ✅ 文档有 provenance 元数据：v312-08-compliance-audit-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-04-embedding-provider-report.md
- ✅ 文档有 provenance 元数据：v312-04-embedding-provider-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：wire-e2e-report.md
- ✅ 文档有 provenance 元数据：wire-e2e-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-round17-scope-table.md
- ✅ 文档有 provenance 元数据：v312-round17-scope-table.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-17-round16-status.md
- ✅ 文档有 provenance 元数据：v312-17-round16-status.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：load-data-report.md
- ✅ 文档有 provenance 元数据：load-data-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：MYSQL_COMPAT_STATUS.md
- ✅ 文档有 provenance 元数据：MYSQL_COMPAT_STATUS.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-29_gate_wiring_report.md
- ✅ 文档有 provenance 元数据：V312-29_gate_wiring_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-07-rag-evidence-bundle-report.md
- ✅ 文档有 provenance 元数据：v312-07-rag-evidence-bundle-report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：BLOCKER_DISPOSITION_V311.md
- ✅ 文档有 provenance 元数据：BLOCKER_DISPOSITION_V311.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：V312-28_corpus_activation_report.md
- ✅ 文档有 provenance 元数据：V312-28_corpus_activation_report.md
- ✅ Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：v312-05-hybrid-retrieval-report.md
- ✅ 文档有 provenance 元数据：v312-05-hybrid-retrieval-report.md

---

## 未验证声明（UNVERIFIED CLAIMS）

✅ 无未验证声明

---

## Anti-Fabrication Policy 合规状态

| 要求 | 状态 |
|------|------|
| PASS/FAIL 声明绑定 CI 证据 | ✅ 合规 |
| 门禁结果绑定 gate_policy_eval_id | ✅ 合规 |
| 计划文档无 GA Final 伪造 | ✅ 合规 |
| provenance 元数据存在 | ⚠️  部分缺失 |

---

## 后续行动

✅ 无需修复，所有检查通过

### 合规确认

- 所有 PASS/FAIL 声明有 CI 证据绑定
- 所有门禁结果有 gate_policy_eval_id
- 无计划文档状态伪造
- provenance 元数据存在


---

*报告生成时间: 2026-08-11 01:13:41*
*检查工具版本: check_evidence_binding.sh v1.0.0*
*依据政策: Anti-Fabrication Policy v1.0.0*

---

## 参考：违规类型定义

| 类型 | 定义 | 严重程度 |
|------|------|----------|
| **Type A** | 虚构执行：AI 声称测试通过但无 CI 日志支撑 | P0 |
| **Type B** | 伪门禁：AI 生成门禁通过但无 gate engine 输出 | P0 |
| **Type C** | 伪证据：AI 引用不存在的 CI run / log hash | P1 |
| **Type D** | 伪任务完成：AI 标记任务完成但代码未合并 | P1 |
