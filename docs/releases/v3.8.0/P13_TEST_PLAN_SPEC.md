# P1-3 SPEC: Integrated Test Plan (Single Source of Truth)

> **Issue**: #2881
> **Version**: v3.8.0
> **Branch**: `fix/p1-3-test-plan`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2881)
> **DAG Node**: N8
> **Status**: COMPLETED (PR pending)

## Background

COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873) §3.2 noted:
- TEST_PLAN.md was Layer-based, partial (12/16 coverage)
- 49/53 tests not in single plan
- No 8-dim gate mapping
- 5-类文档覆盖率: 12/16 TEST_PLAN, 11/16 TEST_DESIGN, 9/16 REVIEW, 8/16 ACCEPTANCE

5-Principle **P3: 测试必审** violated (no single source of truth).

## Scope

### New Documents (3)
1. **TEST_PLAN_INTEGRATED.md** (15K) — what to test
2. **TEST_REVIEW_INTEGRATED.md** (8.5K) — how to test
3. **TEST_ACCEPTANCE_INTEGRATED.md** (5.3K) — pass/fail

### Updates
4. **COMPREHENSIVE_FEATURE_TRACKING.md** — 5-类文档 100% (16/16)
5. **Cargo.toml** — 4 new [[test]] entries for orphan tests
6. **tests/test_plan_integrated_test.rs** — 10 unit tests

## Test Coverage (10 tests)

- test_test_plan_integrated_exists
- test_test_review_integrated_exists
- test_test_acceptance_integrated_exists
- test_test_plan_53_tests
- test_test_plan_8_gate_dimensions
- test_test_plan_4_stages
- test_test_review_audit_complete
- test_test_acceptance_all_pass
- test_cargo_toml_test_entries
- test_no_orphan_tests

## Acceptance

- [x] 10 tests (≥5)
- [x] All tests pass (10/10)
- [x] TEST_PLAN covers 53 tests × 4 stages × 8 dims
- [x] TEST_REVIEW audits 53 tests
- [x] TEST_ACCEPTANCE: 7+ ✅ APPROVED
- [x] 5-类文档 100% (16/16)

## 5-Principle Alignment

| 原则 | 实施 |
|------|------|
| P1: 有计划必有实现 | TEST_PLAN covers 53 tests for 53 [[test]] entries |
| P2: 有实现必有测试 | 10 unit tests for the test plan itself |
| P3: 测试必审 | TEST_REVIEW per-test audit (53 audits) |
| P4: 必须集成到门禁 | Plan references 8-dim gate (D1-D8) |
| P5: 未过必记 | TEST_ACCEPTANCE documents all stages |

## Cargo.toml Updates

Added 4 new [[test]] entries for orphan test files:
- `int_debt_gate_test` (P1-1 self-test)
- `arch_sem_debt_gate_test` (P1-2 self-test)
- `test_plan_integrated_test` (P1-3 self-test)
- `ci_ci_test` (alias for tests/ci/ci_test.rs)
- + 3 P-X self-tests: l3_canonical_binary, wire_protocol_smoke, cross_path_consistency_test

## 5-类文档覆盖率

| 文档 | Before | After |
|------|--------|-------|
| SPEC | 16/16 | 16/16 |
| TEST_PLAN | 12/16 (75%) | **16/16 (100%)** |
| TEST_DESIGN | 11/16 (69%) | **16/16 (100%)** |
| REVIEW | 9/16 (56%) | **16/16 (100%)** |
| ACCEPTANCE | 8/16 (50%) | **16/16 (100%)** |

## References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (P1-3)
- openspec/changes/p1-3-test-plan
- COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873)
- INT5_PLUS_DEBT_INVENTORY.md
- DAG Node N8 (no dependencies)
