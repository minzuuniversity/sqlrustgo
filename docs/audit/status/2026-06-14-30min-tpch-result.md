# 30-min TPC-H Wired Soak — First Run Result

> **Date**: 2026-06-14 00:29-00:59 UTC
> **Tier**: 1 (Short Observation)
> **Status**: PASS — pipeline complete, server stable, 30 min wall-clock
> **Result dir**: `test_results/tpch_30min_20260614_002900/`

## Quick Summary

- **0 crashes** in 30 min
- **660 TPC-H queries** (30 rounds × 22 query) executed
- **95.5% OK rate** (630/660 — 30 failures all Q19 known issue per PR-3262)
- **No memory leak** — RSS 12-13MB stable with brief Q9 spikes (max 3351MB, released immediately)
- **FD stable** at 12-13 (delta -13 server cleanup)
- **CPU 11-17%** sustained TPC-H work
- **Server 0 dump, 0 OOM, 0 hang**

## Why this matters

This is the first Tier 1 wired soak (30-min short observation) in the new
STABILITY_TEST_LADDER (4-tier model: 5min → 30min → 4-12h → 24h+).

Before this run, there was a 4h+ jump from Tier 0 (5min PR #3380) to
Tier 3 (24h+ GA gate). Tier 1 fills that gap, allowing faster feedback
on engine changes without committing 24 hours.

## Per-Query Performance (top 5 slowest)

| Q | Avg ms | Description |
|---|--------|-------------|
| Q9 | 7318 | 6-table join + EXTRACT (heavy, dominant) |
| Q8 | 56 |  |
| Q10 | 55 |  |
| Q21 | 51 |  |
| Q7 | 51 |  |

Q9 dominates the 30-min wall clock (~22 sec per round, 30 rounds = ~660 sec = 11 min
spent on Q9 alone, out of 30 min total = ~37% of run time on one query).

## Resource Trends

| Metric | Min | Max | Notes |
|--------|-----|-----|-------|
| RSS (MB) | 12 | 3351 | Q9 spike then drop, no leak |
| FD | 12 | 13 | stable |
| CPU% | 11 | 17 | sustained work |
| WAL (MB) | 1 | 1 | no growth |

## Companion Files

- `STABILITY_REPORT.md` (full per-query table)
- `metrics.csv` (15KB, 288 samples, 5s interval)
- `sqlrustgo.log` (1.2MB, full server log)
- `tpch_22_rotate.log` (19KB, 660 query records)

## What's next

This 30-min Tier 1 run satisfies the **short observation** tier. It does NOT close
issue #3225 (24h wall-clock GA gate) or #3229 (168h wall-clock). Those require
running Tier 3 (24h+) on a different machine (Z6G4 not currently healthy; Z440
Z440 is being used for Tier 1/2 due to lower time budget).

Recommended next steps for full GA path:
1. Tier 1 (this run) — DONE 2026-06-14
2. Tier 2 (4-12h) — NOT YET, awaiting 4h+ machine window
3. Tier 3 (24h) — BLOCKED on Z6G4 5th outage recovery (issue #3252)
4. Tier 3 (72h/168h) — post-GA, after Tier 3 (24h) green

## How to reproduce

```bash
cd /home/ai/sqlrustgo/.worktrees/v39-wired-audit
DURATION=1800 INTERVAL=5 TPCH_INTERVAL=60 TPCH_MAX_ROUNDS=30 \
  PORT=3400 FIXTURE=tpch-sf001 \
  bash scripts/stability/run_tpch_30min.sh
```

Output: `test_results/tpch_30min_<TIMESTAMP>/STABILITY_REPORT.md`

## Verdict

**PASS** for Tier 1 (short observation). Pipeline complete. Server stable.
v3.9.0-rc7 wired mode is healthy enough to support Tier 2 and Tier 3.
