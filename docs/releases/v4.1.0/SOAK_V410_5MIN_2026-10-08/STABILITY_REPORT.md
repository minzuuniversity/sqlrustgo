# v4.1.0 server-read-perf SOAK — 5min

**Date**: 2026-10-08T06:42:43Z–06:47:49Z
**Duration**: 5min (300s) — completed
**Source commit**: `b1e9a98ee6bac1e863ab8826c54a41500455b71d` (develop/v4.1.0)
**Binary**: `target/release/sqlrustgo-mysql-server` (`cargo build --release -p sqlrustgo-mysql-server`)
**Port**: 3421  **Threads**: 16 driver + 16 server  **Storage**: file
**Data dir**: `/private/tmp/sqlrustgo-v410-soak` (clean)
**Rows**: 10,000 preloaded
**Workload**: 80% reads (10% COUNT agg / 20% range scan / 50% point select) / 20% writes (10% INSERT + 10% UPDATE)
**WAL sync**: `every` (durable; same as the v4.0.0 baseline)
**Harness**: `scripts/soak/v400_1h_soak.sh` 5min mode + `scripts/soak/v400_soak_driver.py` (pymysql)

## Verdict

| Metric | Value |
|--------|-------|
| Server alive at end | 1 |
| Trigger panics | 00 |
| Driver queries done | 129,017 |
| Driver errors | 00 |
| High-CPU episodes (>80%/core sustained >60s) | 0 |
| Sustained throughput | ~430 QPS |
| **Row-count integrity after restart** | **PASS — 23,050 = 10,000 + 13,050 INSERTs, 0 rows lost** |

## Latency stats (driver log)

```json
{
  "total_queries": 129017,
  "total_errors": 0,
  "error_rate_pct": 0.0,
  "latency_ms": {
    "min": 1.1,
    "p50": 29.6,
    "p90": 67.1,
    "p95": 88.8,
    "p99": 170.7,
    "max": 505.6,
    "mean": 37.192
  },
  "by_kind": {
    "point_select": 64499,
    "range_scan": 25568,
    "insert": 13050,
    "count_agg": 12885,
    "point_update": 13015
  }
}
```

### Per-kind latency (driver.log, ms)

| kind | n | p50 | p99 | max |
|------|--:|----:|----:|----:|
| point_select | 64,499 | 25.1 | 72.4 | 463.0 |
| range_scan | 25,568 | 27.9 | 76.4 | 462.7 |
| count_agg | 12,885 | 27.0 | 76.7 | 428.5 |
| point_update | 13,015 | 53.8 | 211.5 | 457.1 |
| insert | 13,050 | 66.9 | 233.0 | 505.6 |

Writes cost ~2.1–2.7× reads at p50 — the WAL `every` fsync path. Reads are flat across
point/range/aggregate, i.e. the B2.4 in-engine filter + `pending_keys` fast path keep
non-PK reads at PK-lookup cost.

## RSS / resource timeline (15s samples)

| t (s) | RSS (MB) | FD | CPU % | queries |
|------:|--------:|---:|------:|--------:|
| 15 | 97.0 | 28 | 85.7 | 0 |
| 30 | 114.4 | 28 | 67.0 | 8,369 |
| 60 | 122.8 | 28 | 92.2 | 16,407 |
| 91 | 115.5 | 28 | 72.2 | 25,096 |
| 121 | 144.8 | 28 | 185.6 | 39,908 |
| 151 | 191.8 | 28 | 263.3 | 57,847 |
| 181 | 224.0 | 28 | 315.1 | 73,616 |
| 212 | 253.1 | 28 | 341.7 | 89,547 |
| 257 | 247.9 | 28 | 346.0 | 109,339 |
| 288 | 271.0 | 28 | 329.3 | 121,617 |
| 303 | 281.9 | 12 | 0.1 | 129,017 |

- **RSS peak 281.9 MB** — monotonically rising, no GC sawtooth, no jetsam (macOS cap ~1.7 GB).
- **FD constant at 28** for the whole run (12 after shutdown) — no descriptor leak.
- CPU 2.5–3.7 cores sustained at full load on a 10-core host; no runaway spin.

## Comparison vs recorded baselines

Same harness, same 16+16 threads, same 10k rows, same `wal-sync every`:

| Run | Commit | Queries | p50 (ms) | p99 (ms) | max (ms) | RSS peak (MB) | Errors |
|-----|--------|--------:|---------:|---------:|---------:|--------------:|-------:|
| **This run (v4.1.0)** | `b1e9a98ee6` | **129,017** | **29.6** | **170.7** | **505.6** | **281.9** | **0** |
| v4.0.0 2026-09-19 | `1ac8583849` | 118,585 | 32.4 | 171.4 | 768.8 | 636.0 | 0 |
| v4.0.0 2026-09-16 (stale baseline) | `39fb79164a` | 8,190 | 385.0 | 2,601.7 | 3,997.9 | 3,823.5 (last sample) | 0 |

- Throughput **+8.8%** and RSS peak **−55.7%** vs the best v4.0.0 baseline.
- vs the 2026-09-16 baseline that the salvaged artifacts still carry: **15.8×** queries,
  **13.0×** p50, **15.2×** p99, RSS **−92.6%**.
- No single v4.1.0 commit owns this; it is the accumulation of #4912 A1 lockfree forwarding
  (`e8c67639e2`), B2.1–B2.4 storage rewrites, BLK-1/BLK-2 concurrency fixes, and the
  `pending_keys` / column-resolver read-path work.

## Data-integrity check (post-run restart)

Server was stopped, restarted against the same data dir, and re-queried:

```
SELECT COUNT(*) FROM sbtest1;              -> 23050   (10,000 preloaded + 13,050 INSERTs)
SELECT COUNT(*) FROM sbtest1 WHERE id>10000; -> 13050
SELECT SUM(k) FROM sbtest1;                  -> 505344
```

**23,050 == 10,000 + 13,050 exactly.** Every acknowledged INSERT survived the restart.
This closes the data-integrity FAIL recorded in `V400_04_20MIN_SOAK.md` (COUNT(*) returned
1,001 of ~1.67M rows under MVCC GC + PK B+Tree overwrite).

## What this run does NOT prove

- **5 minutes is not 168h.** The 1h and 8h runs were OOM-killed at 42 min / 52 min on v4.0.0;
  the RSS curve here is still monotonically rising at t=300 s, so the long-run memory
  question is unanswered. This is a pre-flight, exactly like the 2026-09-19 run was.
- **BLK-3 (AUTO_INCREMENT id reuse) is not covered** — the driver assigns explicit disjoint ids.
  Still OPEN per `PERFORMANCE_OPTIMIZATION_PLAN.md §10.13`.
- **`WAL_SYNC=batch:N` was not exercised** — this run is the durable `every` path only.
- **Single-host, 10 cores, file storage.** No semi-sync / replication soak (#4937 landed
  2026-10-05 and has never been under load).

## Files

- `server.log` — full server log (`--log-level warn`; 0 ERROR/WARN/panic lines)
- `driver.log.gz` — per-query CSV `Q,<ms>,<tid>,<kind>,<rows>` (129,017 rows), gzipped
- `metrics.csv` — 15s samplings
- `summary.json` — aggregate stats
- `STABILITY_REPORT.md` — this file
