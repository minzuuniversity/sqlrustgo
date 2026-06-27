# P1-2 SPEC: ARCH-1~3 + SEM-1~4 Debt Tracking (D8 Dimension)

> **Issue**: #2880
> **Version**: v3.8.0
> **Branch**: `fix/p1-2-arch-sem-debt`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2880)
> **DAG Node**: N7
> **Status**: COMPLETED (PR pending)

## Background

COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873) §4.2 found 7 OPEN
architecture/semantic debt items lacking tracking:
- ARCH-1: execution_engine.rs 6829 lines (since v3.0.0)
- ARCH-2: dual path mysql-server vs bench-cli (since v2.6.0)
- ARCH-3: VTU partial integration (since v3.5.0)
- SEM-1: ROLLBACK MVCC stub (since v3.0.0)
- SEM-2: SHOW TABLES partial (since v3.7.0)
- SEM-3: ALTER TABLE incomplete (since v3.0.0)
- SEM-4: Coverage measurement difference (since v3.0.0)

5-Principle **P5: 未通过的必须有记录和后续改进** violated (no tracking, no plan).

## Scope

- New gate: `scripts/gate/check_arch_sem_debt.sh` (D8 dimension)
- New plan: `docs/releases/v3.8.0/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` (138h v3.9.0+ plan)
- New test: `tests/arch_sem_debt_gate_test.rs` (8 unit tests)

## Test Coverage (8 tests)

- test_d8_gate_exists
- test_d8_tracks_all_7_items
- test_d8_remediation_plan_exists
- test_d8_remediation_plan_effort
- test_d8_remediation_plan_v390_target
- test_d8_gate_3_exit_codes
- test_d8_gate_runs_correctly
- test_d8_no_p0_missing_in_plan

## Acceptance

- [x] 8 tests (≥5)
- [x] All tests pass (8/8)
- [x] Gate runs successfully (DRIFT mode)
- [x] Remediation plan documents all 7 items
- [x] Plan has 138h total effort + v3.9.0+ target

## 5-Principle Alignment

| Principle | Implementation |
|-----------|----------------|
| P1: 有计划必有实现 | Plan has 7 sections, each with steps + effort + owner |
| P2: 有实现必有测试 | 8 unit tests for the gate |
| P3: 测试必审 | Test that runs the gate and validates exit code |
| P4: 必须集成到门禁 | D8 dim is now part of v3.8.0 8-dim gate (D1-D8) |
| P5: 未过必记 | **7 OPEN items now visible at gate with v3.9.0+ plan** |

## Gate Results (实测)

```
=== D8: Architecture/Semantic Debt Gate (5-Principle P5) ===

  [ARCH-1] status=OPEN (has v3.9.0+ plan) ⚠️
  [ARCH-2] status=OPEN (has v3.9.0+ plan) ⚠️
  [ARCH-3] status=OPEN (has v3.9.0+ plan) ⚠️
  [SEM-1] status=OPEN (has v3.9.0+ plan) ⚠️
  [SEM-2] status=OPEN (has v3.9.0+ plan) ⚠️
  [SEM-3] status=OPEN (has v3.9.0+ plan) ⚠️
  [SEM-4] status=OPEN (has v3.9.0+ plan) ⚠️

⚠️  D8 Arch/Sem Debt: PASS-WITH-DRIFT (7 OPEN with plan)
```

**Result**: 7 OPEN items → DRIFT (exit 0) because they have v3.9.0+ plans.
If plan is removed, gate will FAIL (exit 1).

## Total Effort (v3.9.0+ plan)

| Item | Effort | Severity |
|------|--------|----------|
| ARCH-1 execution_engine.rs split | 20h | P0 |
| ARCH-2 dual path consolidation | 24h | P1 |
| ARCH-3 VTU complete integration | 24h | P1 |
| SEM-1 ROLLBACK MVCC | 28h | P0 |
| SEM-2 SHOW TABLES multi-schema | 10h | P2 |
| SEM-3 ALTER TABLE complete | 20h | P1 |
| SEM-4 Coverage methodology | 12h | P1 |
| **Total** | **138h** | (~17 working days, 3-person team) |

## v3.8.0 8-Dimension Gate System

After this change, v3.8.0 has 8 dimensions:
```
D1-D5: check_rc_ga_gate.sh (existing 5-dim)
D6:    check_test_inventory.sh (P0-1)
D7:    check_int_debt.sh (P1-1)
D8:    check_arch_sem_debt.sh (P1-2, NEW)
```

## References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (P1-2)
- openspec/changes/p1-2-arch-sem-debt
- COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873)
- INT5_PLUS_DEBT_INVENTORY.md
- DAG Node N7 (depends on N6, P1-1)
