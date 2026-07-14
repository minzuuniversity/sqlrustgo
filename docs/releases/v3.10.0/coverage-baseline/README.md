# v3.10.0 Coverage Baseline

**Date**: 2026-07-14
**Tool**: `cargo-llvm-cov` (v0.8.7)
**Scope**: workspace `--lib` (full workspace, one merged run; SQLRustGo main crate has real per-file data, sub-crates share the same topline)
**Status**: ⚠️ **BASELINE — well below 80% target**

## Summary

| Metric | Value | Target | Status |
|--------|------:|------:|:------:|
| Region Coverage | **14.61%** | ≥80% | ❌ |
| Function Coverage | **17.92%** | ≥80% | ❌ |
| Line Coverage | **16.30%** | ≥80% | ❌ |
| Branch Coverage | 0% (not instrumented) | — | — |

## Files

| File | Purpose |
|---|---|
| `summary.txt` | One-line total (legacy text) |
| `report.txt` | Per-file text breakdown (legacy) |
| `sqlrustgo-lib.json` | **JSON for the `sqlrustgo` main crate** (RC gate R6 parser consumes this format) |
| `<crate>-lib.json` (× 43) | Per-crate JSON stubs (workspace sharing; see COVERAGE_DIAGNOSIS.md) |
| `summary.json` | Aggregated workspace list |
| **`COVERAGE_DIAGNOSIS.md`** | **Top-priority remediation plan** (read this for ROI guidance) |

## How to regenerate

```bash
# Install (one-time)
cargo install cargo-llvm-cov

# Workspace-wide baseline (3-4 hours for full per-crate, ~30 min merged)
bash scripts/coverage/llvm_cov_baseline.sh

# Or per-crate (slower but accurate)
for c in $(cargo metadata --format-version=1 --no-deps | \
           python3 -c "import json,sys; d=json.load(sys.stdin); print('\n'.join(p['name'] for p in d['packages'] if any('lib' in t['kind'] for t in p['targets'])))"); do
    cargo llvm-cov -p "$c" --lib --json --output-path "docs/releases/v3.10.0/coverage-baseline/${c}-lib.json"
done
```

The script writes one `<crate>-lib.json` per workspace member and a top-level `summary.json` aggregating `percent_covered` across crates. Exits non-zero if any crate is below target.

## R6 gate behavior

RC gate `check_rc_gate_v3.10.0.sh` R6 (lines 195-215) reads each `*-lib.json` and reads `data[0].summary.percent_covered`. At 16.30%, R6 reports **FAIL** (not WARN) for each crate — `R6_COVERAGE_<crate> = 16.3% (target: ≥ 80%) → FAIL`. The format parses correctly; only the percentage is below threshold.

## Remediation priorities

See **`COVERAGE_DIAGNOSIS.md`** for the full per-file breakdown and ROI-sorted test-writing list.

Top 3 highest-ROI additions:
1. Test `engine_ddl.rs` (627 regions, 0% covered) → +10pp overall
2. Test `engine_select.rs` (4944 regions, 5.58% covered) → +30pp if fully covered
3. Instrument the 20+ sub-crates → +20pp

## Notes

- The current 16.30% reflects a single `cargo llvm-cov --workspace --lib` run; many workspace members share this topline because cargo-llvm-cov emits one merged JSON.
- The 80% target is per the v3.10.0 STAGE_CONFIG.yaml GA threshold.
- For RC (V310-10), the target is the same; 80% per crate is a hard GA gate.
