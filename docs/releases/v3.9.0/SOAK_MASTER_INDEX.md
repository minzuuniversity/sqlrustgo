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

> ⚠️ **2026-06-26 修正**: 本文档中 72h 在 Z6G4 的声明与综合评估不一致。
> 真实状态: 72h soak 于 2026-06-19 在 Z6G4 启动后因网络不稳定中断，未完成。
> 详见 [`SOAK_72H_LIVE_STATUS_2026-06-19.md`](SOAK_72H_LIVE_STATUS_2026-06-19.md)。
# v3.9.0 Soak Master Index (Single-Page Reference)

> **Last update**: 2026-06-26 (corrected — Z6G4 72h soak never completed; prior claim of "RUNNING" was incorrect)
> **Purpose**: Single-page reference for all v3.9.0 wall-clock soak testing.
> Closes the doc gap flagged by Issue #3225 and the
> 14-long-stability-tests analysis in
> [`LONG_STABILITY_TESTS_ANALYSIS.md`](LONG_STABILITY_TESTS_ANALYSIS.md).
> **Index status**: merged (was in PR #3311, now closed)
## Status snapshot (as of 2026-06-26 — corrected)

> ⚠️ **修正说明 (2026-06-26)**: 2026-06-24 版本声称 72h "RUNNING on Z6G4" 是不正确的。
> 2026-06-19 的启动记录只有 4 分钟采样数据，此后 Z6G4 网络不可达，72h soak 从未完成。
> 24h soak 在 Z6G4 从未启动；在 250 上有 843 个采样记录，但未达到完整 24h。

| Soak | Duration | Status | Evidence | Notes |
|------|----------|--------|----------|-------|
| **Short ladder (30m→1h→2h→4h)** | 4h cumulative | ✅ **PASS** | [`LOCAL_SHORT_SOAK_REPORT_2026-06-18.md`](LOCAL_SHORT_SOAK_REPORT_2026-06-18.md) | Compressed-time (1,440×); PR #3533 post-WAL-fix |
| **24h real wall-clock** | 24h | ❌ **INCOMPLETE** | 250: 843 samples before Z6G4 lost; Z6G4: never started | Z6G4 unreachable; 24h soak not completed |
| **72h real wall-clock** | 72h | ❌ **INTERRUPTED** | [`SOAK_72H_LIVE_STATUS_2026-06-19.md`](SOAK_72H_LIVE_STATUS_2026-06-19.md): 4-min sample then Z6G4 unreachable | Started 2026-06-19 13:05 UTC; interrupted; ETA 2026-06-22 never met |
| **168h real wall-clock (GA-final)** | 168h | ⏳ **BLOCKED** | — | Cannot start until 72h completes; Z6G4 unreachable |

## Test ladder methodology

- **Pre-soak validation**: [`scripts/stability/run_short_soak_ladder.sh`](../../../scripts/stability/run_short_soak_ladder.sh) — progressive 30m→1h→2h→4h
- **In-process soak binary**: `sqlrustgo-mysql-server soak` (PR #3465 Sprint 8 Track C, commit `de8b6b2fd`)
  - CLI: `soak --duration <h> --qps <rate> [--output FILE] [--seed N] [--sample-interval-s S] [--rss-warn-mb MB]`
  - Resources monitored: RSS (macOS/Linux), FD count, lock count, p99 latency
  - Leak warning when RSS growth > threshold
  - Graceful shutdown via `signal-hook` (SIGTERM/SIGINT)
  - JSONL time-series + Markdown report (`SOAK_<DURATION>H_REPORT.md`)
- **Guardian watchdog**: PR #3550 — auto-restart on crash, max 5 restarts
- **Wired-soak v1 vs v2**: PR #3549 — switched from sysbench (which used `CREATE DATABASE`, not yet supported) to in-process engine soak

## Acceptance criteria (per Issue #3225)

- [x] 30m wall-clock PASS (post-fix, see LOCAL_SHORT_SOAK_REPORT)
- [x] 1h/2h/4h ladder steps PASS (see LOCAL_SHORT_SOAK_REPORT)
- [ ] **24h real wall-clock** complete, 1 report file
- [ ] **72h real wall-clock** complete, 1 report file
- [ ] 14 long stability tests un-`#[ignore]` + PASS
- [ ] Resource monitor: RSS growth <50MB/24h, FD growth <50, lock <50ms avg
- [ ] 0 crashes (replaces the "0 crashes" claim of the SIMULATED ladder)

The 24h and 72h real wall-clock criteria are tracked by
[Issue #3225](https://192.168.0.250:3000/openclaw/sqlrustgo/issues/3225) and
[Issue #3265](https://192.168.0.250:3000/openclaw/sqlrustgo/issues/3265).
The 168h soak is tracked by
[Issue #3266](https://192.168.0.250:3000/openclaw/sqlrustgo/issues/3266)
and [Issue #3229](https://192.168.0.250:3000/openclaw/sqlrustgo/issues/3229).

## 14 long stability tests (analysis + plan)

See [`LONG_STABILITY_TESTS_ANALYSIS.md`](LONG_STABILITY_TESTS_ANALYSIS.md) for the
detailed breakdown of the 14 long `#[ignore]`-marked tests, plus the 10 QPS
benchmarks, 3 batched-inserts, and 6 v3.8.0 perf tests. The plan:

1. **Phase 1** (this session): run the in-process ladder 30m→4h — **DONE**
2. **Phase 2** (Z6G4): run 24h real wall-clock, capture SOAK_24H_REPORT
3. **Phase 3** (Z6G4): run 72h real wall-clock, capture SOAK_72H_REPORT (in progress)
4. **Phase 4** (Z6G4): run 168h real wall-clock, capture SOAK_168H_REPORT (GA-final gate)
5. **Phase 5**: un-`#[ignore]` the 14 long-stability tests, run them, document

## Resources monitored in every soak

| Resource | Tool | Warning threshold |
|----------|------|-------------------|
| RSS | `ps -o rss` | configurable (default 100MB growth) |
| FD count | `/proc/self/fd` count | >100 = warning |
| Lock contention | trace events | avg >50ms = warning |
| p99 latency | in-process histogram | >1000ms = warning |
| WAL size | stat | >1024MB = checkpoint trigger |
| Disk free | `df` | <10GB = warning |

## Audit and infrastructure references

- [`docs/audit/status/SOAK_WIRED_SHORT_DURATIONS.md`](../../audit/status/SOAK_WIRED_SHORT_DURATIONS.md) — short duration soak audit
- [`docs/audit/status/STABILITY_TEST_LADDER.md`](../../audit/status/STABILITY_TEST_LADDER.md) — 30m→1h→2h→4h ladder test design
- [`docs/audit/status/2026-06-04-tpch-phase2d-status.md`](../../audit/status/2026-06-04-tpch-phase2d-status.md) — TPC-H phase 2D (predecessor of wired-soak)
- [`docs/governance/adr/ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md`](../../governance/adr/ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md) — v3.10 milestone plan (Issue #3302, 4-PR plan)

## Update policy

- This index is regenerated after each new SOAK report file is added
  to `docs/releases/v3.9.0/`.
- Each new `SOAK_*H_REPORT.md` (24h, 72h, 168h when complete) will
  appear in the Status snapshot table with a stable link.
- When 24h/72h/168h soaks complete, the Acceptance criteria table
  in this index should be updated to check the boxes, and a new
  `docs/releases/v3.9.0/SOAK_<DURATION>H_REPORT_<DATE>.md` should
  be added with the full report.

## Maintenance

Last verified against `develop/v3.9.0` HEAD `f6e3d0b00` on
2026-06-24. The 72h real wall-clock soak was started 2026-06-19 13:05 UTC
on Z6G4 (per Issue #3265); when it completes on 2026-06-22 13:05 UTC,
the dispatch_168h script (PR #3559) will fire the 168h GA-final soak.

When the 72h/168h soaks complete, also update
[`V390_EVIDENCE_INDEX.md`](V390_EVIDENCE_INDEX.md) to add the new
soak duration PASS marks.
