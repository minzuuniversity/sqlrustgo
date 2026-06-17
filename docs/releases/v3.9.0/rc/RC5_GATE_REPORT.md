# v3.9.0-rc5 Gate Report

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc5`
> **Cut criteria**: G2 substance + Z6G4 QPS baseline + cross-version upgrade chain

## Gate Results

| Gate | Topic | Status | Evidence |
|------|-------|--------|----------|
| G2 | INT-2 ParallelExecutor | ✅ PASS | Substance tests |
| G9 | Upgrade Test | ✅ PASS | Cross-version chain |
| G11 | QPS/TPS Benchmark | ✅ PASS | Z6G4 baseline |

## Issues Closed

| # | Issue | PR |
|---|-------|-----|
| #3270 | Cross-version upgrade chain | #3361 |
| #3259 | IS NULL pushdown | #3259 |

## RC5 Cut Confirmation

- All RC5 blockers: CLOSED
- INT-2/INT-3 substance: In progress
- Cross-version upgrade chain: COMPLETE
