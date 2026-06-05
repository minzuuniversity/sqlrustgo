# P1-1 SPEC: INT-1~INT-4 Cross-Version Debt Tracking (D7 Dimension)

> **Issue**: #2879
> **Version**: v3.8.0
> **Branch**: `fix/p1-1-int-cross-version-debt`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2879)
> **DAG Node**: N6
> **Status**: COMPLETED (PR pending)

## Background

COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873) found 4 ACTIVE INT debt items
that have persisted for 3-7 versions:
- INT-1: DML 不经过 WAL (since v1.2.0, 7 versions)
- INT-2: ParallelVolcanoExecutor 孤岛 (since v2.6.0, 5 versions)
- INT-3: expr crate 功能孤岛 (since v3.0.0, 3 versions)
- INT-4: mysql-server 未集成 (since v2.6.0, 5 versions)

5-Principle **P5: 未通过的必须有记录和后续改进** violated (existing
check_cross_version_debt.sh only WARNs, not fails).

## Scope

- New gate: `scripts/gate/check_int_debt.sh` (D7 dimension)
- New plan: `docs/releases/v3.8.0/INT_DEBT_REMEDIATION_PLAN.md` (120h v3.9.0+ plan)
- New test: `tests/int_debt_gate_test.rs` (8 unit tests)
- Update INT5 inventory (P1-1 → CLOSED)

## Test Coverage (8 tests)

- test_d7_int_debt_gate_exists
- test_d7_checks_int_1_through_4
- test_d7_fails_on_active_without_plan
- test_d7_remediation_plan_exists
- test_d7_remediation_plan_effort
- test_d7_no_unknown_status
- test_d7_gate_runs_correctly
- test_d7_summary_table

## Acceptance

- [x] 8 tests (≥5)
- [x] All tests pass (8/8)
- [x] Gate runs successfully (DRIFT mode)
- [x] Remediation plan documents all 4 INTs
- [x] Plan has 120h total effort + v3.9.0+ target

## 5-Principle Alignment

| Principle | Implementation |
|-----------|----------------|
| P1: 有计划必有实现 | Plan has 4 sections, each with steps + effort + owner |
| P2: 有实现必有测试 | 8 unit tests for the gate |
| P3: 测试必审 | Test that runs the gate and validates exit code |
| P4: 必须集成到门禁 | D7 dim is now part of v3.8.0 7-dim gate (D1-D7) |
| P5: 未过必记 | **4 ACTIVE items now visible at gate with v3.9.0+ plan** |

## Gate Results (实测)

```
=== D7: Cross-Version INT Debt Gate (5-Principle P5) ===

  [INT-1] status=ACTIVE (has v3.9.0+ plan) ⚠️
  [INT-2] status=ACTIVE (has v3.9.0+ plan) ⚠️
  [INT-3] status=ACTIVE (has v3.9.0+ plan) ⚠️
  [INT-4] status=ACTIVE (has v3.9.0+ plan) ⚠️

=== D7 INT Debt Summary ===
  CLOSED:               0
  ACTIVE:               0
  DEFERRED w/ plan:     4
  DEFERRED w/o plan:    0

⚠️  D7 INT Debt: PASS-WITH-DRIFT (4 deferred with plan)
```

**Result**: 4 ACTIVE items → DRIFT (exit 0) because they have v3.9.0+ plan.
If plan is removed, gate will FAIL (exit 1). This is the desired behavior.

## Total Effort (v3.9.0+ plan)

| Item | Effort |
|------|--------|
| INT-1 DML WAL | 30h |
| INT-2 Parallel executor | 30h |
| INT-3 expr migration | 32h |
| INT-4 mysql-server | 28h |
| **Total** | **120h** (~15 working days, 3-person team) |

## References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (P1-1)
- openspec/changes/p1-1-int-cross-version-debt
- COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873)
- INT5_PLUS_DEBT_INVENTORY.md
- CROSS-VERSION-DEBT.md (source of 4 ACTIVE items)
- DAG Node N6 (no dependencies)
