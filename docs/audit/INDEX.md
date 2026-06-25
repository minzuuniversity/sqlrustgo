# Audit 文档索引

> **版本**: v3.9.0
> **最后更新**: 2026-06-24

---

## 概述

本文档收录 SQLRustGo 项目的审计报告和验证文档。

---

## GMP 审计

| 文档 | 说明 |
|------|------|
| [gmp-audit-db-development-plan.md](gmp-audit-db-development-plan.md) | GMP 审计数据库开发计划 |
| [gmp-audit-db-evaluation-report.md](gmp-audit-db-evaluation-report.md) | GMP 审计数据库评估报告 |

---

## v3.8.0 验证

| 文档 | 说明 |
|------|------|
| [V380_VALIDATION_DRIFT_AUDIT_REPORT.md](V380_VALIDATION_DRIFT_AUDIT_REPORT.md) | v3.8.0 验证漂移审计报告 |
| [V380_RECTIFICATION_PLAN_2026-06-03.md](V380_RECTIFICATION_PLAN_2026-06-03.md) | v3.8.0 整改计划 |

---

## 不变式验证

| 文档 | 说明 |
|------|------|
| [tx_invariant_report.md](tx_invariant_report.md) | 事务不变式验证报告 |
| [wal_invariant_report.md](wal_invariant_report.md) | WAL 不变式验证报告 |
| [wal_invariant_report_v380_2026-06-03.md](wal_invariant_report_v380_2026-06-03.md) | v3.8.0 WAL 不变式报告 |

---

## 能力契约

| 文档 | 说明 |
|------|------|
| [capability_contract_report.md](capability_contract_report.md) | 能力契约报告 |

---

## AI Agent 协议

| 文档 | 说明 |
|------|------|
| [AI_AGENT_TASK_CLAIM_PROTOCOL.md](AI_AGENT_TASK_CLAIM_PROTOCOL.md) | AI Agent 任务认领协议 |

---

## v3.9.0 审计 (2026-06)

| 文档 | 说明 |
|------|------|
| [status/2026-06-07-v390-comprehensive-assessment.md](status/2026-06-07-v390-comprehensive-assessment.md) | v3.9.0 综合审计评估 (v1.0) |
| [status/2026-06-06-test-authenticity-analysis-v390.md](status/2026-06-06-test-authenticity-analysis-v390.md) | v3.9.0 测试真实性分析 |
| [status/2026-06-04-tpch-22-launch-report.md](status/2026-06-04-tpch-22-launch-report.md) | TPC-H 22 启动报告 |
| [status/2026-06-05-tpch-22-final-22of22.md](status/2026-06-05-tpch-22-final-22of22.md) | TPC-H 22 22/22 final |
| [status/2026-06-07-tpch-cell-diff-v390.md](status/2026-06-07-tpch-cell-diff-v390.md) | TPC-H cell diff v3.9.0 |
| [status/2026-06-07-tpch-failure-matrix-v390.md](status/2026-06-07-tpch-failure-matrix-v390.md) | TPC-H failure matrix v3.9.0 |
| [status/2026-06-07-tpch-root-cause-board-v390.md](status/2026-06-07-tpch-root-cause-board-v390.md) | TPC-H root cause board |
| [status/2026-06-09-sprint5-merge-report-v390.md](status/2026-06-09-sprint5-merge-report-v390.md) | Sprint 5 merge report |
| [status/2026-06-11-sprint5-final-sync-status.md](status/2026-06-11-sprint5-final-sync-status.md) | Sprint 5 final sync status |
| [status/2026-06-11-sprint5-v15-g1-gate-verified.md](status/2026-06-11-sprint5-v15-g1-gate-verified.md) | Sprint 5 v15 G1 gate verified |
| [status/2026-06-12-v3.9.0-state-snapshot.md](status/2026-06-12-v3.9.0-state-snapshot.md) | v3.9.0 state snapshot |
| [status/2026-06-12-ga-gate-report-correction.md](status/2026-06-12-ga-gate-report-correction.md) | GA gate report correction |
| [status/2026-06-13-v3.9.0-post-3-bug-fix.md](status/2026-06-13-v3.9.0-post-3-bug-fix.md) | v3.9.0 post 3-bug fix |
| [status/2026-06-14-30min-tpch-result.md](status/2026-06-14-30min-tpch-result.md) | 30min TPC-H result |
| [status/2026-06-17-truthfulness-current-state.md](status/2026-06-17-truthfulness-current-state.md) | Truthfulness current state |
| [status/SOAK_WIRED_SHORT_DURATIONS.md](status/SOAK_WIRED_SHORT_DURATIONS.md) | Wired-soak short durations report |
| [status/STABILITY_TEST_LADDER.md](status/STABILITY_TEST_LADDER.md) | Stability test ladder (30m→1h→2h→4h) |

### v3.9.0 Issue 跟踪 (issues/)

| 文档 | 说明 |
|------|------|
| [issues/ISSUE-2740_crash_recovery_unverified.md](issues/ISSUE-2740_crash_recovery_unverified.md) | Crash recovery 验证缺口 |
| [issues/ISSUE-2741_validation_chain_missing.md](issues/ISSUE-2741_validation_chain_missing.md) | Validation chain 缺失 |
| [issues/ISSUE-2742_wal_architecture_clarification.md](issues/ISSUE-2742_wal_architecture_clarification.md) | WAL 架构澄清 |
| [issues/ISSUE-2743_tx_wal_contract_gaps.md](issues/ISSUE-2743_tx_wal_contract_gaps.md) | TX+WAL 契约 gap |
| [issues/ISSUE-2768_tpch_sf01_sf1_real_execution.md](issues/ISSUE-2768_tpch_sf01_sf1_real_execution.md) | TPC-H SF=0.01/SF=1 真实执行 |

### v3.9.0 分析

| 文档 | 说明 |
|------|------|
| [analysis/2026-06-04-tpch-test-design.md](analysis/2026-06-04-tpch-test-design.md) | TPC-H test design 分析 |

---

## 完成报告

| 文档 | 说明 |
|------|------|
| [F09_ACTUAL_COMPLETION_REPORT_2026-06-03.md](F09_ACTUAL_COMPLETION_REPORT_2026-06-03.md) | F09 实际完成报告 |

---

## 审计统计

| 类别 | 文档数 |
|------|--------|
| GMP 审计 | 2 |
| v3.8.0 验证 | 2 |
| 不变式验证 | 3 |
| 能力契约 | 1 |
| AI Agent | 1 |
| v3.9.0 status | 17 |
| v3.9.0 issues | 5 |
| v3.9.0 analysis | 1 |
| 完成报告 | 1 |

---

*本文档由 Hermes Agent 维护*
