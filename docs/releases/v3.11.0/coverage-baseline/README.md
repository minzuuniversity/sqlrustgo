# v3.11.0 Coverage Baseline

**Date**: 2026-07-15 (updated)
**Tool**: `cargo-llvm-cov` (v0.8.4)
**Scope**: Per-crate `--tests` (correct method per ADR-001 G-04)
**Method**: `cargo llvm-cov test --package <crate> --all-features --tests`

> ⚠️ **CRITICAL**: Previous v3.10.0 baseline used `--lib` only (14.71%).
> ADR-001 G-04 requires `--tests` which includes integration tests.

## Summary

| Metric | Value | Target | Status |
|--------|------:|------:|:------:|
| **Average Region Coverage** | **73.20%** | ≥80% | ⚠️ |
| Crates ≥80% | **8/14** | 14/14 | ⚠️ |

## Per-Crate Results

| Crate | Region% | Lines% | Functions% | Target | Status |
|-------|--------:|--------:|--------:|-------:|:------:|
| sqlrustgo-network | 100.00% | 100.00% | 100.00% | ≥80% | ✅ |
| sqlrustgo-catalog | 90.84% | 87.91% | 84.26% | ≥80% | ✅ |
| sqlrustgo-transaction | 89.00% | 85.41% | 83.44% | ≥80% | ✅ |
| sqlrustgo-optimizer | 88.28% | 89.10% | 95.65% | ≥80% | ✅ |
| sqlrustgo-planner | 88.70% | 86.75% | 80.66% | ≥80% | ✅ |
| sqlrustgo-storage | 85.14% | 84.22% | 81.45% | ≥80% | ✅ |
| sqlrustgo-executor | 82.79% | 81.59% | 84.67% | ≥80% | ✅ |
| sqlrustgo-common | 82.44% | 83.08% | 83.45% | ≥80% | ✅ |
| sqlrustgo-server | 78.21% | 75.11% | 70.76% | ≥80% | ⚠️ |
| sqlrustgo-admin | 71.26% | 71.80% | 79.35% | ≥80% | ⚠️ |
| sqlrustgo-parser | 70.62% | 71.17% | 73.44% | ≥80% | ⚠️ |
| sqlrustgo-tools | 54.39% | 53.60% | 66.14% | ≥80% | ❌ |
| sqlrustgo-mysql-server | 43.14% | 40.38% | 53.73% | ≥80% | ❌ |
| sqlrustgo-cli | 0.00% | 0.00% | 0.00% | ≥80% | ❌ |

## Gap Analysis

- ✅ Already ≥80%: network, catalog, transaction, optimizer, planner, storage, executor, common (8 crates)
- ⚠️ Near (70-79%): server (78.21%), admin (71.26%), parser (70.62%)
- ❌ Far (0-55%): tools (54.39%), mysql-server (43.14%), cli (0.00%)

## Tests Added

| PR | Crate | Tests | Effect |
|----|-------|-----:|--------|
| #3482 | admin | +40 | admin: 66.82% → 71.26% |
| #3482 | optimizer | bug fix | optimizer: 89.14% → 88.28% (corrected) |
| #3503 | mysql-server | +27 | no measured change (exercised existing paths) |

## Next Steps

| Crate | Current | Target | Gap | Priority |
|-------|--------:|--------:|----------:|:--------:|
| server | 78.21% | 80% | 1.79 pp | 🔴 HIGH |
| admin | 71.26% | 80% | 8.74 pp | 🔴 HIGH |
| parser | 70.62% | 80% | 9.38 pp | 🔴 HIGH |
| tools | 54.39% | 80% | 25.61 pp | 🟡 MEDIUM |
| mysql-server | 43.14% | 80% | 36.86 pp | 🟡 MEDIUM |
| cli | 0.00% | 80% | 80 pp | 🟢 LOW |
