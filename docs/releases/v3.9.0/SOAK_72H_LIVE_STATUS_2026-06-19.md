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

# Z6G4 72h Soak - Live Status (v2)

> **2026-06-19 13:10 UTC** — Real wall-clock 72h soak in progress on Z6G4 (per #3265).

## PIDs

| Process | PID | Started | RSS | FD | CPU | WAL |
|---------|-----|---------|-----|----|----|-----|
| `serve` (port 13306) | **104549** | 13:05:17 | 8 MB | 11 | 0.1% | 0 MB |
| `soak` (in-process) | **104564** | 13:05:17 | 78 MB | 7 | 0.0% | n/a |
| `guardian` (watchdog) | **107680** | 13:09:53 | n/a | n/a | 0.0% | n/a |

## Why v2 (not v1)

- v1 used `sysbench oltp_read_write` against the MySQL wire protocol
- sysbench prepare uses `CREATE DATABASE sbtest`
- sqlrustgo v3.9.0 does not support `CREATE DATABASE` (early DDL gap)
- Result: sysbench died immediately with "Malformed packet" → wasted 4 min
- v2 (PR #3549) uses in-process `soak` subcommand which exercises the engine directly
- v2 (PR #3550) adds `guardian` watchdog for auto-restart on crash (max 5 restarts)

## In-process soak metrics (sampled at 60s)

| elapsed_s | queries_done | queries_failed | p99_latency_ms | rss_mb | leak_warn |
|-----------|--------------|----------------|----------------|--------|-----------|
| 60 | 60 | 0 | 0.0 | 78.3 | false |
| 120 | 120 | 0 | 0.0 | 78.3 | false |
| 180 | 180 | 0 | 0.0 | 78.3 | false |
| 240 | 240 | 0 | 0.0 | 78.3 | false |

## WAL (PR #3533 verification)

- WAL file: `~/sqlrustgo-soak/soak72h_*/data/sqlrustgo.wal`
- Current size: **0 bytes** (4+ min in, server never wrote WAL)
- Threshold: 1024 MB (truncate fires via `wal_checkpoint_thread`)

## Acceptance criteria

- [ ] **72h** wall-clock duration complete (ETA **2026-06-22 13:05:17 UTC**)
- [ ] Server alive for entire duration
- [ ] Soak query success rate ≥ 99.9%
- [ ] RSS growth < 100 MB over 72h
- [ ] FD count stable (no leak)
- [ ] WAL size stays < 1024 MB
- [ ] Disk free stays > 10 GB
- [ ] Guardian restarts ≤ 5

## Monitor commands

```bash
# Latest metrics
ssh z6g4 'cat ~/sqlrustgo-soak/soak72h_*/server_metrics.csv | tail -3'
ssh z6g4 'tail -1 ~/sqlrustgo-soak/soak72h_*/soak.jsonl'

# Process state
ssh z6g4 'ps -p 104549,104564,107680 -o pid,etime,rss,pcpu,cmd'

# Logs
ssh z6g4 'tail -3 ~/sqlrustgo-soak/soak72h_*/guardian.log'
ssh z6g4 'tail -3 ~/sqlrustgo-soak/soak72h_*/soak.log'
ssh z6g4 'tail -3 ~/sqlrustgo-soak/soak72h_*/server.log'
```

## After 72h passes

```bash
# Generate the final report
ssh z6g4 'cat ~/sqlrustgo-soak/soak72h_*/SOAK_72H_REPORT.md'

# Dispatch 168h GA-final soak (#3266)
ssh z6g4 'cd ~/sqlrustgo-soak && HOURS=168 bash run_72h_soak_v2.sh &'
```
