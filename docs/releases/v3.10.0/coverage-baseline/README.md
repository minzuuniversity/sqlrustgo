# v3.10.0 Coverage Baseline

**Date**: 2026-07-13
**Tool**: `cargo llvm-cov`
**Scope**: `cargo llvm-cov --lib` (sqlrustgo crate only)

## Summary

| Metric | Value |
|--------|-------|
| Region Coverage | 14.71% |
| Function Coverage | 17.92% |
| Line Coverage | 16.28% |
| Branch Coverage | N/A |

## Notes

- This baseline covers the main `sqlrustgo` crate's lib tests (29 tests, 28 run).
- One test excluded: `test_parallel_100k_cell_match_n1_vs_n4` (soak/benchmark, >60s).
- Sub-crates (20+): coverage not yet instrumented.
- Target: ≥80% per crate (STAGE_CONFIG.yaml).
- This is a starting baseline — coverage will increase as more tests are added.

## Files

- `summary.txt` — one-line total coverage
- `report.txt` — per-file coverage breakdown
- `html/` — HTML coverage report (open in browser)

## Next Steps

- Expand coverage to full workspace: `cargo llvm-cov --workspace --lib`
- Add integration test coverage
- Meet ≥80% per-crate threshold for GA
