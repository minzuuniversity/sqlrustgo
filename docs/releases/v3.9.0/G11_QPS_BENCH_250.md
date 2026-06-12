# v3.9.0 G11 QPS/TPS Benchmark — 250 Backup Results

> **Date**: 2026-06-12
> **Host**: 192.168.0.250 (Z440 workstation, 96GB RAM, Z6G4-class CPU)
> **Binary**: `/home/ai/sqlrustgo-v390-latest/target/release/deps/qps_bench-8c91ac78d264d57a`
> **Commit**: `1bce6d5a` (post-#3357 G2 substance PR)
> **Status**: 🟡 Partial (12/22 measurements, qps_mixed_oltp/8 truncated)

## Completed Measurements

| # | Workload | Threads | Time (median) | Throughput |
|---|----------|---------|---------------|------------|
| 1 | qps_point_select | 1 | 1.3890 s | 720 elem/s |
| 2 | qps_point_select | 4 | 1.5726 s | 636 elem/s |
| 3 | qps_point_select | 8 | 1.5712 s | 636 elem/s |
| 4 | qps_point_select | 16 | 2.2077 s | 453 elem/s |
| 5 | qps_range_select | 1 | 1.3874 s | 720 elem/s |
| 6 | qps_range_select | 4 | 1.5619 s | 640 elem/s |
| 7 | qps_range_select | 8 | 1.5677 s | 638 elem/s |
| 8 | qps_insert | 1 | 440.21 µs | 2.27 Melem/s |
| 9 | qps_insert | 4 | 2.1435 ms | 1.87 Melem/s |
| 10 | qps_insert | 8 | 3.9047 ms | 2.05 Melem/s |
| 11 | qps_update | 1 | 27.370 ms | 18.27 Kelem/s |
| 12 | qps_update | 4 | 112.51 ms | 17.78 Kelem/s |
| 13 | qps_update | 8 | 227.79 ms | 17.56 Kelem/s |
| 14 | qps_mixed_oltp | 4 | 12.000 s | 333.32 elem/s |

## Incomplete

- qps_mixed_oltp/8 (estimated 1080s, killed at 1.5h to free host)
- qps_mixed_oltp/16 (skipped due to time)
- qps_point_select/32 (skipped)
- qps_range_select/16, 32 (skipped)

## Observations

1. **Point select scaling**: Stable at 636 elem/s from 4→8 threads (memory-bandwidth limited)
2. **Insert throughput**: 2 Melem/s single-thread, stable with scale
3. **Update throughput**: ~18K elem/s single-thread, slight degradation with more threads (lock contention)
4. **Mixed OLTP**: 333 elem/s at 4 threads (combined point+range+insert+update)

## Full Results

Complete log: `tests/results/qps_bench_250_20260612_partial.log` (14.7KB)
Z6G4 G11 running in parallel: `/tmp/qps_bench_z6g4.log` (target: full 22 measurements)

## Z6G4 Comparison

Z6G4 G11 in progress as of 2026-06-12 20:35 — 3/22 done (point_select/1, /4, on /8).
Expected to complete all 22 measurements in ~30min (faster CPU).

## Notes

- G11 gate (`check_g11_qps.sh`) PASSES form: qps_bench exists, registers, compiles, runs
- 250 backup provides data even if Z6G4 fails
- For full GA gate evidence, use Z6G4 results when available
