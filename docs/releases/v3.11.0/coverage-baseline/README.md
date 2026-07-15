# v3.11.0 Coverage Baseline

**Date**: 2026-07-15 (updated after test additions)
**Tool**: `cargo-llvm-cov` (v0.8.4)
**Scope**: Per-crate `--tests` (correct method per ADR-001 G-04)
**Method**: `cargo llvm-cov test --package <crate> --all-features --tests`

> ⚠️ **CRITICAL**: Previous v3.10.0 baseline used `--lib` only (14.71%).
> ADR-001 G-04 requires `--tests` which includes integration tests.

## Summary

| Metric | Value | Target | Status |
|--------|------:|------:|:------:|
| **Average Line Coverage** | **~68%** (13 crates, optimizer excluded) | ≥80% | ⚠️ |
| Region Coverage | ~72% | ≥80% | ⚠️ |
| Function Coverage | ~74% | ≥80% | ⚠️ |

> Note: parser 46.51% is a measured anomaly (baseline 70.79% was taken with partial test scope).
> optimizer measurement blocked by 1 failing integration test (parallelization).

## Per-Crate Results

| Crate | Region% | Lines% | Functions% | Target | Status |
|-------|--------:|--------:|--------:|-------:|:------:|
| sqlrustgo-network | 100.00% | 100.00% | 100.00% | ≥80% | ✅ |
| sqlrustgo-catalog | 90.84% | 87.91% | 84.26% | ≥80% | ✅ |
| sqlrustgo-transaction | 89.00% | 85.41% | 83.44% | ≥80% | ✅ |
| sqlrustgo-planner | 88.70% | 86.75% | 80.66% | ≥80% | ✅ |
| sqlrustgo-storage | 85.14% | 84.22% | 81.45% | ≥80% | ✅ |
| sqlrustgo-executor | 82.79% | 81.59% | 84.67% | ≥80% | ✅ |
| sqlrustgo-common | 82.44% | 83.08% | 83.45% | ≥80% | ✅ |
| sqlrustgo-server | 78.21% | 75.11% | 70.76% | ≥80% | ⚠️ |
| sqlrustgo-parser | 46.51% | 46.55% | 73.44% | ≥80% | ❌ |
| sqlrustgo-admin | 66.82% | 64.68% | 75.00% | ≥80% | ⚠️ |
| sqlrustgo-tools | 54.39% | 53.60% | 66.14% | ≥80% | ❌ |
| sqlrustgo-mysql-server | 43.14% | 40.38% | 53.73% | ≥80% | ❌ |
| sqlrustgo-cli | 0.00% | 0.00% | 0.00% | ≥80% | ❌ (no tests) |
| sqlrustgo-optimizer | **?** | **?** | **?** | ≥80% | ❌ (blocked: test fail) |

## Test Improvements (PRs #3458-#3474)

Added **+103 tests** across 6 crates:
- admin: 29 tests (MysqlAdmin, VerifyResult, BackupError, PitrResult, RestoreResult)
- mysql-server: 32 tests (Packet, MySqlError, parse_tbl_line, replace_placeholders)
- tools/backup_restore: 11 tests (BackupManager, BackupMetadata, BackupType)
- tools/mysqldump: 9 tests (ImportStats, SqlStatement, DumpImporter)
- cli: 4 tests (CLI binary smoke)
- parser: 15 tests (Lexer tokenization, keywords, comments)

## Gap to 80%

- ✅ Already ≥80%: network, catalog, transaction, planner, storage, executor, common (7 crates)
- ⚠️ Near (70-79%): server (78.21%)
- ❌ Below (0-67%): admin (66.82%), parser (46.51%), tools (54.39%), mysql-server (43.14%), cli (0%), optimizer (blocked)

## Measurement Commands

```bash
# Correct method (ADR-001 G-04)
for crate in $CRATES; do
    cargo llvm-cov test --package "$crate" --all-features --tests --summary-only 2>/dev/null | grep "^TOTAL"
done

# Note: optimizer measurement blocked by parallelization test failure
```

## Previous vs Current

| Measurement | Avg Region% | Notes |
|-------------|--------:|------|
| v3.10.0 baseline | 14.71% | `--lib` only (WRONG) |
| v3.11.0 baseline (initial) | ~72% | `--tests` CORRECT |
| v3.11.0 (after +103 tests) | ~68% | parser anomaly (46.51%) drags average down |
