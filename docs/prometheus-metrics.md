# Prometheus Metrics Export

## Overview

Expose sqlrustgo operational metrics via Prometheus-compatible `/metrics` endpoint.

## Usage

```bash
sqlrustgo-cli serve --metrics-addr :9090
curl http://localhost:9090/metrics
```

## Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `sqlrustgo_qps_total` | Counter | Total queries executed |
| `sqlrustgo_query_duration_seconds` | Histogram | Query latency distribution |
| `sqlrustgo_connections_active` | Gauge | Active connections |
| `sqlrustgo_buffer_pool_hit_ratio` | Gauge | Buffer pool hit ratio |

## Implementation

Requires:
- `InstrumentationHook` trait (V311-06)
- HTTP server integration
- Prometheus text format encoding
