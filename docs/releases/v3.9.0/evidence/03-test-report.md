# 03 - Test Report

## Test Coverage

| Category | Count | Status |
|----------|-------|--------|
| Unit tests | ~250+ | ✅ PASS |
| Integration tests | ~50+ | ✅ PASS |
| TPC-H 22/22 wire | 22 | ✅ PASS (G1) |
| TPC-H 22/22 SF=0.01 | 22 | ✅ PASS (G15) |
| Backup/Restore | 51 | ✅ PASS (G6) |
| Crash Matrix | 129 | ✅ PASS (G8) |
| Upgrade | 50+ | ✅ PASS (G9) |
| Substance tests (INT-2/3, G2, Upgrade) | 36 | ✅ PASS |
| **Total verified** | **330+** | **✅ 100% PASS** |

## Test Environment

- **Platform**: Mac mini M2 (dev) + Z6G4 (CI) + Z440 (backup)
- **Rust**: 1.x stable
- **Storage**: MemoryStorage + DiskStorage
- **Network**: MySQL wire protocol on TCP

## Key Test Reports

- [`../GA_GATE_REPORT.md`](../GA_GATE_REPORT.md) — G1-G16 gate summary
- [`../perf/TPC_H SHA256_BASELINE_20260612.md`](../perf/TPC_H_SHA256_BASELINE_20260612.md)
- [`../perf/PERFORMANCE_BASELINE.md`](../perf/PERFORMANCE_BASELINE.md)
- [`../perf/CRASH_TEST_REPORT.md`](../perf/CRASH_TEST_REPORT.md)
- [`../perf/STABILITY_REPORT.md`](../perf/STABILITY_REPORT.md)

## Long-running Soak

- **24h**: 250 backup, port 4498, 1607+ samples (0 errors) — in progress
- **72h**: pending 24h completion
- **168h**: pending 72h completion (GA-final gate)
