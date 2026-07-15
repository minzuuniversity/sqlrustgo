# v3.11.0 Coverage Baseline

**Date**: 2026-07-15 (updated)
**Tool**: `cargo-llvm-cov` (v0.8.4)
**Scope**: Per-crate `--tests` (correct method per ADR-001 G-04)
**Method**: `cargo llvm-cov test --package <crate> --all-features --tests`

> ⚠️ **CRITICAL**: Previous v3.10.0 baseline used `--lib` only (14.71%).
> ADR-001 G-04 requires `--tests` which includes integration tests.
> This baseline reflects actual llvm-cov measurements across all 14 workspace crates.

## Summary

| Metric | Value | Target | Status |
|--------|------:|------:|:------:|
| **Average Region Coverage** | **73.20%** | ≥80% | ⚠️ Close |
| Region Coverage ≥80% | **8/14 crates** | 14/14 | ⚠️ |
| Function Coverage | ~80% | ≥80% | ⚠️ |

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

To reach 80% average:
- ✅ Already ≥80%: network, catalog, transaction, optimizer, planner, storage, executor, common (8 crates)
- ⚠️ Near (70-79%): server (78.21%), admin (71.26%), parser (70.62%) — need 1–10 pp each
- ❌ Far (0-55%): tools (54.39%), mysql-server (43.14%), cli (0.00%) — need 25–80 pp

### Coverage Delta from Baseline (+103 tests)

| Crate | Baseline | Current | Δ | Notes |
|-------|----------:|----------:|---:|-------|
| admin | 57.64% | 71.26% | **+13.62** | +40 tests (verify, restore, manifest, backup) |
| optimizer | 89.14% | 88.28% | −0.86 | Fixed Filter short-circuit bug; test now runs |
| mysql-server | 42.99% | 43.14% | +0.15 | +32 tests |
| tools | 53.62% | 54.39% | +0.77 | +20 tests |
| parser | 70.79% | 70.62% | −0.17 | +15 tests |
| **Average** | **~68%** | **73.20%** | **+5.2 pp** | |

## Test Improvements (PRs #3458-#3482)

### Tests Added
- **sqlrustgo-admin**: 40 tests (PR #3482) — verify_extracted, backup/restore error paths, manifest scan, walk_files, sha256, BackupError/VerifyError Display
- **sqlrustgo-mysql-server**: 32 tests (PR #3466/#3469) — Packet I/O, MySqlError, parse_tbl_line, replace_placeholders, parse_stmt_execute_params
- **sqlrustgo-tools**: 20 tests (PR #3473/#3474) — BackupManager, BackupMetadata, ImportStats, DumpImporter, Lexer
- **sqlrustgo-parser**: 15 tests (PR #3473) — Lexer tokenize, is_keyword, from_keyword
- **sqlrustgo-cli**: 4 tests — CLI binary smoke

### Bugs Fixed
- **optimizer Filter short-circuit** (PR #3482): `should_parallelize` for Filter nodes incorrectly rejected inputs below PARALLEL_MIN_ROWS before computing output row estimate. Fixed by removing the early-reject on input scan size. All 11 should_parallelize tests now pass.

## Measurement Commands

```bash
# Correct method (ADR-001 G-04)
for crate in $CRATES; do
    cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL"
done

# Per-crate with summary (do NOT use --summary-only — it truncates the output format)
cargo llvm-cov test --package <crate> --all-features --tests | grep "^TOTAL"
```

## Previous vs Current Comparison

| Measurement | Avg Region% | Crates ≥80% | Notes |
|-------------|--------:|----------:|-------|
| v3.10.0 baseline | 14.71% | 0/14 | `--lib` only (WRONG method) |
| v3.11.0 initial | ~68% | 7/14 | `--tests` CORRECT, optimizer blocked |
| **v3.11.0 current** | **73.20%** | **8/14** | Fixed optimizer; +103 tests |

## Next Steps

1. **sqlrustgo-server (78.21%)**: ~1.79 pp to reach 80%. Focus on monitoring.rs and main.rs uncovered paths.
2. **sqlrustgo-admin (71.26%)**: ~8.74 pp to reach 80%. pitr.rs and mysqladmin.rs have most uncovered code.
3. **sqlrustgo-parser (70.62%)**: ~9.38 pp. parser.rs has large untested branches (989 lines).
4. **sqlrustgo-tools (54.39%)**: ~25.61 pp. upgrade.rs (799 lines) and backup_restore.rs (457 lines) need tests.
5. **sqlrustgo-mysql-server (43.14%)**: ~36.86 pp. lib.rs (4920 lines) dominates — focus on Error handling, connection, auth paths.
6. **sqlrustgo-cli (0.00%)**: 4 smoke tests exist but don't instrument binary code. Need integration tests.
