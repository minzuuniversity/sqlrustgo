# P1-5 SPEC: PR Template + D9 Full Gate Verification

> **Issue**: #2883
> **Version**: v3.8.0
> **Branch**: `fix/p1-5-pr-template-v2`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2883)
> **DAG Node**: N10
> **Status**: COMPLETED (PR pending)

## Background

COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873) §3.4 noted P1-5: PR 模板
强制要求测试+门禁 (5-类文档 + gate integration). 5-Principle P3, P4
violated (no PR template forces documentation).

## Scope

### New Files
1. **`.gitea/pull_request_template.md`** (2.7K) — PR body template
2. **`scripts/gate/check_full_gate_verification.sh`** (5.5K) — D9 dimension
3. **`tests/pr_template_and_full_gate_test.rs`** (11 tests)
4. **`docs/releases/v3.8.0/P15_PR_TEMPLATE_SPEC.md`** (this file)

## Test Coverage (11 tests)

- test_pr_template_exists
- test_pr_template_has_5_docs
- test_pr_template_has_5_principles
- test_pr_template_has_8_dimensions
- test_pr_template_has_issue_ref
- test_d9_gate_exists
- test_d9_gate_executable
- test_d9_gate_runs
- test_d9_gate_8_dimensions
- test_d9_gate_3_exit_codes

## Acceptance

- [x] 11 tests (≥5)
- [x] All tests pass (11/11)
- [x] PR template has 5-类文档 + 5-原则 + 8-dim
- [x] D9 gate script created
- [x] D9 references all 8 dimensions

## 5-原则 Alignment

| 原则 | Implementation |
|-----------|----------------|
| P1: 有计划必有实现 | PR template forces 5-类文档 |
| P2: 有实现必有测试 | 11 unit tests for template + D9 |
| P3: 测试必审 | Test that checks PR template has 5-类文档 |
| P4: 必须集成到门禁 | D9 is new gate dimension; PR template has 8-dim checkboxes |
| P5: 未过必记 | D9 generates evidence JSON (d9_full_gate.json) |

## v3.8.0 9-Dimension Gate System

After this change, v3.8.0 has 9 dimensions:
```
D1-D5: check_rc_ga_gate.sh (existing 5-dim)
D6:    check_test_inventory.sh (P0-1)
D7:    check_int_debt.sh (P1-1)
D8:    check_arch_sem_debt.sh (P1-2)
D9:    check_full_gate_verification.sh (P1-5, NEW — orchestrator)
```

D9 runs D1-D8 + Test Plan Consistency + PR Template + Evidence Generation.

## References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (P1-5)
- openspec/changes/p1-5-pr-template
- COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873)
- INT5_PLUS_DEBT_INVENTORY.md
- DAG Node N10 (depends on D6/D7/D8 being in place)
