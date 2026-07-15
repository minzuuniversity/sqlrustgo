# v3.11.0 Coverage Baseline

**Date**: 2026-07-15
**Tool**: `cargo-llvm-cov` (v0.8.4)
**Scope**: Per-crate `--tests` (correct method per ADR-001 G-04)
**Method**: `cargo llvm-cov test --package <crate> --all-features --tests`

> ⚠️ **CRITICAL**: Previous v3.10.0 baseline used `--lib` only (14.71%).
> ADR-001 G-04 requires `--tests` which includes integration tests.
> **This is the correct measurement — 71.97% average across 14 crates.**

## Summary

| Metric | Value | Target | Status |
|--------|------:|------:|:------:|
| **Average Line Coverage** | **71.97%** | ≥80% | ⚠️ Close |
| Region Coverage | ~70% | ≥80% | ⚠️ |
| Function Coverage | ~75% | ≥80% | ⚠️ |

## Per-Crate Results

| Crate | Line% | Target | Status |
|-------|------:|------:|:------:|
| sqlrustgo-network | 100.00% | ≥80% | ✅ |
| sqlrustgo-catalog | 91.27% | ≥80% | ✅ |
| sqlrustgo-optimizer | 89.14% | ≥80% | ✅ |
| sqlrustgo-transaction | 89.00% | ≥80% | ✅ |
| sqlrustgo-planner | 88.70% | ≥80% | ✅ |
| sqlrustgo-storage | 87.30% | ≥80% | ✅ |
| sqlrustgo-executor | 82.71% | ≥80% | ✅ |
| sqlrustgo-common | 82.44% | ≥80% | ✅ |
| sqlrustgo-server | 78.21% | ≥80% | ⚠️ |
| sqlrustgo-parser | 70.79% | ≥80% | ⚠️ |
| sqlrustgo-admin | 57.64% | ≥80% | ❌ |
| sqlrustgo-tools | 53.62% | ≥80% | ❌ |
| sqlrustgo-mysql-server | 42.99% | ≥80% | ❌ |
| sqlrustgo-cli | 0.00% | ≥80% | ❌ (no tests) |

## Measurement Command

```bash
# Correct method (ADR-001 G-04)
for crate in <list>; do
    cargo llvm-cov test --package "$crate" --all-features --tests \
        2>/dev/null | grep "^TOTAL"
done

# Fallback if --tests produces no output
cargo llvm-cov test --package "$crate" --lib \
    2>/dev/null | grep "^TOTAL"
```

## Gap Analysis

To reach 80% average:
- 9/14 crates already ≥80%
- 3 crates near (70-79%): parser, server → need ~10 pp each
- 2 crates far (42-57%): admin, tools → need ~30 pp each
- 1 crate no tests: cli → needs tests written

## Previous Baseline Comparison

| Measurement | Average | Method |
|-------------|--------:|--------|
| v3.10.0 baseline | **14.71%** | `--lib` only (WRONG) |
| v3.11.0 baseline | **71.97%** | `--tests` (CORRECT) |
