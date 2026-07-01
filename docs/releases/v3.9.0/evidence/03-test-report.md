<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

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
