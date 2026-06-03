# P0-2 SPEC: Cargo.toml Test Path Configuration

> **Issue**: #2875
> **Version**: v3.8.0
> **Branch**: `fix/p0-2-cargo-toml-test-paths`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2875)
> **DAG Node**: N2
> **Status**: COMPLETED (PR pending)

## Background

COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873) reported "6 Cargo.toml test path
errors". Audit verification:
- All 16 existing [[test]] paths are **correct**
- 36 test files use Cargo's default discovery (not explicitly registered)
- No actual path errors — issue is **discoverability**, not correctness

5-Principle P2 (有实现必要有测试) requires explicit registration for IDE,
CI, and gate-level enumeration.

## Scope

- Add 37 explicit `[[test]]` entries to Cargo.toml (36 new + 1 P0-2 self-test)
- Each entry has correct `name` + `path` matching the .rs file
- Self-test verifies all paths exist, no duplicates, e2e subdir aliases
- Cargo compiles successfully

## Test Coverage (7 tests)

- test_cargo_toml_has_test_section
- test_cargo_toml_test_count_52 (53 actual after self-test)
- test_cargo_toml_all_paths_exist
- test_cargo_toml_no_duplicate_test_names
- test_cargo_toml_p02_section_exists
- test_cargo_toml_p02_critical_tests
- test_cargo_toml_e2e_subdir_aliases

## Acceptance

- [x] 7 tests (≥5)
- [x] All tests pass (7/7)
- [x] cargo check passes
- [x] 53 [[test]] entries in Cargo.toml
- [x] All paths verified to exist

## Before vs After

| Metric | Before | After |
|--------|--------|-------|
| Total [[test]] entries | 16 | 53 |
| Explicit test registration | 16/52 (30.8%) | 53/53 (100%) |
| Default-discovered tests | 36 | 0 |

## 5-Principle Alignment

| Principle | Implementation |
|-----------|----------------|
| P1: 有计划必有实现 | Self-test verifies each test entry |
| P2: 有实现必有测试 | All tests now have explicit [[test]] |
| P3: 测试必审 | 7 unit tests verify Cargo.toml correctness |
| P4: 必须集成到门禁 | D6 dim (P0-1) now sees all 53 entries |
| P5: 未过必记 | cargo check fails fast on path errors |

## References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (P0-2)
- openspec/changes/p0-2-cargo-toml-test-paths
- COMPREHENSIVE_FEATURE_TRACKING.md (PR-2873)
- INT5_PLUS_DEBT_INVENTORY.md
- DAG Node N2
