# P2-4 SPEC: R-Gate YAML Upgrade
<!-- env:blocked:no-ci -->
<!-- gate_policy_eval_id: P24-20260603-no-ci -->

> **Issue**: #2888
> **Version**: v3.8.0
> **Branch**: `fix/p2-4-r-gate-yaml-upgrade`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2888)
> **DAG Node**: N15
> **Status**: COMPLETED (PR pending)

## Background

COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873) found that
`.github/workflows/r-gate.yml` is v2.9.0-era:
- Targets `develop/v2.9.0` (stale)
- Calls 7 deprecated gate scripts (check_coverage.sh, check_security.sh, etc.)
- Has no test inventory check
- No cross-version debt gate

The v3.8.0 modern gate is `scripts/gate/check_rc_ga_gate.sh` with 5 dimensions
(D1-Alpha, D2-Beta, D3-SGL, D4-WAL, D5-DeepSeek) + `check_alpha_v380.sh` for
Alpha.

## Scope

- Replace `.github/workflows/r-gate.yml` with v3.8.0-aware workflow
- Target `develop/v3.8.0` + `main` (drop v2.9.0)
- Invoke `check_alpha_v380.sh` for Alpha
- Invoke `check_rc_ga_gate.sh` for RC + GA (5-dim)
- Add **Test Inventory Audit** job (5-Principle P4 enforcement)
- Add **Cross-Version Debt** job (5-Principle P5 enforcement)
- Make r-gate a **required status check** for merge

## 5-Principle Alignment

| Principle | Implementation |
|-----------|----------------|
| P1: 有计划必有实现 | All 5 dimensions have concrete scripts |
| P2: 有实现必有测试 | Test inventory audit counts tests |
| P3: 测试必审 | YAML parses structurally + content checks |
| P4: 必须集成到门禁 | Test integration ratio check (≥95% target) |
| P5: 未过必记 | Cross-version debt gate (0 ACTIVE required) |

## Test Coverage (10 tests)

- test_rgate_targets_v380 (v3.8.0 ✓, v2.9.0 ✗)
- test_rgate_invokes_modern_scripts
- test_rgate_no_deprecated_scripts (7 deprecated)
- test_rgate_has_5_dim_integration (D1-D5)
- test_rgate_has_test_inventory_check
- test_rgate_has_cross_version_debt_check
- test_rgate_has_required_status_check
- test_rgate_yaml_parses
- test_yaml_lints_clean
- test_existing_scripts_still_exist

## Acceptance

- [x] 10 tests (≥5)
- [x] All tests pass (10/10)
- [x] YAML syntactically valid (yaml.safe_load)
- [x] INT5 inventory updated (P2-4 → CLOSED)
- [ ] gate P2-4 CLOSED (after PR merge)

## Limitations

- Requires Gitea CI to use this workflow (current .gitea/ not changed)
- Test inventory check is best-effort grep (could miss tests called differently)
- Cross-version debt check uses `check_cross_version_debt.sh` (currently 4 ACTIVE)
  → This r-gate WILL FAIL until #2879 (P1-1) is completed. **This is intentional
  and the point of the audit** — make debt visible at gate level.

## References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (P2-4)
- openspec/changes/p2-4-r-gate-yaml-upgrade
- COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873)
- INT5_PLUS_DEBT_INVENTORY.md
- DAG Node N15 (depends on N5)
