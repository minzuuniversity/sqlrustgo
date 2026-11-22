# v4.0.0 server-read-perf SOAK — 10080min

**Date**: 2026-11-19T16:15:34Z
**Duration**: 10080min (604800s)
**Port**: 3414  **Threads**: 16  **Server threads**: 16
**Storage**: file  **Rows**: 10000
**Workload**: 80% reads (incl. point/range/agg) / 20% writes (INSERT 10% + UPDATE)
**Data dir**: /tmp/sqlrustgo-v410-168h-20261119-168h-v410-r5
**Source commit**: 64269eaa0848c4d47485d1f45b1b91779502a089

## Verdict

| Metric | Value |
|--------|-------|
| Server alive at end | 0 |
| Load driver alive at end | 0 |
| Load proof (driver actually loaded the server) | PASS |
| Trigger panics | 00 |
| Driver queries done | 1260501 |
| Driver errors | 4 |
| Driver error rate | 0.000 % (max 1 %) |
| Server send_result_set done | 00 |
| High-CPU episodes (>80%/core sustained >60s) | 0 |
| High-CPU longest streak (s) | 0 |
| Host load avg (1min) peak | 39.62 |
| Samples with a busy host (>=1 proc at >=50% CPU) | 2715 / 2715 |

## Latency stats (driver log)

```json
{
  "total_queries": 1260501,
  "total_errors": 4,
  "error_rate_pct": 0.0003,
  "latency_ms": {
    "min": 0.3,
    "p50": 391.9,
    "p90": 1110.3,
    "p95": 1481.9,
    "p99": 2460.5,
    "max": 13688.5,
    "mean": 527.91
  },
  "by_kind": {
    "point_select": 629862,
    "point_update": 126400,
    "range_scan": 252325,
    "insert": 125648,
    "count_agg": 126266
  }
}
```

## Files

- `server.log` — full server log
- `driver.log` — per-query CSV (Q,<ms>,<tid>,<kind>,<rows>)
- `metrics.csv` — 15s samplings
- `summary.json` — aggregate stats

FAIL — server crashed during the soak run.
