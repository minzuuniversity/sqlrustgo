# v4.0.0 server-read-perf SOAK — 60min

**Date**: 2026-11-11T22:51:56Z
**Duration**: 60min (3600s)
**Port**: 3412  **Threads**: 16  **Server threads**: 16
**Storage**: file  **Rows**: 10000
**Workload**: 80% reads (incl. point/range/agg) / 20% writes (INSERT 10% + UPDATE)
**Data dir**: /tmp/sqlrustgo-v400-soak-ab003
**Source commit**: aec1a3673cb83d8800b27e8a7aa11f690f346817

## Verdict

| Metric | Value |
|--------|-------|
| Server alive at end | 1 |
| Trigger panics | 00 |
| Driver queries done | 13260 |
| Driver errors | 461 |
| Server send_result_set done | 00 |
| High-CPU episodes (>80%/core sustained >60s) | 0 |
| High-CPU longest streak (s) | 0 |

## Latency stats (driver log)

```json
{
  "total_queries": 13260,
  "total_errors": 461,
  "error_rate_pct": 3.3598,
  "latency_ms": {
    "min": 0.8,
    "p50": 2669.1,
    "p90": 8614.6,
    "p95": 10965.5,
    "p99": 13697.4,
    "max": 14984.2,
    "mean": 3543.434
  },
  "by_kind": {
    "point_update": 1224,
    "insert": 1079,
    "count_agg": 1321,
    "range_scan": 2744,
    "point_select": 6892
  }
}
```

## Files

- `server.log` — full server log
- `driver.log` — per-query CSV (Q,<ms>,<tid>,<kind>,<rows>)
- `metrics.csv` — 15s samplings
- `summary.json` — aggregate stats

PASS — 13260 queries via 16 threads in 60min, 461 errors, 0 high-CPU episodes.
