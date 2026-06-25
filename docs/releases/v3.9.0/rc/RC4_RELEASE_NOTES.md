# v3.9.0-rc4 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc4`

## What's New Since rc3

### Engineering

- **INT-2 substance**: ParallelExecutor 实质性测试 (PR #3357)
- **Z6G4 QPS baseline**: TPC-H SF=0.01 性能基线 (PR #3359)
- **Soak script 兼容性**: run_24h_soak.sh v3.9.0 兼容修复 (PR #3358)
- **Long-run unignore**: 10个稳定性测试从 ignore 状态移除 (PR #3351)

## All RC4 P0/P1 Blockers Closed

| # | Issue | Status |
|---|-------|--------|
| #3224 | Z6G4 perf measurement + QPS baseline | ✅ Closed (PR #3359) |
| #3228 | Unignore 10 long_run_stability | ✅ Closed (PR #3351) |

## Next: RC5

RC5 focuses on: INT-2/INT-3 full substance tests, cross-version upgrade chain, IS NULL pushdown
