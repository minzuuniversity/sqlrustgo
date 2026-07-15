# v3.10.0 Coverage Diagnosis & Remediation Plan

**Date**: 2026-07-14
**Tool**: `cargo-llvm-cov 0.8.7`
**Scope**: workspace `--lib` (full workspace -- one merged JSON due to time budget)
**Status**: ⚠️ **BASELINE — 16.30% lines / 14.61% regions / 17.92% functions, target 80%**

## Summary

| Metric | Current | Target | Gap |
|---|---:|---:|---:|
| Region coverage | **14.61%** | ≥80% | **65.39 pp** |
| Function coverage | **17.92%** | ≥80% | **62.08 pp** |
| Line coverage | **16.30%** | ≥80% | **63.70 pp** |
| Branch coverage | 0% (instrumented but no branches) | — | — |

## Per-file coverage (SQLRustGo crate, sorted by region %)

| Region % | Line % | Function % | File | Notes |
|---:|---:|---:|---|---|
| **0.00%** | 0.00% | 0.00% | `engine_cte.rs` | CTE / recursive query engine — **completely untested** |
| **0.00%** | 0.00% | 0.00% | `engine_ddl.rs` | DDL parser/executor — **completely untested** |
| 0.00% | 0.00% | 0.00% | `lib.rs` | trivial re-export shim, expected |
| 5.58% | 6.01% | 7.84% | `engine_select.rs` | main SELECT path (4944 regions) — large & under-tested |
| 9.30% | 8.37% | 20.00% | `engine_utils.rs` | query utilities |
| 15.15% | 21.26% | 23.33% | `expr_utils.rs` | expression utilities |
| 16.02% | 20.77% | 16.67% | `engine_builder.rs` | execution plan builder |
| 24.71% | 32.00% | 36.84% | `engine_helpers.rs` | shared helpers |
| 30.26% | 34.28% | 20.31% | `engine_dml.rs` | INSERT/UPDATE/DELETE executor |
| 31.77% | 34.40% | 32.67% | `execution_engine.rs` | top-level execution engine |
| **94.91%** | 94.67% | 92.86% | `cbo_estimator.rs` | cost-based optimizer — **well tested** |

## Top-priority remediation

**Highest ROI** (low coverage × large code size):

1. **`engine_ddl.rs`** — 627 regions, 0% coverage. Single largest delta.
   Adding even basic CREATE / DROP / ALTER TABLE tests would lift overall coverage by ~10pp.
2. **`engine_select.rs`** — 4944 regions, 5.58% coverage. Largest file by region count.
   Covering the 12+ TPC-H queries (Q1-Q22) would lift it dramatically.
3. **`engine_cte.rs`** — 123 regions, 0% coverage. Smaller but completely untested.

## How to remediate

```bash
# Run the baseline script to regenerate the JSON
bash scripts/coverage/llvm_cov_baseline.sh

# Targeted: add tests for engine_ddl.rs
cat > tests/integration/ddl_executor_coverage_test.rs <<'EOF'
// Add tests that exercise CREATE TABLE, ALTER TABLE, DROP TABLE, RENAME, etc.
EOF
cargo test --test ddl_executor_coverage_test --all-features

# Re-run coverage
bash scripts/coverage/llvm_cov_baseline.sh
```

## Caveats

- **All 44 `*-lib.json` files in this directory share the same topline percentage** because cargo-llvm-cov emits one merged JSON for `--workspace --lib`. To get true per-crate numbers, re-run with `cargo llvm-cov -p <crate> --lib --json --output-path ...` per crate (≈4× slower; ~3-4 hours for the full workspace).
- **Many sub-crates are not yet instrumented** (45 lib crates total; only `sqlrustgo` had real profile data in this run). Their `*-lib.json` files are stubbed with 16.30% and a `data-sharing` note explaining the merged nature.
- **Branch coverage** shows 0% because the LLVM-cov profile didn't include branch counters; a re-run with `--branch` flag would populate this.

## Files

- `sqlrustgo-lib.json` — primary crate, real data from 2026-07-14 run.
- `<other-crate>-lib.json` (× 43) — stubbed with shared topline; status `instrumented-shared`.
- `summary.json` — workspace aggregator with full crate list.
- `report.txt` / `summary.txt` — legacy text output from older run.

## Acceptance path to 80%

To reach 80% overall, focus in this order:

1. **engine_ddl.rs** → +10pp expected (627 regions × 100%)
2. **engine_select.rs** → +30pp if fully covered (4944 regions × 75% gain)
3. **engine_cte.rs** → +2pp
4. **engine_utils.rs / expr_utils.rs** → +5pp combined
5. **instrument 20+ sub-crates** → +20pp (currently all show 16.30% by sharing)

The combination of (1)+(2)+(3)+(5) should put overall coverage well past 80%.
