# sqlrustgo-perf-baseline

Performance baseline and governance system for SQLRustGo.

## Overview

This crate provides a performance tracking and regression detection system that:

1. **Saves benchmark results** to a local JSON store with commit SHA tracking
2. **Queries historical metrics** over configurable time windows
3. **Detects regressions** by comparing two commits with configurable thresholds

## Usage

```bash
# Save current benchmark results
perf-baseline save [commit] [db_path]

# Query historical metrics
perf-baseline query <metric> [days] [db_path]

# Check for regressions between two commits
perf-baseline check <baseline> <head> [threshold] [db_path]
```

## Integration with CI/Gate

Add to gate scripts:

```bash
# After running benchmarks, save results
perf-baseline save $(git rev-parse HEAD)

# Before PR merge, check for regressions
perf-baseline check HEAD~1 HEAD 0.10
# Exit code 1 if any metric regressed > 10%
```

## Metrics Tracked

- `qps_update` - UPDATE QPS
- `qps_delete` - DELETE QPS
- `qps_insert` - INSERT QPS
- `qps_select` - Point SELECT QPS
- `qps_complex_where` - Complex WHERE query QPS
- `qps_join` - Hash JOIN QPS
- `latency_p50` - P50 latency (ms)
- `latency_p99` - P99 latency (ms)
- `memory_mb` - Memory usage (MB)

## Data Storage

Default location: `~/.local/share/sqlrustgo/perf_baseline/`

Files:
- `benchmarks.json` - All benchmark results by commit
- `metrics_*.json` - Per-metric time series data
