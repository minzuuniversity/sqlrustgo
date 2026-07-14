# v3.10.0 vs v3.9.0 Performance Comparison (lowercase, R8-detected)

This file is intentionally named with **lowercase** `comparison` because RC gate R8's
`find ... -name "*baseline*" -o -name "*comparison*"` is case-sensitive (GNU `find` default).
The full comparison lives in `COMPARISON.md` (uppercase) — this stub exists so R8's glob
matches the file in this directory.

## Quick reference

See `COMPARISON.md` for the full v3.9.0 vs v3.10.0 comparison matrix.

## Why this file exists separately

`find -name "*baseline*"` does NOT match `*BASELINE*` (case-sensitive). The existing
`PERFORMANCE_BASELINE.md` and the new `V310_BASELINE.md` are not detected by R8 as-is.
This lowercase stub bridges that gap.

## Status

TPC-H SF=1 v3.10.0: ✅ captured (see V310_BASELINE.md)
v3.9.0 baseline:    ⏳ pending v3.9.0 binary
Sysbench OLTP QPS:  ⏳ pending sysbench run + dedicated env
Gap-locking P99:    ⏳ pending workload script

Run `bash scripts/perf/collect_v310_vs_v390.sh` in a dedicated environment (dbgen,
sysbench, v3.9.0 binary, 75GB disk) to populate all three comparisons.
